//! Фактури: хедър, ред, и четени структури.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::invoices)]
pub struct Invoice {
    pub id: i64,
    pub company_id: i64,
    pub direction: String,
    pub document_type: String,
    pub number: String,
    pub issue_date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub counterpart_id: i64,
    pub counterpart_name: String,
    pub currency: String,
    pub currency_rate: BigDecimal,
    pub net_amount: BigDecimal,
    pub vat_amount: BigDecimal,
    pub total_amount: BigDecimal,
    pub status: String,
    pub payment_method: String,
    pub notes: String,
    pub accounting_month: String,
    pub original_invoice_id: i64,
    pub journal_entry_id: i64,
    pub tax_event_date: Option<NaiveDate>,
    pub discount_percent: BigDecimal,
    pub discount_amount: BigDecimal,
    pub vat_exemption_reason: String,
    pub location_id: i64,
    pub exclude_vat: bool,
    pub vat_article: String,
    pub customs_statistical: String,
    pub customs_duty: BigDecimal,
    pub customs_vat: BigDecimal,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::invoices)]
#[allow(clippy::too_many_arguments)]
pub struct NewInvoice<'a> {
    pub company_id: i64,
    pub direction: &'a str,
    pub document_type: &'a str,
    pub number: &'a str,
    pub issue_date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub counterpart_id: i64,
    pub counterpart_name: &'a str,
    pub currency: &'a str,
    pub currency_rate: BigDecimal,
    pub accounting_month: &'a str,
    pub original_invoice_id: i64,
    pub payment_method: &'a str,
    pub tax_event_date: Option<NaiveDate>,
    pub discount_percent: BigDecimal,
    pub discount_amount: BigDecimal,
    pub vat_exemption_reason: &'a str,
    pub location_id: i64,
    pub exclude_vat: bool,
    pub vat_article: &'a str,
    pub customs_statistical: &'a str,
    pub customs_duty: BigDecimal,
    pub customs_vat: BigDecimal,
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::invoice_lines)]
pub struct InvoiceLine {
    pub id: i64,
    pub invoice_id: i64,
    pub code: String,
    pub description: String,
    pub quantity: BigDecimal,
    pub unit: String,
    pub unit_price: BigDecimal,
    pub vat_rate: BigDecimal,
    pub product_id: i64,
    pub net_amount: BigDecimal,
    pub vat_amount: BigDecimal,
    pub total_amount: BigDecimal,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::invoice_lines)]
pub struct NewInvoiceLine<'a> {
    pub invoice_id: i64,
    pub code: &'a str,
    pub description: &'a str,
    pub quantity: BigDecimal,
    pub unit: &'a str,
    pub unit_price: BigDecimal,
    pub vat_rate: BigDecimal,
    pub product_id: i64,
    pub net_amount: BigDecimal,
    pub vat_amount: BigDecimal,
    pub total_amount: BigDecimal,
}
