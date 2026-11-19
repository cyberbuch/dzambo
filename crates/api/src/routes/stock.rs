//! Склад: справка по партиди (WAC).

use axum::{extract::{Extension, State}, Json};
use domain::dto::StockItemDto;

use crate::{errors::ApiError, routes::common::active_company, state::{AppState, Session}};

fn internal() -> ApiError { ApiError::new(axum::http::StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка") }

pub async fn report(State(st): State<AppState>, Extension(s): Extension<Session>) -> Result<Json<Vec<StockItemDto>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let rows = storage::stock::report_wac(&mut conn, c.id).await?;
    Ok(Json(rows.iter().map(|r| StockItemDto {
        product_id: r.product_id, name: r.name.clone(), code: r.code.clone(), unit: r.unit.clone(),
        quantity: format!("{:.4}", r.quantity), avg_unit_cost: format!("{:.2}", r.avg_unit_cost), value: format!("{:.2}", r.value),
    }).collect()))
}
