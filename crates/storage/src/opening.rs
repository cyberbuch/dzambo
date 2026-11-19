//! Начални салда по сметка и година.

use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{schema::opening_balances, StorageError};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = opening_balances)]
pub struct OpeningBalance {
    pub id: i64,
    pub company_id: i64,
    pub account_id: i64,
    pub year: i32,
    pub debit: BigDecimal,
    pub credit: BigDecimal,
}

pub async fn list(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    year: i32,
) -> Result<Vec<OpeningBalance>, StorageError> {
    opening_balances::table
        .filter(opening_balances::company_id.eq(company_id))
        .filter(opening_balances::year.eq(year))
        .select(OpeningBalance::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn upsert(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    account_id: i64,
    year: i32,
    debit: BigDecimal,
    credit: BigDecimal,
) -> Result<(), StorageError> {
    diesel::sql_query("INSERT INTO opening_balances (company_id, account_id, year, debit, credit) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (company_id, account_id, year) DO UPDATE SET debit=EXCLUDED.debit, credit=EXCLUDED.credit")
        .bind::<diesel::sql_types::BigInt,_>(company_id)
        .bind::<diesel::sql_types::BigInt,_>(account_id)
        .bind::<diesel::sql_types::Integer,_>(year)
        .bind::<diesel::sql_types::Numeric,_>(debit)
        .bind::<diesel::sql_types::Numeric,_>(credit)
        .execute(conn).await?;
    Ok(())
}

pub async fn delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(opening_balances::table.find(id))
        .execute(conn)
        .await?;
    Ok(())
}
