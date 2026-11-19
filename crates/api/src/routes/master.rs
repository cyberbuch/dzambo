//! Сметкоплан: accounts CRUD + seed-saft + мерни единици.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use domain::dto::{AccountDto, SeedSaftDto, UnitDto};
use storage::{accounts, models::NewAccount};

use crate::{
    errors::ApiError,
    mappers,
    routes::common::{active_company, PageQuery, PageSlice},
    state::{AppState, Session},
};

const ANALYTIC_TYPES: [&str; 3] = ["none", "counterpart", "product"];

pub async fn list(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PageQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_user, company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let rows = accounts::list(&mut conn, company.id).await?;
    let slice = PageSlice::from_vec(rows, &q);
    let items: Vec<_> = slice
        .items
        .iter()
        .map(|a| serde_json::to_value(mappers::account_dto(a)).unwrap())
        .collect();
    Ok(Json(serde_json::json!({
        "items": items, "count": slice.count(), "total": slice.total,
        "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more(),
    })))
}

fn validate(dto: &AccountDto) -> Result<(), ApiError> {
    if dto.number.trim().is_empty() || dto.name.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "номер и име са задължителни",
        ));
    }
    if !ANALYTIC_TYPES.contains(&dto.analytic_type.as_str()) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "analytic_type трябва да е none, counterpart или product",
        ));
    }
    Ok(())
}

/// is_currency по подразбиране: номерите от групите 4xx/5xx са валутни.
fn currency_default(dto: &AccountDto) -> bool {
    if dto.is_currency {
        true
    } else {
        let n = dto.number.trim();
        n.starts_with('4') || n.starts_with('5')
    }
}

pub async fn create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(dto): Json<AccountDto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    validate(&dto)?;
    let (_user, company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    if accounts::number_taken(&mut conn, company.id, 0, &dto.number).await? {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "номерът вече съществува",
        ));
    }
    let saved = accounts::insert(
        &mut conn,
        &NewAccount {
            company_id: company.id,
            number: dto.number.trim(),
            name: dto.name.trim(),
            saft_account_type: &dto.saft_account_type,
            is_currency: currency_default(&dto),
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(mappers::account_dto(&saved)).unwrap()),
    ))
}

pub async fn show(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_user, _company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let row = accounts::by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("сметката не е намерена"))?;
    Ok(Json(
        serde_json::to_value(mappers::account_dto(&row)).unwrap(),
    ))
}

pub async fn update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(dto): Json<AccountDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validate(&dto)?;
    let (_user, _company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let existing = accounts::by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("сметката не е намерена"))?;
    if accounts::number_taken(&mut conn, existing.company_id, id, &dto.number).await? {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "номерът вече съществува",
        ));
    }
    let saved = accounts::update(
        &mut conn,
        id,
        dto.number.trim(),
        dto.name.trim(),
        &dto.saft_account_type,
        &dto.analytic_type,
        currency_default(&dto),
    )
    .await?;
    Ok(Json(
        serde_json::to_value(mappers::account_dto(&saved)).unwrap(),
    ))
}

pub async fn delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_user, _company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    accounts::delete(&mut conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub search: Option<String>,
}

/// GET /v1/units — статичен каталог НАП с търсене.
pub async fn units(
    State(_st): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let items: Vec<UnitDto> = domain::noms::units(q.search.as_deref())
        .into_iter()
        .map(|u| UnitDto {
            code: u.code,
            name: u.name,
            name_en: u.name_en,
            symbol: u.symbol,
            category: u.category,
            source: u.source,
        })
        .collect();
    let count = items.len();
    Ok(Json(serde_json::json!({
        "items": items, "count": count, "limit": 50, "offset": 0
    })))
}

/// POST /v1/accounts/seed-saft — досевва липсващите сметки НАП за активната фирма.
pub async fn seed_saft(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<SeedSaftDto>, ApiError> {
    let (_user, company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let created = storage::seed::fill_chart(&mut conn, company.id).await?;
    let total = domain::noms::chart().count() as i64;
    Ok(Json(SeedSaftDto {
        created,
        skipped: total - created,
        total,
    }))
}
