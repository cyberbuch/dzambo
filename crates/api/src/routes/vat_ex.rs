//! GET /v1/vat-exemptions — основания за неначисляване на ДДС.

use axum::{extract::{Extension, Query, State}, Json};
use std::collections::HashMap;
use crate::{errors::ApiError, routes::common::active_company, state::{AppState, Session}};

pub async fn index(State(st): State<AppState>, Extension(s): Extension<Session>, Query(q): Query<HashMap<String,String>>) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u,_c) = active_company(&st, &s.claims.sub).await?;
    let items = domain::vat_ex::items(q.get("search").map(|x| x.as_str()).unwrap_or(""));
    let vals: Vec<serde_json::Value> = items.iter().map(|e| serde_json::json!({
        "code": e.code, "name": e.name, "name_en": e.name_en, "legal_basis": e.legal_basis,
        "legal_basis_en": e.legal_basis_en, "ubl_category_code": e.ubl_category_code,
        "ubl_exemption_code": e.ubl_exemption_code, "description": e.description
    })).collect();
    let n = vals.len();
    Ok(Json(serde_json::json!({"items": vals, "count": n, "total": n, "limit": 500, "offset": 0, "has_more": false})))
}
