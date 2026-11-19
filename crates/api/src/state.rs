//! Състояние на API-то и auth middleware.

use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use domain::auth::Claims;
use storage::DbPool;

use crate::errors::ApiError;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub jwt_secret: String,
    /// Фиктивен argon2 хеш за проверка при несъществуващ потребител (timing).
    pub dummy_hash: String,
}

#[derive(Clone)]
pub struct Session {
    pub claims: Claims,
}

/// Bearer токен → Claims, плюс проверка активност и версия на сесията.
pub async fn require_auth(
    State(st): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let token = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .ok_or_else(|| ApiError::unauthorized("липсва токен"))?;
    let claims = domain::auth::verify_token(&st.jwt_secret, token)
        .ok_or_else(|| ApiError::unauthorized("невалиден токен"))?;

    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let user = storage::users::by_login(&mut conn, &claims.sub).await?;
    match user {
        Some(u) if !u.is_active => {
            return Err(ApiError::forbidden("потребителят е неактивен"));
        }
        Some(u) if u.session_ver != claims.ver => {
            return Err(ApiError::unauthorized(
                "сесията е приключила — влезте отново",
            ));
        }
        _ => {}
    }

    req.extensions_mut().insert(Session { claims });
    Ok(next.run(req).await)
}
