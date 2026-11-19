//! Ръчни складови документи: нужда, списание, движение между обекти.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    schema::{stock_document_lines, stock_documents},
    StorageError,
};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = stock_documents)]
pub struct StockDoc {
    pub id: i64,
    pub company_id: i64,
    pub number: String,
    pub doc_date: NaiveDate,
    pub kind: String,
    pub from_location_id: i64,
    pub to_location_id: i64,
    pub status: String,
    pub notes: String,
    pub client_key: Option<String>,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = stock_document_lines)]
pub struct StockDocLine {
    pub id: i64,
    pub document_id: i64,
    pub product_id: i64,
    pub quantity: BigDecimal,
    pub unit_cost: BigDecimal,
    pub description: String,
}

pub async fn list(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<StockDoc>, StorageError> {
    stock_documents::table
        .filter(stock_documents::company_id.eq(company_id))
        .order(stock_documents::id.desc())
        .select(StockDoc::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn lines_of(
    conn: &mut AsyncPgConnection,
    doc_id: i64,
) -> Result<Vec<StockDocLine>, StorageError> {
    stock_document_lines::table
        .filter(stock_document_lines::document_id.eq(doc_id))
        .select(StockDocLine::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_document(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    doc_date: NaiveDate,
    kind: &str,
    from_location_id: i64,
    to_location_id: i64,
    notes: &str,
    client_key: &str,
) -> Result<i64, StorageError> {
    let id: i64 = diesel::insert_into(stock_documents::table)
        .values((
            stock_documents::company_id.eq(company_id),
            stock_documents::doc_date.eq(doc_date),
            stock_documents::kind.eq(kind),
            stock_documents::from_location_id.eq(from_location_id),
            stock_documents::to_location_id.eq(to_location_id),
            stock_documents::notes.eq(notes),
            stock_documents::client_key.eq(client_key),
            stock_documents::number.eq(String::new()),
        ))
        .returning(stock_documents::id)
        .get_result(conn)
        .await?;
    diesel::update(stock_documents::table.find(id))
        .set(stock_documents::number.eq(format!("DOC-{id}")))
        .execute(conn)
        .await?;
    Ok(id)
}

pub async fn insert_line(
    conn: &mut AsyncPgConnection,
    doc_id: i64,
    product_id: i64,
    quantity: BigDecimal,
    unit_cost: BigDecimal,
    description: &str,
) -> Result<(), StorageError> {
    diesel::insert_into(stock_document_lines::table)
        .values((
            stock_document_lines::document_id.eq(doc_id),
            stock_document_lines::product_id.eq(product_id),
            stock_document_lines::quantity.eq(quantity),
            stock_document_lines::unit_cost.eq(unit_cost),
            stock_document_lines::description.eq(description),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

/// Движение: мести отворените партиди на стоката в новия обект.
pub async fn move_lots(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    product_id: i64,
    to_location: i64,
) -> Result<(), StorageError> {
    diesel::sql_query("UPDATE inventory_lots SET location_id = $1 WHERE company_id = $2 AND product_id = $3 AND remaining <> 0")
        .bind::<diesel::sql_types::BigInt,_>(to_location)
        .bind::<diesel::sql_types::BigInt,_>(company_id)
        .bind::<diesel::sql_types::BigInt,_>(product_id)
        .execute(conn).await?;
    Ok(())
}

/// Идемпотентност: намира документ по клиентски ключ (повторен опит = същия резултат).
pub async fn find_by_key(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    client_key: &str,
) -> Result<Option<StockDoc>, StorageError> {
    stock_documents::table
        .filter(stock_documents::company_id.eq(company_id))
        .filter(stock_documents::client_key.eq(client_key))
        .select(StockDoc::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}
