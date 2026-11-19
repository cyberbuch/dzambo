//! Съвместими read-endpoint-и за оригиналния фронт (празни/реални списъци).

use crate::{
    errors::ApiError,
    routes::common::active_company,
    state::{AppState, Session},
};
use axum::{
    extract::{Extension, State},
    Json,
};

fn empty() -> Json<serde_json::Value> {
    Json(
        serde_json::json!({"items": [], "count": 0, "total": 0, "limit": 50, "offset": 0, "has_more": false}),
    )
}

pub async fn nomenclatures(
    State(_st): State<AppState>,
    Extension(_s): Extension<Session>,
    q: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(crate::routes::nomenclatures::index(q).await)
}
pub async fn exchange_rates(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = active_company(&st, &s.claims.sub).await?;
    Ok(empty())
}
pub async fn audit_logs(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = active_company(&st, &s.claims.sub).await?;
    Ok(empty())
}
pub async fn roles(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = active_company(&st, &s.claims.sub).await?;
    Ok(empty())
}
pub async fn internal_docs(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = active_company(&st, &s.claims.sub).await?;
    Ok(empty())
}
pub async fn material_issues(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = active_company(&st, &s.claims.sub).await?;
    Ok(empty())
}
pub async fn stock_rules(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = active_company(&st, &s.claims.sub).await?;
    Ok(empty())
}
pub async fn fs_formulas(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = active_company(&st, &s.claims.sub).await?;
    Ok(empty())
}
pub async fn movement_mappings(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = active_company(&st, &s.claims.sub).await?;
    Ok(empty())
}
pub async fn system_settings(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = active_company(&st, &s.claims.sub).await?;
    Ok(Json(serde_json::json!({})))
}
pub async fn users(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, _c) = active_company(&st, &s.claims.sub).await?;
    let mut c = st.pool.get().await.map_err(|_| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "вътрешна грешка",
        )
    })?;
    let rows = storage::users::list_all(&mut c).await?;
    let items: Vec<serde_json::Value>=rows.iter().map(|u| serde_json::json!({"id":u.id,"email":u.email,"name":u.name,"is_super_admin":u.is_super_admin,"is_active":u.is_active})).collect();
    let n = items.len();
    Ok(Json(
        serde_json::json!({"items":items,"count":n,"total":n,"limit":50,"offset":0,"has_more":false}),
    ))
}
