//! Производство: рецепти и нареждания.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use bigdecimal::BigDecimal;
use domain::dto::{OrderDto, OrderLineDto, RecipeDto, RecipeLineDto};

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

fn recipe_dto(r: &storage::mfg::Recipe, lines: &[storage::mfg::RecipeLine]) -> RecipeDto {
    RecipeDto {
        id: r.id,
        code: r.code.clone(),
        name: r.name.clone(),
        output_product_id: r.output_product_id,
        output_quantity: format!("{:.4}", r.output_quantity),
        lines: lines
            .iter()
            .map(|l| RecipeLineDto {
                product_id: l.product_id,
                quantity: format!("{:.4}", l.quantity),
                unit: l.unit.clone(),
                wastage_percent: format!("{}", l.wastage_percent),
            })
            .collect(),
    }
}

fn order_dto(o: &storage::mfg::Order, lines: &[storage::mfg::OrderLine]) -> OrderDto {
    OrderDto {
        id: o.id,
        number: o.number.clone(),
        recipe_id: o.recipe_id,
        output_product_id: o.output_product_id,
        quantity: format!("{:.4}", o.quantity),
        status: o.status.clone(),
        material_cost: format!("{:.2}", o.material_cost),
        order_date: o.order_date.to_string(),
        lines: lines
            .iter()
            .map(|l| OrderLineDto {
                product_id: l.product_id,
                product_name: l.product_name.clone(),
                quantity: format!("{:.4}", l.quantity),
                unit: l.unit.clone(),
                unit_cost: format!("{:.2}", l.unit_cost),
                amount: format!("{:.2}", l.amount),
            })
            .collect(),
    }
}

pub async fn recipes(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<Vec<RecipeDto>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = st.pool.get().await.map_err(|_| internal())?;
    let rows = storage::mfg::list_recipes(&mut co, c.id).await?;
    let mut out = Vec::new();
    for r in &rows {
        out.push(recipe_dto(
            r,
            &storage::mfg::recipe_lines(&mut co, r.id).await?,
        ));
    }
    Ok(Json(out))
}

pub async fn create_recipe(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<RecipeDto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if b.name.trim().is_empty() || b.output_product_id <= 0 {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "името и изходящата стока са задължителни",
        ));
    }
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = st.pool.get().await.map_err(|_| internal())?;
    let lines: Vec<(i64, BigDecimal, String, BigDecimal)> = b
        .lines
        .iter()
        .map(|l| {
            (
                l.product_id,
                dec(&l.quantity),
                l.unit.clone(),
                dec(&l.wastage_percent),
            )
        })
        .collect();
    let id = storage::mfg::create_recipe(
        &mut co,
        c.id,
        &b.name,
        b.output_product_id,
        dec(&b.output_quantity),
        "",
        &lines,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({"id": id}))))
}

pub async fn delete_recipe(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_u, _c) = active_company(&st, &s.claims.sub).await?;
    let mut co = st.pool.get().await.map_err(|_| internal())?;
    storage::mfg::delete_recipe(&mut co, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn orders(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<Vec<OrderDto>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = st.pool.get().await.map_err(|_| internal())?;
    let rows = storage::mfg::list_orders(&mut co, c.id).await?;
    let mut out = Vec::new();
    for o in &rows {
        out.push(order_dto(
            o,
            &storage::mfg::order_lines_of(&mut co, o.id).await?,
        ));
    }
    Ok(Json(out))
}

#[derive(serde::Deserialize)]
pub struct OrderIn {
    recipe_id: i64,
    quantity: String,
    order_date: String,
}

pub async fn create_order(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<OrderIn>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let date = storage::invoices::parse_date(&b.order_date)
        .ok_or_else(|| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "невалидна дата"))?;
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = st.pool.get().await.map_err(|_| internal())?;
    let id = storage::mfg::create_order(&mut co, c.id, b.recipe_id, dec(&b.quantity), date).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({"id": id}))))
}

pub async fn confirm(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = st.pool.get().await.map_err(|_| internal())?;
    let cost = storage::mfg::confirm_order(&mut co, c.id, id)
        .await
        .map_err(|e| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;
    Ok(Json(
        serde_json::json!({"ok": true, "material_cost": format!("{cost:.2}")}),
    ))
}
