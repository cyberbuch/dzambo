//! /v1/internal-docs — реална логика (CRUD, confirm, print, next-number).

use axum::extract::{Extension, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::Json;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::{active_company, PageQuery, PageSlice};
use crate::state::{AppState, Session};
use storage::{access, idocs, periods, stockops};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}
fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}
fn conflict(m: &str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, m)
}
fn date(s: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| unproc("doc_date трябва да е YYYY-MM-DD"))
}

#[derive(Deserialize)]
pub struct DocBody {
    pub number: Option<String>,
    pub doc_date: String,
    pub from_location_id: Option<i64>,
    pub to_location_id: Option<i64>,
    pub notes: Option<String>,
    pub handed_by: Option<String>,
    pub received_by: Option<String>,
    pub lines: Option<Vec<Value>>,
}

pub async fn next_number(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let n = idocs::next_number(&mut c, company.id).await?;
    Ok(Json(json!({"number": n})))
}

pub async fn list(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let rows = idocs::list(&mut c, company.id).await?;
    let items: Vec<Value> = rows.iter().map(|(d, l)| idocs::doc_json(d, l)).collect();
    let slice = PageSlice::from_vec(items, &q);
    Ok(Json(
        json!({"items": slice.items, "count": slice.count(), "total": slice.total, "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more()}),
    ))
}

pub async fn show(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    match idocs::get(&mut c, company.id, id).await? {
        Some((d, l)) => Ok(Json(idocs::doc_json(&d, &l))),
        None => Err(ApiError::not_found("документът не е намерен")),
    }
}

pub async fn create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<DocBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let number = match b.number {
        Some(n) if !n.is_empty() => n,
        _ => idocs::next_number(&mut c, company.id).await?,
    };
    let (d, l) = idocs::create(
        &mut c,
        company.id,
        &number,
        date(&b.doc_date)?,
        b.from_location_id.unwrap_or(0),
        b.to_location_id.unwrap_or(0),
        &b.notes.unwrap_or_default(),
        &b.handed_by.unwrap_or_default(),
        &b.received_by.unwrap_or_default(),
        &b.lines.unwrap_or_default(),
    )
    .await?;
    let _ = access::record(
        &mut c,
        company.id,
        u.id,
        &u.email,
        &u.name,
        "internal_doc",
        d.id,
        &d.number,
        "create",
        &format!("Протокол {}", d.number),
        "",
        "",
        None,
        Some(idocs::doc_json(&d, &l)),
    )
    .await;
    Ok((StatusCode::CREATED, Json(idocs::doc_json(&d, &l))))
}

pub async fn update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(b): Json<DocBody>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let Some((d, _)) = idocs::get(&mut c, company.id, id).await? else {
        return Err(ApiError::not_found("документът не е намерен"));
    };
    if d.status == "confirmed" {
        return Err(conflict("потвърден протокол не се редактира"));
    }
    if let Some(lines) = &b.lines {
        idocs::replace_lines(&mut c, id, lines).await?;
    }
    let (d, l) = idocs::get(&mut c, company.id, id).await?.unwrap();
    Ok(Json(idocs::doc_json(&d, &l)))
}

pub async fn delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let Some((d, _)) = idocs::get(&mut c, company.id, id).await? else {
        return Err(ApiError::not_found("документът не е намерен"));
    };
    if d.status == "confirmed" {
        return Err(conflict("потвърден протокол не се трие"));
    }
    idocs::delete(&mut c, company.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn confirm(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let (u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let Some((d, l)) = idocs::get(&mut c, company.id, id).await? else {
        return Err(ApiError::not_found("документът не е намерен"));
    };
    if d.from_location_id == 0 || d.to_location_id == 0 || d.from_location_id == d.to_location_id {
        return Err(unproc("нужни са два различни активни обекта"));
    }
    if periods::is_closed(&mut c, company.id, &periods::month_key(d.doc_date)).await? {
        return Err(conflict("периодът е затворен"));
    }
    idocs::confirm_assets(&mut c, company.id, &d, &l).await?;
    idocs::set_status(&mut c, id, "confirmed").await?;
    let _ = stockops::rebuild(&mut c, company.id).await?;
    let (d, l) = idocs::get(&mut c, company.id, id).await?.unwrap();
    let _ = access::record(
        &mut c,
        company.id,
        u.id,
        &u.email,
        &u.name,
        "internal_doc",
        d.id,
        &d.number,
        "post",
        &format!("Потвърден протокол {}", d.number),
        "",
        "",
        None,
        Some(idocs::doc_json(&d, &l)),
    )
    .await;
    Ok(Json(idocs::doc_json(&d, &l)))
}

#[derive(Deserialize)]
pub struct PrintQuery {
    pub format: Option<String>,
}

pub async fn print(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Query(q): Query<PrintQuery>,
) -> Result<axum::response::Response, ApiError> {
    use axum::response::IntoResponse;
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let Some((d, l)) = idocs::get(&mut c, company.id, id).await? else {
        return Err(ApiError::not_found("документът не е намерен"));
    };
    let html = crate::print::internal_doc_html(&d, &l);
    let fmt = q.format.as_deref().unwrap_or("pdf");
    let (bytes, mime, ext) = crate::print::render_html(&html, fmt)?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"protokol-{}.{ext}\"", d.number),
            ),
        ],
        bytes,
    )
        .into_response())
}
