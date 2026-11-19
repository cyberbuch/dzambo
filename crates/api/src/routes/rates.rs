//! /v1/exchange-rates — ЕЦБ курсове за 1 EUR, as-of и кръстосан курс.

use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::Json;
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};
use std::str::FromStr;
use std::time::Duration;

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use storage::rates;

const DAILY: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";
const HISTORY: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist-90d.xml";

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
fn bad(m: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, m)
}
fn not_found(m: &str) -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, m)
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}
fn parse_date(s: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map_err(|_| bad("date трябва да е YYYY-MM-DD"))
}
fn f(v: &BigDecimal) -> f64 {
    v.to_string().parse().unwrap_or(0.0)
}

#[derive(Deserialize)]
pub struct DateQuery {
    pub date: Option<String>,
}

pub async fn index(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<DateQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, _c) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let date = match q.date.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(d) => parse_date(d)?,
        None => match rates::latest_date(&mut c).await? {
            Some(d) => d,
            None => return Ok(Json(json!({"date": "", "base": "EUR", "items": []}))),
        },
    };
    let rows = rates::by_date(&mut c, date).await?;
    let items: Vec<Value> = rows
        .iter()
        .map(|(id, code, rate, d)| json!({"id": id, "currency_code": code, "rate": format!("{rate}"), "date": d.to_string()}))
        .collect();
    Ok(Json(
        json!({"date": date.to_string(), "base": "EUR", "items": items}),
    ))
}

#[derive(Deserialize)]
pub struct RateQuery {
    pub currency: Option<String>,
    pub date: Option<String>,
}

pub async fn rate(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<RateQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, _c) = active_company(&st, &session.claims.sub).await?;
    let currency = q.currency.unwrap_or_default().trim().to_ascii_uppercase();
    if currency.is_empty() {
        return Err(bad("currency е задължителен"));
    }
    let date = parse_date(&q.date.unwrap_or_default())?;
    let mut c = conn(&st).await?;
    if currency == "EUR" {
        return Ok(Json(
            json!({"currency_code": "EUR", "date": date.to_string(), "rate": "1", "base": "EUR"}),
        ));
    }
    let Some((r, day)) = rates::per_eur(&mut c, &currency, date).await? else {
        return Err(not_found("няма курс за тази валута"));
    };
    Ok(Json(
        json!({"currency_code": currency, "date": day.to_string(), "rate": format!("{r}"), "base": "EUR"}),
    ))
}

#[derive(Deserialize)]
pub struct CrossQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub date: Option<String>,
}

async fn per_eur_f(
    c: &mut storage::DbConn,
    code: &str,
    date: NaiveDate,
) -> Result<(f64, NaiveDate), ApiError> {
    if code == "EUR" {
        return Ok((1.0, date));
    }
    rates::per_eur(c, code, date)
        .await?
        .map(|(r, d)| (f(&r), d))
        .ok_or_else(|| not_found(&format!("няма курс за {code}")))
}

pub async fn cross(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<CrossQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, _c) = active_company(&st, &session.claims.sub).await?;
    let from = q.from.unwrap_or_default().trim().to_ascii_uppercase();
    let to = q.to.unwrap_or_default().trim().to_ascii_uppercase();
    if from.is_empty() || to.is_empty() {
        return Err(bad("from и to са задължителни"));
    }
    let date = parse_date(&q.date.unwrap_or_default())?;
    let mut c = conn(&st).await?;
    let (rf, _) = per_eur_f(&mut c, &from, date).await?;
    let (rt, rate_date) = per_eur_f(&mut c, &to, date).await?;
    if rf == 0.0 {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "курсът е 0",
        ));
    }
    Ok(Json(json!({
        "from": from, "to": to, "rate": format!("{:.6}", rt / rf),
        "rate_date": rate_date.to_string(), "requested_date": date.to_string(), "base": "EUR",
    })))
}

#[derive(Deserialize)]
pub struct ImportBody {
    pub mode: Option<String>,
}

pub async fn import(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    body: Option<Json<ImportBody>>,
) -> Result<Json<Value>, ApiError> {
    let (_u, _c) = active_company(&st, &session.claims.sub).await?;
    let mode = body
        .and_then(|Json(b)| b.mode)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "daily".to_string());
    if mode != "daily" && mode != "history" {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "mode трябва да е daily или history",
        ));
    }
    let url = if mode == "history" { HISTORY } else { DAILY };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(40))
        .build()
        .map_err(|_| internal())?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|_| ApiError::new(StatusCode::BAD_GATEWAY, "ЕЦБ фидът е недостъпен"))?;
    if resp.status() != reqwest::StatusCode::OK {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            format!("ЕЦБ фидът върна статус {}", resp.status()),
        ));
    }
    let xml = resp
        .text()
        .await
        .map_err(|_| ApiError::new(StatusCode::BAD_GATEWAY, "ЕЦБ фидът е недостъпен"))?;
    let parsed = cubes(&xml).map_err(|e| ApiError::new(StatusCode::BAD_GATEWAY, e))?;

    let mut c = conn(&st).await?;
    let (mut inserted, mut updated, mut unchanged) = (0, 0, 0);
    let (mut first, mut last) = (String::new(), String::new());
    for (code, rate, date) in parsed {
        if first.is_empty() || date < first {
            first = date.clone();
        }
        if last.is_empty() || date > last {
            last = date.clone();
        }
        let Ok(d) = NaiveDate::parse_from_str(&date, "%Y-%m-%d") else {
            continue;
        };
        let Ok(r) = BigDecimal::from_str(&rate) else {
            continue;
        };
        match rates::upsert(&mut c, &code, &r, d).await? {
            rates::Upsert::Inserted => inserted += 1,
            rates::Upsert::Updated => updated += 1,
            rates::Upsert::Unchanged => unchanged += 1,
        }
    }
    Ok(Json(json!({
        "mode": mode, "inserted": inserted, "updated": updated, "unchanged": unchanged,
        "from_date": first, "to_date": last,
    })))
}

fn quoted(tag: &str, key: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let needle = format!("{key}={quote}");
        let Some(pos) = tag.find(&needle) else {
            continue;
        };
        let rest = &tag[pos + needle.len()..];
        let Some(stop) = rest.find(quote) else {
            continue;
        };
        return Some(rest[..stop].to_string());
    }
    None
}

/// Минимален парсер на ЕЦБ Cube XML: (currency, rate, time).
fn cubes(xml: &str) -> Result<Vec<(String, String, String)>, String> {
    let mut out = Vec::new();
    let mut date = String::new();
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        if rest.starts_with('/') || rest.starts_with('!') || rest.starts_with('?') {
            continue;
        }
        let Some(end) = rest.find('>') else { break };
        let tag = &rest[..end];
        rest = &rest[end + 1..];
        if let Some(day) = quoted(tag, "time") {
            date = day;
            continue;
        }
        let (Some(code), Some(rate)) = (quoted(tag, "currency"), quoted(tag, "rate")) else {
            continue;
        };
        if date.is_empty() {
            continue;
        }
        out.push((code, rate, date.clone()));
    }
    if out.is_empty() && xml.contains('<') {
        return Err("невалиден XML от ЕЦБ".into());
    }
    Ok(out)
}
