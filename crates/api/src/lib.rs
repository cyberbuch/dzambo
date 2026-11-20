//! axum router на dzambo. Публични пътища: auth; всичко останало — зад
//! require_auth middleware.

pub mod errors;
pub mod mappers;
pub mod mappers_invoices;
pub mod posting;
pub mod print;
pub mod routes;
pub mod saft;
pub mod state;
pub mod ubl;
pub mod vatfile;

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
        .route("/v1/auth/token", post(routes::auth::token))
        .route(
            "/v1/auth/registration-status",
            get(routes::auth_flow::registration_status),
        )
        .route("/v1/auth/register", post(routes::auth_flow::register))
        .route(
            "/v1/auth/forgot-password",
            post(routes::auth_flow::forgot_password),
        )
        .route(
            "/v1/auth/reset-password",
            post(routes::auth_flow::reset_password),
        )
        .route("/v1/auth/mfa", post(routes::auth::mfa));

    let protected = Router::new()
        .route("/v1/me", get(routes::me::me))
        .route("/v1/me/totp/start", post(routes::profile_totp::totp_start))
        .route(
            "/v1/me/totp/confirm",
            post(routes::profile_totp::totp_confirm),
        )
        .route(
            "/v1/me/totp/disable",
            post(routes::profile_totp::totp_disable),
        )
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
        .route("/v1/vat-exemptions", get(routes::vat_ex::index))
        .route("/v1/saft/nomenclatures", get(routes::compat::nomenclatures))
        .route(
            "/v1/saft/movement-mappings",
            get(routes::smap::index).post(routes::smap::create),
        )
        .route(
            "/v1/saft/movement-mappings/defaults",
            post(routes::smap::defaults),
        )
        .route(
            "/v1/saft/movement-mappings/{id}",
            axum::routing::patch(routes::smap::update).delete(routes::smap::delete),
        )
        .route("/v1/exchange-rates", get(routes::rates::index))
        .route("/v1/exchange-rates/rate", get(routes::rates::rate))
        .route("/v1/exchange-rates/cross", get(routes::rates::cross))
        .route("/v1/exchange-rates/import", post(routes::rates::import))
        .route("/v1/audit-logs", get(routes::access::list_audit))
        .route(
            "/v1/audit-logs/{entity}/{id}",
            get(routes::access::audit_for),
        )
        .route(
            "/v1/roles",
            get(routes::access::list_roles).post(routes::access::create_role),
        )
        .route(
            "/v1/roles/{id}",
            axum::routing::patch(routes::access::update_role).delete(routes::access::delete_role),
        )
        .route("/v1/users", get(routes::compat::users))
        .route(
            "/v1/internal-docs",
            get(routes::idocs::list).post(routes::idocs::create),
        )
        .route(
            "/v1/internal-docs/next-number",
            get(routes::idocs::next_number),
        )
        .route(
            "/v1/internal-docs/{id}",
            get(routes::idocs::show)
                .patch(routes::idocs::update)
                .delete(routes::idocs::delete),
        )
        .route(
            "/v1/internal-docs/{id}/confirm",
            post(routes::idocs::confirm),
        )
        .route("/v1/internal-docs/{id}/print", get(routes::idocs::print))
        .route(
            "/v1/material-issues",
            get(routes::stockops::list_issues).post(routes::stockops::create_issue),
        )
        .route(
            "/v1/material-issues/{id}",
            get(routes::stockops::show_issue)
                .patch(routes::stockops::update_issue)
                .delete(routes::stockops::delete_issue),
        )
        .route(
            "/v1/material-issues/{id}/confirm",
            post(routes::stockops::confirm_issue),
        )
        .route(
            "/v1/material-issues/{id}/unpost",
            post(routes::stockops::unpost_issue),
        )
        .route(
            "/v1/stock/rules",
            get(routes::stockops::list_rules).post(routes::stockops::create_rule),
        )
        .route(
            "/v1/stock/rules/{id}",
            axum::routing::delete(routes::stockops::delete_rule),
        )
        .route("/v1/stock/writeoff", post(routes::stockops::writeoff))
        .route("/v1/stock/rebuild", post(routes::stockops::rebuild))
        .route(
            "/v1/locations",
            get(routes::stockops::list_locations).post(routes::stockops::create_location),
        )
        .route("/v1/fs/lines", get(routes::fsform::lines))
        .route("/v1/reports", get(routes::reports::index))
        .route("/v1/reports/export", get(routes::reports::export))
        .route(
            "/v1/fs/formulas",
            get(routes::fsform::index).put(routes::fsform::put),
        )
        .route("/v1/fs/formulas/defaults", post(routes::fsform::defaults))
        .route("/v1/fs/formulas/validate", post(routes::fsform::validate))
        .route("/v1/fs/formulas/clear", post(routes::fsform::clear))
        .route(
            "/v1/system-settings",
            get(routes::system_settings::index).put(routes::system_settings::update),
        )
        .route(
            "/v1/system-settings/db-check",
            get(routes::system_settings::db_check),
        )
        .route(
            "/v1/system-settings/smtp-test",
            post(routes::system_settings::smtp_test),
        )
        .route(
            "/v1/system-settings/s3-test",
            post(routes::system_settings::s3_test),
        )
        .route(
            "/v1/system-settings/s3-backup",
            post(routes::system_settings::s3_backup),
        )
        .route(
            "/v1/system-settings/s3-backups",
            get(routes::system_settings::s3_backups),
        )
        .route(
            "/v1/system-settings/s3-delete",
            post(routes::system_settings::s3_delete),
        )
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
        // Фактури
        .route(
            "/v1/invoices",
            get(routes::invoices::list).post(routes::invoices::create),
        )
        .route(
            "/v1/invoices/next-number",
            get(routes::invoices::next_number),
        )
        .route("/v1/invoices/bulk", post(routes::invoices::bulk))
        .route(
            "/v1/invoices/{id}",
            get(routes::invoices::show)
                .patch(routes::invoices::update)
                .delete(routes::invoices::delete),
        )
        .route("/v1/invoices/{id}/post", post(routes::invoices::post))
        .route("/v1/invoices/{id}/unpost", post(routes::invoices::unpost))
        .route("/v1/invoices/{id}/ubl", get(routes::invoices::ubl))
        .route("/v1/invoices/{id}/print", get(routes::invoices::print))
        // Дневник
        .route(
            "/v1/journal",
            get(routes::journal::list).post(routes::journal::create),
        )
        .route(
            "/v1/journal/{id}",
            get(routes::journal::show)
                .put(routes::journal::update)
                .patch(routes::journal::update)
                .delete(routes::journal::delete),
        )
        // Периоди
        .route("/v1/accounting-periods", get(routes::periods::index))
        .route("/v1/accounting-periods/close", post(routes::periods::close))
        .route(
            "/v1/accounting-periods/reopen",
            post(routes::periods::reopen),
        )
        // ДДС
        .route("/v1/vat/sales", get(routes::vat::sales))
        .route("/v1/vat/purchases", get(routes::vat::purchases))
        .route("/v1/vat/file", get(routes::vat::file))
        .route("/v1/vat/export", get(routes::vat_export::export))
        .route("/v1/vat/registers", get(routes::vat_export::registers))
        .route("/v1/vat/return", get(routes::vat_export::registers))
        .route(
            "/v1/opening-balances",
            get(routes::opening::list).put(routes::opening::save),
        )
        .route(
            "/v1/opening-balances/{id}",
            axum::routing::delete(routes::opening::delete),
        )
        .route(
            "/v1/closing-operations/preview",
            get(routes::closing::preview),
        )
        .route("/v1/closing-operations/run", post(routes::closing::run))
        .route(
            "/v1/bank-accounts",
            get(routes::banks::accounts).post(routes::banks::create_account),
        )
        .route("/v1/bank-transactions", get(routes::banks::transactions))
        .route("/v1/bank-transactions/import", post(routes::banks::import))
        .route("/v1/bank-transactions/{id}/post", post(routes::banks::post))
        .route(
            "/v1/bank-matching-rules",
            get(routes::bank_rules::index).post(routes::bank_rules::create),
        )
        .route(
            "/v1/bank-matching-rules/{id}",
            axum::routing::delete(routes::bank_rules::delete),
        )
        .route(
            "/v1/bank-matching-rules/defaults",
            post(routes::bank_rules::defaults),
        )
        .route(
            "/v1/bank-matching-rules/preview",
            post(routes::bank_rules::preview),
        )
        .route(
            "/v1/bank-matching-rules/auto-allocate",
            post(routes::bank_rules::auto_allocate),
        )
        .route(
            "/v1/fixed-assets",
            get(routes::fixed_assets::index).post(routes::fixed_assets::create),
        )
        .route(
            "/v1/fixed-assets/{id}",
            axum::routing::delete(routes::fixed_assets::delete).patch(routes::asset_ops::update),
        )
        .route(
            "/v1/fixed-assets/{id}/revalue",
            post(routes::asset_ops::revalue),
        )
        .route(
            "/v1/fixed-assets/{id}/move",
            post(routes::asset_ops::move_asset),
        )
        .route(
            "/v1/fixed-assets/{id}/conserve",
            post(routes::asset_ops::conserve),
        )
        .route(
            "/v1/fixed-assets/{id}/end-conserve",
            post(routes::asset_ops::end_conserve),
        )
        .route(
            "/v1/fixed-assets/{id}/dispose",
            post(routes::asset_ops::dispose),
        )
        .route(
            "/v1/fixed-assets/{id}/events",
            get(routes::asset_ops::events),
        )
        .route(
            "/v1/fixed-asset-categories",
            get(routes::asset_ops::categories).post(routes::asset_ops::create_category),
        )
        .route(
            "/v1/fixed-assets/depreciation/preview",
            post(routes::fixed_assets::preview),
        )
        .route(
            "/v1/fixed-assets/depreciation/post",
            post(routes::fixed_assets::post_dep),
        )
        .route("/v1/stock/report", get(routes::stock_report::report))
        .route(
            "/v1/stock-documents",
            get(routes::stock_docs::list).post(routes::stock_docs::create),
        )
        .route(
            "/v1/dividends",
            get(routes::dividends::index).post(routes::dividends::create),
        )
        .route("/v1/saft/export", axum::routing::get(saft::export))
        .route("/v1/nomenclatures", get(routes::nomenclatures::index))
        .route("/v1/nomenclature-kinds", get(routes::nomenclatures::kinds))
        .route(
            "/v1/recipes",
            get(routes::mfg::recipes).post(routes::mfg::create_recipe),
        )
        .route(
            "/v1/recipes/{id}",
            axum::routing::delete(routes::mfg::delete_recipe),
        )
        .route(
            "/v1/production-orders",
            get(routes::mfg::orders).post(routes::mfg::create_order),
        )
        .route(
            "/v1/production-orders/{id}/confirm",
            post(routes::mfg::confirm),
        )
        .layer(from_fn_with_state(st.clone(), state::require_auth));

    Router::new().merge(public).merge(protected).with_state(st)
}
