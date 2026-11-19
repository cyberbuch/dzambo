//! /v1/reports — оборотна ведомост и финансови отчети (balance/pl/cashflow/equity).

use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use domain::reports_data::{self, Built};
use domain::{reports_book, reports_corr, reports_fs, reports_trial};
use storage::{fsform, reports};

fn bad(m: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, m)
}
fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}

#[derive(Deserialize)]
pub struct Rq {
    pub kind: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub account: Option<String>,
    pub counterpart_id: Option<String>,
}

pub async fn index(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<Rq>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let kind = q.kind.unwrap_or_default().trim().to_string();
    let kind = if kind.is_empty() {
        "trial_balance".to_string()
    } else {
        kind
    };
    if !reports_data::kind_ok(&kind) {
        return Err(bad("непознат kind"));
    }
    let from = q.from.unwrap_or_default().trim().to_string();
    let to = q.to.unwrap_or_default().trim().to_string();
    if !reports_data::day_ok(&from) || !reports_data::day_ok(&to) {
        return Err(bad("from и to са задължителни (YYYY-MM-DD)"));
    }
    if from > to {
        return Err(bad("from трябва да е преди to"));
    }
    let account = q.account.unwrap_or_default().trim().to_string();
    let cpid: i64 = q
        .counterpart_id
        .unwrap_or_default()
        .trim()
        .parse()
        .unwrap_or(0);
    let wants_cp = kind == "counterpart_trial" || kind == "counterpart_chrono";
    if wants_cp && cpid <= 0 {
        return Err(bad("counterpart_id е задължителен"));
    }
    let filter_cp = if wants_cp { cpid } else { 0 };

    let mut c = st.pool.get().await.map_err(|_| internal())?;
    let books = reports::load(&mut c, company.id, &from, &to, &account, filter_cp).await?;
    let mut built: Built = if let Some(statement) = reports_data::fs_statement(&kind) {
        let rows = fsform::rows_of(&mut c, company.id, statement).await?;
        let formulas: HashMap<String, String> =
            rows.into_iter().map(|r| (r.line_code, r.formula)).collect();
        reports_fs::build(&kind, &books, &formulas)
    } else {
        match kind.as_str() {
            "chronological" | "counterpart_chrono" => reports_book::chrono(&kind, &books),
            "general_ledger" => reports_book::ledger(&kind, &books),
            "correspondence_ledger" => reports_corr::build(&kind, &books),
            _ => reports_trial::build(&kind, &books),
        }
    };
    let line = reports_data::firm_line(&books.firm_name, &books.firm_eik);
    if !line.is_empty() {
        built.grid.insert(0, vec![line]);
    }
    Ok(Json(built.body))
}
