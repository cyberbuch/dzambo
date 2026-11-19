//! /v1/roles, /v1/audit-logs — реална логика.

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::{active_company, PageQuery, PageSlice};
use crate::state::{AppState, Session};
use storage::access;

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}
fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}

#[derive(Deserialize)]
pub struct RoleBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Value>,
}

pub async fn list_roles(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<Value>, ApiError> {
    let (_u, _c) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let roles = access::list_roles(&mut c).await?;
    let items: Vec<Value> = roles.iter().map(access::role_json).collect();
    Ok(Json(json!({"items": items, "count": items.len()})))
}

pub async fn create_role(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<RoleBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (u, _c) = active_company(&st, &session.claims.sub).await?;
    if !u.is_super_admin {
        return Err(ApiError::forbidden("само супер администратор"));
    }
    let name = body.name.unwrap_or_default();
    if name.trim().is_empty() {
        return Err(unproc("name е задължително"));
    }
    let mut c = conn(&st).await?;
    let r = access::create_role(
        &mut c,
        &name,
        &body.description.unwrap_or_default(),
        body.permissions,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(access::role_json(&r))))
}

pub async fn update_role(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(body): Json<RoleBody>,
) -> Result<Json<Value>, ApiError> {
    let (u, _c) = active_company(&st, &session.claims.sub).await?;
    if !u.is_super_admin {
        return Err(ApiError::forbidden("само супер администратор"));
    }
    let mut c = conn(&st).await?;
    match access::update_role(
        &mut c,
        id,
        body.name.as_deref(),
        body.description.as_deref(),
        body.permissions,
    )
    .await?
    {
        Some(r) => Ok(Json(access::role_json(&r))),
        None => Err(ApiError::not_found("ролята не е намерена")),
    }
}

pub async fn delete_role(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (u, _c) = active_company(&st, &session.claims.sub).await?;
    if !u.is_super_admin {
        return Err(ApiError::forbidden("само супер администратор"));
    }
    let mut c = conn(&st).await?;
    if access::delete_role(&mut c, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("ролята не е намерена"))
    }
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub action: Option<String>,
    pub user: Option<String>,
    pub q: Option<String>,
    pub changed_only: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_audit(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let rows =
        access::list_audit(&mut c, company.id, q.entity_type.as_deref(), q.entity_id).await?;
    let mut items: Vec<Value> = rows.iter().map(access::audit_json).collect();
    if let Some(a) = &q.action {
        items.retain(|v| v["action"].as_str() == Some(a.as_str()));
    }
    if let Some(u) = &q.user {
        let lu = u.to_lowercase();
        items.retain(|v| {
            v["user_login"]
                .as_str()
                .unwrap_or("")
                .to_lowercase()
                .contains(&lu)
                || v["user_name"]
                    .as_str()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&lu)
        });
    }
    if let Some(search) = &q.q {
        let ls = search.to_lowercase();
        items.retain(|v| v.to_string().to_lowercase().contains(&ls));
    }
    if matches!(
        q.changed_only.as_deref(),
        Some("1") | Some("true") | Some("yes")
    ) {
        items.retain(|v| {
            v["amount_changed"].as_bool() == Some(true)
                || matches!(
                    v["action"].as_str(),
                    Some("update") | Some("delete") | Some("unpost")
                )
        });
    }
    let page = PageQuery {
        limit: q.limit.unwrap_or(50),
        offset: q.offset.unwrap_or(0),
    };
    let slice = PageSlice::from_vec(items, &page);
    Ok(Json(json!({
        "items": slice.items, "count": slice.count(), "total": slice.total,
        "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more(),
    })))
}

pub async fn audit_for(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path((entity, id)): Path<(String, i64)>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let rows = access::list_audit(&mut c, company.id, Some(&entity), Some(id)).await?;
    let items: Vec<Value> = rows.iter().map(access::audit_json).collect();
    Ok(Json(json!({"items": items, "count": items.len()})))
}
