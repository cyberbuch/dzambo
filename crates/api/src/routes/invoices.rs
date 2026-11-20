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
        payment_method: &dto.payment_method,
        tax_event_date: invoices::parse_date(&dto.tax_event_date),
        discount_percent: dec(&dto.discount_percent, "0"),
        discount_amount: dec(&dto.discount_amount, "0"),
        vat_exemption_reason: &dto.vat_exemption_reason,
        location_id: dto.location_id,
        exclude_vat: dto.exclude_vat != 0,
        vat_article: &dto.vat_article,
        customs_statistical: &dto.customs_statistical,
        customs_duty: dec(&dto.customs_duty, "0"),
        customs_vat: dec(&dto.customs_vat, "0"),
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
            storage::schema::invoices::due_date.eq(invoices::parse_date(&dto.due_date)),
            storage::schema::invoices::payment_method.eq(&dto.payment_method),
            storage::schema::invoices::accounting_month.eq(&dto.accounting_month),
            storage::schema::invoices::currency_rate.eq(dec(&dto.currency_rate, "1")),
            storage::schema::invoices::tax_event_date.eq(invoices::parse_date(&dto.tax_event_date)),
            storage::schema::invoices::discount_percent.eq(dec(&dto.discount_percent, "0")),
            storage::schema::invoices::discount_amount.eq(dec(&dto.discount_amount, "0")),
            storage::schema::invoices::vat_exemption_reason.eq(&dto.vat_exemption_reason),
            storage::schema::invoices::location_id.eq(dto.location_id),
            storage::schema::invoices::exclude_vat.eq(dto.exclude_vat != 0),
            storage::schema::invoices::vat_article.eq(&dto.vat_article),
            storage::schema::invoices::customs_statistical.eq(&dto.customs_statistical),
            storage::schema::invoices::customs_duty.eq(dec(&dto.customs_duty, "0")),
            storage::schema::invoices::customs_vat.eq(dec(&dto.customs_vat, "0")),
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
    let (u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let old = invoices::by_id(&mut c, id).await?;
    storage::journal::delete_by_document(&mut c, id).await?;
    invoices::delete(&mut c, id).await?;
    if let Some(row) = old {
        let amt = format!("{:.2} {}", row.0.total_amount, row.0.currency);
        let _ = storage::access::record(
            &mut c,
            company.id,
            u.id,
            &u.email,
            &u.name,
            "invoice",
            id,
            &row.0.number,
            "delete",
            &format!("Изтрита фактура {}", row.0.number),
            &amt,
            "",
            None,
            None,
        )
        .await;
    }
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
    let (u, company) = active_company(&st, &session.claims.sub).await?;
    let row = posting::post_invoice(&st.pool, id).await?;
    let mut c = conn(&st).await?;
    if let Some(inv) = invoices::by_id(&mut c, id).await? {
        let amt = format!("{:.2} {}", inv.0.total_amount, inv.0.currency);
        let summary = format!("Постната фактура {}", inv.0.number);
        let _ = storage::access::record(
            &mut c,
            company.id,
            u.id,
            &u.email,
            &u.name,
            "invoice",
            id,
            &inv.0.number,
            "post",
            &summary,
            "",
            &amt,
            None,
            None,
        )
        .await;
        let _ = storage::access::record(
            &mut c,
            company.id,
            u.id,
            &u.email,
            &u.name,
            "journal",
            row.0.id,
            &inv.0.number,
            "create",
            &summary,
            "",
            &amt,
            None,
            None,
        )
        .await;
    }
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
    let (u, company) = active_company(&st, &session.claims.sub).await?;
    posting::unpost_invoice(&st.pool, id).await?;
    let mut c = conn(&st).await?;
    if let Some(inv) = invoices::by_id(&mut c, id).await? {
        let _ = storage::access::record(
            &mut c,
            company.id,
            u.id,
            &u.email,
            &u.name,
            "invoice",
            id,
            &inv.0.number,
            "unpost",
            &format!("Отпостната фактура {}", inv.0.number),
            "",
            "",
            None,
            None,
        )
        .await;
    }
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

#[derive(serde::Deserialize)]
pub struct BulkBody {
    pub ids: Vec<i64>,
}

fn bulk_result(id: i64, ok: bool, error: Option<String>) -> serde_json::Value {
    serde_json::json!({"id": id, "ok": ok, "error": error})
}

/// POST /v1/invoices/bulk {action: post|unpost|delete, ids:[...]} — до 500 документа.
pub async fn bulk(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    #[derive(serde::Deserialize)]
    struct In {
        action: String,
        ids: Vec<i64>,
    }
    let parsed: In = serde_json::from_value(b)
        .map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "невалидно тяло"))?;
    if parsed.ids.is_empty() || parsed.ids.len() > 500 {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "до 500 документа",
        ));
    }
    let (_u, _company) = active_company(&st, &session.claims.sub).await?;
    let mut results = Vec::new();
    for id in parsed.ids {
        let r = match parsed.action.as_str() {
            "post" => posting::post_invoice(&st.pool, id)
                .await
                .map(|_| (true, None)),
            "unpost" => posting::unpost_invoice(&st.pool, id)
                .await
                .map(|_| (true, None)),
            "delete" => {
                let mut c = st.pool.get().await.map_err(|_| internal())?;
                let _ = storage::journal::delete_by_document(&mut c, id).await;
                match storage::invoices::delete(&mut c, id).await {
                    Ok(_) => Ok((true, None)),
                    Err(e) => Ok((false, Some(e.to_string()))),
                }
            }
            _ => Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "action е post, unpost или delete",
            )),
        };
        match r {
            Ok((ok, err)) => results.push(bulk_result(id, ok, err)),
            Err(e) => results.push(bulk_result(id, false, Some(e.detail))),
        }
    }
    let posted = results.iter().filter(|r| r["ok"] == true).count();
    Ok(Json(
        serde_json::json!({ "processed": results.len(), "posted": posted, "results": results }),
    ))
}

