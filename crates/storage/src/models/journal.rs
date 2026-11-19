//! Дневник: статия (journal_entries) и ред (journal_lines).

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::journal_entries)]
pub struct JournalEntry {
    pub id: i64,
    pub company_id: i64,
    pub entry_date: NaiveDate,
    pub document_type: String,
    pub document_id: i64,
    pub document_number: String,
    pub description: String,
    pub vat_type: String,
    pub counterpart_id: i64,
    pub counterpart_name: String,
    pub document_date: NaiveDate,
    pub vat_period: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::journal_entries)]
#[allow(clippy::too_many_arguments)]
pub struct NewJournalEntry {
    pub company_id: i64,
    pub entry_date: NaiveDate,
    pub document_type: String,
    pub document_id: i64,
    pub document_number: String,
    pub description: String,
    pub vat_type: String,
    pub counterpart_id: i64,
    pub counterpart_name: String,
    pub document_date: NaiveDate,
    pub vat_period: String,
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::journal_lines)]
pub struct JournalLine {
    pub id: i64,
    pub entry_id: i64,
    pub account_id: i64,
    pub account_number: String,
    pub account_name: String,
    pub direction: String,
    pub amount: BigDecimal,
    pub vat_amount: BigDecimal,
    pub currency: String,
    pub fx_quantity: BigDecimal,
    pub fx_rate: BigDecimal,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::journal_lines)]
#[allow(clippy::too_many_arguments)]
pub struct NewJournalLine {
    pub entry_id: i64,
    pub account_id: i64,
    pub account_number: String,
    pub account_name: String,
    pub direction: String,
    pub amount: BigDecimal,
    pub vat_amount: BigDecimal,
    pub currency: String,
    pub fx_quantity: BigDecimal,
    pub fx_rate: BigDecimal,
}
