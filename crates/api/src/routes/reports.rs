//! /v1/reports — справки (оборотна, главна книга, хронологичен, кореспонденции,
//! финансови отчети) + export pdf/xlsx/ods.

use axum::extract::{Extension, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use domain::reports_data::{self, Built};
use domain::{reports_book, reports_corr, reports_fs, reports_sheet, reports_trial};
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
    pub format: Option<String>,
    pub bg: Option<String>,
}

async fn prepare(st: &AppState, sub: &str, q: &Rq) -> Result<Built, ApiError> {
    let (_u, company) = active_company(st, sub).await?;
    let kind = q.kind.clone().unwrap_or_default().trim().to_string();
    let kind = if kind.is_empty() {
        "trial_balance".to_string()
    } else {
        kind
    };
    if !reports_data::kind_ok(&kind) {
        return Err(bad("непознат kind"));
    }
    let from = q.from.clone().unwrap_or_default().trim().to_string();
    let to = q.to.clone().unwrap_or_default().trim().to_string();
    if !reports_data::day_ok(&from) || !reports_data::day_ok(&to) {
        return Err(bad("from и to са задължителни (YYYY-MM-DD)"));
    }
    if from > to {
        return Err(bad("from трябва да е преди to"));
    }
    let account = q.account.clone().unwrap_or_default().trim().to_string();
    let cpid: i64 = q
        .counterpart_id
        .clone()
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
    Ok(built)
}

pub async fn index(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<Rq>,
) -> Result<Json<Value>, ApiError> {
    let built = prepare(&st, &session.claims.sub, &q).await?;
    Ok(Json(built.body))
}

pub async fn export(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<Rq>,
) -> Result<Response, ApiError> {
    let format = q.format.clone().unwrap_or_default().trim().to_string();
    if !matches!(format.as_str(), "pdf" | "xlsx" | "ods") {
        return Err(bad("format трябва да е pdf, xlsx или ods"));
    }
    let built = prepare(&st, &session.claims.sub, &q).await?;
    let bg = q.bg.as_deref() == Some("1");
    let (bytes, mime) = match format.as_str() {
        "pdf" => (crate::print::grid_pdf(&built.grid)?, "application/pdf"),
        "xlsx" => (
            reports_sheet::xlsx(&built.grid, bg),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
        _ => (
            reports_sheet::ods(&built.grid, bg),
            "application/vnd.oasis.opendocument.spreadsheet",
        ),
    };
    // Само цифри/тирета в името на файла (защита срещу инжектиране в header-а).
    let period: String = [
        q.from.as_deref().unwrap_or(""),
        q.to.as_deref().unwrap_or(""),
    ]
    .join("-")
    .chars()
    .map(|c| {
        if c.is_ascii_alphanumeric() || c == '-' {
            c
        } else {
            '_'
        }
    })
    .collect();
    let filename = format!("{}-{period}.{format}", built.slug);
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}
