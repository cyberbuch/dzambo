//! Производство: рецепти и производствени нареждания.

use bigdecimal::{BigDecimal, Zero};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{schema::{recipes, recipe_lines, production_orders, production_order_lines}, StorageError};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = recipes)]
pub struct Recipe {
    pub id: i64,
    pub company_id: i64,
    pub code: String,
    pub name: String,
    pub output_product_id: i64,
    pub output_quantity: BigDecimal,
    pub output_unit: String,
    pub notes: String,
    pub is_active: bool,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = recipe_lines)]
pub struct RecipeLine {
    pub id: i64,
    pub recipe_id: i64,
    pub product_id: i64,
    pub quantity: BigDecimal,
    pub unit: String,
    pub wastage_percent: BigDecimal,
    pub line_no: i32,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = production_orders)]
pub struct Order {
    pub id: i64,
    pub company_id: i64,
    pub number: String,
    pub recipe_id: i64,
    pub output_product_id: i64,
    pub quantity: BigDecimal,
    pub location_id: i64,
    pub order_date: NaiveDate,
    pub status: String,
    pub notes: String,
    pub material_cost: BigDecimal,
    pub journal_entry_id: i64,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = production_order_lines)]
pub struct OrderLine {
    pub id: i64,
    pub order_id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub quantity: BigDecimal,
    pub unit: String,
    pub unit_cost: BigDecimal,
    pub amount: BigDecimal,
}

pub async fn list_recipes(conn: &mut AsyncPgConnection, company_id: i64) -> Result<Vec<Recipe>, StorageError> {
    recipes::table.filter(recipes::company_id.eq(company_id)).order(recipes::id.asc())
        .select(Recipe::as_select()).load(conn).await.map_err(StorageError::from)
}

pub async fn recipe_lines(conn: &mut AsyncPgConnection, recipe_id: i64) -> Result<Vec<RecipeLine>, StorageError> {
    recipe_lines::table.filter(recipe_lines::recipe_id.eq(recipe_id)).order(recipe_lines::line_no.asc())
        .select(RecipeLine::as_select()).load(conn).await.map_err(StorageError::from)
}

pub async fn create_recipe(
    conn: &mut AsyncPgConnection, company_id: i64, name: &str, output_product_id: i64, output_quantity: BigDecimal, output_unit: &str, lines: &[(i64, BigDecimal, String, BigDecimal)],
) -> Result<i64, StorageError> {
    let id: i64 = diesel::insert_into(recipes::table)
        .values((recipes::company_id.eq(company_id), recipes::name.eq(name),
            recipes::output_product_id.eq(output_product_id), recipes::output_quantity.eq(&output_quantity), recipes::output_unit.eq(output_unit)))
        .returning(recipes::id).get_result(conn).await?;
    for (i, (pid, qty, unit, waste)) in lines.iter().enumerate() {
        diesel::insert_into(recipe_lines::table)
            .values((recipe_lines::recipe_id.eq(id), recipe_lines::product_id.eq(*pid), recipe_lines::quantity.eq(qty),
                recipe_lines::unit.eq(unit), recipe_lines::wastage_percent.eq(waste), recipe_lines::line_no.eq(i as i32))).execute(conn).await?;
    }
    Ok(id)
}

pub async fn delete_recipe(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(recipes::table.find(id)).execute(conn).await?;
    Ok(())
}

pub async fn list_orders(conn: &mut AsyncPgConnection, company_id: i64) -> Result<Vec<Order>, StorageError> {
    production_orders::table.filter(production_orders::company_id.eq(company_id)).order(production_orders::id.desc())
        .select(Order::as_select()).load(conn).await.map_err(StorageError::from)
}

pub async fn order_lines_of(conn: &mut AsyncPgConnection, order_id: i64) -> Result<Vec<OrderLine>, StorageError> {
    production_order_lines::table.filter(production_order_lines::order_id.eq(order_id))
        .select(OrderLine::as_select()).load(conn).await.map_err(StorageError::from)
}

/// Създава чернова нареждане, разширявайки рецептата за заявеното количество.
pub async fn create_order(
    conn: &mut AsyncPgConnection, company_id: i64, recipe_id: i64, quantity: BigDecimal, date: NaiveDate,
) -> Result<i64, StorageError> {
    let r = recipes::table.find(recipe_id).select(Recipe::as_select()).first::<Recipe>(conn).await?;
    let factor = if r.output_quantity.is_zero() { BigDecimal::from(1) } else { &quantity / &r.output_quantity };
    let id: i64 = diesel::insert_into(production_orders::table)
        .values((production_orders::company_id.eq(company_id), production_orders::recipe_id.eq(recipe_id),
            production_orders::output_product_id.eq(r.output_product_id), production_orders::quantity.eq(&quantity),
            production_orders::order_date.eq(date), production_orders::status.eq("draft")))
        .returning(production_orders::id).get_result(conn).await?;
    for l in recipe_lines(conn, recipe_id).await? {
        let need = &l.quantity * &factor * (BigDecimal::from(1) + &l.wastage_percent / 100);
        diesel::insert_into(production_order_lines::table)
            .values((production_order_lines::order_id.eq(id), production_order_lines::product_id.eq(l.product_id),
                production_order_lines::quantity.eq(need), production_order_lines::unit.eq(&l.unit))).execute(conn).await?;
    }
    Ok(id)
}

/// Потвърждаване: FIFO/LIFO консумация на материалите, себестойност → партида от готов продукт.
pub async fn confirm_order(conn: &mut AsyncPgConnection, company_id: i64, order_id: i64) -> Result<BigDecimal, StorageError> {
    use crate::{companies, products, stock};
    let order = production_orders::table.find(order_id).select(Order::as_select()).first::<Order>(conn).await?;
    if order.status == "posted" { return Err(StorageError::Connection("нареждането вече е потвърдено".into())); }
    let lifo = companies::by_id(conn, company_id).await?.map(|(c, _)| c.inventory_valuation_method == "LIFO").unwrap_or(false);
    let prods = products::list(conn, company_id).await?;
    let mut total = BigDecimal::default();
    for l in order_lines_of(conn, order_id).await? {
        let cost = stock::consume_fifo(conn, company_id, l.product_id, l.quantity.clone(), lifo).await?;
        total += &cost;
        diesel::update(production_order_lines::table.find(l.id))
            .set((production_order_lines::unit_cost.eq(if l.quantity.is_zero() { BigDecimal::default() } else { &cost / &l.quantity }),
                production_order_lines::amount.eq(&cost),
                production_order_lines::product_name.eq(prods.iter().find(|(p, _)| p.id == l.product_id).map(|(p, _)| p.name.clone()).unwrap_or_default())))
            .execute(conn).await?;
    }
    if !order.quantity.is_zero() && !total.is_zero() {
        let unit = &total / &order.quantity;
        stock::add_lot(conn, company_id, order.output_product_id, order_id, 0, order.order_date, order.quantity.clone(), unit).await?;
    }
    diesel::update(production_orders::table.find(order_id))
        .set((production_orders::status.eq("posted"), production_orders::material_cost.eq(&total),
            production_orders::number.eq(format!("ORDER-{order_id}")))).execute(conn).await?;
    Ok(total)
}
