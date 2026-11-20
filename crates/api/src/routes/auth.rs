//! POST /v1/auth/token — вход с sub/email + парола, връща JWT (или mfa_token при 2FA).
//! POST /v1/auth/mfa — втора факторна стъпка (TOTP или резервен код).

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use domain::{auth, totp};
use storage::{totp_store, users};

use crate::{errors::ApiError, state::AppState};

const MFA_MAX_FAILS: i32 = 5;
const MFA_LOCK_SEC: i64 = 15 * 60;

pub async fn token(
    State(st): State<AppState>,
    Json(body): Json<domain::dto::TokenBody>,
) -> Result<Json<Value>, ApiError> {
    let login = body
        .sub
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| body.email.as_deref().filter(|s| !s.is_empty()))
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "email или sub е задължителен",
            )
        })?;

    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;

    let user = users::by_login(&mut conn, login).await?;
    // При липсващ потребител се прави фиктивна argon2 проверка —
    // отговорът не лича по време за несъществуващ логин.
    let password_ok = match &user {
        Some(u) => auth::verify_password(&body.password, &u.password_hash),
        None => {
            auth::verify_password(&body.password, &st.dummy_hash);
            false
        }
    };

    let Some(user) = user else {
        return Err(ApiError::unauthorized("грешен потребител или парола"));
    };
    if !password_ok {
        return Err(ApiError::unauthorized("грешен потребител или парола"));
    }
    if !user.is_active {
        return Err(ApiError::forbidden("потребителят е неактивен"));
    }

    if user.totp_enabled {
        let mfa = auth::issue_mfa(&st.jwt_secret, &user.email)
            .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        return Ok(Json(json!({
            "mfa": "totp", "mfa_token": mfa, "expires_in": auth::MFA_TTL_SECS,
            "message": "Въведи кода от приложението или резервен код",
        })));
    }

    let (access_token, expires_in) =
        auth::issue_token(&st.jwt_secret, &user.email, user.session_ver)
            .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(json!({
        "access_token": access_token, "token_type": "bearer",
        "sub": user.email, "expires_in": expires_in,
    })))
}

#[derive(Deserialize)]
pub struct MfaBody {
    pub mfa_token: Option<String>,
    pub code: Option<String>,
}

pub async fn mfa(
    State(st): State<AppState>,
    Json(body): Json<MfaBody>,
) -> Result<Json<Value>, ApiError> {
    let raw = body.mfa_token.unwrap_or_default();
    if raw.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "mfa_token е задължителен",
        ));
    }
    let code = body.code.unwrap_or_default();
    if code.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "кодът е задължителен",
        ));
    }
    let Some(sub) = auth::mfa_subject(&st.jwt_secret, raw.trim()) else {
        return Err(ApiError::unauthorized("невалиден или изтекъл mfa_token"));
    };
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let Some(user) = users::by_login(&mut conn, &sub).await? else {
        return Err(ApiError::unauthorized("грешен код"));
    };
    let now = chrono::Utc::now().timestamp();
    if user.mfa_lock_until > now {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "2FA е заключен за 15 минути след неуспешни опити",
        ));
    }
    let mut ok = match totp::totp_counter(&user.totp_secret, code.trim(), now) {
        Some(counter) => totp_store::consume_counter(&mut conn, user.id, counter).await?,
        None => false,
    };
    if !ok {
        ok = totp_store::take_recovery(&mut conn, user.id, code.trim()).await?;
    }
    if ok {
        totp_store::mfa_reset(&mut conn, user.id).await?;
        let (access_token, expires_in) =
            auth::issue_token(&st.jwt_secret, &user.email, user.session_ver)
                .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        return Ok(Json(json!({
            "access_token": access_token, "token_type": "bearer",
            "sub": user.email, "expires_in": expires_in,
        })));
    }
    let fails = user.mfa_fails + 1;
    totp_store::mfa_failure(&mut conn, user.id).await?;
    if fails >= MFA_MAX_FAILS {
        totp_store::mfa_lock(&mut conn, user.id, now + MFA_LOCK_SEC).await?;
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "изчерпани опити за 2FA — заключено за 15 минути",
        ));
    }
    Err(ApiError::unauthorized("грешен код"))
}
