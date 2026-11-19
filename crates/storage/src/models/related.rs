//! Настройки на фирмата: обекти, собственици, крайни майки, серии.

use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::company_locations)]
pub struct Location {
    pub id: i64,
    pub company_id: i64,
    pub name: String,
    pub location_type: String,
    pub street_name: String,
    pub building_number: String,
    pub city: String,
    pub post_code: String,
    pub region: String,
    pub country: String,
    pub is_main: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::company_locations)]
#[allow(clippy::too_many_arguments)]
pub struct NewLocation<'a> {
    pub company_id: i64,
    pub name: &'a str,
    pub location_type: &'a str,
    pub street_name: &'a str,
    pub building_number: &'a str,
    pub city: &'a str,
    pub post_code: &'a str,
    pub region: &'a str,
    pub country: &'a str,
    pub is_main: bool,
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::beneficial_owners)]
pub struct BeneficialOwner {
    pub id: i64,
    pub company_id: i64,
    pub first_name_bg: String,
    pub last_name_bg: String,
    pub egn: String,
    pub first_name_latin: String,
    pub last_name_latin: String,
    pub country: String,
    pub ownership_percentage: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::beneficial_owners)]
#[allow(clippy::too_many_arguments)]
pub struct NewBeneficialOwner<'a> {
    pub company_id: i64,
    pub first_name_bg: &'a str,
    pub last_name_bg: &'a str,
    pub egn: &'a str,
    pub first_name_latin: &'a str,
    pub last_name_latin: &'a str,
    pub country: &'a str,
    pub ownership_percentage: &'a str,
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::ultimate_parents)]
pub struct UltimateParent {
    pub id: i64,
    pub company_id: i64,
    pub name_bg: String,
    pub uic: String,
    pub name_latin: String,
    pub country: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::ultimate_parents)]
pub struct NewUltimateParent<'a> {
    pub company_id: i64,
    pub name_bg: &'a str,
    pub uic: &'a str,
    pub name_latin: &'a str,
    pub country: &'a str,
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::document_series)]
pub struct DocumentSeries {
    pub id: i64,
    pub company_id: i64,
    pub name: String,
    pub document_types: String,
    pub pad_width: i32,
    pub start_number: i32,
    pub next_number: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::document_series)]
pub struct NewDocumentSeries<'a> {
    pub company_id: i64,
    pub name: &'a str,
    pub document_types: &'a str,
    pub pad_width: i32,
    pub start_number: i32,
    pub next_number: &'a str,
}
