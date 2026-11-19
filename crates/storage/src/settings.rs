//! Настройки по ключ (settings таблица) и JWT тайната.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{schema::settings, StorageError};
use domain::auth::random_hex;

pub const JWT_SECRET_KEY: &str = "auth.jwt_secret";

pub async fn get(conn: &mut AsyncPgConnection, key: &str) -> Result<Option<String>, StorageError> {
    settings::table
        .filter(settings::setting_key.eq(key))
        .select(settings::value)
        .first::<String>(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn put(conn: &mut AsyncPgConnection, key: &str, value: &str) -> Result<(), StorageError> {
    diesel::sql_query(
        "INSERT INTO settings (setting_key, value) VALUES ($1, $2)
         ON CONFLICT (setting_key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind::<diesel::sql_types::Text, _>(key)
    .bind::<diesel::sql_types::Text, _>(value)
    .execute(conn)
    .await?;
    Ok(())
}

/// Тайната се генерира веднъж и се пази в базата — рестарт не инвалидира токените.
pub async fn ensure_jwt_secret(conn: &mut AsyncPgConnection) -> Result<String, StorageError> {
    if let Some(existing) = get(conn, JWT_SECRET_KEY).await? {
        return Ok(existing);
    }
    let secret = random_hex(32);
    put(conn, JWT_SECRET_KEY, &secret).await?;
    Ok(secret)
}

pub fn active_company_key(username: &str) -> String {
    format!("active_company:{username}")
}
