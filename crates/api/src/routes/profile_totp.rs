//! /v1/me/totp/* — включване/изключване на 2FA (TOTP + резервни кодове).

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use domain::{auth, totp};
use storage::totp_store;

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}
fn unauth(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, m)
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}
fn str_val(b: &Value, k: &str) -> String {
    b.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

pub async fn totp_start(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (u, _c) = active_company(&st, &session.claims.sub).await?;
    if !auth::verify_password(&str_val(&b, "current_password"), &u.password_hash) {
        return Err(unauth("грешна парола"));
    }
    if u.totp_enabled {
        return Err(ApiError::new(StatusCode::CONFLICT, "2FA вече е включена"));
    }
    let secret = totp::new_secret();
    let uri = totp::otpauth_uri(&secret, &u.email);
    let svg =
        totp::qr_svg(&uri).map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let made = totp::recovery_make();
    let codes: Vec<String> = made.iter().map(|(c, _)| c.clone()).collect();
    let hashes: Vec<String> = made.iter().map(|(_, h)| h.clone()).collect();
    let mut c = conn(&st).await?;
    totp_store::wipe_recovery(&mut c, u.id).await?;
    totp_store::insert_recovery(&mut c, u.id, &hashes).await?;
    totp_store::set_totp(&mut c, u.id, false, "", &secret).await?;
    Ok(Json(json!({
        "secret": secret, "uri": uri, "qr_svg": svg, "recovery_codes": codes,
        "message": "Сканирай кода и запази резервните кодове. Показват се само сега.",
    })))
}

pub async fn totp_confirm(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (u, _c) = active_company(&st, &session.claims.sub).await?;
    if !auth::verify_password(&str_val(&b, "current_password"), &u.password_hash) {
        return Err(unauth("грешна парола"));
    }
    let code = str_val(&b, "code");
    if code.is_empty() {
        return Err(unproc("кодът е задължителен"));
    }
    if u.totp_pending.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "няма започнато включване на 2FA",
        ));
    }
    let now = chrono::Utc::now().timestamp();
    let Some(counter) = totp::totp_counter(&u.totp_pending, &code, now) else {
        return Err(unauth("грешен код"));
    };
    let mut c = conn(&st).await?;
    if !totp_store::consume_counter(&mut c, u.id, counter).await? {
        return Err(unauth("грешен код"));
    }
    totp_store::set_totp(&mut c, u.id, true, &u.totp_pending, "").await?;
    Ok(Json(
        json!({"ok": 1, "totp_enabled": 1, "message": "2FA е включена"}),
    ))
}

pub async fn totp_disable(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (u, _c) = active_company(&st, &session.claims.sub).await?;
    let password = str_val(&b, "password");
    let code = str_val(&b, "code");
    if password.is_empty() || code.is_empty() {
        return Err(unproc("парола и код са задължителни"));
    }
    if !auth::verify_password(&password, &u.password_hash) {
        return Err(unauth("грешна парола"));
    }
    let now = chrono::Utc::now().timestamp();
    let mut c = conn(&st).await?;
    let mut ok = match totp::totp_counter(&u.totp_secret, &code, now) {
        Some(counter) => totp_store::consume_counter(&mut c, u.id, counter).await?,
        None => false,
    };
    if !ok {
        ok = totp_store::take_recovery(&mut c, u.id, &code).await?;
    }
    if !ok {
        return Err(unauth("грешен код"));
    }
    totp_store::set_totp(&mut c, u.id, false, "", "").await?;
    totp_store::wipe_recovery(&mut c, u.id).await?;
    Ok(Json(
        json!({"ok": 1, "totp_enabled": 0, "message": "2FA е изключена"}),
    ))
}
