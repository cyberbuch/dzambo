//! Дивиденти: разпределения по собственици с данък.

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use bigdecimal::BigDecimal;
use domain::dto::{CreateDistributionBody, DividendShareDto};

use crate::{
    errors::ApiError,
    routes::common::active_company,
    state::{AppState, Session},
};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}

#[derive(serde::Deserialize)]
pub struct YearQuery {
    #[serde(default)]
    pub year: Option<i32>,
}

fn share_dto(s: &storage::dividends::Share) -> DividendShareDto {
    DividendShareDto {
        id: s.id,
        distribution_id: s.distribution_id,
        owner_name: s.owner_name.clone(),
        owner_egn: s.owner_egn.clone(),
        ownership_percentage: s.ownership_percentage.clone(),
        gross_amount: format!("{:.2}", s.gross_amount),
        tax_rate: format!("{}", s.tax_rate),
        tax_amount: format!("{:.2}", s.tax_amount),
        net_amount: format!("{:.2}", s.net_amount),
    }
}

pub async fn index(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Query(q): Query<YearQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = st.pool.get().await.map_err(|_| internal())?;
    let rows = storage::dividends::list_shares(&mut co, c.id, q.year).await?;
    let shares: Vec<DividendShareDto> = rows.iter().map(|(_, sh)| share_dto(sh)).collect();
    let total_net: BigDecimal = shares
        .iter()
        .map(|x| x.net_amount.parse::<BigDecimal>().unwrap_or_default())
        .sum();
    Ok(Json(
        serde_json::json!({"items": shares, "count": shares.len(), "total_net": format!("{total_net:.2}")}),
    ))
}

pub async fn create(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<CreateDistributionBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let date = storage::invoices::parse_date(&b.decision_date)
        .ok_or_else(|| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "невалидна дата"))?;
    let total: BigDecimal = b.total_amount.trim().parse().unwrap_or_default();
    if total <= BigDecimal::default() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "сумата трябва да е положителна",
        ));
    }
    let rate: BigDecimal = b
        .tax_rate
        .trim()
        .parse()
        .unwrap_or_else(|_| BigDecimal::from(5));
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = st.pool.get().await.map_err(|_| internal())?;
    let (dist_id, n) = storage::dividends::create_distribution(
        &mut co,
        c.id,
        b.year,
        total,
        date,
        &b.decision_number,
        &b.notes,
        rate,
    )
    .await
    .map_err(|e| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"distribution_id": dist_id, "shares": n})),
    ))
}
