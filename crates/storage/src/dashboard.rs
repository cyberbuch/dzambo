//! Табло: суми по фактурите и банките на една фирма.

use bigdecimal::{BigDecimal, Zero};
use chrono::{Datelike, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    models::invoices::Invoice,
    schema::{bank_accounts, invoices},
    StorageError,
};

#[derive(Debug)]
pub struct DashboardNumbers {
    pub year: i32,
    pub invoices_draft: i64,
    pub invoices_posted: i64,
    pub receivables: BigDecimal,
    pub payables: BigDecimal,
    pub vat_output: BigDecimal,
    pub vat_input: BigDecimal,
    pub revenue_ytd: BigDecimal,
    pub expense_ytd: BigDecimal,
    pub bank_total: BigDecimal,
    pub recent: Vec<crate::models::invoices::Invoice>,
}

pub async fn numbers(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<DashboardNumbers, StorageError> {
    let rows = invoices::table
        .filter(invoices::company_id.eq(company_id))
        .order(invoices::id.desc())
        .select(crate::models::invoices::Invoice::as_select())
        .load::<Invoice>(conn)
        .await?;

    let mut out = DashboardNumbers {
        year: Utc::now().year(),
        invoices_draft: 0,
        invoices_posted: 0,
        receivables: BigDecimal::zero(),
        payables: BigDecimal::zero(),
        vat_output: BigDecimal::zero(),
        vat_input: BigDecimal::zero(),
        revenue_ytd: BigDecimal::zero(),
        expense_ytd: BigDecimal::zero(),
        bank_total: BigDecimal::zero(),
        recent: Vec::new(),
    };

    for row in &rows {
        match row.status.as_str() {
            "draft" => out.invoices_draft += 1,
            "posted" => out.invoices_posted += 1,
            _ => {}
        }
        match row.direction.as_str() {
            "out" => {
                out.receivables += &row.total_amount;
                out.revenue_ytd += &row.net_amount;
                out.vat_output += &row.vat_amount;
            }
            "in" => {
                out.payables += &row.total_amount;
                out.expense_ytd += &row.net_amount;
                out.vat_input += &row.vat_amount;
            }
            _ => {}
        }
    }
    out.recent = rows.into_iter().take(5).collect();

    let balances = bank_accounts::table
        .filter(bank_accounts::company_id.eq(company_id))
        .select(bank_accounts::balance)
        .load::<BigDecimal>(conn)
        .await?;
    out.bank_total = balances.into_iter().sum();
    Ok(out)
}
