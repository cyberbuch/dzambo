//! POST /v1/auth/token — вход с sub/email + парола, връща JWT.

use axum::{extract::State, http::StatusCode, Json};

use domain::{
    auth,
    dto::{TokenBody, TokenResponse},
};
use storage::users;

use crate::{errors::ApiError, state::AppState};

pub async fn token(
    State(st): State<AppState>,
    Json(body): Json<TokenBody>,
) -> Result<Json<TokenResponse>, ApiError> {
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

    let (access_token, expires_in) =
        auth::issue_token(&st.jwt_secret, &user.email, user.session_ver)
            .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "bearer".to_string(),
        sub: user.email,
        expires_in,
    }))
}
