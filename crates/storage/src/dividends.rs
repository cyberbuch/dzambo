//! Дивиденти: разпределение по собствениците и данък (по подразбиране 5%).

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    related,
    schema::{dividend_distributions, dividends},
    StorageError,
};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = dividend_distributions)]
pub struct Distribution {
    pub id: i64,
    pub company_id: i64,
    pub year: i32,
    pub total_amount: BigDecimal,
    pub decision_date: NaiveDate,
    pub decision_number: String,
    pub status: String,
    pub notes: String,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = dividends)]
pub struct Share {
    pub id: i64,
    pub company_id: i64,
    pub distribution_id: i64,
    pub beneficial_owner_id: i64,
    pub owner_name: String,
    pub owner_egn: String,
    pub ownership_percentage: String,
    pub gross_amount: BigDecimal,
    pub tax_rate: BigDecimal,
    pub tax_amount: BigDecimal,
    pub net_amount: BigDecimal,
    pub decision_date: NaiveDate,
    pub payment_date: Option<NaiveDate>,
    pub is_paid: bool,
    pub notes: String,
}

pub async fn list_distributions(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<Distribution>, StorageError> {
    dividend_distributions::table
        .filter(dividend_distributions::company_id.eq(company_id))
        .order(dividend_distributions::id.desc())
        .select(Distribution::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn list_shares(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    year: Option<i32>,
) -> Result<Vec<(Distribution, Share)>, StorageError> {
    let mut q = dividends::table
        .inner_join(
            dividend_distributions::table
                .on(dividend_distributions::id.eq(dividends::distribution_id)),
        )
        .filter(dividends::company_id.eq(company_id))
        .into_boxed();
    if let Some(y) = year {
        q = q.filter(dividend_distributions::year.eq(y));
    }
    q.order((dividend_distributions::id.desc(), dividends::id.asc()))
        .select((Distribution::as_select(), Share::as_select()))
        .load(conn)
        .await
        .map_err(StorageError::from)
}

fn to_cents(v: &BigDecimal) -> i64 {
    (v * BigDecimal::from(100))
        .round(0)
        .to_string()
        .parse()
        .unwrap_or(0)
}
fn from_cents(c: i64) -> BigDecimal {
    BigDecimal::from(c) / 100
}

/// Създава разпределение за годината по действителните собственици.
/// Последният участник получава остатъка, за да суми излизат точно.
#[allow(clippy::too_many_arguments)]
pub async fn create_distribution(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    year: i32,
    total_amount: BigDecimal,
    decision_date: NaiveDate,
    decision_number: &str,
    notes: &str,
    tax_rate: BigDecimal,
) -> Result<(i64, i64), StorageError> {
    let owners = related::owners(conn, company_id).await?;
    if owners.is_empty() {
        return Err(StorageError::Connection(
            "няма регистрирани собственици".into(),
        ));
    }

    let id: i64 = diesel::insert_into(dividend_distributions::table)
        .values((
            dividend_distributions::company_id.eq(company_id),
            dividend_distributions::year.eq(year),
            dividend_distributions::total_amount.eq(&total_amount),
            dividend_distributions::decision_date.eq(decision_date),
            dividend_distributions::decision_number.eq(decision_number),
            dividend_distributions::notes.eq(notes),
        ))
        .returning(dividend_distributions::id)
        .get_result(conn)
        .await?;

    let n = insert_shares(
        conn,
        company_id,
        id,
        &total_amount,
        decision_date,
        &tax_rate,
    )
    .await?;
    Ok((id, n))
}

/// Разпределя сумата по собствениците (последният получава остатъка).
pub async fn insert_shares(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    dist_id: i64,
    total_amount: &BigDecimal,
    decision_date: NaiveDate,
    tax_rate: &BigDecimal,
) -> Result<i64, StorageError> {
    let owners = related::owners(conn, company_id).await?;
    if owners.is_empty() {
        return Err(StorageError::Connection(
            "няма регистрирани собственици".into(),
        ));
    }
    let total_cents = to_cents(total_amount);
    let rate: f64 = tax_rate.to_string().parse().unwrap_or(5.0);
    let n = owners.len();
    let mut assigned = 0i64;
    for (i, o) in owners.iter().enumerate() {
        let pct: f64 = o.ownership_percentage.trim().parse().unwrap_or(0.0);
        let gross_cents = if i + 1 == n {
            total_cents - assigned
        } else {
            let c = (total_cents as f64 * pct / 100.0).round() as i64;
            assigned += c;
            c
        };
        let gross = from_cents(gross_cents);
        let tax = from_cents(((gross_cents as f64) * rate / 100.0).round() as i64);
        let net = &gross - &tax;
        diesel::insert_into(dividends::table)
            .values((
                dividends::company_id.eq(company_id),
                dividends::distribution_id.eq(dist_id),
                dividends::beneficial_owner_id.eq(o.id),
                dividends::owner_name.eq(format!("{} {}", o.first_name_bg, o.last_name_bg)),
                dividends::owner_egn.eq(&o.egn),
                dividends::ownership_percentage.eq(&o.ownership_percentage),
                dividends::gross_amount.eq(&gross),
                dividends::tax_rate.eq(tax_rate.clone()),
                dividends::tax_amount.eq(&tax),
                dividends::net_amount.eq(&net),
                dividends::decision_date.eq(decision_date),
            ))
            .execute(conn)
            .await?;
    }
    Ok(n as i64)
}

pub async fn get_distribution(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    id: i64,
) -> Result<Option<Distribution>, StorageError> {
    dividend_distributions::table
        .filter(dividend_distributions::company_id.eq(company_id))
        .find(id)
        .select(Distribution::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn shares_for(
    conn: &mut AsyncPgConnection,
    dist_id: i64,
) -> Result<Vec<Share>, StorageError> {
    dividends::table
        .filter(dividends::distribution_id.eq(dist_id))
        .order(dividends::id.asc())
        .select(Share::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn any_paid(conn: &mut AsyncPgConnection, dist_id: i64) -> Result<bool, StorageError> {
    let n: i64 = dividends::table
        .filter(dividends::distribution_id.eq(dist_id))
        .filter(dividends::is_paid.eq(true))
        .count()
        .get_result(conn)
        .await?;
    Ok(n > 0)
}

pub async fn set_status(
    conn: &mut AsyncPgConnection,
    id: i64,
    status: &str,
) -> Result<(), StorageError> {
    diesel::update(dividend_distributions::table.find(id))
        .set(dividend_distributions::status.eq(status))
        .execute(conn)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn update_fields(
    conn: &mut AsyncPgConnection,
    id: i64,
    year: i32,
    total: &BigDecimal,
    date: NaiveDate,
    number: &str,
    notes: &str,
) -> Result<(), StorageError> {
    diesel::update(dividend_distributions::table.find(id))
        .set((
            dividend_distributions::year.eq(year),
            dividend_distributions::total_amount.eq(total),
            dividend_distributions::decision_date.eq(date),
            dividend_distributions::decision_number.eq(number),
            dividend_distributions::notes.eq(notes),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn delete_shares(conn: &mut AsyncPgConnection, dist_id: i64) -> Result<(), StorageError> {
    diesel::delete(dividends::table.filter(dividends::distribution_id.eq(dist_id)))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn delete_distribution(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    id: i64,
) -> Result<bool, StorageError> {
    delete_shares(conn, id).await?;
    let n = diesel::delete(
        dividend_distributions::table
            .filter(dividend_distributions::company_id.eq(company_id))
            .find(id),
    )
    .execute(conn)
    .await?;
    Ok(n > 0)
}
