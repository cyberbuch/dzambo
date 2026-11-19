//! ДДС: дневници за продажби и покупки + CSV файл за НАП.

use axum::{
    extract::{Extension, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use domain::dto::VatRowDto;

use crate::{
    errors::ApiError,
    routes::common::active_company,
    state::{AppState, Session},
};

#[derive(serde::Deserialize)]
pub struct PeriodQuery {
    pub period: String,
}

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}

fn valid_period(p: &str) -> bool {
    let parts: Vec<&str> = p.split('-').collect();
    parts.len() == 2
        && parts[0]
            .parse::<i32>()
            .map(|y| (1900..=2200).contains(&y))
            .unwrap_or(false)
        && parts[1]
            .parse::<u32>()
            .map(|m| (1..=12).contains(&m))
            .unwrap_or(false)
}

async fn rows(
    st: &AppState,
    session: &Session,
    direction: &str,
    period: &str,
) -> Result<Vec<VatRowDto>, ApiError> {
    if !valid_period(period) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "периодът трябва да е във формат YYYY-MM",
        ));
    }
    let (_u, company) = active_company(st, &session.claims.sub).await?;
    let mut c = st.pool.get().await.map_err(|_| internal())?;
    let vat_type = if direction == "out" { "sales" } else { "purchase" };
    let aggs = storage::journal::vat_rows(&mut c, company.id, vat_type, period).await?;
    let cps = storage::counterparts::list(&mut c, company.id).await?;
    let map: std::collections::HashMap<i64, String> =
        cps.iter().map(|(cp, _)| (cp.id, cp.eik.clone())).collect();
    Ok(aggs
        .iter()
        .map(|a| VatRowDto {
            number: a.document_number.clone(),
            issue_date: a.entry_date.to_string(),
            counterpart_name: a.counterpart_name.clone(),
            eik: map.get(&a.counterpart_id).cloned().unwrap_or_default(),
            net_amount: format!("{:.2}", a.net),
            vat_amount: format!("{:.2}", a.vat),
            total_amount: format!("{:.2}", a.total),
        })
        .collect())
}

pub async fn sales(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PeriodQuery>,
) -> Result<Json<Vec<VatRowDto>>, ApiError> {
    Ok(Json(rows(&st, &session, "out", &q.period).await?))
}

pub async fn purchases(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PeriodQuery>,
) -> Result<Json<Vec<VatRowDto>>, ApiError> {
    Ok(Json(rows(&st, &session, "in", &q.period).await?))
}

fn csv_line(cols: &[&str]) -> String {
    cols.iter()
        .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(";")
}

/// CSV файл за НАП: дневник продажби + покупки за месеца.
pub async fn file(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PeriodQuery>,
) -> Result<Response, ApiError> {
    let sales = rows(&st, &session, "out", &q.period).await?;
    let purchases = rows(&st, &session, "in", &q.period).await?;
    let mut out = String::new();
    let title = format!("ДДС дневник — продажби (месец {})", q.period);
    out.push_str(&csv_line(&[title.as_str()]));
    out.push('\n');
    out.push_str(&csv_line(&[
        "Номер",
        "Дата",
        "Контрагент",
        "ЕИК",
        "Нето",
        "ДДС",
        "Общо",
    ]));
    out.push('\n');
    for r in &sales {
        out.push_str(&csv_line(&[
            &r.number,
            &r.issue_date,
            &r.counterpart_name,
            &r.eik,
            &r.net_amount,
            &r.vat_amount,
            &r.total_amount,
        ]));
        out.push('\n');
    }
    out.push('\n');
    let title = format!("ДДС дневник — покупки (месец {})", q.period);
    out.push_str(&csv_line(&[title.as_str()]));
    out.push('\n');
    out.push_str(&csv_line(&[
        "Номер",
        "Дата",
        "Контрагент",
        "ЕИК",
        "Нето",
        "ДДС",
        "Общо",
    ]));
    out.push('\n');
    for r in &purchases {
        out.push_str(&csv_line(&[
            &r.number,
            &r.issue_date,
            &r.counterpart_name,
            &r.eik,
            &r.net_amount,
            &r.vat_amount,
            &r.total_amount,
        ]));
        out.push('\n');
    }
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"ddes-{}.csv\"", q.period),
            ),
        ],
        out,
    )
        .into_response())
}
