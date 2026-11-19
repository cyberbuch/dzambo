//! Потребители: вход по email, бройка, създаване.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    models::{NewUser, User},
    schema::users,
    StorageError,
};

pub async fn count(conn: &mut AsyncPgConnection) -> Result<i64, StorageError> {
    users::table
        .count()
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn by_login(
    conn: &mut AsyncPgConnection,
    login: &str,
) -> Result<Option<User>, StorageError> {
    users::table
        .filter(users::email.eq(login))
        .select(User::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn insert(conn: &mut AsyncPgConnection, new: &NewUser<'_>) -> Result<User, StorageError> {
    diesel::insert_into(users::table)
        .values(new)
        .returning(User::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn list_all(conn: &mut AsyncPgConnection) -> Result<Vec<User>, StorageError> {
    users::table.order(users::id.asc()).select(User::as_select()).load(conn).await.map_err(StorageError::from)
}
