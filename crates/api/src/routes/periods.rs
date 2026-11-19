//! Счетоводни периоди: списък, затваряне, отваряне.

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Datelike;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use domain::dto::PeriodDto;
use storage::periods;

use crate::{
    errors::ApiError,
    routes::common::active_company,
    state::{AppState, Session},
};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}

async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}

#[derive(serde::Deserialize)]
pub struct YearQuery {
    #[serde(default = "default_year")]
    year: i32,
}

fn default_year() -> i32 {
    chrono::Utc::now().year()
}

pub async fn index(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<YearQuery>,
) -> Result<Json<Vec<PeriodDto>>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let rows = storage::schema::accounting_periods::table
        .filter(storage::schema::accounting_periods::company_id.eq(company.id))
        .filter(storage::schema::accounting_periods::year.eq(q.year))
        .select((
            storage::schema::accounting_periods::year,
            storage::schema::accounting_periods::month,
            storage::schema::accounting_periods::status,
        ))
        .load::<(i32, i32, String)>(&mut c)
        .await?;
    let out: Vec<PeriodDto> = (1..=12)
        .map(|m| PeriodDto {
            year: q.year,
            month: m,
            status: rows
                .iter()
                .find(|(_, mm, _)| *mm == m)
                .map(|(_, _, s)| s.clone())
                .unwrap_or_else(|| "open".to_string()),
        })
        .collect();
    Ok(Json(out))
}

#[derive(serde::Deserialize)]
pub struct MonthBody {
    pub year: i32,
    pub month: i32,
    #[serde(default)]
    pub notes: String,
}

pub async fn close(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<MonthBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    periods::close(&mut c, company.id, body.year, body.month, &body.notes).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn reopen(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<MonthBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    periods::reopen(&mut c, company.id, body.year, body.month).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}
