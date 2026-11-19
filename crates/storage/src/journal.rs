//! Дневник: статии и редове.

use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub use crate::models::journal::{JournalEntry, JournalLine, NewJournalEntry, NewJournalLine};

use crate::{
    schema::{journal_entries, journal_lines},
    StorageError,
};

pub type JournalRow = (JournalEntry, Vec<JournalLine>);

pub async fn list(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<JournalEntry>, StorageError> {
    journal_entries::table
        .filter(journal_entries::company_id.eq(company_id))
        .order(journal_entries::id.desc())
        .select(JournalEntry::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn entry_with_lines(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<JournalRow>, StorageError> {
    let Some(entry) = journal_entries::table
        .find(id)
        .select(JournalEntry::as_select())
        .first(conn)
        .await
        .optional()?
    else {
        return Ok(None);
    };
    let lines = journal_lines::table
        .filter(journal_lines::entry_id.eq(id))
        .order(journal_lines::id.asc())
        .select(JournalLine::as_select())
        .load(conn)
        .await?;
    Ok(Some((entry, lines)))
}

pub async fn insert_entry(
    conn: &mut AsyncPgConnection,
    new: &NewJournalEntry,
    lines: &[NewJournalLine],
) -> Result<i64, StorageError> {
    let id: i64 = diesel::insert_into(journal_entries::table)
        .values(new)
        .returning(journal_entries::id)
        .get_result(conn)
        .await?;
    for line in lines {
        diesel::insert_into(journal_lines::table)
            .values(NewJournalLine {
                entry_id: id,
                ..clone_line(line)
            })
            .execute(conn)
            .await?;
    }
    Ok(id)
}

fn clone_line(l: &NewJournalLine) -> NewJournalLine {
    NewJournalLine {
        entry_id: l.entry_id,
        account_id: l.account_id,
        account_number: l.account_number.clone(),
        account_name: l.account_name.clone(),
        direction: l.direction.clone(),
        amount: l.amount.clone(),
        vat_amount: l.vat_amount.clone(),
        currency: l.currency.clone(),
        fx_quantity: l.fx_quantity.clone(),
        fx_rate: l.fx_rate.clone(),
    }
}

/// Трие дневните статии на документ (при unpost/изтриване).
pub async fn delete_by_document(
    conn: &mut AsyncPgConnection,
    document_id: i64,
) -> Result<(), StorageError> {
    diesel::sql_query("DELETE FROM journal_lines WHERE entry_id IN (SELECT id FROM journal_entries WHERE document_id = $1)")
        .bind::<diesel::sql_types::BigInt, _>(document_id)
        .execute(conn)
        .await?;
    diesel::delete(journal_entries::table.filter(journal_entries::document_id.eq(document_id)))
        .execute(conn)
        .await?;
    Ok(())
}

/// ДДС агрегация по статия за дневника: нето/ДДС/общо от редовете.
pub struct VatAgg {
    pub document_number: String,
    pub entry_date: chrono::NaiveDate,
    pub counterpart_id: i64,
    pub counterpart_name: String,
    pub net: BigDecimal,
    pub vat: BigDecimal,
    pub total: BigDecimal,
}

pub async fn vat_rows(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    vat_type: &str,
    period: &str,
) -> Result<Vec<VatAgg>, StorageError> {
    let entries = journal_entries::table
        .filter(journal_entries::company_id.eq(company_id))
        .filter(journal_entries::vat_type.eq(vat_type))
        .filter(journal_entries::vat_period.eq(period))
        .order(journal_entries::entry_date.asc())
        .select(JournalEntry::as_select())
        .load::<JournalEntry>(conn)
        .await?;
    let mut out = Vec::new();
    for e in entries {
        let lines = journal_lines::table
            .filter(journal_lines::entry_id.eq(e.id))
            .select(JournalLine::as_select())
            .load::<JournalLine>(conn)
            .await?;
        let is_vat = |n: &str| matches!(n, "4531" | "4532");
        let mut net = BigDecimal::default();
        let mut vat = BigDecimal::default();
        let mut total = BigDecimal::default();
        for l in lines {
            match (vat_type, l.direction.as_str()) {
                ("sales", "credit") if is_vat(&l.account_number) => vat += &l.amount,
                ("sales", "credit") => net += &l.amount,
                ("sales", "debit") => total += &l.amount,
                ("purchase", "debit") if is_vat(&l.account_number) => vat += &l.amount,
                ("purchase", "debit") => net += &l.amount,
                ("purchase", "credit") => total += &l.amount,
                _ => {}
            }
        }
        out.push(VatAgg {
            document_number: e.document_number,
            entry_date: e.entry_date,
            counterpart_id: e.counterpart_id,
            counterpart_name: e.counterpart_name,
            net,
            vat,
            total,
        });
    }
    Ok(out)
}
