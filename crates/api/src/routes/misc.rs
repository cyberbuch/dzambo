//! /v1/me/password, /v1/me/email, /v1/addresses/parse, /v1/inventory/at-location.

use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use domain::{addr, auth};
use storage::{stock, users};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}
fn unauth(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, m)
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}
fn sval(b: &Value, k: &str) -> String {
    b.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

pub async fn change_password(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (u, _c) = active_company(&st, &s.claims.sub).await?;
    let current = sval(&b, "current_password");
    let new = sval(&b, "new_password");
    if !auth::verify_password(&current, &u.password_hash) {
        return Err(unauth("грешна текуща парола"));
    }
    if new.len() < 8 {
        return Err(unproc("новата парола трябва да е поне 8 символа"));
    }
    let hash = auth::hash_password(&new)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let mut c = conn(&st).await?;
    users::set_password(&mut c, u.id, &hash).await?;
    let fresh = users::by_id(&mut c, u.id).await?.expect("потребител");
    let (token, expires_in) = auth::issue_token(&st.jwt_secret, &fresh.email, fresh.session_ver)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(
        json!({"message": "Паролата е променена", "access_token": token, "expires_in": expires_in}),
    ))
}

pub async fn change_email(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (u, _c) = active_company(&st, &s.claims.sub).await?;
    let email = sval(&b, "email").trim().to_string();
    let current = sval(&b, "current_password");
    if !auth::verify_password(&current, &u.password_hash) {
        return Err(unauth("грешна парола"));
    }
    if !email.contains('@') {
        return Err(unproc("валиден email е задължителен"));
    }
    let mut c = conn(&st).await?;
    if let Some(other) = users::by_login(&mut c, &email).await? {
        if other.id != u.id {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "email-ът вече се използва",
            ));
        }
    }
    users::update_email(&mut c, u.id, &email).await?;
    let fresh = users::by_id(&mut c, u.id).await?.expect("потребител");
    let (token, expires_in) = auth::issue_token(&st.jwt_secret, &fresh.email, fresh.session_ver)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(
        json!({"message": "Email-ът е променен", "access_token": token, "expires_in": expires_in}),
    ))
}

pub async fn parse_address(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, _c) = active_company(&st, &s.claims.sub).await?;
    let address = sval(&b, "address");
    let country = sval(&b, "country");
    let city = sval(&b, "city");
    let post = sval(&b, "post_code");
    let parts = addr::split_local(&address, &country, &city, &post);
    let note = if parts.ok {
        ""
    } else {
        "адресът не можа да се раздели"
    };
    Ok(Json(addr::parts_json(&parts, note)))
}

#[derive(Deserialize)]
pub struct LocationQuery {
    pub location_id: Option<i64>,
}

pub async fn inventory_at_location(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Query(q): Query<LocationQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let loc = q.location_id.unwrap_or(0);
    let mut co = conn(&st).await?;
    let rows = stock::at_location(&mut co, c.id, loc).await?;
    let items: Vec<Value> = rows.iter().map(|(pid, name, code, unit, qty, cost)| json!({
        "product_id": pid, "name": name, "code": code, "unit": unit, "quantity": qty, "unit_cost": cost,
    })).collect();
    Ok(Json(json!({"items": items, "count": items.len()})))
}
