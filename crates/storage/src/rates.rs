//! Курсове на ЕЦБ за 1 EUR + фиксиран BGN. As-of към най-близка предходна дата.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::schema::exchange_rates;
use crate::StorageResult;

pub async fn latest_date(conn: &mut crate::DbConn) -> StorageResult<Option<NaiveDate>> {
    Ok(exchange_rates::table
        .select(exchange_rates::rate_date)
        .order(exchange_rates::rate_date.desc())
        .first(conn)
        .await
        .optional()?)
}

pub async fn by_date(
    conn: &mut crate::DbConn,
    date: NaiveDate,
) -> StorageResult<Vec<(i64, String, BigDecimal, NaiveDate)>> {
    Ok(exchange_rates::table
        .filter(exchange_rates::rate_date.eq(date))
        .order(exchange_rates::currency_code.asc())
        .select((
            exchange_rates::id,
            exchange_rates::currency_code,
            exchange_rates::rate,
            exchange_rates::rate_date,
        ))
        .load(conn)
        .await?)
}

/// Курс за 1 EUR към валута, as-of към най-близката предходна дата.
pub async fn per_eur(
    conn: &mut crate::DbConn,
    code: &str,
    date: NaiveDate,
) -> StorageResult<Option<(BigDecimal, NaiveDate)>> {
    Ok(exchange_rates::table
        .filter(exchange_rates::currency_code.eq(code))
        .filter(exchange_rates::rate_date.le(date))
        .order(exchange_rates::rate_date.desc())
        .select((exchange_rates::rate, exchange_rates::rate_date))
        .first(conn)
        .await
        .optional()?)
}

pub enum Upsert {
    Inserted,
    Updated,
    Unchanged,
}

pub async fn upsert(
    conn: &mut crate::DbConn,
    code: &str,
    rate: &BigDecimal,
    date: NaiveDate,
) -> StorageResult<Upsert> {
    let existing: Option<BigDecimal> = exchange_rates::table
        .filter(exchange_rates::currency_code.eq(code))
        .filter(exchange_rates::rate_date.eq(date))
        .select(exchange_rates::rate)
        .first(conn)
        .await
        .optional()?;
    match existing {
        None => {
            diesel::insert_into(exchange_rates::table)
                .values((
                    exchange_rates::currency_code.eq(code),
                    exchange_rates::rate.eq(rate),
                    exchange_rates::rate_date.eq(date),
                ))
                .execute(conn)
                .await?;
            Ok(Upsert::Inserted)
        }
        Some(old) if &old == rate => Ok(Upsert::Unchanged),
        Some(_) => {
            diesel::update(
                exchange_rates::table
                    .filter(exchange_rates::currency_code.eq(code))
                    .filter(exchange_rates::rate_date.eq(date)),
            )
            .set(exchange_rates::rate.eq(rate))
            .execute(conn)
            .await?;
            Ok(Upsert::Updated)
        }
    }
}
