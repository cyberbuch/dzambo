//! Приключвателни операции: preview и run.

use axum::{extract::{Extension, Query, State}, http::StatusCode, Json};
use domain::dto::ClosingRowDto;

use crate::{errors::ApiError, routes::common::active_company, state::{AppState, Session}};

#[derive(serde::Deserialize)]
pub struct MonthQuery { pub year: i32, pub month: i32 }

pub async fn preview(State(st): State<AppState>, Extension(s): Extension<Session>, Query(q): Query<MonthQuery>) -> Result<Json<Vec<ClosingRowDto>>, ApiError> {
    let (_u, company) = active_company(&st, &s.claims.sub).await?;
    let mut c = st.pool.get().await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let rows = storage::closing::preview(&mut c, company.id, q.year, q.month).await?;
    Ok(Json(rows.iter().map(|r| ClosingRowDto { source: r.source.clone(), source_name: r.source_name.clone(), target: r.target.clone(), amount: format!("{:.2}", r.amount) }).collect()))
}

pub async fn run(State(st): State<AppState>, Extension(s): Extension<Session>, Json(q): Json<MonthQuery>) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, company) = active_company(&st, &s.claims.sub).await?;
    let mut c = st.pool.get().await.map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let id = storage::closing::run(&mut c, company.id, q.year, q.month).await
        .map_err(|e| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true, "entry_id": id})))
}
