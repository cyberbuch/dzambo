//! Фактури: чернова → пост → сторно; номерция; UBL.

use axum::{
    extract::{Extension, Path, Query, State},
    http::{header, StatusCode},
    Json,
};
use bigdecimal::BigDecimal;
use chrono::Datelike;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use domain::dto::{InvoiceDto, InvoiceLineDto, JournalEntryDto, NextNumberDto};
use storage::{
    invoices::{self, InvoiceRow},
    models::invoices::{NewInvoice, NewInvoiceLine},
};

use crate::{
    errors::ApiError,
    mappers_invoices, posting,
    routes::common::{active_company, PageQuery, PageSlice},
    state::{AppState, Session},
};

fn dec(raw: &Option<String>, default: &str) -> BigDecimal {
    raw.as_deref()
        .unwrap_or(default)
        .trim()
        .parse()
        .unwrap_or_else(|_| default.parse().unwrap())
}

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}

async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}

pub async fn list(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PageQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let rows = invoices::list(&mut c, company.id).await?;
    let slice = PageSlice::from_vec(rows, &q);
    let items: Vec<_> = slice
        .items
        .iter()
        .map(|h| serde_json::to_value(mappers_invoices::invoice_header_dto(h)).unwrap())
        .collect();
    Ok(Json(serde_json::json!({
        "items": items, "count": slice.count(), "total": slice.total,
        "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more(),
    })))
}

pub async fn create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(dto): Json<InvoiceDto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    if !matches!(dto.direction.as_str(), "out" | "in") {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "direction трябва да е out или in",
        ));
    }
    let issue_date = invoices::parse_date(&dto.issue_date)
        .ok_or_else(|| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "невалидна дата"))?;
    let due_date = if dto.due_date.trim().is_empty() {
        None
    } else {
        Some(invoices::parse_date(&dto.due_date).ok_or_else(|| {
            ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "невалидна дата на падеж")
        })?)
    };
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;

    let auto_number =
        dto.number.trim().is_empty() && (dto.direction == "out" || dto.document_type == "09");
    let number = if auto_number {
        invoices::next_out_number(&mut c, company.id, &dto.document_type)
            .await?
            .0
    } else {
        dto.number.trim().to_string()
    };
    let incoming_dup = !auto_number
        && dto.direction == "in"
        && dto.document_type != "09"
        && invoices::incoming_number_taken(&mut c, company.id, dto.counterpart_id, &number).await?;
    if incoming_dup {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "входящ номер този вече е регистриран за доставчика",
        ));
    }

    let month = if dto.accounting_month.trim().is_empty() {
        format!("{}-{:02}", issue_date.year(), issue_date.month())
    } else {
        dto.accounting_month.trim().to_string()
    };

    let new = NewInvoice {
        company_id: company.id,
        direction: &dto.direction,
        document_type: &dto.document_type,
        number: &number,
        issue_date,
        due_date,
        counterpart_id: dto.counterpart_id,
        counterpart_name: &dto.counterpart_name,
        currency: &dto.currency,
        currency_rate: dec(&dto.currency_rate, "1"),
        accounting_month: &month,
        original_invoice_id: dto.original_invoice_id,
    };
    let id = invoices::insert_draft(&mut c, &new, &[]).await?;
    insert_lines(&mut c, id, &dto.lines).await?;
    let row = invoices::by_id(&mut c, id).await?.ok_or_else(internal)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(mappers_invoices::invoice_dto(&row)).unwrap()),
    ))
}

