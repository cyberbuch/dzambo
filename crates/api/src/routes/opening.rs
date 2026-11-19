//! Начални салда: списък по година, запис, изтриване.

use axum::{extract::{Extension, Path, Query, State}, http::StatusCode, Json};
use bigdecimal::BigDecimal;
use chrono::Datelike;
use domain::dto::OpeningDto;

use crate::{errors::ApiError, routes::common::active_company, state::{AppState, Session}};

#[derive(serde::Deserialize)]
pub struct YearQuery { #[serde(default = "default_year")] year: i32 }
fn default_year() -> i32 { chrono::Utc::now().year() }

pub async fn list(State(st): State<AppState>, Extension(s): Extension<Session>, Query(q): Query<YearQuery>) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, company) = active_company(&st, &s.claims.sub).await?;
    let mut c = st.pool.get().await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let rows = storage::opening::list(&mut c, company.id, q.year).await?;
    let items: Vec<_> = rows.iter().map(|r| serde_json::json!({"id": r.id, "account_id": r.account_id, "year": r.year, "debit": format!("{:.2}", r.debit), "credit": format!("{:.2}", r.credit)})).collect();
    Ok(Json(serde_json::json!({"items": items, "count": items.len()})))
}

pub async fn save(State(st): State<AppState>, Extension(s): Extension<Session>, Json(b): Json<OpeningDto>) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, company) = active_company(&st, &s.claims.sub).await?;
    let mut c = st.pool.get().await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let dec = |v: &str| v.trim().parse::<BigDecimal>().unwrap_or_default();
    storage::opening::upsert(&mut c, company.id, b.account_id, b.year, dec(&b.debit), dec(&b.credit)).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn delete(State(st): State<AppState>, Extension(s): Extension<Session>, Path(id): Path<i64>) -> Result<StatusCode, ApiError> {
    let (_u, _company) = active_company(&st, &s.claims.sub).await?;
    let mut c = st.pool.get().await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    storage::opening::delete(&mut c, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
