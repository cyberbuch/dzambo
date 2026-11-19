//! Правила за банково разнасяне: CRUD, defaults, preview, auto-allocate.

use axum::{extract::{Extension, Path, State}, http::StatusCode, Json};
use domain::dto::BankRuleDto;
use storage::{accounts, bank_rules};

use crate::{errors::ApiError, routes::{banks::post_with_account, common::active_company}, state::{AppState, Session}};

fn internal() -> ApiError { ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка") }

fn rule_dto(r: &bank_rules::Rule) -> BankRuleDto {
    BankRuleDto { id: r.id, name: r.name.clone(), keywords: r.keywords.clone(), direction: r.direction.clone(), priority: r.priority, is_active: r.is_active, account_id: r.account_id, counterpart_id: r.counterpart_id }
}

pub async fn index(State(st): State<AppState>, Extension(s): Extension<Session>) -> Result<Json<Vec<BankRuleDto>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    Ok(Json(bank_rules::list(&mut conn, c.id).await?.iter().map(rule_dto).collect()))
}

pub async fn create(State(st): State<AppState>, Extension(s): Extension<Session>, Json(b): Json<BankRuleDto>) -> Result<(StatusCode, Json<BankRuleDto>), ApiError> {
    if b.keywords.trim().is_empty() { return Err(ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "keywords е задължително")); }
    if !matches!(b.direction.as_str(), "any" | "debit" | "credit") { return Err(ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "direction е any, debit или credit")); }
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let r = bank_rules::create(&mut conn, c.id, &b.name, &b.keywords, &b.direction, b.priority, b.is_active, b.account_id, b.counterpart_id).await?;
    Ok((StatusCode::CREATED, Json(rule_dto(&r))))
}

pub async fn delete(State(st): State<AppState>, Extension(s): Extension<Session>, Path(id): Path<i64>) -> Result<StatusCode, ApiError> {
    let (_u, _c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    bank_rules::delete(&mut conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn defaults(State(st): State<AppState>, Extension(s): Extension<Session>) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let n = bank_rules::seed_defaults(&mut conn, c.id).await?;
    Ok(Json(serde_json::json!({"created": n})))
}

#[derive(serde::Deserialize)]
pub struct AcctBody { pub account_id: i64 }

fn tx_text(t: &storage::banks::BankTransaction) -> String {
    format!("{} {} {}", t.description, t.counterpart_name, t.reference)
}

pub async fn preview(State(st): State<AppState>, Extension(s): Extension<Session>, Json(b): Json<AcctBody>) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let rules = bank_rules::list(&mut conn, c.id).await?;
    let txs = storage::banks::transactions(&mut conn, c.id, b.account_id).await?;
    let mut out = Vec::new();
    for t in txs.iter().filter(|t| t.status != "posted") {
        let amount = format!("{:.2}", t.amount);
        if let Some(r) = bank_rules::find_rule(&rules, &tx_text(t), &amount) {
            out.push(serde_json::json!({"tx_id": t.id, "rule": r.name, "account_id": r.account_id}));
        }
    }
    Ok(Json(serde_json::json!({"matches": out})))
}

pub async fn auto_allocate(State(st): State<AppState>, Extension(s): Extension<Session>, Json(b): Json<AcctBody>) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let rules = bank_rules::list(&mut conn, c.id).await?;
    let accts = accounts::list(&mut conn, c.id).await?;
    let txs = storage::banks::transactions(&mut conn, c.id, b.account_id).await?;
    let mut posted = 0i64;
    let mut failed = 0i64;
    for t in txs.iter().filter(|t| t.status != "posted") {
        let amount = format!("{:.2}", t.amount);
        let Some(r) = bank_rules::find_rule(&rules, &tx_text(t), &amount) else { continue };
        if r.account_id <= 0 { continue; }
        let Some(number) = accts.iter().find(|a| a.id == r.account_id).map(|a| a.number.clone()) else { continue };
        match post_with_account(&mut conn, c.id, t, &number).await {
            Ok(_) => posted += 1,
            Err(_) => failed += 1,
        }
    }
    Ok(Json(serde_json::json!({"posted": posted, "failed": failed})))
}