/// Заменя редовете на фактура и преизчислява сумите.
async fn insert_lines(
    c: &mut storage::DbConn,
    invoice_id: i64,
    lines: &[InvoiceLineDto],
) -> Result<(), ApiError> {
    diesel::delete(
        storage::schema::invoice_lines::table
            .filter(storage::schema::invoice_lines::invoice_id.eq(invoice_id)),
    )
    .execute(&mut *c)
    .await?;
    for l in lines {
        let line = NewInvoiceLine {
            invoice_id,
            code: l.code.as_str(),
            description: l.description.as_str(),
            quantity: dec(&l.quantity, "1"),
            unit: l.unit.as_str(),
            unit_price: dec(&l.unit_price, "0"),
            vat_rate: dec(&l.vat_rate, "20"),
            product_id: l.product_id,
            net_amount: dec(&l.net_amount, "0"),
            vat_amount: dec(&l.vat_amount, "0"),
            total_amount: dec(&l.total_amount, "0"),
        };
        diesel::insert_into(storage::schema::invoice_lines::table)
            .values(&line)
            .execute(&mut *c)
            .await?;
    }
    let (net, vat, total) = storage::invoices::sum_of(c, invoice_id).await?;
    diesel::sql_query(
        "UPDATE invoices SET net_amount=$2, vat_amount=$3, total_amount=$4 WHERE id=$1",
    )
    .bind::<diesel::sql_types::BigInt, _>(invoice_id)
    .bind::<diesel::sql_types::Numeric, _>(net)
    .bind::<diesel::sql_types::Numeric, _>(vat)
    .bind::<diesel::sql_types::Numeric, _>(total)
    .execute(&mut *c)
    .await?;
    Ok(())
}

pub async fn show(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, _company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let row = invoices::by_id(&mut c, id)
        .await?
        .ok_or_else(|| ApiError::not_found("фактурата не е намерена"))?;
    Ok(Json(
        serde_json::to_value(mappers_invoices::invoice_dto(&row)).unwrap(),
    ))
}

pub async fn update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(dto): Json<InvoiceDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, _company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let existing = invoices::by_id(&mut c, id)
        .await?
        .ok_or_else(|| ApiError::not_found("фактурата не е намерена"))?;
    if existing.0.status != "draft" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "постната фактура не може да се редактира",
        ));
    }
    let issue_date = invoices::parse_date(&dto.issue_date).unwrap_or(existing.0.issue_date);
    diesel::update(storage::schema::invoices::table.find(id))
        .set((
            storage::schema::invoices::direction.eq(&dto.direction),
            storage::schema::invoices::document_type.eq(&dto.document_type),
            storage::schema::invoices::issue_date.eq(issue_date),
            storage::schema::invoices::counterpart_id.eq(dto.counterpart_id),
            storage::schema::invoices::counterpart_name.eq(&dto.counterpart_name),
            storage::schema::invoices::currency.eq(&dto.currency),
            storage::schema::invoices::notes.eq(&dto.notes),
            storage::schema::invoices::original_invoice_id.eq(dto.original_invoice_id),
        ))
        .execute(&mut c)
        .await?;
    insert_lines(&mut c, id, &dto.lines).await?;
    let row: InvoiceRow = invoices::by_id(&mut c, id).await?.ok_or_else(internal)?;
    Ok(Json(
        serde_json::to_value(mappers_invoices::invoice_dto(&row)).unwrap(),
    ))
}

pub async fn delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_u, _company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    storage::journal::delete_by_document(&mut c, id).await?;
    invoices::delete(&mut c, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct NextNumberQuery {
    pub document_type: String,
    pub direction: String,
}

pub async fn next_number(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<NextNumberQuery>,
) -> Result<Json<NextNumberDto>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let (number, _pad) = invoices::next_out_number(&mut c, company.id, &q.document_type).await?;
    Ok(Json(NextNumberDto {
        number,
        document_type: q.document_type,
        direction: q.direction,
    }))
}

pub async fn post(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<JournalEntryDto>), ApiError> {
    let (_u, _company) = active_company(&st, &session.claims.sub).await?;
    let row = posting::post_invoice(&st.pool, id).await?;
    Ok((
        StatusCode::CREATED,
        Json(mappers_invoices::journal_dto(&row)),
    ))
}

pub async fn unpost(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, _company) = active_company(&st, &session.claims.sub).await?;
    posting::unpost_invoice(&st.pool, id).await?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn ubl(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<axum::response::Response, ApiError> {
    use axum::response::IntoResponse;
    let (_u, _company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let row = invoices::by_id(&mut c, id)
        .await?
        .ok_or_else(|| ApiError::not_found("фактурата не е намерена"))?;
    let xml = crate::ubl::render(&row)?;
    let filename = format!("ubl-invoice-{}.xml", row.0.number);
    Ok((
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "application/xml; charset=utf-8".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        xml,
    )
        .into_response())
}
