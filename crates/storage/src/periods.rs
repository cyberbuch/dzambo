//! Счетоводни периоди: ключ YYYY-MM, липсващ ред = отворен.

use chrono::Datelike;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{schema::accounting_periods, StorageError};

pub fn month_key(date: chrono::NaiveDate) -> String {
    format!("{:04}-{:02}", date.year(), date.month())
}

pub async fn is_closed(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    key: &str,
) -> Result<bool, StorageError> {
    let status: Option<String> = accounting_periods::table
        .filter(accounting_periods::company_id.eq(company_id))
        .filter(accounting_periods::year.eq(year_of(key)))
        .filter(accounting_periods::month.eq(month_of(key)))
        .select(accounting_periods::status)
        .first(conn)
        .await
        .optional()?;
    Ok(matches!(status.as_deref(), Some("closed")))
}

fn year_of(key: &str) -> i32 {
    key.split('-')
        .next()
        .and_then(|y| y.parse().ok())
        .unwrap_or(0)
}

fn month_of(key: &str) -> i32 {
    key.split('-')
        .nth(1)
        .and_then(|m| m.parse().ok())
        .unwrap_or(0)
}

pub async fn close(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    year: i32,
    month: i32,
    notes: &str,
) -> Result<(), StorageError> {
    diesel::sql_query(
        "INSERT INTO accounting_periods (company_id, year, month, status, closed_at, notes)
         VALUES ($1, $2, $3, 'closed', now(), $4)
         ON CONFLICT (company_id, year, month) DO UPDATE SET status='closed', closed_at=now()",
    )
    .bind::<diesel::sql_types::BigInt, _>(company_id)
    .bind::<diesel::sql_types::Integer, _>(year)
    .bind::<diesel::sql_types::Integer, _>(month)
    .bind::<diesel::sql_types::Text, _>(notes)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn reopen(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    year: i32,
    month: i32,
) -> Result<(), StorageError> {
    diesel::update(
        accounting_periods::table
            .filter(accounting_periods::company_id.eq(company_id))
            .filter(accounting_periods::year.eq(year))
            .filter(accounting_periods::month.eq(month)),
    )
    .set(accounting_periods::status.eq("open"))
    .execute(conn)
    .await?;
    Ok(())
}
