//! DTO-та на API-то. Суми са низове (договорът от ytbuch), не decimal.
//! ts-rs експортира типовете в frontend/src/bindings при `cargo test`.

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub fn fmt_money(v: &BigDecimal) -> String {
    format!("{v:.2}")
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TokenBody {
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    pub password: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub sub: String,
    pub expires_in: u64,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct MembershipDto {
    pub company_id: i64,
    pub role_id: i32,
    pub role: Option<String>,
    pub company_name: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct MeDto {
    pub sub: String,
    pub id: i64,
    pub email: String,
    pub username: String,
    pub name: String,
    pub is_super_admin: bool,
    pub role_id: i32,
    pub totp_enabled: bool,
    pub permissions: Vec<String>,
    pub companies: Vec<MembershipDto>,
}

#[derive(Serialize, TS)]
#[ts(export)]
#[allow(clippy::struct_excessive_bools)]
pub struct CompanyDto {
    pub id: i64,
    pub name: String,
    pub eik: String,
    pub vat_number: String,
    pub is_vat_registered: bool,
    pub vat_period: String,
    pub currency: String,
    pub address: String,
    pub city: String,
    pub post_code: String,
    pub phone: String,
    pub email: String,
    pub website: String,
    pub mol: String,
    pub manager_eik: String,
    pub accountant_name: String,
    pub accountant_egn: String,
    pub tax_authority: String,
    pub nap_office: String,
    pub iban: String,
    pub bic: String,
    pub inventory_valuation_method: String,
    pub fiscal_year_start_month: i32,
    pub country: String,
    pub manager_name: String,
    pub manager_egn: String,
    pub authorized_person_name: String,
    pub authorized_person_egn: String,
    pub vat_branch_number: String,
    pub tax_accounting_basis: String,
    pub tax_entity: String,
    pub software_company_name: String,
    pub software_id: String,
    pub software_version: String,
    pub is_part_of_group: String,
    pub street_name: String,
    pub building_number: String,
    pub address_building: String,
    pub additional_address_detail: String,
    pub address_type: String,
    pub region: String,
    pub created_at: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct Paged<T> {
    pub items: Vec<T>,
    pub count: i64,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct RecentInvoiceDto {
    pub id: i64,
    pub number: String,
    pub issue_date: String,
    pub counterpart_id: i64,
    pub counterpart_name: String,
    pub currency: String,
    pub total_amount: String,
    pub status: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct DashboardDto {
    pub company_id: i64,
    pub year: i32,
    pub invoices_draft: i64,
    pub invoices_posted: i64,
    pub receivables: String,
    pub payables: String,
    pub vat_output: String,
    pub vat_input: String,
    pub revenue_ytd: String,
    pub expense_ytd: String,
    pub bank_total: String,
    pub recent: Vec<RecentInvoiceDto>,
}
