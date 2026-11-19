//! Пост на фактура в дневника — цялата операция в една транзакция.
//! Черно сторно (03/13/23): отрицателен знак, без обръщане на Дт/Кт.

use std::collections::HashMap;

use axum::http::StatusCode;
use bigdecimal::{BigDecimal, Zero};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use storage::{
    accounts, invoices,
    journal::{self, JournalRow, NewJournalEntry, NewJournalLine},
    periods, products,
};

use crate::errors::ApiError;

const CREDIT_TYPES: [&str; 3] = ["03", "13", "23"];

pub const A_CUSTOMERS: &str = "411";
pub const A_SUPPLIERS: &str = "401";
pub const A_VAT_SALE: &str = "4532";
pub const A_VAT_PURCHASE: &str = "4531";
pub const A_REVENUES: &str = "702";
pub const A_EXPENSES: &str = "602";

fn title(doc_type: &str) -> &'static str {
    match doc_type {
        "01" => "Фактура",
        "02" => "Дебитно известие",
        "03" => "Кредитно известие",
        "04" => "Опростена фактура",
        "05" => "Входяща фактура",
        "07" => "Митническа декларация",
        "09" => "Протокол по чл. 117 ЗДДС",
        _ => "Документ",
    }
}

struct Acct {
    id: i64,
    number: String,
    name: String,
}

async fn load_accounts(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<HashMap<String, Acct>, ApiError> {
    let all = accounts::list(conn, company_id).await?;
    Ok(all
        .into_iter()
        .map(|a| {
            (
                a.number.clone(),
                Acct {
                    id: a.id,
                    number: a.number,
                    name: a.name,
                },
            )
        })
        .collect())
}

async fn std_acct(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    number: &str,
) -> Result<Acct, ApiError> {
    let a = accounts::get_or_create(conn, company_id, number).await?;
    Ok(Acct {
        id: a.id,
        number: a.number,
        name: a.name,
    })
}

type ProdRow = (
    storage::models::products::Product,
    storage::models::products::ProductExtra,
);

fn line_account<'a>(
    prods: &HashMap<i64, &'a ProdRow>,
    ids: &'a HashMap<i64, Acct>,
    product_id: i64,
    revenue_side: bool,
    fallback: &'a Acct,
) -> &'a Acct {
    let Some((p, extra)) = prods.get(&product_id) else {
        return fallback;
    };
    let acct_id = if revenue_side {
        p.revenue_account_id
    } else if p.inventory_account_id > 0 {
        p.inventory_account_id
    } else {
        extra.expense_account_id
    };
    ids.get(&acct_id).unwrap_or(fallback)
}

