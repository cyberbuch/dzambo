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
        // Сметкоплан
        .route(
            "/v1/accounts",
            get(routes::master::list).post(routes::master::create),
        )
        .route("/v1/accounts/seed-saft", post(routes::master::seed_saft))
        .route("/v1/units", get(routes::master::units))
        .route(
            "/v1/accounts/{id}",
            get(routes::master::show)
                .patch(routes::master::update)
                .delete(routes::master::delete),
        )
        // Контрагенти
        .route(
            "/v1/counterparts",
            get(routes::counterparts::list).post(routes::counterparts::create),
        )
        .route(
            "/v1/counterparts/{id}",
            get(routes::counterparts::show)
                .patch(routes::counterparts::update)
                .delete(routes::counterparts::delete),
        )
        // Стоки
        .route(
            "/v1/products",
            get(routes::products::list).post(routes::products::create),
        )
        .route(
            "/v1/products/{id}",
            get(routes::products::show)
                .patch(routes::products::update)
                .delete(routes::products::delete),
        )
        // Настройки на фирмата
        .route("/v1/companies/{id}/settings", get(routes::settings::show))
        .route(
            "/v1/companies/{id}/locations",
            post(routes::related::location_create),
        )
        .route(
            "/v1/company-locations/{id}",
            axum::routing::patch(routes::related::location_update)
                .delete(routes::related::location_delete),
        )
        .route(
            "/v1/companies/{id}/beneficial-owners",
            post(routes::related::owner_create),
        )
        .route(
            "/v1/beneficial-owners/{id}",
            axum::routing::patch(routes::related::owner_update)
                .delete(routes::related::owner_delete),
        )
        .route(
            "/v1/companies/{id}/ultimate-parents",
            post(routes::related::parent_create),
        )
        .route(
            "/v1/ultimate-parents/{id}",
            axum::routing::patch(routes::related::parent_update)
                .delete(routes::related::parent_delete),
        )
        .route(
            "/v1/companies/{id}/document-series",
            post(routes::related::series_create),
        )
        .route(
            "/v1/document-series/{id}",
            axum::routing::patch(routes::related::series_update)
                .delete(routes::related::series_delete),
        )
        .layer(from_fn_with_state(st.clone(), state::require_auth));

    Router::new().merge(public).merge(protected).with_state(st)
}
