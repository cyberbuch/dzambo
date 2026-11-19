//! Дневник: списък, показване, ръчен запис/редакция/изтриване.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use domain::dto::{JournalEntryDto, JournalIn};
use storage::{
    accounts,
    journal::{self, NewJournalEntry, NewJournalLine},
    periods,
};

use crate::{
    errors::ApiError,
    mappers_invoices::journal_dto,
    routes::common::{active_company, PageQuery, PageSlice},
    state::{AppState, Session},
};

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
    let rows = journal::list(&mut c, company.id).await?;
    let slice = PageSlice::from_vec(rows, &q);
    let items: Vec<_> = slice
        .items
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id, "entry_date": e.entry_date.to_string(),
                "document_type": e.document_type, "document_number": e.document_number,
                "description": e.description, "vat_type": e.vat_type,
                "counterpart_name": e.counterpart_name, "vat_period": e.vat_period,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({
        "items": items, "count": slice.count(), "total": slice.total,
        "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more(),
    })))
}

pub async fn show(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<JournalEntryDto>, ApiError> {
    let (_u, _company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let row = journal::entry_with_lines(&mut c, id)
        .await?
        .ok_or_else(|| ApiError::not_found("статията не е намерена"))?;
    Ok(Json(journal_dto(&row)))
}

fn parse_amount(raw: &str) -> BigDecimal {
    raw.trim().parse().unwrap_or_default()
}

struct Prepared {
    entry_date: chrono::NaiveDate,
    document_date: chrono::NaiveDate,
    vat_period: String,
    document_number: String,
    lines: Vec<NewJournalLine>,
}

/// Валидации + сметки + баланс; връща готови редове (низовете са owned).
fn prepare(
    body: &JournalIn,
    all: &[storage::models::Account],
    _company_id: i64,
) -> Result<Prepared, ApiError> {
    let entry_date = storage::invoices::parse_date(&body.entry_date)
        .ok_or_else(|| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "невалидна дата"))?;
    let vat_period = if body.vat_period.trim().is_empty() {
        periods::month_key(entry_date)
    } else {
        body.vat_period.trim().to_string()
    };
    let document_date = if body.document_date.trim().is_empty() {
        entry_date
    } else {
        storage::invoices::parse_date(&body.document_date)
            .ok_or_else(|| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "невалидна дата"))?
    };
    if body.lines.is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "статията няма редове",
        ));
    }

    let mut lines: Vec<NewJournalLine> = Vec::new();
    let mut debit = BigDecimal::default();
    let mut credit = BigDecimal::default();
    for l in &body.lines {
        let acct = all
            .iter()
            .find(|a| a.id == l.account_id)
            .ok_or_else(|| ApiError::not_found("сметката не е намерена"))?;
        // количество × курс, ако са дадени; иначе сумата се взема както е.
        let amount = match (l.fx_quantity.as_deref(), l.fx_rate.as_deref()) {
            (Some(q), Some(r)) => q.trim().parse::<BigDecimal>().unwrap_or_default() * r.trim().parse::<BigDecimal>().unwrap_or_default(),
            _ => parse_amount(&l.amount),
        };
        match l.direction.as_str() {
            "debit" => debit += &amount,
            "credit" => credit += &amount,
            _ => {
                return Err(ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "direction трябва да е debit или credit",
                ))
            }
        }
        let fx_quantity = l.fx_quantity.as_deref().and_then(|q| q.trim().parse::<BigDecimal>().ok()).unwrap_or_else(|| amount.clone());
        let fx_rate = l.fx_rate.as_deref().and_then(|r| r.trim().parse::<BigDecimal>().ok()).unwrap_or_else(|| BigDecimal::from(1));
        lines.push(NewJournalLine {
            entry_id: 0,
            account_id: acct.id,
            account_number: acct.number.clone(),
            account_name: acct.name.clone(),
            direction: l.direction.clone(),
            amount,
            vat_amount: parse_amount(&l.vat_amount),
            currency: l.currency.clone(),
            fx_quantity,
            fx_rate,
        });
    }
    if debit != credit {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "дебитът и кредитът не са балансирани",
        ));
    }
    Ok(Prepared {
        entry_date,
        document_date,
        vat_period,
        document_number: body.document_number.trim().to_string(),
        lines,
    })
}

