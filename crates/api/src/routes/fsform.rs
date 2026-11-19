//! /v1/fs/lines и /v1/fs/formulas — каталог на бланките и формули по фирма.

use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use domain::{fsform, fsnom};
use storage::fsform as store;

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
fn bad(m: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, m)
}
fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}

#[derive(Deserialize)]
pub struct StQ {
    pub statement: Option<String>,
}

pub async fn lines(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<StQ>,
) -> Result<Json<Value>, ApiError> {
    let (_u, _c) = active_company(&st, &session.claims.sub).await?;
    let statement = q.statement.unwrap_or_default().trim().to_string();
    if statement.is_empty() {
        let items: Vec<Value> = fsnom::statements()
            .into_iter()
            .map(|(id, name)| json!({"id": id, "name": name}))
            .collect();
        return Ok(Json(json!({"items": items, "count": items.len()})));
    }
    if !fsnom::known(&statement) {
        return Err(bad("statement трябва да е balance/pl/cashflow/equity"));
    }
    let items: Vec<Value> = fsnom::lines(&statement).into_iter()
        .map(|l| json!({"code": l.code, "parent": l.parent, "kind": l.kind, "side": l.side, "name": l.name}))
        .collect();
    Ok(Json(
        json!({"statement": statement, "items": items, "count": items.len()}),
    ))
}

fn row_json(r: &store::FsFormula) -> Value {
    json!({"id": r.id, "company_id": r.company_id, "statement": r.statement, "line_code": r.line_code, "formula": r.formula})
}

fn known_write(s: &str) -> Result<&str, ApiError> {
    if fsnom::known(s) {
        Ok(s)
    } else {
        Err(unproc("statement трябва да е balance/pl/cashflow/equity"))
    }
}

pub async fn index(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<StQ>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let statement = known_write(q.statement.unwrap_or_default().trim())?.to_string();
    let mut c = conn(&st).await?;
    let rows = store::rows_of(&mut c, company.id, &statement).await?;
    let items: Vec<Value> = rows.iter().map(row_json).collect();
    Ok(Json(
        json!({"statement": statement, "items": items, "count": items.len()}),
    ))
}

pub async fn put(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let statement = known_write(
        body.get("statement")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim(),
    )?
    .to_string();
    let code = body
        .get("line_code")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if code.is_empty() || !fsnom::has_code(&statement, &code) {
        return Err(unproc("непозната статия"));
    }
    let formula = body
        .get("formula")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if let Err(e) = fsform::check(&formula) {
        return Err(unproc(&e));
    }
    let mut c = conn(&st).await?;
    let r = store::upsert(&mut c, company.id, &statement, &code, &formula).await?;
    Ok(Json(row_json(&r)))
}

pub async fn defaults(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let statement = known_write(
        body.get("statement")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim(),
    )?
    .to_string();
    let mut c = conn(&st).await?;
    if !store::rows_of(&mut c, company.id, &statement)
        .await?
        .is_empty()
    {
        return Err(unproc("вече има формули за този отчет. Изтрийте ги първо."));
    }
    let defs = fsnom::defaults(&statement);
    for (code, formula) in &defs {
        store::upsert(&mut c, company.id, &statement, code, formula).await?;
    }
    Ok(Json(json!({"statement": statement, "count": defs.len()})))
}

pub async fn clear(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let statement = known_write(
        body.get("statement")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim(),
    )?
    .to_string();
    let mut c = conn(&st).await?;
    let n = store::clear(&mut c, company.id, &statement).await?;
    Ok(Json(json!({"statement": statement, "count": n})))
}

pub async fn validate(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, _c) = active_company(&st, &session.claims.sub).await?;
    let formula = body
        .get("formula")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    match fsform::check(&formula) {
        Ok(tokens) => Ok(Json(json!({"ok": 1, "error": "", "tokens": tokens}))),
        Err(e) => Ok(Json(json!({"ok": 0, "error": e, "tokens": 0}))),
    }
}
