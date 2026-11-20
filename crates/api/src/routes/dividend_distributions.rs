//! /v1/dividend-distributions — протоколи за дивидент (Dist формата на фронта).

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use bigdecimal::BigDecimal;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::{active_company, PageQuery, PageSlice};
use crate::state::{AppState, Session};
use storage::dividends::{self as dv, Distribution, Share};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}
fn dec(raw: &str) -> BigDecimal {
    raw.trim().replace(',', ".").parse().unwrap_or_default()
}
fn sval(b: &Value, k: &str) -> String {
    b.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

fn share_json(s: &Share) -> Value {
    json!({
        "id": s.id, "distribution_id": s.distribution_id, "owner_name": s.owner_name,
        "owner_egn": s.owner_egn, "ownership_percentage": s.ownership_percentage,
        "gross_amount": format!("{:.2}", s.gross_amount), "tax_rate": format!("{}", s.tax_rate),
        "tax_amount": format!("{:.2}", s.tax_amount), "net_amount": format!("{:.2}", s.net_amount),
        "is_paid": s.is_paid, "payment_date": s.payment_date.map(|d| d.to_string()).unwrap_or_default(),
    })
}

fn dist_json(d: &Distribution, shares: &[Share], with_dividends: bool) -> Value {
    let total_net: BigDecimal = shares.iter().map(|s| s.net_amount.clone()).sum();
    let total_tax: BigDecimal = shares.iter().map(|s| s.tax_amount.clone()).sum();
    let total_paid: BigDecimal = shares
        .iter()
        .filter(|s| s.is_paid)
        .map(|s| s.net_amount.clone())
        .sum();
    let mut v = json!({
        "id": d.id, "year": d.year, "total_amount": format!("{:.2}", d.total_amount),
        "decision_date": d.decision_date.to_string(), "decision_number": d.decision_number,
        "status": d.status, "notes": d.notes, "dividends_count": shares.len(),
        "total_net": format!("{total_net:.2}"), "total_paid": format!("{total_paid:.2}"),
        "total_tax": format!("{total_tax:.2}"),
    });
    if with_dividends {
        v["dividends"] = json!(shares.iter().map(share_json).collect::<Vec<_>>());
    }
    v
}

async fn load_dist(co: &mut storage::DbConn, cid: i64, id: i64) -> Result<Value, ApiError> {
    let d = dv::get_distribution(co, cid, id)
        .await?
        .ok_or_else(|| ApiError::not_found("протоколът не е намерен"))?;
    let shares = dv::shares_for(co, id).await?;
    Ok(dist_json(&d, &shares, true))
}

pub async fn index(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    let dists = dv::list_distributions(&mut co, c.id).await?;
    let mut items = Vec::new();
    for d in &dists {
        let shares = dv::shares_for(&mut co, d.id).await?;
        items.push(dist_json(d, &shares, false));
    }
    let owners = storage::related::owners(&mut co, c.id).await?;
    let owners_json: Vec<Value> = owners
        .iter()
        .map(|o| {
            json!({
                "id": o.id, "name": format!("{} {}", o.first_name_bg, o.last_name_bg),
                "egn": o.egn, "ownership_percentage": o.ownership_percentage,
            })
        })
        .collect();
    let slice = PageSlice::from_vec(items, &q);
    Ok(Json(json!({
        "items": slice.items, "count": slice.count(), "total": slice.total,
        "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more(),
        "owners": owners_json, "currency": c.currency,
    })))
}

pub async fn show(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    Ok(Json(load_dist(&mut co, c.id, id).await?))
}

pub async fn create(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let year = b.get("year").and_then(Value::as_i64).unwrap_or(0) as i32;
    if year <= 0 {
        return Err(unproc("year е задължителна"));
    }
    let total = dec(&sval(&b, "total_amount"));
    if total <= 0 {
        return Err(unproc("сумата трябва да е положителна"));
    }
    let date = storage::invoices::parse_date(&sval(&b, "decision_date"))
        .ok_or_else(|| unproc("decision_date е задължително"))?;
    let rate = dec(&sval(&b, "tax_rate"));
    let rate = if rate <= 0 {
        BigDecimal::from(5)
    } else {
        rate
    };
    let mut co = conn(&st).await?;
    let (id, _n) = dv::create_distribution(
        &mut co,
        c.id,
        year,
        total,
        date,
        &sval(&b, "decision_number"),
        &sval(&b, "notes"),
        rate,
    )
    .await
    .map_err(|e| unproc(&e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(load_dist(&mut co, c.id, id).await?),
    ))
}

pub async fn update(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    let cur = dv::get_distribution(&mut co, c.id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("протоколът не е намерен"))?;
    let touches = ["year", "total_amount", "tax_rate", "decision_date"]
        .iter()
        .any(|k| b.get(*k).is_some());
    if touches && dv::any_paid(&mut co, id).await? {
        return Err(unproc("Не може да се променя сумата или данъкът при изплатени дивиденти. Първо отменете плащанията."));
    }
    if let Some(status) = b.get("status").and_then(Value::as_str) {
        if !matches!(status, "draft" | "approved" | "partially_paid" | "paid") {
            return Err(unproc("невалиден статус"));
        }
        dv::set_status(&mut co, id, status).await?;
    }
    if touches {
        let year = b
            .get("year")
            .and_then(Value::as_i64)
            .unwrap_or(cur.year as i64) as i32;
        let total = if b.get("total_amount").is_some() {
            dec(&sval(&b, "total_amount"))
        } else {
            cur.total_amount.clone()
        };
        if total <= 0 {
            return Err(unproc("сумата трябва да е положителна"));
        }
        let date = if b.get("decision_date").is_some() {
            storage::invoices::parse_date(&sval(&b, "decision_date"))
                .ok_or_else(|| unproc("decision_date е задължително"))?
        } else {
            cur.decision_date
        };
        let rate = if b.get("tax_rate").is_some() {
            dec(&sval(&b, "tax_rate"))
        } else {
            BigDecimal::from(5)
        };
        let number = if b.get("decision_number").is_some() {
            sval(&b, "decision_number")
        } else {
            cur.decision_number.clone()
        };
        let notes = if b.get("notes").is_some() {
            sval(&b, "notes")
        } else {
            cur.notes.clone()
        };
        dv::update_fields(&mut co, id, year, &total, date, &number, &notes).await?;
        dv::delete_shares(&mut co, id).await?;
        dv::insert_shares(&mut co, c.id, id, &total, date, &rate)
            .await
            .map_err(|e| unproc(&e.to_string()))?;
    } else {
        if b.get("decision_number").is_some() || b.get("notes").is_some() {
            let number = if b.get("decision_number").is_some() {
                sval(&b, "decision_number")
            } else {
                cur.decision_number.clone()
            };
            let notes = if b.get("notes").is_some() {
                sval(&b, "notes")
            } else {
                cur.notes.clone()
            };
            dv::update_fields(
                &mut co,
                id,
                cur.year,
                &cur.total_amount,
                cur.decision_date,
                &number,
                &notes,
            )
            .await?;
        }
    }
    Ok(Json(load_dist(&mut co, c.id, id).await?))
}

pub async fn delete(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    if dv::delete_distribution(&mut co, c.id, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("протоколът не е намерен"))
    }
}
