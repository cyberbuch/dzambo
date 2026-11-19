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

#[derive(Serialize, TS, Clone)]
#[ts(export)]
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

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AccountDto {
    #[serde(default)]
    pub id: i64,
    pub number: String,
    pub name: String,
    pub saft_account_type: String,
    pub analytic_type: String,
    #[serde(default)]
    pub is_currency: bool,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
#[allow(clippy::struct_excessive_bools)]
pub struct CounterpartDto {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default = "default_cp_type")]
    pub counterpart_type: String,
    #[serde(default)]
    pub eik: String,
    #[serde(default)]
    pub vat_number: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub post_code: String,
    #[serde(default = "default_country")]
    pub country: String,
    #[serde(default)]
    pub contact_person: String,
    #[serde(default)]
    pub vies_address: String,
    #[serde(default)]
    pub street_name: String,
    #[serde(default)]
    pub building_number: String,
    #[serde(default)]
    pub address_building: String,
    #[serde(default)]
    pub additional_address_detail: String,
    #[serde(default)]
    pub address_type: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
}

fn default_cp_type() -> String {
    "both".into()
}

fn default_country() -> String {
    "BG".into()
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
#[allow(clippy::struct_excessive_bools)]
pub struct ProductDto {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_unit")]
    pub unit: String,
    #[serde(default)]
    pub price: Option<String>,
    #[serde(default)]
    pub cost_price: Option<String>,
    #[serde(default)]
    pub vat_rate: Option<String>,
    #[serde(default)]
    pub is_service: bool,
    #[serde(default)]
    pub is_inventory: bool,
    #[serde(default)]
    pub inventory_account_id: i64,
    #[serde(default)]
    pub revenue_account_id: i64,
    #[serde(default)]
    pub stock_quantity: Option<String>,
    #[serde(default)]
    pub stock_value: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub product_type: String,
    #[serde(default)]
    pub cn8_code: String,
    #[serde(default)]
    pub cn8_name: String,
    #[serde(default)]
    pub expense_account_id: i64,
    #[serde(default)]
    pub auto_write_off: bool,
}

fn default_unit() -> String {
    "C62".into()
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LocationDto {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_location_type")]
    pub location_type: String,
    #[serde(default)]
    pub street_name: String,
    #[serde(default)]
    pub building_number: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub post_code: String,
    #[serde(default)]
    pub region: String,
    #[serde(default = "default_country")]
    pub country: String,
    #[serde(default)]
    pub is_main: bool,
}

fn default_location_type() -> String {
    "OFFICE".into()
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OwnerDto {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub first_name_bg: String,
    #[serde(default)]
    pub last_name_bg: String,
    #[serde(default)]
    pub egn: String,
    #[serde(default)]
    pub first_name_latin: String,
    #[serde(default)]
    pub last_name_latin: String,
    #[serde(default = "default_country")]
    pub country: String,
    #[serde(default)]
    pub ownership_percentage: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParentDto {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name_bg: String,
    #[serde(default)]
    pub uic: String,
    #[serde(default)]
    pub name_latin: String,
    #[serde(default = "default_country")]
    pub country: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SeriesDto {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub document_types: String,
    #[serde(default = "default_pad")]
    pub pad_width: i32,
    #[serde(default = "default_one")]
    pub start_number: i32,
    #[serde(default = "default_one_str")]
    pub next_number: String,
}

fn default_pad() -> i32 {
    8
}

fn default_one() -> i32 {
    1
}

fn default_one_str() -> String {
    "1".into()
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct CompanySettingsDto {
    pub company: CompanyDto,
    pub locations: Vec<LocationDto>,
    pub beneficial_owners: Vec<OwnerDto>,
    pub ultimate_parents: Vec<ParentDto>,
    pub document_series: Vec<SeriesDto>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct UnitDto {
    pub code: String,
    pub name: String,
    pub name_en: String,
    pub symbol: String,
    pub category: String,
    pub source: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct SeedSaftDto {
    pub created: i64,
    pub skipped: i64,
    pub total: i64,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InvoiceLineDto {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub code: String,
    pub description: String,
    pub quantity: Option<String>,
    #[serde(default)]
    pub unit: String,
    pub unit_price: Option<String>,
    pub vat_rate: Option<String>,
    pub net_amount: Option<String>,
    pub vat_amount: Option<String>,
    pub total_amount: Option<String>,
    #[serde(default)]
    pub product_id: i64,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
#[allow(clippy::struct_excessive_bools)]
pub struct InvoiceDto {
    #[serde(default)]
    pub id: i64,
    pub direction: String,
    #[serde(default = "default_doc_type")]
    pub document_type: String,
    #[serde(default)]
    pub number: String,
    pub issue_date: String,
    #[serde(default)]
    pub due_date: String,
    #[serde(default)]
    pub counterpart_id: i64,
    #[serde(default)]
    pub counterpart_name: String,
    #[serde(default = "default_country")]
    pub currency: String,
    #[serde(default)]
    pub currency_rate: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub net_amount: Option<String>,
    #[serde(default)]
    pub vat_amount: Option<String>,
    #[serde(default)]
    pub total_amount: Option<String>,
    #[serde(default)]
    pub accounting_month: String,
    #[serde(default)]
    pub original_invoice_id: i64,
    #[serde(default)]
    pub lines: Vec<InvoiceLineDto>,
}

fn default_doc_type() -> String {
    "01".into()
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct JournalLineDto {
    pub id: i64,
    pub account_id: i64,
    pub account_number: String,
    pub account_name: String,
    pub direction: String,
    pub amount: String,
    pub vat_amount: String,
    pub currency: String,
    pub fx_quantity: String,
    pub fx_rate: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct JournalEntryDto {
    pub id: i64,
    pub entry_date: String,
    pub document_type: String,
    pub document_id: i64,
    pub document_number: String,
    pub description: String,
    pub vat_type: String,
    pub counterpart_id: i64,
    pub counterpart_name: String,
    pub document_date: String,
    pub vat_period: String,
    pub lines: Vec<JournalLineDto>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JournalLineIn {
    pub account_id: i64,
    pub direction: String,
    pub amount: String,
    #[serde(default)]
    pub vat_amount: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub fx_quantity: Option<String>,
    #[serde(default)]
    pub fx_rate: Option<String>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JournalIn {
    pub entry_date: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub vat_type: String,
    #[serde(default)]
    pub counterpart_id: i64,
    #[serde(default)]
    pub counterpart_name: String,
    #[serde(default)]
    pub document_number: String,
    #[serde(default)]
    pub document_date: String,
    #[serde(default)]
    pub vat_period: String,
    pub lines: Vec<JournalLineIn>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct NextNumberDto {
    pub number: String,
    pub document_type: String,
    pub direction: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PeriodDto {
    pub year: i32,
    pub month: i32,
    #[serde(default = "default_open")]
    pub status: String,
}

fn default_open() -> String {
    "open".into()
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct VatRowDto {
    pub number: String,
    pub issue_date: String,
    pub counterpart_name: String,
    pub eik: String,
    pub net_amount: String,
    pub vat_amount: String,
    pub total_amount: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OpeningDto {
    #[serde(default)]
    pub id: i64,
    pub account_id: i64,
    pub year: i32,
    pub debit: String,
    pub credit: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ClosingRowDto {
    pub source: String,
    pub source_name: String,
    pub target: String,
    pub amount: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BankAccountDto {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub iban: String,
    #[serde(default = "default_country")]
    pub currency: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct BankTransactionDto {
    pub id: i64,
    pub tx_date: String,
    pub amount: String,
    pub direction: String,
    pub counterpart_name: String,
    pub description: String,
    pub reference: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BankRuleDto {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    pub keywords: String,
    #[serde(default = "default_any")]
    pub direction: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub account_id: i64,
    #[serde(default)]
    pub counterpart_id: i64,
}

fn default_zero() -> i64 {
    0
}

fn default_any() -> String {
    "any".into()
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AssetDto {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub inventory_number: String,
    #[serde(default)]
    pub category_id: i64,
    pub acquisition_date: String,
    #[serde(default)]
    pub cost: String,
    #[serde(default)]
    pub salvage_value: String,
    #[serde(default)]
    pub useful_life_months: i32,
    #[serde(default)]
    pub accumulated_depreciation: String,
    #[serde(default = "default_active")]
    pub status: String,
}

fn default_active() -> String {
    "active".into()
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct DepRowDto {
    pub asset_id: i64,
    pub name: String,
    pub amount: String,
    pub expense_account: String,
    pub depreciation_account: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct StockItemDto {
    pub product_id: i64,
    pub name: String,
    pub code: String,
    pub unit: String,
    pub quantity: String,
    pub avg_unit_cost: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StockDocDto {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub number: String,
    pub doc_date: String,
    pub kind: String,
    #[serde(default = "default_zero")]
    pub to_location_id: i64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub lines: Vec<StockDocLineDto>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StockDocLineDto {
    #[serde(default)]
    pub product_id: i64,
    pub quantity: String,
    #[serde(default)]
    pub unit_cost: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct DividendShareDto {
    pub id: i64,
    pub distribution_id: i64,
    pub owner_name: String,
    pub owner_egn: String,
    pub ownership_percentage: String,
    pub gross_amount: String,
    pub tax_rate: String,
    pub tax_amount: String,
    pub net_amount: String,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateDistributionBody {
    pub year: i32,
    pub total_amount: String,
    pub decision_date: String,
    #[serde(default)]
    pub decision_number: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "default_five")]
    pub tax_rate: String,
}

fn default_five() -> String {
    "5".into()
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RecipeDto {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub output_product_id: i64,
    #[serde(default)]
    pub output_quantity: String,
    #[serde(default)]
    pub lines: Vec<RecipeLineDto>,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RecipeLineDto {
    #[serde(default)]
    pub product_id: i64,
    #[serde(default)]
    pub quantity: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub wastage_percent: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct OrderDto {
    pub id: i64,
    #[serde(default)]
    pub number: String,
    pub recipe_id: i64,
    pub output_product_id: i64,
    pub quantity: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub material_cost: String,
    #[serde(default)]
    pub order_date: String,
    #[serde(default)]
    pub lines: Vec<OrderLineDto>,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct OrderLineDto {
    pub product_id: i64,
    #[serde(default)]
    pub product_name: String,
    pub quantity: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub unit_cost: String,
    #[serde(default)]
    pub amount: String,
}
