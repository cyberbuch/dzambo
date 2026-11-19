//! Запис на настройките на фирмата: обекти, собственици, крайни майки, серии.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use domain::dto::{LocationDto, OwnerDto, ParentDto, SeriesDto};
use storage::{
    models::related::{NewBeneficialOwner, NewDocumentSeries, NewLocation, NewUltimateParent},
    related,
};

use crate::{
    errors::ApiError,
    mappers,
    routes::common::active_company,
    state::{AppState, Session},
};

const LOCATION_TYPES: [&str; 5] = ["OFFICE", "STORE", "WAREHOUSE", "BRANCH", "OTHER"];

fn created<T: serde::Serialize>(value: T) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::CREATED,
        Json(serde_json::to_value(value).unwrap()),
    )
}

fn ok<T: serde::Serialize>(value: T) -> Json<serde_json::Value> {
    Json(serde_json::to_value(value).unwrap())
}

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}

async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}

/// id-то в path трябва да е активната фирма (за POST пътищата).
async fn require_company(
    st: &AppState,
    session: &Session,
    path_company: i64,
) -> Result<i64, ApiError> {
    let (_u, company) = active_company(st, &session.claims.sub).await?;
    if company.id != path_company {
        return Err(ApiError::not_found("фирмата не е намерена"));
    }
    Ok(company.id)
}

/// Активна фирма без path-проверка (за PATCH/DELETE по id на под-обект).
async fn active_id(st: &AppState, session: &Session) -> Result<i64, ApiError> {
    let (_u, company) = active_company(st, &session.claims.sub).await?;
    Ok(company.id)
}

// ---------- обекти ----------

pub async fn location_create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(company_id): Path<i64>,
    Json(dto): Json<LocationDto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let cid = require_company(&st, &session, company_id).await?;
    if !LOCATION_TYPES.contains(&dto.location_type.as_str()) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "location_type трябва да е OFFICE, STORE, WAREHOUSE, BRANCH или OTHER",
        ));
    }
    let mut conn = conn(&st).await?;
    let saved = related::location_insert(
        &mut conn,
        &NewLocation {
            company_id: cid,
            name: &dto.name,
            location_type: &dto.location_type,
            street_name: &dto.street_name,
            building_number: &dto.building_number,
            city: &dto.city,
            post_code: &dto.post_code,
            region: &dto.region,
            country: &dto.country,
            is_main: dto.is_main,
        },
    )
    .await?;
    Ok(created(mappers::location_dto(&saved)))
}

pub async fn location_update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(dto): Json<LocationDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cid = active_id(&st, &session).await?;
    let mut conn = conn(&st).await?;
    let existing = related::location_by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("обектът не е намерен"))?;
    if existing.company_id != cid {
        return Err(ApiError::not_found("обектът не е намерен"));
    }
    let saved = related::location_update(
        &mut conn,
        id,
        &NewLocation {
            company_id: cid,
            name: &dto.name,
            location_type: &dto.location_type,
            street_name: &dto.street_name,
            building_number: &dto.building_number,
            city: &dto.city,
            post_code: &dto.post_code,
            region: &dto.region,
            country: &dto.country,
            is_main: dto.is_main,
        },
    )
    .await?;
    Ok(ok(mappers::location_dto(&saved)))
}

pub async fn location_delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let cid = active_id(&st, &session).await?;
    let mut conn = conn(&st).await?;
    let existing = related::location_by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("обектът не е намерен"))?;
    if existing.company_id != cid {
        return Err(ApiError::not_found("обектът не е намерен"));
    }
    related::location_delete(&mut conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- действителни собственици ----------

pub async fn owner_create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(company_id): Path<i64>,
    Json(dto): Json<OwnerDto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let cid = require_company(&st, &session, company_id).await?;
    if dto.first_name_bg.trim().is_empty() || dto.last_name_bg.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "името и фамилията са задължителни",
        ));
    }
    let mut conn = conn(&st).await?;
    let saved = related::owner_insert(&mut conn, &owner_new(cid, &dto)).await?;
    Ok(created(mappers::owner_dto(&saved)))
}

fn owner_new<'a>(cid: i64, d: &'a OwnerDto) -> NewBeneficialOwner<'a> {
    NewBeneficialOwner {
        company_id: cid,
        first_name_bg: &d.first_name_bg,
        last_name_bg: &d.last_name_bg,
        egn: &d.egn,
        first_name_latin: &d.first_name_latin,
        last_name_latin: &d.last_name_latin,
        country: &d.country,
        ownership_percentage: &d.ownership_percentage,
    }
}