/// POST /v1/invoices/bulk/{action} {ids:[...]} — action от пътя (post|unpost|delete).
pub async fn bulk_action(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(action): Path<String>,
    Json(b): Json<BulkBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !matches!(action.as_str(), "post" | "unpost" | "delete") {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "action е post, unpost или delete",
        ));
    }
    if b.ids.is_empty() || b.ids.len() > 500 {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "до 500 документа",
        ));
    }
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let (mut posted, mut unposted, mut deleted, mut failed) = (0, 0, 0, 0);
    let mut results = Vec::new();
    for id in b.ids {
        let r = match action.as_str() {
            "post" => posting::post_invoice(&st.pool, id).await.map(|_| ()),
            "unpost" => posting::unpost_invoice(&st.pool, id).await,
            _ => {
                let mut c = conn(&st).await?;
                let owns = invoices::by_id(&mut c, id)
                    .await?
                    .map(|row| row.0.company_id == company.id)
                    .unwrap_or(false);
                if !owns {
                    Err(ApiError::not_found("фактурата не е намерена"))
                } else {
                    let _ = storage::journal::delete_by_document(&mut c, id).await;
                    storage::invoices::delete(&mut c, id)
                        .await
                        .map_err(ApiError::from)
                }
            }
        };
        match r {
            Ok(()) => {
                match action.as_str() {
                    "post" => posted += 1,
                    "unpost" => unposted += 1,
                    _ => deleted += 1,
                }
                results.push(bulk_result(id, true, None));
            }
            Err(e) => {
                failed += 1;
                results.push(bulk_result(id, false, Some(e.detail)));
            }
        }
    }
    Ok(Json(serde_json::json!({
        "processed": results.len(), "posted": posted, "unposted": unposted,
        "deleted": deleted, "failed": failed, "results": results,
    })))
}

pub async fn print(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response, ApiError> {
    use axum::response::IntoResponse;
    let (_u, _c) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let row = invoices::by_id(&mut c, id)
        .await?
        .ok_or_else(|| ApiError::not_found("фактурата не е намерена"))?;
    let format = q.get("format").cloned().unwrap_or_else(|| "pdf".into());
    let (bytes, mime) = crate::print::render(&row, &format)?;
    let ext = match format.as_str() {
        "docx" => "docx",
        "odt" => "odt",
        _ => "pdf",
    };
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"invoice-{}.{}\"", row.0.number, ext),
            ),
        ],
        bytes,
    )
        .into_response())
}
