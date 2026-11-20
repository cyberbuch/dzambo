//! TOTP състояние по потребител: тайна, консумация на прозорец, резервни кодове,
//! mfa заключване. Порт на ytbuch identity (totp/recovery/mfa).

use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::schema::{recovery_codes, users};
use crate::StorageResult;
use domain::totp;

pub async fn set_totp(
    conn: &mut crate::DbConn,
    user_id: i64,
    enabled: bool,
    secret: &str,
    pending: &str,
) -> StorageResult<()> {
    diesel::update(users::table.find(user_id))
        .set((
            users::totp_enabled.eq(enabled),
            users::totp_secret.eq(secret),
            users::totp_pending.eq(pending),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

/// Консумира прозореца: същият (или по-стар) код не минава повторно.
pub async fn consume_counter(
    conn: &mut crate::DbConn,
    user_id: i64,
    counter: u64,
) -> StorageResult<bool> {
    let n = diesel::update(users::table.find(user_id))
        .filter(users::totp_last_counter.lt(counter as i64))
        .set(users::totp_last_counter.eq(counter as i64))
        .execute(conn)
        .await?;
    Ok(n > 0)
}

pub async fn mfa_reset(conn: &mut crate::DbConn, user_id: i64) -> StorageResult<()> {
    diesel::update(users::table.find(user_id))
        .set((users::mfa_fails.eq(0), users::mfa_lock_until.eq(0)))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn mfa_failure(conn: &mut crate::DbConn, user_id: i64) -> StorageResult<()> {
    diesel::update(users::table.find(user_id))
        .set(users::mfa_fails.eq(users::mfa_fails + 1))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn mfa_lock(conn: &mut crate::DbConn, user_id: i64, until: i64) -> StorageResult<()> {
    diesel::update(users::table.find(user_id))
        .set(users::mfa_lock_until.eq(until))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn wipe_recovery(conn: &mut crate::DbConn, user_id: i64) -> StorageResult<()> {
    diesel::delete(recovery_codes::table.filter(recovery_codes::user_id.eq(user_id)))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn insert_recovery(
    conn: &mut crate::DbConn,
    user_id: i64,
    hashes: &[String],
) -> StorageResult<()> {
    for h in hashes {
        diesel::insert_into(recovery_codes::table)
            .values((
                recovery_codes::user_id.eq(user_id),
                recovery_codes::code_hash.eq(h),
            ))
            .execute(conn)
            .await?;
    }
    Ok(())
}

/// Използва резервен код (еднократно). Връща true при успех.
pub async fn take_recovery(
    conn: &mut crate::DbConn,
    user_id: i64,
    code: &str,
) -> StorageResult<bool> {
    let hash = totp::recovery_hash(code);
    if hash.is_empty() {
        return Ok(false);
    }
    let row: Option<i64> = recovery_codes::table
        .filter(recovery_codes::user_id.eq(user_id))
        .filter(recovery_codes::code_hash.eq(&hash))
        .filter(recovery_codes::used.eq(false))
        .select(recovery_codes::id)
        .first(conn)
        .await
        .optional()?;
    let Some(id) = row else { return Ok(false) };
    diesel::update(recovery_codes::table.find(id))
        .set(recovery_codes::used.eq(true))
        .execute(conn)
        .await?;
    Ok(true)
}
