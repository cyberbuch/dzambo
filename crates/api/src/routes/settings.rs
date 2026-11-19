//! GET /v1/companies/{id}/settings — комбиниран обект на активната фирма.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};

use storage::related;

use crate::{
    errors::ApiError,
    mappers,
    routes::common::active_company,
    state::{AppState, Session},
};

pub async fn show(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_user, company) = active_company(&st, &session.claims.sub).await?;
    if company.id != id {
        return Err(ApiError::not_found("фирмата не е намерена"));
    }
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let row = storage::companies::by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("фирмата не е намерена"))?;
    let locations = related::locations(&mut conn, id).await?;
    let owners = related::owners(&mut conn, id).await?;
    let parents = related::parents(&mut conn, id).await?;
    let series = related::series_list(&mut conn, id).await?;
    let dto = mappers::settings_dto(
        &mappers::company_dto(&row),
        locations,
        owners,
        parents,
        series,
    );
    Ok(Json(serde_json::to_value(dto).unwrap()))
}
