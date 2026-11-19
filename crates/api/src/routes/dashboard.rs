//! GET /v1/dashboard — табло на активната фирма.

use axum::{
    extract::{Extension, State},
    Json,
};
use chrono::Datelike;
use serde_json::json;
use storage::{companies, dashboard, users};

use crate::{
    errors::ApiError,
    mappers,
    state::{AppState, Session},
};

pub async fn dashboard(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut conn = st.pool.get().await.map_err(|_| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "вътрешна грешка",
        )
    })?;
    let user = users::by_login(&mut conn, &session.claims.sub).await?;

    let Some(user) = user else {
        return Ok(Json(json!({"sub": session.claims.sub, "permissions": []})));
    };

    let Some((company, _details)) =
        companies::resolve_active(&mut conn, &session.claims.sub, user.id).await?
    else {
        // Без фирма — нули, както ytbuch.
        return Ok(Json(json!({
            "company_id": 0,
            "year": chrono::Utc::now().year(),
            "invoices_draft": 0,
            "invoices_posted": 0,
            "receivables": "0.00",
            "payables": "0.00",
            "vat_output": "0.00",
            "vat_input": "0.00",
            "revenue_ytd": "0.00",
            "expense_ytd": "0.00",
            "bank_total": "0.00",
            "recent": []
        })));
    };

    let numbers = dashboard::numbers(&mut conn, company.id).await?;
    Ok(Json(
        serde_json::to_value(mappers::dashboard_dto(company.id, numbers)).unwrap(),
    ))
}
