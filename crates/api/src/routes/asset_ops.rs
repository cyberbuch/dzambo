//! /v1/fixed-assets/{id} операции (преоценка/преместване/консервация/брак/събития)
//! + /v1/fixed-asset-categories. Порт на ytbuch asset_ops.

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::{active_company, PageQuery, PageSlice};
use crate::state::{AppState, Session};
use storage::fixed_assets::{self as fa, Asset};

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
fn event_date(b: &Value) -> NaiveDate {
    NaiveDate::parse_from_str(&sval(b, "event_date"), "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Utc::now().date_naive())
}

pub fn asset_json(a: &Asset) -> Value {
    json!({
        "id": a.id, "company_id": a.company_id, "name": a.name,
        "inventory_number": a.inventory_number, "category_id": a.category_id,
        "acquisition_date": a.acquisition_date.to_string(),
        "put_into_service_date": a.put_into_service_date.map(|d| d.to_string()).unwrap_or_default(),
        "cost": format!("{:.2}", a.cost), "salvage_value": format!("{:.2}", a.salvage_value),
        "useful_life_months": a.useful_life_months, "depreciation_method": a.depreciation_method,
        "accounting_depreciation_rate": format!("{}", a.accounting_depreciation_rate),
        "tax_depreciation_rate": format!("{}", a.tax_depreciation_rate),
        "accumulated_depreciation": format!("{:.2}", a.accumulated_depreciation),
        "tax_accumulated_depreciation": format!("{:.2}", a.tax_accumulated_depreciation),
        "status": a.status, "serial_number": a.serial_number, "manufacturer": a.manufacturer,
        "model": a.model, "responsible_person": a.responsible_person,
        "is_conserved": a.is_conserved, "location_id": a.location_id,
    })
}

async fn need(
    st: &AppState,
    sub: &str,
    id: i64,
) -> Result<(storage::DbConn, i64, Asset), ApiError> {
    let (_u, c) = active_company(st, sub).await?;
    let mut co = conn(st).await?;
    let a = fa::get(&mut co, c.id, id)
        .await?
        .ok_or_else(|| ApiError::not_found("активът не е намерен"))?;
    Ok((co, c.id, a))
}

pub async fn update(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (mut co, cid, a) = need(&st, &s.claims.sub, id).await?;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use storage::schema::fixed_assets as t;
    if let Some(v) = b.get("name").and_then(Value::as_str) {
        diesel::update(t::table.find(id))
            .set(t::name.eq(v))
            .execute(&mut co)
            .await
            .map_err(internal_e)?;
    }
    if let Some(v) = b.get("inventory_number").and_then(Value::as_str) {
        diesel::update(t::table.find(id))
            .set(t::inventory_number.eq(v))
            .execute(&mut co)
            .await
            .map_err(internal_e)?;
    }
    if let Some(v) = b.get("category_id").and_then(Value::as_i64) {
        diesel::update(t::table.find(id))
            .set(t::category_id.eq(v))
            .execute(&mut co)
            .await
            .map_err(internal_e)?;
    }
    if let Some(v) = b.get("cost").and_then(Value::as_str) {
        diesel::update(t::table.find(id))
            .set(t::cost.eq(dec(v)))
            .execute(&mut co)
            .await
            .map_err(internal_e)?;
    }
    if let Some(v) = b.get("salvage_value").and_then(Value::as_str) {
        diesel::update(t::table.find(id))
            .set(t::salvage_value.eq(dec(v)))
            .execute(&mut co)
            .await
            .map_err(internal_e)?;
    }
    if let Some(v) = b.get("useful_life_months").and_then(Value::as_i64) {
        diesel::update(t::table.find(id))
            .set(t::useful_life_months.eq(v as i32))
            .execute(&mut co)
            .await
            .map_err(internal_e)?;
    }
    if let Some(v) = b.get("status").and_then(Value::as_str) {
        diesel::update(t::table.find(id))
            .set(t::status.eq(v))
            .execute(&mut co)
            .await
            .map_err(internal_e)?;
    }
    let _ = a;
    let updated = fa::get(&mut co, cid, id).await?.unwrap();
    Ok(Json(asset_json(&updated)))
}

fn internal_e(e: diesel::result::Error) -> ApiError {
    tracing::error!("заявка: {e}");
    internal()
}

pub async fn revalue(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (mut co, cid, a) = need(&st, &s.claims.sub, id).await?;
    let nv = sval(&b, "new_value");
    if nv.trim().is_empty() {
        return Err(unproc("new_value е задължително"));
    }
    let new_value = dec(&nv);
    let prev = format!("{:.2}", a.cost);
    fa::set_cost(&mut co, id, &new_value).await?;
    fa::add_event(
        &mut co,
        cid,
        id,
        "revaluation",
        event_date(&b),
        &new_value,
        &prev,
        "",
        &sval(&b, "reason"),
    )
    .await?;
    Ok(Json(asset_json(&fa::get(&mut co, cid, id).await?.unwrap())))
}

pub async fn move_asset(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (mut co, cid, _a) = need(&st, &s.claims.sub, id).await?;
    let loc = b.get("location_id").and_then(Value::as_i64).unwrap_or(0);
    let place = storage::idocs::location_name(&mut co, cid, loc).await?;
    fa::set_location(&mut co, id, loc).await?;
    fa::add_event(
        &mut co,
        cid,
        id,
        "movement",
        event_date(&b),
        &BigDecimal::from(0),
        "",
        &place,
        &sval(&b, "reason"),
    )
    .await?;
    Ok(Json(asset_json(&fa::get(&mut co, cid, id).await?.unwrap())))
}

async fn flag_event(
    st: &AppState,
    sub: &str,
    id: i64,
    b: &Value,
    flag: bool,
    kind: &str,
) -> Result<Json<Value>, ApiError> {
    let (mut co, cid, _a) = need(st, sub, id).await?;
    fa::set_conserved(&mut co, id, flag).await?;
    fa::add_event(
        &mut co,
        cid,
        id,
        kind,
        event_date(b),
        &BigDecimal::from(0),
        "",
        "",
        &sval(b, "reason"),
    )
    .await?;
    Ok(Json(asset_json(&fa::get(&mut co, cid, id).await?.unwrap())))
}

pub async fn conserve(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    flag_event(&st, &s.claims.sub, id, &b, true, "conservation_start").await
}

pub async fn end_conserve(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    flag_event(&st, &s.claims.sub, id, &b, false, "conservation_end").await
}

pub async fn dispose(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (mut co, cid, _a) = need(&st, &s.claims.sub, id).await?;
    let kind = sval(&b, "disposal_type");
    let status = if kind == "sale" { "sold" } else { "disposed" };
    let amount = dec(&sval(&b, "sale_amount"));
    fa::set_status(&mut co, id, status).await?;
    fa::add_event(
        &mut co,
        cid,
        id,
        "disposal",
        event_date(&b),
        &amount,
        "",
        "",
        &sval(&b, "reason"),
    )
    .await?;
    Ok(Json(asset_json(&fa::get(&mut co, cid, id).await?.unwrap())))
}

pub async fn events(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    let (mut co, _cid, _a) = need(&st, &s.claims.sub, id).await?;
    let rows = fa::events(&mut co, id).await?;
    let items: Vec<Value> = rows
        .iter()
        .map(|e| {
            json!({
                "id": e.id, "asset_id": e.asset_id, "event_type": e.event_type,
                "event_date": e.event_date.to_string(), "amount": format!("{:.2}", e.amount),
                "prev_value": e.prev_value, "location": e.location, "reason": e.reason,
            })
        })
        .collect();
    let slice = PageSlice::from_vec(items, &q);
    Ok(Json(
        json!({"items": slice.items, "count": slice.count(), "total": slice.total, "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more()}),
    ))
}

pub async fn categories(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    let rows = fa::categories(&mut co, c.id).await?;
    let items: Vec<Value> = rows.iter().map(|k| json!({
        "id": k.id, "name": k.name, "cita_category": k.cita_category,
        "min_depreciation_rate": format!("{}", k.min_depreciation_rate),
        "max_depreciation_rate": format!("{}", k.max_depreciation_rate),
        "default_method": k.default_method, "asset_account_id": k.asset_account_id,
        "depreciation_account_id": k.depreciation_account_id, "expense_account_id": k.expense_account_id,
    })).collect();
    Ok(Json(json!({"items": items, "count": items.len()})))
}

pub async fn create_category(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let name = sval(&b, "name");
    if name.trim().is_empty() {
        return Err(unproc("името е задължително"));
    }
    let mut co = conn(&st).await?;
    let k = fa::create_category(
        &mut co,
        c.id,
        &name,
        &sval(&b, "cita_category"),
        &dec(&sval(&b, "min_depreciation_rate")),
        &dec(&sval(&b, "max_depreciation_rate")),
        &sval(&b, "default_method"),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"id": k.id, "name": k.name})),
    ))
}
