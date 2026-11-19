//! Фирми: списъци, членство, активна фирма. Ядро + детайли (1:1).

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    models::{Company, CompanyDetails, NewCompany, NewCompanyDetails},
    schema::{companies, company_details, user_companies},
    settings, StorageError,
};

pub async fn count(conn: &mut AsyncPgConnection) -> Result<i64, StorageError> {
    companies::table
        .count()
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub type CompanyRow = (Company, Option<CompanyDetails>);

pub async fn by_id(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<CompanyRow>, StorageError> {
    companies::table
        .find(id)
        .left_join(company_details::table.on(company_details::company_id.eq(companies::id)))
        .select((Company::as_select(), Option::<CompanyDetails>::as_select()))
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn list_all(conn: &mut AsyncPgConnection) -> Result<Vec<CompanyRow>, StorageError> {
    companies::table
        .order(companies::id.asc())
        .left_join(company_details::table.on(company_details::company_id.eq(companies::id)))
        .select((Company::as_select(), Option::<CompanyDetails>::as_select()))
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn list_for_user(
    conn: &mut AsyncPgConnection,
    user_id: i64,
) -> Result<Vec<CompanyRow>, StorageError> {
    companies::table
        .inner_join(user_companies::table.on(user_companies::company_id.eq(companies::id)))
        .filter(user_companies::user_id.eq(user_id))
        .order(companies::id.asc())
        .left_join(company_details::table.on(company_details::company_id.eq(companies::id)))
        .select((Company::as_select(), Option::<CompanyDetails>::as_select()))
        .load(conn)
        .await
        .map_err(StorageError::from)
}

/// Вмъква фирма + празни детайли по подразбиране.
pub async fn insert(
    conn: &mut AsyncPgConnection,
    new: &NewCompany<'_>,
) -> Result<CompanyRow, StorageError> {
    let company: Company = diesel::insert_into(companies::table)
        .values(new)
        .returning(Company::as_returning())
        .get_result(conn)
        .await?;
    let details = insert_details(
        conn,
        company.id,
        &NewCompanyDetails {
            company_id: company.id,
            tax_authority: "",
            tax_accounting_basis: "",
            street_name: "",
            building_number: "",
        },
    )
    .await?;
    Ok((company, Some(details)))
}

pub async fn insert_details(
    conn: &mut AsyncPgConnection,
    _company_id: i64,
    new: &NewCompanyDetails<'_>,
) -> Result<CompanyDetails, StorageError> {
    diesel::insert_into(company_details::table)
        .values(new)
        .returning(CompanyDetails::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

/// Записва полета върху вече съществуващ ред на детайлите.
pub async fn set_details(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    new: &NewCompanyDetails<'_>,
) -> Result<(), StorageError> {
    diesel::sql_query(
        "UPDATE company_details SET
            tax_authority = $2,
            tax_accounting_basis = $3,
            street_name = $4,
            building_number = $5
         WHERE company_id = $1",
    )
    .bind::<diesel::sql_types::BigInt, _>(company_id)
    .bind::<diesel::sql_types::Text, _>(new.tax_authority)
    .bind::<diesel::sql_types::Text, _>(new.tax_accounting_basis)
    .bind::<diesel::sql_types::Text, _>(new.street_name)
    .bind::<diesel::sql_types::Text, _>(new.building_number)
    .execute(conn)
    .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn add_member(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    company_id: i64,
    role_id: i32,
) -> Result<(), StorageError> {
    diesel::sql_query(
        "INSERT INTO user_companies (user_id, company_id, role_id) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, company_id) DO NOTHING",
    )
    .bind::<diesel::sql_types::BigInt, _>(user_id)
    .bind::<diesel::sql_types::BigInt, _>(company_id)
    .bind::<diesel::sql_types::Integer, _>(role_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub struct Membership {
    pub company_id: i64,
    pub role_id: i32,
    pub company_name: String,
}

pub async fn memberships_of(
    conn: &mut AsyncPgConnection,
    user_id: i64,
) -> Result<Vec<Membership>, StorageError> {
    user_companies::table
        .inner_join(companies::table.on(companies::id.eq(user_companies::company_id)))
        .filter(user_companies::user_id.eq(user_id))
        .order(companies::id.asc())
        .select((
            user_companies::company_id,
            user_companies::role_id,
            companies::name,
        ))
        .load::<(i64, i32, String)>(conn)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(company_id, role_id, company_name)| Membership {
                    company_id,
                    role_id,
                    company_name,
                })
                .collect()
        })
        .map_err(StorageError::from)
}

/// Активна фирма: записаната настройка, иначе първото членство.
pub async fn resolve_active(
    conn: &mut AsyncPgConnection,
    username: &str,
    user_id: i64,
) -> Result<Option<CompanyRow>, StorageError> {
    let key = settings::active_company_key(username);
    if let Some(raw) = settings::get(conn, &key).await? {
        if let Ok(id) = raw.trim().parse::<i64>() {
            if let Some(row) = by_id(conn, id).await? {
                return Ok(Some(row));
            }
        }
    }
    let first = memberships_of(conn, user_id).await?;
    match first.first() {
        Some(m) => by_id(conn, m.company_id).await,
        None => Ok(None),
    }
}
