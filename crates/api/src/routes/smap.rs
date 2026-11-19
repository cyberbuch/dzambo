//! /v1/saft/movement-mappings — Дт/Кт кореспонденции към тип движение в SAF-T.

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use domain::{noms, smap_def};
use storage::smap::{self, Smap};

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
fn kind_ok(k: &str) -> bool {
    matches!(k, "stock" | "asset" | "cash")
}

fn type_rows(kind: &str) -> Vec<String> {
    match kind {
        "cash" => smap_def::CASH_TYPES.iter().map(|s| s.to_string()).collect(),
        "asset" => noms::entries("asset_movements").clone(),
        _ => noms::entries("stock_movements").clone(),
    }
}

fn type_known(kind: &str, code: &str) -> bool {
    type_rows(kind)
        .iter()
        .any(|l| l.split('|').next() == Some(code))
}

fn type_name(kind: &str, code: &str) -> String {
    type_rows(kind)
        .iter()
        .find_map(|l| {
            let mut p = l.splitn(3, '|');
            let c = p.next().unwrap_or("");
            let name = p.next().unwrap_or("");
            (c == code).then(|| name.to_string())
        })
        .unwrap_or_else(|| code.to_string())
}

fn types_json(kind: &str) -> Vec<Value> {
    type_rows(kind)
        .iter()
        .map(|l| {
            let mut p = l.splitn(3, '|');
            json!({"code": p.next().unwrap_or(""), "name": p.next().unwrap_or(""), "name_en": p.next().unwrap_or("")})
        })
        .collect()
}

fn row_json(s: &Smap) -> Value {
    json!({
        "id": s.id, "kind": s.kind, "type_code": s.type_code,
        "type_name": type_name(&s.kind, &s.type_code),
        "debit_account": s.debit_account, "credit_account": s.credit_account,
        "debit_analytical": s.debit_analytical, "credit_analytical": s.credit_analytical,
        "description": s.description, "is_active": if s.is_active { 1 } else { 0 },
    })
}

#[derive(Deserialize)]
pub struct KindQuery {
    pub kind: Option<String>,
}

pub async fn index(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<KindQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let kind = q.kind.unwrap_or_default().trim().to_string();
    if !kind_ok(&kind) {
        return Err(bad("непознат вид кореспонденция (kind=stock|asset|cash)"));
    }
    let mut c = conn(&st).await?;
    let rows = smap::list(&mut c, company.id, &kind).await?;
    let items: Vec<Value> = rows.iter().map(row_json).collect();
    Ok(Json(
        json!({"kind": kind, "items": items, "count": items.len(), "types": types_json(&kind)}),
    ))
}

fn accounts(b: &Value) -> Result<(String, String, String, String, String), ApiError> {
    let debit = b
        .get("debit_account")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let credit = b
        .get("credit_account")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if debit.is_empty() || credit.is_empty() {
        return Err(unproc("дебит и кредит сметките са задължителни"));
    }
    let g = |k: &str| {
        b.get(k)
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string()
    };
    Ok((
        debit,
        credit,
        g("debit_analytical"),
        g("credit_analytical"),
        g("description"),
    ))
}

pub async fn create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let kind = b
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if !kind_ok(&kind) {
        return Err(unproc(
            "непознат вид кореспонденция (kind=stock|asset|cash)",
        ));
    }
    let code = b
        .get("type_code")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if !type_known(&kind, &code) {
        return Err(unproc("непознат тип движение за този вид"));
    }
    let (debit, credit, dan, kan, desc) = accounts(&b)?;
    let mut c = conn(&st).await?;
    let s = smap::create(
        &mut c, company.id, &kind, &code, &debit, &credit, &dan, &kan, &desc,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(row_json(&s))))
}

pub async fn update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let Some(mut s) = smap::get(&mut c, company.id, id).await? else {
        return Err(ApiError::not_found("кореспонденцията не е намерена"));
    };
    if let Some(code) = b.get("type_code").and_then(Value::as_str) {
        let code = code.trim();
        if !type_known(&s.kind, code) {
            return Err(unproc("непознат тип движение за този вид"));
        }
        s.type_code = code.to_string();
    }
    for (field, slot) in [
        ("debit_account", &mut s.debit_account),
        ("credit_account", &mut s.credit_account),
        ("debit_analytical", &mut s.debit_analytical),
        ("credit_analytical", &mut s.credit_analytical),
        ("description", &mut s.description),
    ] {
        if let Some(v) = b.get(field).and_then(Value::as_str) {
            if (field == "debit_account" || field == "credit_account") && v.trim().is_empty() {
                return Err(unproc("дебит и кредит сметките са задължителни"));
            }
            *slot = v.trim().to_string();
        }
    }
    if let Some(flag) = b.get("is_active") {
        s.is_active = match flag {
            Value::Bool(x) => *x,
            Value::Number(n) => n.as_i64().unwrap_or(1) != 0,
            Value::String(x) => x.trim() != "0",
            _ => s.is_active,
        };
    }
    smap::update(&mut c, id, &s).await?;
    Ok(Json(row_json(&s)))
}

pub async fn delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    if smap::delete(&mut c, company.id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("кореспонденцията не е намерена"))
    }
}

pub async fn defaults(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let kind = b
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if !kind_ok(&kind) {
        return Err(unproc(
            "непознат вид кореспонденция (kind=stock|asset|cash)",
        ));
    }
    let mut c = conn(&st).await?;
    if !smap::list(&mut c, company.id, &kind).await?.is_empty() {
        return Err(unproc(
            "вече има кореспонденции за този вид. Изтрийте ги първо.",
        ));
    }
    let mut n = 0;
    for line in smap_def::rows(&kind) {
        let mut p = line.splitn(4, '|');
        let code = p.next().unwrap_or("");
        let debit = p.next().unwrap_or("");
        let credit = p.next().unwrap_or("");
        let desc = p.next().unwrap_or("");
        if code.is_empty() {
            continue;
        }
        smap::create(&mut c, company.id, &kind, code, debit, credit, "", "", desc).await?;
        n += 1;
    }
    Ok(Json(json!({"kind": kind, "count": n})))
}
