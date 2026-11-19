//! ДМА: активи, категории, месечна амортизация.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use bigdecimal::BigDecimal;
use domain::dto::{AssetDto, DepRowDto};

use crate::{
    errors::ApiError,
    routes::common::active_company,
    state::{AppState, Session},
};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
fn dec(raw: &str) -> BigDecimal {
    raw.trim().parse().unwrap_or_default()
}

async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}

pub async fn index(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    let rows = storage::fixed_assets::list(&mut co, c.id).await?;
    Ok(Json(rows.iter().map(|a| serde_json::json!({
        "id": a.id, "name": a.name, "inventory_number": a.inventory_number, "category_id": a.category_id,
        "acquisition_date": a.acquisition_date.to_string(), "cost": format!("{:.2}", a.cost),
        "salvage_value": format!("{:.2}", a.salvage_value), "useful_life_months": a.useful_life_months,
        "accumulated_depreciation": format!("{:.2}", a.accumulated_depreciation), "status": a.status,
    })).collect()))
}

pub async fn create(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<AssetDto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if b.name.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "името е задължително",
        ));
    }
    let date = storage::invoices::parse_date(&b.acquisition_date)
        .ok_or_else(|| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "невалидна дата"))?;
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    let a = storage::fixed_assets::create_asset(
        &mut co,
        c.id,
        &b.name,
        &b.inventory_number,
        b.category_id,
        date,
        dec(&b.cost),
        dec(&b.salvage_value),
        b.useful_life_months,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({"id": a.id}))))
}

pub async fn delete(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_u, _c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    storage::fixed_assets::delete(&mut co, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct MonthBody {
    pub year: i32,
    pub month: i32,
}

pub async fn preview(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<MonthBody>,
) -> Result<Json<Vec<DepRowDto>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    let rows = storage::fixed_assets::preview(&mut co, c.id, b.year, b.month).await?;
    Ok(Json(
        rows.iter()
            .map(|r| DepRowDto {
                asset_id: r.asset_id,
                name: r.name.clone(),
                amount: format!("{:.2}", r.amount),
                expense_account: r.expense_number.clone(),
                depreciation_account: r.dep_number.clone(),
            })
            .collect(),
    ))
}

pub async fn post_dep(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<MonthBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    let eid = storage::fixed_assets::post_depreciation(&mut co, c.id, b.year, b.month)
        .await
        .map_err(|e| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true, "entry_id": eid})))
}
