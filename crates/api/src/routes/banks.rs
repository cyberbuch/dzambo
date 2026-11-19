//! Банки: сметки (списък/създаване), транзакции (списък/CSV импорт).

use axum::{extract::{Extension, Query, State}, http::StatusCode, Json};
use domain::dto::{BankAccountDto, BankTransactionDto};

use crate::{errors::ApiError, routes::common::active_company, state::{AppState, Session}};

fn internal() -> ApiError { ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка") }

pub async fn accounts(State(st): State<AppState>, Extension(s): Extension<Session>) -> Result<Json<Vec<BankAccountDto>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let rows = storage::banks::accounts(&mut conn, c.id).await?;
    Ok(Json(rows.iter().map(|a| BankAccountDto { id: a.id, name: a.name.clone(), iban: a.iban.clone(), currency: a.currency.clone() }).collect()))
}

pub async fn create_account(State(st): State<AppState>, Extension(s): Extension<Session>, Json(b): Json<BankAccountDto>) -> Result<(StatusCode, Json<BankAccountDto>), ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let a = storage::banks::create_account(&mut conn, c.id, &b.name, &b.iban, &b.currency).await?;
    Ok((StatusCode::CREATED, Json(BankAccountDto { id: a.id, name: a.name, iban: a.iban, currency: a.currency })))
}

#[derive(serde::Deserialize)]
pub struct TxQuery { pub account_id: i64 }

pub async fn transactions(State(st): State<AppState>, Extension(s): Extension<Session>, Query(q): Query<TxQuery>) -> Result<Json<Vec<BankTransactionDto>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let rows = storage::banks::transactions(&mut conn, c.id, q.account_id).await?;
    Ok(Json(rows.iter().map(|t| BankTransactionDto { id: t.id, tx_date: t.tx_date.to_string(), amount: format!("{:.2}", t.amount), direction: t.direction.clone(), counterpart_name: t.counterpart_name.clone(), description: t.description.clone(), reference: t.reference.clone(), status: t.status.clone() }).collect()))
}

#[derive(serde::Deserialize)]
pub struct ImportBody { pub account_id: i64, pub csv: String }

pub async fn import(State(st): State<AppState>, Extension(s): Extension<Session>, Json(b): Json<ImportBody>) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let n = storage::banks::import_csv(&mut conn, c.id, b.account_id, &b.csv).await?;
    Ok(Json(serde_json::json!({"imported": n})))
}
