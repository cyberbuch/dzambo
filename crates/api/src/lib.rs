//! axum router на dzambo. Публични пътища: auth; всичко останало — зад
//! require_auth middleware.

pub mod errors;
pub mod mappers;
pub mod routes;
pub mod state;

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use crate::state::AppState;

pub fn router(st: AppState) -> Router {
    let public = Router::new()
        .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
        .route(
            "/ready",
            get(|| async { Json(json!({"status": "ok", "database": "up"})) }),
        )
        .route("/v1/auth/token", post(routes::auth::token));

    let protected = Router::new()
        .route("/v1/me", get(routes::me::me))
        .route("/v1/companies", get(routes::companies::list))
        .route(
            "/v1/active-company",
            get(routes::companies::get_active).put(routes::companies::put_active),
        )
        .route("/v1/dashboard", get(routes::dashboard::dashboard))
        .layer(from_fn_with_state(st.clone(), state::require_auth));

    Router::new().merge(public).merge(protected).with_state(st)
}
