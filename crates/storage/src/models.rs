//! Моделите на storage. Групирани по домейн; всеки файл под 800 реда.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

pub mod counterparts;
pub mod invoices;
pub mod journal;
pub mod products;
pub mod related;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub is_active: bool,
    pub is_super_admin: bool,
    pub role_id: i32,
    pub totp_enabled: bool,
    pub session_ver: i32,
    pub totp_secret: String,
    pub totp_pending: String,
    pub totp_last_counter: i64,
    pub mfa_fails: i32,
    pub mfa_lock_until: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub name: &'a str,
    pub password_hash: &'a str,
    pub is_super_admin: bool,
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::companies)]
pub struct Company {
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
    pub nap_office: String,
    pub iban: String,
    pub bic: String,
    pub inventory_valuation_method: String,
    pub fiscal_year_start_month: i32,
    pub country: String,
    pub manager_name: String,
    pub region: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::companies)]
#[allow(clippy::too_many_arguments)]
pub struct NewCompany<'a> {
    pub name: &'a str,
    pub eik: &'a str,
    pub vat_number: &'a str,
    pub is_vat_registered: bool,
    pub vat_period: &'a str,
    pub currency: &'a str,
    pub address: &'a str,
    pub city: &'a str,
    pub post_code: &'a str,
    pub phone: &'a str,
    pub email: &'a str,
    pub website: &'a str,
    pub mol: &'a str,
    pub manager_eik: &'a str,
    pub accountant_name: &'a str,
    pub nap_office: &'a str,
    pub iban: &'a str,
    pub bic: &'a str,
    pub inventory_valuation_method: &'a str,
    pub fiscal_year_start_month: i32,
    pub country: &'a str,
    pub manager_name: &'a str,
    pub region: &'a str,
    pub created_at: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::company_details)]
pub struct CompanyDetails {
    pub company_id: i64,
    pub accountant_egn: String,
    pub manager_egn: String,
    pub tax_authority: String,
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
    pub settings: serde_json::Value,
}

/// Празни детайли по подразбиране — за фирми без записана опашка.
impl Default for CompanyDetails {
    fn default() -> Self {
        Self {
            company_id: 0,
            accountant_egn: String::new(),
            manager_egn: String::new(),
            tax_authority: String::new(),
            authorized_person_name: String::new(),
            authorized_person_egn: String::new(),
            vat_branch_number: String::new(),
            tax_accounting_basis: String::new(),
            tax_entity: String::new(),
            software_company_name: String::new(),
            software_id: String::new(),
            software_version: String::new(),
            is_part_of_group: String::new(),
            street_name: String::new(),
            building_number: String::new(),
            address_building: String::new(),
            additional_address_detail: String::new(),
            address_type: String::new(),
            settings: serde_json::Value::Object(Default::default()),
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::company_details)]
#[allow(clippy::too_many_arguments)]
pub struct NewCompanyDetails<'a> {
    pub company_id: i64,
    pub tax_authority: &'a str,
    pub tax_accounting_basis: &'a str,
    pub street_name: &'a str,
    pub building_number: &'a str,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::accounts)]
pub struct Account {
    pub id: i64,
    pub company_id: i64,
    pub number: String,
    pub name: String,
    pub saft_account_type: String,
    pub analytic_type: String,
    pub is_currency: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::accounts)]
pub struct NewAccount<'a> {
    pub company_id: i64,
    pub number: &'a str,
    pub name: &'a str,
    pub saft_account_type: &'a str,
    pub is_currency: bool,
}
