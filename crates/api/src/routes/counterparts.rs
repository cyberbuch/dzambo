//! Контрагенти: GET/POST /v1/counterparts, GET/PATCH/DELETE /v1/counterparts/{id}.
//! Дедупликация по ДДС/ЕИК, авто-ЕИК от ДДС BG, placeholder 999999999.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use domain::dto::CounterpartDto;
use storage::{
    counterparts::{self, CounterpartRow},
    models::counterparts::{NewCounterpart, NewCounterpartExtra},
};

use crate::{
    errors::ApiError,
    mappers,
    routes::common::{active_company, PageQuery, PageSlice},
    state::{AppState, Session},
};

const TYPES: [&str; 3] = ["customer", "supplier", "both"];
const ADDRESS_TYPES: [&str; 5] = [
    "StreetAddress",
    "PostalAddress",
    "BillingAddress",
    "ShipToAddress",
    "ShipFromAddress",
];

fn norm_eik(vat: &str) -> String {
    let vat = vat.trim().to_uppercase();
    if let Some(rest) = vat.strip_prefix("BG") {
        let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits;
        }
    }
    "999999999".to_string()
}

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
    let rows = counterparts::list(&mut conn, company.id).await?;
    let slice = PageSlice::from_vec(rows, &q);
    let items: Vec<_> = slice
        .items
        .iter()
        .map(|r| serde_json::to_value(mappers::counterpart_dto(r)).unwrap())
        .collect();
    Ok(Json(serde_json::json!({
        "items": items, "count": slice.count(), "total": slice.total,
        "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more(),
    })))
}

/// Валидация на нормализираните полета.
fn validate(dto: &CounterpartDto) -> Result<(), ApiError> {
    if dto.name.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "името е задължително",
        ));
    }
    if !TYPES.contains(&dto.counterpart_type.as_str()) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "counterpart_type трябва да е customer, supplier или both",
        ));
    }
    if !dto.address_type.is_empty() && !ADDRESS_TYPES.contains(&dto.address_type.as_str()) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "невалиден address_type",
        ));
    }
    Ok(())
}

async fn resolve_ids(st: &AppState, session: &Session) -> Result<(i64, String), ApiError> {
    let (_user, company) = active_company(st, &session.claims.sub).await?;
    Ok((company.id, session.claims.sub.clone()))
}

pub async fn create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(dto): Json<CounterpartDto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    validate(&dto)?;
    let (company_id, _) = resolve_ids(&st, &session).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;

    let eik = {
        let t = dto.eik.trim();
        if t.is_empty() {
            norm_eik(&dto.vat_number)
        } else {
            t.to_uppercase()
        }
    };
    let dup =
        counterparts::duplicate_error(&mut conn, company_id, 0, &dto.vat_number, &eik).await?;
    if let Some(msg) = dup {
        return Err(ApiError::new(StatusCode::CONFLICT, msg));
    }

    let row: CounterpartRow = counterparts::insert(
        &mut conn,
        &NewCounterpart {
            company_id,
            name: &dto.name,
            counterpart_type: &dto.counterpart_type,
            eik: &eik,
            vat_number: &dto.vat_number,
            address: &dto.address,
            city: &dto.city,
            post_code: &dto.post_code,
            country: &dto.country,
            contact_person: &dto.contact_person,
        },
        &NewCounterpartExtra {
            id: 0,
            vies_address: &dto.vies_address,
            street_name: &dto.street_name,
            building_number: &dto.building_number,
            address_building: &dto.address_building,
            additional_address_detail: &dto.additional_address_detail,
            address_type: &dto.address_type,
            region: &dto.region,
            email: &dto.email,
            phone: &dto.phone,
        },
    )
    .await?;
    let _ = row;
    let saved = counterparts::by_id(&mut conn, row.0.id)
        .await?
        .expect("записан контрагент");
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(mappers::counterpart_dto(&saved)).unwrap()),
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
    let row = counterparts::by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("контрагентът не е намерен"))?;
    Ok(Json(
        serde_json::to_value(mappers::counterpart_dto(&row)).unwrap(),
    ))
}

pub async fn update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(dto): Json<CounterpartDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validate(&dto)?;
    let (_user, _company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let existing = counterparts::by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("контрагентът не е намерен"))?;
    let eik = {
        let t = dto.eik.trim();
        if t.is_empty() {
            norm_eik(&dto.vat_number)
        } else {
            t.to_uppercase()
        }
    };
    let dup =
        counterparts::duplicate_error(&mut conn, existing.0.company_id, id, &dto.vat_number, &eik)
            .await?;
    if let Some(msg) = dup {
        return Err(ApiError::new(StatusCode::CONFLICT, msg));
    }
    let _ = existing;
    let row = counterparts::update(
        &mut conn,
        id,
        &NewCounterpart {
            company_id: 0,
            name: &dto.name,
            counterpart_type: &dto.counterpart_type,
            eik: &eik,
            vat_number: &dto.vat_number,
            address: &dto.address,
            city: &dto.city,
            post_code: &dto.post_code,
            country: &dto.country,
            contact_person: &dto.contact_person,
        },
        &NewCounterpartExtra {
            id,
            vies_address: &dto.vies_address,
            street_name: &dto.street_name,
            building_number: &dto.building_number,
            address_building: &dto.address_building,
            additional_address_detail: &dto.additional_address_detail,
            address_type: &dto.address_type,
            region: &dto.region,
            email: &dto.email,
            phone: &dto.phone,
        },
    )
    .await?;
    Ok(Json(
        serde_json::to_value(mappers::counterpart_dto(&row)).unwrap(),
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
    counterparts::delete(&mut conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
