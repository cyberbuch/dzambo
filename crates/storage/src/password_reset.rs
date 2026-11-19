//! Токени за възстановяване на парола (sha256 хеш, 1 час живот, еднократни).

use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::schema::password_reset_tokens;
use crate::StorageResult;

pub async fn create(
    conn: &mut crate::DbConn,
    user_id: i64,
    token_hash: &str,
    expires_at: i64,
) -> StorageResult<()> {
    diesel::insert_into(password_reset_tokens::table)
        .values((
            password_reset_tokens::user_id.eq(user_id),
            password_reset_tokens::token_hash.eq(token_hash),
            password_reset_tokens::expires_at.eq(expires_at),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn find_live(
    conn: &mut crate::DbConn,
    token_hash: &str,
    now: i64,
) -> StorageResult<Option<(i64, i64)>> {
    Ok(password_reset_tokens::table
        .filter(password_reset_tokens::token_hash.eq(token_hash))
        .filter(password_reset_tokens::used.eq(false))
        .filter(password_reset_tokens::expires_at.gt(now))
        .select((password_reset_tokens::id, password_reset_tokens::user_id))
        .first(conn)
        .await
        .optional()?)
}

pub async fn mark_used(conn: &mut crate::DbConn, id: i64) -> StorageResult<()> {
    diesel::update(password_reset_tokens::table.find(id))
        .set(password_reset_tokens::used.eq(true))
        .execute(conn)
        .await?;
    Ok(())
}