pub async fn owner_update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(dto): Json<OwnerDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cid = active_id(&st, &session).await?;
    let mut conn = conn(&st).await?;
    let existing = related::owner_by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("собственикът не е намерен"))?;
    if existing.company_id != cid {
        return Err(ApiError::not_found("собственикът не е намерен"));
    }
    let saved = related::owner_update(&mut conn, id, &owner_new(cid, &dto)).await?;
    Ok(ok(mappers::owner_dto(&saved)))
}

pub async fn owner_delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let cid = active_id(&st, &session).await?;
    let mut conn = conn(&st).await?;
    let existing = related::owner_by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("собственикът не е намерен"))?;
    if existing.company_id != cid {
        return Err(ApiError::not_found("собственикът не е намерен"));
    }
    related::owner_delete(&mut conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- крайни майки ----------

pub async fn parent_create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(company_id): Path<i64>,
    Json(dto): Json<ParentDto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let cid = require_company(&st, &session, company_id).await?;
    if dto.name_bg.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "името е задължително",
        ));
    }
    let mut conn = conn(&st).await?;
    let saved = related::parent_insert(&mut conn, &parent_new(cid, &dto)).await?;
    Ok(created(mappers::parent_dto(&saved)))
}

fn parent_new<'a>(cid: i64, d: &'a ParentDto) -> NewUltimateParent<'a> {
    NewUltimateParent {
        company_id: cid,
        name_bg: &d.name_bg,
        uic: &d.uic,
        name_latin: &d.name_latin,
        country: &d.country,
    }
}

pub async fn parent_update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(dto): Json<ParentDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cid = active_id(&st, &session).await?;
    let mut conn = conn(&st).await?;
    let existing = related::parent_by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("крайната майка не е намерена"))?;
    if existing.company_id != cid {
        return Err(ApiError::not_found("крайната майка не е намерена"));
    }
    let saved = related::parent_update(&mut conn, id, &parent_new(cid, &dto)).await?;
    Ok(ok(mappers::parent_dto(&saved)))
}

pub async fn parent_delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let cid = active_id(&st, &session).await?;
    let mut conn = conn(&st).await?;
    let existing = related::parent_by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("крайната майка не е намерена"))?;
    if existing.company_id != cid {
        return Err(ApiError::not_found("крайната майка не е намерена"));
    }
    related::parent_delete(&mut conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- серии ----------

pub async fn series_create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(company_id): Path<i64>,
    Json(dto): Json<SeriesDto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let cid = require_company(&st, &session, company_id).await?;
    if dto.name.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "името е задължително",
        ));
    }
    let mut conn = conn(&st).await?;
    let pad = dto.pad_width.clamp(1, 10);
    let start = dto.start_number.max(1);
    let saved = related::series_insert(
        &mut conn,
        &NewDocumentSeries {
            company_id: cid,
            name: &dto.name,
            document_types: &dto.document_types,
            pad_width: pad,
            start_number: start,
            next_number: &start.to_string(),
        },
    )
    .await?;
    Ok(created(mappers::series_dto(&saved)))
}

pub async fn series_update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(dto): Json<SeriesDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cid = active_id(&st, &session).await?;
    let mut conn = conn(&st).await?;
    let existing = related::series_by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("серията не е намерена"))?;
    if existing.company_id != cid {
        return Err(ApiError::not_found("серията не е намерена"));
    }
    let pad = dto.pad_width.clamp(1, 10);
    let start = dto.start_number.max(1);
    let saved = related::series_update(
        &mut conn,
        id,
        &NewDocumentSeries {
            company_id: cid,
            name: &dto.name,
            document_types: &dto.document_types,
            pad_width: pad,
            start_number: start,
            next_number: &start.to_string(),
        },
    )
    .await?;
    Ok(ok(mappers::series_dto(&saved)))
}

pub async fn series_delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let cid = active_id(&st, &session).await?;
    let mut conn = conn(&st).await?;
    let existing = related::series_by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("серията не е намерена"))?;
    if existing.company_id != cid {
        return Err(ApiError::not_found("серията не е намерена"));
    }
    related::series_delete(&mut conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
