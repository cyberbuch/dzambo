//! /v1/counterparts/vies-lookup + /vies — справка в VIES (REST, без ключ) и
//! създаване на контрагент от нея. Порт на ytbuch vies (без address split).

use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::Json;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use crate::errors::ApiError;
use crate::mappers;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use storage::counterparts;
use storage::models::counterparts::{NewCounterpart, NewCounterpartExtra};

const EU: &[&str] = &[
    "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "ES", "FI", "FR", "EL", "HR", "HU", "IE", "IT",
    "LT", "LU", "LV", "MT", "NL", "PL", "PT", "RO", "SE", "SI", "SK", "XI", "GR",
];

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}
fn bad_gateway(m: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_GATEWAY, m)
}

fn normalize(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}
fn shape_ok(vat: &str) -> bool {
    vat.len() >= 4 && vat.len() <= 15 && EU.contains(&&vat[..2])
}
fn api_cc(cc: &str) -> &str {
    if cc == "GR" {
        "EL"
    } else {
        cc
    }
}

struct Hit {
    valid: bool,
    name: String,
    address: String,
    user_error: String,
    request_date: String,
}

async fn fetch(vat: &str) -> Result<Hit, ApiError> {
    let cc = api_cc(&vat[..2]);
    let num = &vat[2..];
    let url = format!("https://ec.europa.eu/taxation_customs/vies/rest-api/ms/{cc}/vat/{num}");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| internal())?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|_| bad_gateway("VIES услугата е недостъпна (мрежова грешка)"))?;
    if resp.status() != reqwest::StatusCode::OK {
        return Err(bad_gateway(&format!(
            "VIES върна статус {} — опитайте пак",
            resp.status()
        )));
    }
    let body = resp
        .text()
        .await
        .map_err(|_| bad_gateway("невалиден JSON отговор от VIES"))?;
    let doc: Value =
        serde_json::from_str(&body).map_err(|_| bad_gateway("невалиден JSON отговор от VIES"))?;
    let text = |key: &str| {
        doc.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };
    Ok(Hit {
        valid: doc
            .get("isValid")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        name: text("name"),
        address: text("address"),
        user_error: text("userError"),
        request_date: text("requestDate"),
    })
}

#[derive(Deserialize)]
pub struct VatQuery {
    pub vat: Option<String>,
}

pub async fn lookup(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Query(q): Query<VatQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, _c) = active_company(&st, &s.claims.sub).await?;
    let vat = normalize(&q.vat.unwrap_or_default());
    if !shape_ok(&vat) {
        return Err(unproc("vat трябва да е ЕС ДДС номер (напр. BG123456789)"));
    }
    let hit = fetch(&vat).await?;
    let addr = if hit.address == "---" {
        ""
    } else {
        hit.address.as_str()
    };
    Ok(Json(json!({
        "valid": hit.valid, "name": hit.name, "address": addr, "vies_address": addr,
        "user_error": hit.user_error, "request_date": hit.request_date,
        "vat_number": vat, "country_code": &vat[..2], "country": &vat[..2],
        "street_name": "", "building_number": "", "address_building": "",
        "additional_address_detail": "", "address_type": "", "city": "", "post_code": "", "region": "",
        "parse_ok": false, "parse_note": "адресът не е разделен автоматично",
    })))
}

pub async fn create(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let vat = normalize(b.get("vat_number").and_then(Value::as_str).unwrap_or(""));
    if !shape_ok(&vat) {
        return Err(unproc(
            "vat_number трябва да е ЕС ДДС номер (напр. BG123456789)",
        ));
    }
    let kind = b
        .get("counterpart_type")
        .and_then(Value::as_str)
        .unwrap_or("both")
        .trim()
        .to_string();
    if !matches!(kind.as_str(), "customer" | "supplier" | "both") {
        return Err(unproc(
            "counterpart_type трябва да е customer/supplier/both",
        ));
    }
    let hit = fetch(&vat).await?;
    if !hit.valid {
        return Err(unproc("ДДС номерът не е намерен във VIES"));
    }
    let cc = &vat[..2];
    let eik = if cc == "BG" {
        vat[2..].to_string()
    } else {
        "999999999".to_string()
    };
    let mut conn = st.pool.get().await.map_err(|_| internal())?;

    // Съществуващ контрагент със същия ДДС/ЕИК → връщаме него.
    let existing: Option<i64> = storage::schema::counterparts::table
        .filter(storage::schema::counterparts::company_id.eq(c.id))
        .filter(
            storage::schema::counterparts::vat_number
                .eq(&vat)
                .or(storage::schema::counterparts::eik.eq(&eik)),
        )
        .select(storage::schema::counterparts::id)
        .first(&mut conn)
        .await
        .optional()
        .map_err(|e| {
            tracing::error!("заявка: {e}");
            internal()
        })?;
    if let Some(id) = existing {
        let saved = counterparts::by_id(&mut conn, id)
            .await?
            .expect("контрагент");
        return Ok((
            StatusCode::OK,
            Json(serde_json::to_value(mappers::counterpart_dto(&saved)).unwrap()),
        ));
    }

    let addr = if hit.address == "---" {
        String::new()
    } else {
        hit.address.clone()
    };
    let name = if hit.name.is_empty() || hit.name == "---" {
        format!("VIES {vat}")
    } else {
        hit.name.clone()
    };
    let row = counterparts::insert(
        &mut conn,
        &NewCounterpart {
            company_id: c.id,
            name: &name,
            counterpart_type: &kind,
            eik: &eik,
            vat_number: &vat,
            address: &addr,
            city: "",
            post_code: "",
            country: cc,
            contact_person: "",
        },
        &NewCounterpartExtra {
            id: 0,
            vies_address: &addr,
            street_name: "",
            building_number: "",
            address_building: "",
            additional_address_detail: "",
            address_type: "",
            region: "",
            email: "",
            phone: "",
        },
    )
    .await?;
    let saved = counterparts::by_id(&mut conn, row.0.id)
        .await?
        .expect("записан контрагент");
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(mappers::counterpart_dto(&saved)).unwrap()),
    ))
}
