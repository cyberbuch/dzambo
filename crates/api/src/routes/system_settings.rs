//! /v1/system-settings — глобални настройки (CRUD) + интегритет на базата.
//! SMTP/S3 не са конфигурирани в тази версия — връщат честно съобщение.

use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::Json;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Instant;

use crate::errors::ApiError;
use crate::routes::common::{active_company, PageQuery, PageSlice};
use crate::state::{AppState, Session};
use storage::settings;

const SECRET_MASK: &str = "••••••••";

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}
fn is_private(k: &str) -> bool {
    k == "auth.jwt_secret" || k.starts_with("active_company:") || k.starts_with("totp_last:")
}
fn is_secret(k: &str) -> bool {
    matches!(k, "smtp.password" | "s3.secret_key")
}

async fn require_admin(st: &AppState, sub: &str) -> Result<(), ApiError> {
    let (u, _c) = active_company(st, sub).await?;
    if !u.is_super_admin {
        return Err(ApiError::forbidden("само супер администратор"));
    }
    Ok(())
}

fn items_json(rows: &[(String, String)]) -> Vec<Value> {
    rows.iter()
        .filter(|(k, _)| !is_private(k))
        .map(|(k, v)| {
            let value = if is_secret(k) && !v.is_empty() {
                SECRET_MASK.to_string()
            } else {
                v.clone()
            };
            json!({"key": k, "value": value})
        })
        .collect()
}

pub async fn index(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let rows = settings::all(&mut c).await?;
    let slice = PageSlice::from_vec(items_json(&rows), &q);
    Ok(Json(
        json!({"items": slice.items, "count": slice.count(), "total": slice.total, "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more()}),
    ))
}

pub async fn update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PageQuery>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&st, &session.claims.sub).await?;
    let items = body
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "items е задължителен масив"))?;
    let mut c = conn(&st).await?;
    for item in items {
        let key = item.get("key").and_then(Value::as_str).unwrap_or("").trim();
        if key.is_empty() || is_private(key) {
            continue;
        }
        let value = match item.get("value") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Bool(b)) => {
                if *b {
                    "1".into()
                } else {
                    "0".into()
                }
            }
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Null) | None => String::new(),
            _ => {
                return Err(ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!("{key} трябва да е низ"),
                ))
            }
        };
        if is_secret(key) && (value.is_empty() || value == SECRET_MASK) {
            continue;
        }
        settings::put(&mut c, key, &value).await?;
    }
    let rows = settings::all(&mut c).await?;
    let slice = PageSlice::from_vec(items_json(&rows), &q);
    Ok(Json(
        json!({"items": slice.items, "count": slice.count(), "total": slice.total, "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more()}),
    ))
}

// ---- Интегритет на базата ----

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    n: i64,
}

async fn count(c: &mut storage::DbConn, sql: &str) -> Result<i64, ApiError> {
    let row: Count = diesel::sql_query(sql).get_result(c).await.map_err(|e| {
        tracing::error!("db-check: {e}");
        internal()
    })?;
    Ok(row.n)
}

fn check(id: &str, ok: bool, detail: impl Into<String>) -> Value {
    json!({"id": id, "ok": ok, "detail": detail.into()})
}

const TABLES: &[&str] = &[
    "companies",
    "accounts",
    "counterparts",
    "products",
    "invoices",
    "invoice_lines",
    "journal_entries",
    "journal_lines",
    "users",
    "roles",
    "user_companies",
    "bank_accounts",
    "bank_transactions",
    "fixed_assets",
    "material_issues",
    "internal_docs",
    "production_orders",
    "recipes",
    "dividends",
    "opening_balances",
    "accounting_periods",
    "audit_logs",
    "settings",
];

