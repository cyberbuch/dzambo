//! GET /v1/me — текущият потребител и членствата му.

use axum::{
    extract::{Extension, State},
    Json,
};
use serde_json::json;

use storage::{companies, users};

use crate::{
    errors::ApiError,
    mappers,
    state::{AppState, Session},
};

pub async fn me(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut conn = st.pool.get().await.map_err(|_| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "вътрешна грешка",
        )
    })?;
    let Some(user) = users::by_login(&mut conn, &session.claims.sub).await? else {
        return Ok(Json(json!({"sub": session.claims.sub, "permissions": []})));
    };
    let memberships = companies::memberships_of(&mut conn, user.id).await?;
    Ok(Json(
        serde_json::to_value(mappers::me_dto(&user, &memberships)).unwrap(),
    ))
}
