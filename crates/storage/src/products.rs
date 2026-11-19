//! Стоки: списък, четене, запис (основен + разширение), изтриване,
//! дедупликация по код, следващ числов код.

use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    models::products::{NewProduct, NewProductExtra, Product, ProductExtra},
    schema::{product_extra, products},
    StorageError,
};

pub type ProductRow = (Product, ProductExtra);

type Joined = (Product, Option<ProductExtra>);

fn row_of(j: Joined) -> ProductRow {
    (j.0, j.1.unwrap_or_default())
}

pub async fn list(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<ProductRow>, StorageError> {
    let rows: Vec<Joined> = products::table
        .filter(products::company_id.eq(company_id))
        .order(products::id.asc())
        .left_join(product_extra::table.on(product_extra::id.eq(products::id)))
        .select((Product::as_select(), Option::<ProductExtra>::as_select()))
        .load(conn)
        .await?;
    Ok(rows.into_iter().map(row_of).collect())
}

pub async fn by_id(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<ProductRow>, StorageError> {
    let row: Option<Joined> = products::table
        .find(id)
        .left_join(product_extra::table.on(product_extra::id.eq(products::id)))
        .select((Product::as_select(), Option::<ProductExtra>::as_select()))
        .first(conn)
        .await
        .optional()?;
    Ok(row.map(row_of))
}

async fn save_extra(
    conn: &mut AsyncPgConnection,
    e: &NewProductExtra<'_>,
) -> Result<ProductExtra, StorageError> {
    diesel::insert_into(product_extra::table)
        .values(e)
        .on_conflict(product_extra::id)
        .do_update()
        .set((
            product_extra::product_type.eq(e.product_type),
            product_extra::cn8_code.eq(e.cn8_code),
            product_extra::cn8_name.eq(e.cn8_name),
            product_extra::expense_account_id.eq(e.expense_account_id),
            product_extra::auto_write_off.eq(e.auto_write_off),
        ))
        .returning(ProductExtra::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn insert(
    conn: &mut AsyncPgConnection,
    new: &NewProduct<'_>,
    extra: &NewProductExtra<'_>,
) -> Result<ProductRow, StorageError> {
    let p: Product = diesel::insert_into(products::table)
        .values(new)
        .returning(Product::as_returning())
        .get_result(conn)
        .await?;
    let extra_row = save_extra(
        conn,
        &NewProductExtra {
            id: p.id,
            product_type: extra.product_type,
            cn8_code: extra.cn8_code,
            cn8_name: extra.cn8_name,
            expense_account_id: extra.expense_account_id,
            auto_write_off: extra.auto_write_off,
        },
    )
    .await?;
    Ok((p, extra_row))
}

/// Обновява продукт + разширение; stock_quantity/stock_value се запазват.
pub async fn update(
    conn: &mut AsyncPgConnection,
    id: i64,
    new: &NewProduct<'_>,
    extra: &NewProductExtra<'_>,
) -> Result<ProductRow, StorageError> {
    let p: Product = diesel::update(products::table.find(id))
        .set((
            products::name.eq(new.name),
            products::code.eq(new.code),
            products::description.eq(new.description),
            products::unit.eq(new.unit),
            products::price.eq(new.price.clone()),
            products::cost_price.eq(new.cost_price.clone()),
            products::vat_rate.eq(new.vat_rate.clone()),
            products::is_service.eq(new.is_service),
            products::is_inventory.eq(new.is_inventory),
            products::inventory_account_id.eq(new.inventory_account_id),
            products::revenue_account_id.eq(new.revenue_account_id),
            products::is_active.eq(new.is_active),
        ))
        .returning(Product::as_returning())
        .get_result(conn)
        .await?;
    let extra_row = save_extra(conn, extra).await?;
    Ok((p, extra_row))
}

pub async fn delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(products::table.find(id))
        .execute(conn)
        .await?;
    Ok(())
}

/// case-insensitive дубликат по код; празен код се игнорира.
pub async fn code_taken(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    exclude_id: i64,
    code: &str,
) -> Result<bool, StorageError> {
    let code = code.trim();
    if code.is_empty() {
        return Ok(false);
    }
    let taken: Option<i64> = products::table
        .filter(products::company_id.eq(company_id))
        .filter(products::id.ne(exclude_id))
        .filter(products::code.ilike(code))
        .select(products::id)
        .first(conn)
        .await
        .optional()?;
    Ok(taken.is_some())
}

/// Следващият числов код след максималния числов код в фирмата.
pub async fn next_numeric_code(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<String, StorageError> {
    let codes = products::table
        .filter(products::company_id.eq(company_id))
        .select(products::code)
        .load::<String>(conn)
        .await?;
    let max = codes
        .iter()
        .filter(|c| c.chars().all(|ch| ch.is_ascii_digit()) && !c.is_empty())
        .filter_map(|c| c.parse::<i64>().ok())
        .max()
        .unwrap_or(0);
    Ok((max + 1).to_string())
}

/// Десетичен низ → BigDecimal; невалидност дава 0.
pub fn parse_decimal(raw: &str) -> BigDecimal {
    raw.trim().parse().unwrap_or_default()
}