/// Поства фактура → дневник; връща статията с редове.
pub async fn post_invoice(pool: &storage::DbPool, invoice_id: i64) -> Result<JournalRow, ApiError> {
    let mut conn = pool.get().await.map_err(|_| internal())?;
    let (inv, lines) = invoices::by_id(&mut conn, invoice_id)
        .await?
        .ok_or_else(|| ApiError::not_found("фактурата не е намерена"))?;

    if inv.status == "posted" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "фактурата вече е постната",
        ));
    }
    if !matches!(inv.direction.as_str(), "out" | "in") {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "невалидна посока на документа",
        ));
    }
    let month = if inv.accounting_month.is_empty() {
        periods::month_key(inv.issue_date)
    } else {
        inv.accounting_month.clone()
    };
    if periods::is_closed(&mut conn, inv.company_id, &month).await? {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("периодът {month} е затворен"),
        ));
    }

    let tx = conn.as_mut();
    diesel::sql_query("BEGIN").execute(tx).await?;
    let result: Result<JournalRow, ApiError> = async {
        let accts = load_accounts(tx, inv.company_id).await?;
        let prods: Vec<ProdRow> = products::list(tx, inv.company_id).await?;
        let prod_map: HashMap<i64, &ProdRow> = prods.iter().map(|row| (row.0.id, row)).collect();
        let id_map: HashMap<i64, Acct> = accts.values().map(|a| (a.id, clone_acct(a))).collect();

        let is_credit = CREDIT_TYPES.contains(&inv.document_type.as_str());
        let signed = |v: &BigDecimal| if is_credit { -v.clone() } else { v.clone() };

        let customers = std_acct(tx, inv.company_id, A_CUSTOMERS).await?;
        let suppliers = std_acct(tx, inv.company_id, A_SUPPLIERS).await?;
        let vat_sale = std_acct(tx, inv.company_id, A_VAT_SALE).await?;
        let vat_purchase = std_acct(tx, inv.company_id, A_VAT_PURCHASE).await?;
        let revenues = std_acct(tx, inv.company_id, A_REVENUES).await?;
        let expenses = std_acct(tx, inv.company_id, A_EXPENSES).await?;

        fn mk(account: &Acct, direction: &str, amount: BigDecimal) -> NewJournalLine {
            NewJournalLine {
                entry_id: 0,
                account_id: account.id,
                account_number: account.number.clone(),
                account_name: account.name.clone(),
                direction: direction.to_string(),
                amount,
                vat_amount: BigDecimal::zero(),
                currency: String::new(),
                fx_quantity: BigDecimal::zero(),
                fx_rate: BigDecimal::from(1),
            }
        }

        let vat_total: BigDecimal = lines.iter().map(|l| l.vat_amount.clone()).sum();
        let mut new_lines: Vec<NewJournalLine> = Vec::new();

        match inv.direction.as_str() {
            "out" => {
                new_lines.push(mk(&customers, "debit", signed(&inv.total_amount)));
                for line in &lines {
                    let acct = line_account(&prod_map, &id_map, line.product_id, true, &revenues);
                    new_lines.push(mk(acct, "credit", signed(&line.net_amount)));
                }
                if !vat_total.is_zero() {
                    new_lines.push(mk(&vat_sale, "credit", signed(&vat_total)));
                }
            }
            _ => {
                for line in &lines {
                    let acct = line_account(&prod_map, &id_map, line.product_id, false, &expenses);
                    new_lines.push(mk(acct, "debit", signed(&line.net_amount)));
                }
                if !vat_total.is_zero() {
                    new_lines.push(mk(&vat_purchase, "debit", signed(&vat_total)));
                }
                new_lines.push(mk(&suppliers, "credit", signed(&inv.total_amount)));
            }
        }

        for l in &mut new_lines {
            if matches!(l.account_number.as_str(), "4531" | "4532") {
                l.vat_amount = l.amount.clone();
            }
        }

        let description = format!(
            "{} {} / {}",
            title(&inv.document_type),
            inv.number,
            inv.direction
        );
        let vat_type: &str = if vat_total.is_zero() {
            "no_vat"
        } else if inv.direction == "out" {
            "sales"
        } else {
            "purchase"
        };

        let entry = NewJournalEntry {
            company_id: inv.company_id,
            entry_date: inv.issue_date,
            document_type: title(&inv.document_type).to_string(),
            document_id: inv.id,
            document_number: inv.number.clone(),
            description,
            vat_type: vat_type.to_string(),
            counterpart_id: inv.counterpart_id,
            counterpart_name: inv.counterpart_name.clone(),
            document_date: inv.issue_date,
            vat_period: month.clone(),
        };
        let entry_id = journal::insert_entry(tx, &entry, &new_lines).await?;
        invoices::set_status(tx, invoice_id, "posted", entry_id).await?;
        if inv.direction == "in" {
            storage::stock::lots_from_invoice(tx, inv.company_id, inv.id, &lines, inv.issue_date).await?;
        }
        journal::entry_with_lines(tx, entry_id)
            .await?
            .ok_or_else(internal)
    }
    .await;
    match result {
        Ok(v) => {
            diesel::sql_query("COMMIT").execute(tx).await?;
            Ok(v)
        }
        Err(e) => {
            let _ = diesel::sql_query("ROLLBACK").execute(tx).await;
            Err(e)
        }
    }
}

fn clone_acct(a: &Acct) -> Acct {
    Acct {
        id: a.id,
        number: a.number.clone(),
        name: a.name.clone(),
    }
}

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}

/// Unpost: трие дневника на фактурата и връща я в чернова.
pub async fn unpost_invoice(pool: &storage::DbPool, invoice_id: i64) -> Result<(), ApiError> {
    let mut conn = pool.get().await.map_err(|_| internal())?;
    let (inv, _) = invoices::by_id(&mut conn, invoice_id)
        .await?
        .ok_or_else(|| ApiError::not_found("фактурата не е намерена"))?;
    if inv.status != "posted" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "фактурата не е постната",
        ));
    }
    let month = if inv.accounting_month.is_empty() {
        periods::month_key(inv.issue_date)
    } else {
        inv.accounting_month.clone()
    };
    if periods::is_closed(&mut conn, inv.company_id, &month).await? {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("периодът {month} е затворен"),
        ));
    }
    let tx = conn.as_mut();
    diesel::sql_query("BEGIN").execute(tx).await?;
    let result: Result<(), storage::StorageError> = async {
        journal::delete_by_document(tx, invoice_id).await?;
        invoices::set_status(tx, invoice_id, "draft", 0).await?;
        Ok(())
    }
    .await;
    match result {
        Ok(()) => {
            diesel::sql_query("COMMIT").execute(tx).await?;
            Ok(())
        }
        Err(_) => {
            let _ = diesel::sql_query("ROLLBACK").execute(tx).await;
            Err(ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "вътрешна грешка",
            ))
        }
    }
}
