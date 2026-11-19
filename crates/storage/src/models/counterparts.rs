//! Контрагенти: основен ред + адресна опашка (1:1).

use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::counterparts)]
pub struct Counterpart {
    pub id: i64,
    pub company_id: i64,
    pub name: String,
    pub counterpart_type: String,
    pub eik: String,
    pub vat_number: String,
    pub address: String,
    pub city: String,
    pub post_code: String,
    pub country: String,
    pub contact_person: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::counterparts)]
pub struct NewCounterpart<'a> {
    pub company_id: i64,
    pub name: &'a str,
    pub counterpart_type: &'a str,
    pub eik: &'a str,
    pub vat_number: &'a str,
    pub address: &'a str,
    pub city: &'a str,
    pub post_code: &'a str,
    pub country: &'a str,
    pub contact_person: &'a str,
}

#[derive(Queryable, Selectable, Debug, Clone, Default)]
#[diesel(table_name = crate::schema::counterpart_extra)]
pub struct CounterpartExtra {
    pub id: i64,
    pub vies_address: String,
    pub street_name: String,
    pub building_number: String,
    pub address_building: String,
    pub additional_address_detail: String,
    pub address_type: String,
    pub region: String,
    pub email: String,
    pub phone: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::counterpart_extra)]
pub struct NewCounterpartExtra<'a> {
    pub id: i64,
    pub vies_address: &'a str,
    pub street_name: &'a str,
    pub building_number: &'a str,
    pub address_building: &'a str,
    pub additional_address_detail: &'a str,
    pub address_type: &'a str,
    pub region: &'a str,
    pub email: &'a str,
    pub phone: &'a str,
}
