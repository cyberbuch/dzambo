//! Стоки: основен ред + разширение (вид запас, КН8, сметки) 1:1.

use bigdecimal::BigDecimal;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::products)]
pub struct Product {
    pub id: i64,
    pub company_id: i64,
    pub name: String,
    pub code: String,
    pub description: String,
    pub unit: String,
    pub price: BigDecimal,
    pub cost_price: BigDecimal,
    pub vat_rate: BigDecimal,
    pub is_service: bool,
    pub is_inventory: bool,
    pub inventory_account_id: i64,
    pub revenue_account_id: i64,
    pub stock_quantity: BigDecimal,
    pub stock_value: BigDecimal,
    pub is_active: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::products)]
#[allow(clippy::too_many_arguments)]
pub struct NewProduct<'a> {
    pub company_id: i64,
    pub name: &'a str,
    pub code: &'a str,
    pub description: &'a str,
    pub unit: &'a str,
    pub price: BigDecimal,
    pub cost_price: BigDecimal,
    pub vat_rate: BigDecimal,
    pub is_service: bool,
    pub is_inventory: bool,
    pub inventory_account_id: i64,
    pub revenue_account_id: i64,
    pub is_active: bool,
}

#[derive(Queryable, Selectable, Debug, Clone, Default)]
#[diesel(table_name = crate::schema::product_extra)]
pub struct ProductExtra {
    pub id: i64,
    pub product_type: String,
    pub cn8_code: String,
    pub cn8_name: String,
    pub expense_account_id: i64,
    pub auto_write_off: bool,
}

impl ProductExtra {
    pub fn for_product(id: i64) -> Self {
        Self {
            id,
            product_type: String::new(),
            cn8_code: String::new(),
            cn8_name: String::new(),
            expense_account_id: 0,
            auto_write_off: false,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::product_extra)]
pub struct NewProductExtra<'a> {
    pub id: i64,
    pub product_type: &'a str,
    pub cn8_code: &'a str,
    pub cn8_name: &'a str,
    pub expense_account_id: i64,
    pub auto_write_off: bool,
}