pub async fn create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<JournalIn>,
) -> Result<(StatusCode, Json<JournalEntryDto>), ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let all = accounts::list(&mut c, company.id).await?;
    let p = prepare(&body, &all, company.id)?;
    if periods::is_closed(&mut c, company.id, &p.vat_period).await? {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("периодът {} е затворен", p.vat_period),
        ));
    }
    let description = body.description.clone();
    let vat_type: String = if matches!(body.vat_type.as_str(), "purchase" | "sales") {
        body.vat_type.clone()
    } else {
        "no_vat".to_string()
    };
    let counterpart_name = body.counterpart_name.clone();
    let entry = NewJournalEntry {
        company_id: company.id,
        entry_date: p.entry_date,
        document_type: "manual".to_string(),
        document_id: 0,
        document_number: String::new(),
        description,
        vat_type: vat_type.to_string(),
        counterpart_id: body.counterpart_id,
        counterpart_name,
        document_date: p.document_date,
        vat_period: p.vat_period.clone(),
    };
    let id = journal::insert_entry(&mut c, &entry, &p.lines).await?;
    let number = if p.document_number.is_empty() {
        format!("MANUAL-{id}")
    } else {
        p.document_number.clone()
    };
    diesel::update(storage::schema::journal_entries::table.find(id))
        .set(storage::schema::journal_entries::document_number.eq(&number))
        .execute(&mut c)
        .await?;
    let row = journal::entry_with_lines(&mut c, id)
        .await?
        .ok_or_else(internal)?;
    Ok((StatusCode::CREATED, Json(journal_dto(&row))))
}

pub async fn update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(body): Json<JournalIn>,
) -> Result<Json<JournalEntryDto>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let existing = journal::entry_with_lines(&mut c, id)
        .await?
        .ok_or_else(|| ApiError::not_found("статията не е намерена"))?;
    if !matches!(existing.0.document_type.as_str(), "" | "manual") {
        return Err(ApiError::new(StatusCode::CONFLICT, "само ръчни статии"));
    }
    let all = accounts::list(&mut c, company.id).await?;
    let p = prepare(&body, &all, company.id)?;
    if periods::is_closed(&mut c, company.id, &p.vat_period).await? {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("периодът {} е затворен", p.vat_period),
        ));
    }
    diesel::delete(
        storage::schema::journal_lines::table
            .filter(storage::schema::journal_lines::entry_id.eq(id)),
    )
    .execute(&mut c)
    .await?;
    for line in &p.lines {
        diesel::insert_into(storage::schema::journal_lines::table)
            .values(line)
            .execute(&mut c)
            .await?;
    }
    let number = if p.document_number.is_empty() {
        format!("MANUAL-{id}")
    } else {
        p.document_number.clone()
    };
    diesel::update(storage::schema::journal_entries::table.find(id))
        .set((
            storage::schema::journal_entries::entry_date.eq(p.entry_date),
            storage::schema::journal_entries::document_date.eq(p.document_date),
            storage::schema::journal_entries::document_number.eq(&number),
            storage::schema::journal_entries::vat_period.eq(&p.vat_period),
        ))
        .execute(&mut c)
        .await?;
    let row = journal::entry_with_lines(&mut c, id)
        .await?
        .ok_or_else(internal)?;
    Ok(Json(journal_dto(&row)))
}

pub async fn delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_u, _company) = active_company(&st, &session.claims.sub).await?;
    let mut c = conn(&st).await?;
    let existing = journal::entry_with_lines(&mut c, id)
        .await?
        .ok_or_else(|| ApiError::not_found("статията не е намерена"))?;
    if !matches!(existing.0.document_type.as_str(), "" | "manual" | "closing") {
        return Err(ApiError::new(StatusCode::CONFLICT, "само ръчни статии"));
    }
    diesel::delete(
        storage::schema::journal_lines::table
            .filter(storage::schema::journal_lines::entry_id.eq(id)),
    )
    .execute(&mut c)
    .await?;
    diesel::delete(storage::schema::journal_entries::table.find(id))
        .execute(&mut c)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
