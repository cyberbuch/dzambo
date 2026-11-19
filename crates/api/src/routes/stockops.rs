//! /v1/stock/rules, /v1/material-issues, /v1/stock/writeoff|rebuild, /v1/locations.

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::{active_company, PageQuery, PageSlice};
use crate::state::{AppState, Session};
use storage::{access, periods, stockops};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}
fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}
fn conflict(m: &str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, m)
}
fn date(s: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| unproc("doc_date трябва да е YYYY-MM-DD"))
}

#[derive(Deserialize)]
pub struct RuleBody {
    pub kind: String,
    pub inventory_account_id: i64,
    pub offset_account_id: i64,
    pub clearing_account_id: Option<i64>,
}

pub async fn list_rules(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let rules = stockops::list_rules(&mut c, company.id).await?;
    let items: Vec<Value> = rules.iter().map(stockops::rule_json).collect();
    Ok(Json(json!({"items": items, "count": items.len()})))
}

pub async fn create_rule(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<RuleBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    if !matches!(b.kind.as_str(), "goods" | "production" | "material") {
        return Err(unproc("kind трябва да е goods, production или material"));
    }
    if b.inventory_account_id == 0 || b.offset_account_id == 0 {
        return Err(unproc("inventory и offset сметките са задължителни"));
    }
    let mut c = conn(&st).await?;
    let r = stockops::create_rule(
        &mut c,
        company.id,
        &b.kind,
        b.inventory_account_id,
        b.offset_account_id,
        b.clearing_account_id.unwrap_or(0),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(stockops::rule_json(&r))))
}

pub async fn delete_rule(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    if stockops::delete_rule(&mut c, company.id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("правилото не е намерено"))
    }
}

pub async fn list_locations(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let items = stockops::list_locations(&mut c, company.id).await?;
    Ok(Json(json!({"items": items, "count": items.len()})))
}

#[derive(Deserialize)]
pub struct LocationBody {
    pub name: String,
}

pub async fn create_location(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<LocationBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let l = stockops::create_location(&mut c, company.id, &b.name).await?;
    Ok((StatusCode::CREATED, Json(l)))
}

#[derive(Deserialize)]
pub struct IssueBody {
    pub doc_date: String,
    pub location_id: Option<i64>,
    pub notes: Option<String>,
    pub lines: Option<Vec<Value>>,
}

pub async fn list_issues(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let rows = stockops::list_issues(&mut c, company.id).await?;
    let items: Vec<Value> = rows
        .iter()
        .map(|(i, l)| stockops::issue_json(i, l))
        .collect();
    let slice = PageSlice::from_vec(items, &q);
    Ok(Json(
        json!({"items": slice.items, "count": slice.count(), "total": slice.total, "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more()}),
    ))
}

pub async fn show_issue(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    match stockops::get_issue(&mut c, company.id, id).await? {
        Some((i, l)) => Ok(Json(stockops::issue_json(&i, &l))),
        None => Err(ApiError::not_found("документът не е намерен")),
    }
}

pub async fn create_issue(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<IssueBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let (i, l) = stockops::create_issue(
        &mut c,
        company.id,
        date(&b.doc_date)?,
        b.location_id.unwrap_or(0),
        &b.notes.unwrap_or_default(),
        &b.lines.unwrap_or_default(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(stockops::issue_json(&i, &l))))
}

pub async fn update_issue(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(b): Json<IssueBody>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let Some((i, _)) = stockops::get_issue(&mut c, company.id, id).await? else {
        return Err(ApiError::not_found("документът не е намерен"));
    };
    if i.status != "draft" {
        return Err(conflict("само чернова се редактира"));
    }
    if let Some(lines) = &b.lines {
        diesel::delete(
            storage::schema::material_issue_lines::table
                .filter(storage::schema::material_issue_lines::doc_id.eq(id)),
        )
        .execute(&mut c)
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        stockops::insert_issue_lines(&mut c, id, lines).await?;
    }
    let (i, l) = stockops::get_issue(&mut c, company.id, id).await?.unwrap();
    Ok(Json(stockops::issue_json(&i, &l)))
}

pub async fn delete_issue(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let Some((i, _)) = stockops::get_issue(&mut c, company.id, id).await? else {
        return Err(ApiError::not_found("документът не е намерен"));
    };
    if i.status != "draft" {
        return Err(conflict("само чернова се трие"));
    }
    stockops::delete_issue(&mut c, company.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_status(c: &mut storage::DbConn, id: i64, status: &str) -> Result<(), ApiError> {
    diesel::update(storage::schema::material_issues::table.find(id))
        .set(storage::schema::material_issues::status.eq(status))
        .execute(c)
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(())
}

pub async fn confirm_issue(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let (u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let Some((i, l)) = stockops::get_issue(&mut c, company.id, id).await? else {
        return Err(ApiError::not_found("документът не е намерен"));
    };
    if i.location_id == 0 || l.is_empty() {
        return Err(unproc("нужни са обект и редове"));
    }
    if periods::is_closed(&mut c, company.id, &periods::month_key(i.doc_date)).await? {
        return Err(conflict("периодът е затворен"));
    }
    set_status(&mut c, id, "confirmed").await?;
    let res = stockops::rebuild(&mut c, company.id).await?;
    let (i, l) = stockops::get_issue(&mut c, company.id, id).await?.unwrap();
    let _ = access::record(
        &mut c,
        company.id,
        u.id,
        &u.email,
        &u.name,
        "material_issue",
        i.id,
        &i.number,
        "post",
        &format!("Изписване {}", i.number),
        "",
        "",
        None,
        Some(stockops::issue_json(&i, &l)),
    )
    .await;
    Ok(Json(
        json!({"doc": stockops::issue_json(&i, &l), "warnings": res["warnings"]}),
    ))
}

pub async fn unpost_issue(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let (u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let Some((_i, _)) = stockops::get_issue(&mut c, company.id, id).await? else {
        return Err(ApiError::not_found("документът не е намерен"));
    };
    set_status(&mut c, id, "draft").await?;
    let res = stockops::rebuild(&mut c, company.id).await?;
    let (i, l) = stockops::get_issue(&mut c, company.id, id).await?.unwrap();
    let _ = access::record(
        &mut c,
        company.id,
        u.id,
        &u.email,
        &u.name,
        "material_issue",
        i.id,
        &i.number,
        "unpost",
        &format!("Отписване {}", i.number),
        "",
        "",
        None,
        None,
    )
    .await;
    Ok(Json(
        json!({"doc": stockops::issue_json(&i, &l), "warnings": res["warnings"]}),
    ))
}

#[derive(Deserialize)]
pub struct WriteoffBody {
    pub month: Option<String>,
    pub until: Option<String>,
}

pub async fn writeoff(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<WriteoffBody>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let month = match (&b.month, &b.until) {
        (Some(m), None) => m.clone(),
        (None, Some(u)) if u.len() >= 7 => u[..7].to_string(),
        _ => return Err(unproc("подайте month или until")),
    };
    let mut c = conn(&st).await?;
    let res = stockops::writeoff(&mut c, company.id, &month).await?;
    Ok(Json(res))
}

#[derive(Deserialize)]
pub struct RebuildBody {
    pub until: Option<String>,
}

pub async fn rebuild(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(_b): Json<RebuildBody>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let res = stockops::rebuild(&mut c, company.id).await?;
    Ok(Json(res))
}