pub async fn db_check(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&st, &session.claims.sub).await?;
    let started = Instant::now();
    let mut c = conn(&st).await?;
    let mut checks = vec![check("connection", true, "")];
    let mut tables = Vec::new();
    let mut table_errors = 0;
    for name in TABLES {
        match count(&mut c, &format!("SELECT COUNT(*) AS n FROM {name}")).await {
            Ok(n) => tables.push(json!({"name": name, "rows": n})),
            Err(_) => {
                table_errors += 1;
                tables.push(json!({"name": name, "rows": 0}));
            }
        }
    }
    checks.push(check(
        "tables",
        table_errors == 0,
        format!("{}/{}", TABLES.len() - table_errors, TABLES.len()),
    ));

    let oi = count(
        &mut c,
        "SELECT COUNT(*) AS n FROM invoices WHERE company_id NOT IN (SELECT id FROM companies)",
    )
    .await?;
    checks.push(check("orphan_invoices", oi == 0, oi.to_string()));
    let oj = count(&mut c, "SELECT COUNT(*) AS n FROM journal_entries WHERE company_id NOT IN (SELECT id FROM companies)").await?;
    checks.push(check("orphan_journal_entries", oj == 0, oj.to_string()));
    let oa = count(
        &mut c,
        "SELECT COUNT(*) AS n FROM accounts WHERE company_id NOT IN (SELECT id FROM companies)",
    )
    .await?;
    checks.push(check("orphan_accounts", oa == 0, oa.to_string()));
    let op = count(
        &mut c,
        "SELECT COUNT(*) AS n FROM products WHERE company_id NOT IN (SELECT id FROM companies)",
    )
    .await?;
    checks.push(check("orphan_products", op == 0, op.to_string()));
    let oil = count(
        &mut c,
        "SELECT COUNT(*) AS n FROM invoice_lines WHERE invoice_id NOT IN (SELECT id FROM invoices)",
    )
    .await?;
    checks.push(check("orphan_invoice_lines", oil == 0, oil.to_string()));
    let ojl = count(&mut c, "SELECT COUNT(*) AS n FROM journal_lines WHERE entry_id NOT IN (SELECT id FROM journal_entries)").await?;
    checks.push(check("orphan_journal_lines", ojl == 0, ojl.to_string()));

    let bad = count(&mut c, "SELECT COUNT(*) AS n FROM (SELECT entry_id FROM journal_lines GROUP BY entry_id HAVING SUM(CASE WHEN LOWER(direction) LIKE 'd%' THEN amount ELSE 0 END) <> SUM(CASE WHEN LOWER(direction) LIKE 'c%' THEN amount ELSE 0 END)) t").await?;
    checks.push(check("journal_balance", bad == 0, bad.to_string()));

    let ok = checks.iter().all(|c| c["ok"].as_bool().unwrap_or(false));
    Ok(Json(json!({
        "ok": ok, "checked_at": chrono::Utc::now().to_rfc3339(),
        "duration_ms": started.elapsed().as_millis() as u64,
        "checks": checks, "tables": tables,
    })))
}

// ---- SMTP / S3 (не са конфигурирани в тази версия) ----

pub async fn smtp_test(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&st, &session.claims.sub).await?;
    Ok(Json(
        json!({"message": "SMTP не е конфигуриран в тази версия — задайте smtp.* и външен SMTP сървър."}),
    ))
}

pub async fn s3_test(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&st, &session.claims.sub).await?;
    Ok(Json(
        json!({"message": "S3 не е конфигуриран в тази версия — задайте s3.* и външно S3 хранилище."}),
    ))
}

pub async fn s3_backup(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&st, &session.claims.sub).await?;
    Ok(Json(
        json!({"message": "S3 backup не е наличен — конфигурирайте s3.* първо."}),
    ))
}

pub async fn s3_backups(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&st, &session.claims.sub).await?;
    Ok(Json(json!({"items": []})))
}

#[derive(Deserialize)]
pub struct S3DeleteBody {
    pub s3_key: Option<String>,
}

pub async fn s3_delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(_b): Json<S3DeleteBody>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&st, &session.claims.sub).await?;
    Ok(Json(
        json!({"message": "S3 не е конфигуриран — няма какво да се трие."}),
    ))
}
