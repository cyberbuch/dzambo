//! Сметкоплан: списък, четене, запис, изтриване, дедупликация,
//! досеване на каталога НАП за фирма.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    models::{Account, NewAccount},
    schema::accounts,
    StorageError,
};

pub async fn list(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<Account>, StorageError> {
    accounts::table
        .filter(accounts::company_id.eq(company_id))
        .order(accounts::number.asc())
        .select(Account::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn by_id(conn: &mut AsyncPgConnection, id: i64) -> Result<Option<Account>, StorageError> {
    accounts::table
        .find(id)
        .select(Account::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn insert(
    conn: &mut AsyncPgConnection,
    new: &NewAccount<'_>,
) -> Result<Account, StorageError> {
    diesel::insert_into(accounts::table)
        .values(new)
        .returning(Account::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

/// Обновява number/name/saft_account_type/analytic_type/is_currency.
pub async fn update(
    conn: &mut AsyncPgConnection,
    id: i64,
    number: &str,
    name: &str,
    saft_account_type: &str,
    analytic_type: &str,
    is_currency: bool,
) -> Result<Account, StorageError> {
    diesel::update(accounts::table.find(id))
        .set((
            accounts::number.eq(number),
            accounts::name.eq(name),
            accounts::saft_account_type.eq(saft_account_type),
            accounts::analytic_type.eq(analytic_type),
            accounts::is_currency.eq(is_currency),
        ))
        .returning(Account::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(accounts::table.find(id))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn number_taken(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    exclude_id: i64,
    number: &str,
) -> Result<bool, StorageError> {
    let taken: Option<i64> = accounts::table
        .filter(accounts::company_id.eq(company_id))
        .filter(accounts::id.ne(exclude_id))
        .filter(accounts::number.eq(number.trim()))
        .select(accounts::id)
        .first(conn)
        .await
        .optional()?;
    Ok(taken.is_some())
}
