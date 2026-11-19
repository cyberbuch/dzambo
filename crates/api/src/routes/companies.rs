//! Фирми: списък (пагиниран), активна фирма.

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use storage::{companies, users};

use crate::{
    errors::ApiError,
    mappers,
    state::{AppState, Session},
};

#[derive(Deserialize)]
pub struct PageQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Serialize)]
pub struct PagedResponse {
    items: Vec<serde_json::Value>,
    count: i64,
    total: i64,
    limit: i64,
    offset: i64,
    has_more: bool,
}

fn paginate(items: Vec<serde_json::Value>, q: &PageQuery) -> PagedResponse {
    let total = items.len() as i64;
    let limit = q.limit.clamp(1, 500);
    let offset = q.offset.max(0);
    let page: Vec<_> = items
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();
    let count = page.len() as i64;
    PagedResponse {
        count,
        total,
        limit,
        offset,
        has_more: offset + count < total,
        items: page,
    }
}

async fn current_user(st: &AppState, sub: &str) -> Result<Option<storage::models::User>, ApiError> {
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    users::by_login(&mut conn, sub)
        .await
        .map_err(ApiError::from)
}

pub async fn list(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PageQuery>,
) -> Result<Json<PagedResponse>, ApiError> {
    let user = current_user(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let rows = match &user {
        Some(u) if u.is_super_admin => companies::list_all(&mut conn).await?,
        Some(u) => companies::list_for_user(&mut conn, u.id).await?,
        None => Vec::new(),
    };
    let items = rows
        .iter()
        .map(|row| serde_json::to_value(mappers::company_dto(row)).unwrap())
        .collect();
    Ok(Json(paginate(items, &q)))
}

pub async fn get_active(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = current_user(&st, &session.claims.sub).await?;
    let Some(user) = user else {
        return Err(ApiError::unauthorized("потребителят не съществува"));
    };
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    match companies::resolve_active(&mut conn, &session.claims.sub, user.id).await? {
        Some(row) => Ok(Json(
            serde_json::to_value(mappers::company_dto(&row)).unwrap(),
        )),
        None => Ok(Json(serde_json::json!({}))),
    }
}

#[derive(Deserialize)]
pub struct ActiveBody {
    pub company_id: i64,
}

pub async fn put_active(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<ActiveBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = current_user(&st, &session.claims.sub).await?;
    let Some(user) = user else {
        return Err(ApiError::unauthorized("потребителят не съществува"));
    };
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let memberships = companies::memberships_of(&mut conn, user.id).await?;
    if !memberships.iter().any(|m| m.company_id == body.company_id) {
        return Err(ApiError::not_found("нямате достъп до тази фирма"));
    }
    storage::settings::put(
        &mut conn,
        &storage::settings::active_company_key(&session.claims.sub),
        &body.company_id.to_string(),
    )
    .await?;
    let row = companies::by_id(&mut conn, body.company_id)
        .await?
        .ok_or_else(|| ApiError::not_found("фирмата не съществува"))?;
    Ok(Json(
        serde_json::to_value(mappers::company_dto(&row)).unwrap(),
    ))
}
