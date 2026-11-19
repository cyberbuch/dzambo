//! Фактури: списък, четене с редове, запис на чернова, номерция.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    models::invoices::{Invoice, InvoiceLine, NewInvoice, NewInvoiceLine},
    schema::{document_series, invoice_lines, invoices},
    StorageError,
};

pub type InvoiceRow = (Invoice, Vec<InvoiceLine>);

pub async fn list(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<Invoice>, StorageError> {
    invoices::table
        .filter(invoices::company_id.eq(company_id))
        .order(invoices::id.desc())
        .select(Invoice::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn by_id(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<InvoiceRow>, StorageError> {
    let Some(header) = invoices::table
        .find(id)
        .select(Invoice::as_select())
        .first(conn)
        .await
        .optional()?
    else {
        return Ok(None);
    };
    let lines = invoice_lines::table
        .filter(invoice_lines::invoice_id.eq(id))
        .order(invoice_lines::id.asc())
        .select(InvoiceLine::as_select())
        .load(conn)
        .await?;
    Ok(Some((header, lines)))
}

/// Вмъква чернова; сумите се преизчисляват от редовете.
#[allow(clippy::too_many_arguments)]
pub async fn insert_draft(
    conn: &mut AsyncPgConnection,
    new: &NewInvoice<'_>,
    lines: &[NewInvoiceLine<'_>],
) -> Result<i64, StorageError> {
    let (net, vat, total) = line_totals(lines);
    let id: i64 = diesel::insert_into(invoices::table)
        .values(new)
        .returning(invoices::id)
        .get_result(conn)
        .await?;
    diesel::sql_query(
        "UPDATE invoices SET net_amount=$2, vat_amount=$3, total_amount=$4 WHERE id=$1",
    )
    .bind::<diesel::sql_types::BigInt, _>(id)
    .bind::<diesel::sql_types::Numeric, _>(net)
    .bind::<diesel::sql_types::Numeric, _>(vat)
    .bind::<diesel::sql_types::Numeric, _>(total)
    .execute(conn)
    .await?;
    for line in lines {
        diesel::insert_into(invoice_lines::table)
            .values(NewInvoiceLine {
                invoice_id: id,
                ..clone_line(line)
            })
            .execute(conn)
            .await?;
    }
    Ok(id)
}

fn clone_line<'a>(l: &NewInvoiceLine<'a>) -> NewInvoiceLine<'a> {
    NewInvoiceLine {
        invoice_id: l.invoice_id,
        code: l.code,
        description: l.description,
        quantity: l.quantity.clone(),
        unit: l.unit,
        unit_price: l.unit_price.clone(),
        vat_rate: l.vat_rate.clone(),
        product_id: l.product_id,
        net_amount: l.net_amount.clone(),
        vat_amount: l.vat_amount.clone(),
        total_amount: l.total_amount.clone(),
    }
}

pub fn line_totals(lines: &[NewInvoiceLine<'_>]) -> (BigDecimal, BigDecimal, BigDecimal) {
    let mut net = BigDecimal::default();
    let mut vat = BigDecimal::default();
    let mut total = BigDecimal::default();
    for l in lines {
        net += &l.net_amount;
        vat += &l.vat_amount;
        total += &l.total_amount;
    }
    (net, vat, total)
}

pub async fn set_status(
    conn: &mut AsyncPgConnection,
    id: i64,
    status: &str,
    entry_id: i64,
) -> Result<(), StorageError> {
    diesel::update(invoices::table.find(id))
        .set((
            invoices::status.eq(status),
            invoices::journal_entry_id.eq(entry_id),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(invoices::table.find(id))
        .execute(conn)
        .await?;
    Ok(())
}

/// Следващ номер по серия: base = max(start-1, най-голям числов номер на серията).
pub async fn next_out_number(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    document_type: &str,
) -> Result<(String, i32), StorageError> {
    let mut pad = 10i32;
    let mut start = 1i64;
    let series = document_series::table
        .filter(document_series::company_id.eq(company_id))
        .order(document_series::id.asc())
        .select((
            document_series::document_types,
            document_series::pad_width,
            document_series::start_number,
        ))
        .load::<(String, i32, i32)>(conn)
        .await?;
    for (types, p, s) in series {
        if types.split(',').any(|t| t.trim() == document_type) {
            pad = p.clamp(1, 10);
            start = s.max(1) as i64;
            break;
        }
    }
    let numbers = invoices::table
        .filter(invoices::company_id.eq(company_id))
        .filter(
            invoices::direction
                .eq("out")
                .or(invoices::document_type.eq("09")),
        )
        .select(invoices::number)
        .load::<String>(conn)
        .await?;
    let max_used = numbers
        .iter()
        .filter_map(|n| n.trim().parse::<i64>().ok())
        .max()
        .unwrap_or(0);
    let next = (start - 1).max(max_used) + 1;
    Ok((format!("{:0width$}", next, width = pad as usize), pad))
}

/// Дали входящ номер се дублира за същия доставчик.
pub async fn incoming_number_taken(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    counterpart_id: i64,
    number: &str,
) -> Result<bool, StorageError> {
    if number.trim().is_empty() || counterpart_id <= 0 {
        return Ok(false);
    }
    let taken: Option<i64> = invoices::table
        .filter(invoices::company_id.eq(company_id))
        .filter(invoices::direction.eq("in"))
        .filter(invoices::counterpart_id.eq(counterpart_id))
        .filter(invoices::number.eq(number.trim()))
        .select(invoices::id)
        .first(conn)
        .await
        .optional()?;
    Ok(taken.is_some())
}

/// Суми по редовете на фактура.
pub async fn sum_of(
    conn: &mut AsyncPgConnection,
    invoice_id: i64,
) -> Result<(BigDecimal, BigDecimal, BigDecimal), StorageError> {
    let lines = invoice_lines::table
        .filter(invoice_lines::invoice_id.eq(invoice_id))
        .select((
            invoice_lines::net_amount,
            invoice_lines::vat_amount,
            invoice_lines::total_amount,
        ))
        .load::<(BigDecimal, BigDecimal, BigDecimal)>(conn)
        .await?;
    let mut net = BigDecimal::default();
    let mut vat = BigDecimal::default();
    let mut total = BigDecimal::default();
    for (n, v, t) in lines {
        net += n;
        vat += v;
        total += t;
    }
    Ok((net, vat, total))
}

pub fn parse_date(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").ok()
}

/// Постнати фактури за счетоводен месец (ДДС дневници).
pub async fn list_posted_by_month(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    direction: &str,
    month: &str,
) -> Result<Vec<Invoice>, StorageError> {
    invoices::table
        .filter(invoices::company_id.eq(company_id))
        .filter(invoices::direction.eq(direction))
        .filter(invoices::status.eq("posted"))
        .filter(invoices::accounting_month.eq(month))
        .order(invoices::issue_date.asc())
        .select(Invoice::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}
