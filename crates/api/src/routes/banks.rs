//! Банки: сметки (списък/създаване), транзакции (списък/CSV импорт).

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use bigdecimal::BigDecimal;
use diesel_async::RunQueryDsl;
use domain::dto::{BankAccountDto, BankTransactionDto};
use storage::{
    accounts,
    journal::{self, NewJournalEntry, NewJournalLine},
    periods,
};

use crate::{
    errors::ApiError,
    routes::common::active_company,
    state::{AppState, Session},
};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}

pub async fn accounts(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<Vec<BankAccountDto>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let rows = storage::banks::accounts(&mut conn, c.id).await?;
    Ok(Json(
        rows.iter()
            .map(|a| BankAccountDto {
                id: a.id,
                name: a.name.clone(),
                iban: a.iban.clone(),
                currency: a.currency.clone(),
            })
            .collect(),
    ))
}

pub async fn create_account(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<BankAccountDto>,
) -> Result<(StatusCode, Json<BankAccountDto>), ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let a = storage::banks::create_account(&mut conn, c.id, &b.name, &b.iban, &b.currency).await?;
    Ok((
        StatusCode::CREATED,
        Json(BankAccountDto {
            id: a.id,
            name: a.name,
            iban: a.iban,
            currency: a.currency,
        }),
    ))
}

#[derive(serde::Deserialize)]
pub struct TxQuery {
    pub account_id: i64,
}

pub async fn transactions(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Query(q): Query<TxQuery>,
) -> Result<Json<Vec<BankTransactionDto>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let rows = storage::banks::transactions(&mut conn, c.id, q.account_id).await?;
    Ok(Json(
        rows.iter()
            .map(|t| BankTransactionDto {
                id: t.id,
                tx_date: t.tx_date.to_string(),
                amount: format!("{:.2}", t.amount),
                direction: t.direction.clone(),
                counterpart_name: t.counterpart_name.clone(),
                description: t.description.clone(),
                reference: t.reference.clone(),
                status: t.status.clone(),
            })
            .collect(),
    ))
}

#[derive(serde::Deserialize)]
pub struct ImportBody {
    pub account_id: i64,
    pub csv: String,
}

pub async fn import(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<ImportBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let (n, format) = match storage::bank_formats::parse(&b.csv) {
        Some(parsed) => (
            storage::banks::import_parsed(&mut conn, c.id, b.account_id, &parsed.txs).await?,
            parsed.format_name,
        ),
        None => (
            storage::banks::import_csv(&mut conn, c.id, b.account_id, &b.csv).await?,
            "CSV".to_string(),
        ),
    };
    Ok(Json(serde_json::json!({"imported": n, "format": format})))
}

/// Разнасяне на транзакция с избрана контрагентска сметка (по правило или посока).
pub async fn post_with_account(
    conn: &mut storage::DbConn,
    company_id: i64,
    tx: &storage::banks::BankTransaction,
    cp_number: &str,
) -> Result<i64, ApiError> {
    if tx.status == "posted" {
        return Err(ApiError::new(StatusCode::CONFLICT, "вече е разнесена"));
    }
    let month = periods::month_key(tx.tx_date);
    if periods::is_closed(conn, company_id, &month).await? {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("периодът {month} е затворен"),
        ));
    }
    let ba = storage::banks::account_by_id(conn, tx.bank_account_id)
        .await?
        .ok_or_else(|| ApiError::not_found("сметката не е намерена"))?;
    let bank_number = if ba.gl_account_id > 0 {
        accounts::list(conn, company_id)
            .await?
            .iter()
            .find(|a| a.id == ba.gl_account_id)
            .map(|a| a.number.clone())
            .unwrap_or_else(|| "501".into())
    } else {
        "501".into()
    };
    let bank_acct = accounts::get_or_create(conn, company_id, &bank_number).await?;
    let cp_acct = accounts::get_or_create(conn, company_id, cp_number).await?;
    let amount = tx.amount.abs();
    let (bank_dir, cp_dir) = if tx.direction == "in" {
        ("debit", "credit")
    } else {
        ("credit", "debit")
    };
    let mk = |a: &storage::models::Account, dir: &str| NewJournalLine {
        entry_id: 0,
        account_id: a.id,
        account_number: a.number.clone(),
        account_name: a.name.clone(),
        direction: dir.into(),
        amount: amount.clone(),
        vat_amount: BigDecimal::default(),
        currency: String::new(),
        fx_quantity: BigDecimal::default(),
        fx_rate: BigDecimal::from(1),
    };
    let lines = vec![mk(&bank_acct, bank_dir), mk(&cp_acct, cp_dir)];
    let number = if tx.reference.is_empty() {
        format!("BANK-{}", tx.id)
    } else {
        tx.reference.clone()
    };
    let entry = NewJournalEntry {
        company_id,
        entry_date: tx.tx_date,
        document_type: "bank".into(),
        document_id: tx.id,
        document_number: number,
        description: tx.description.clone(),
        vat_type: "no_vat".into(),
        counterpart_id: 0,
        counterpart_name: tx.counterpart_name.clone(),
        document_date: tx.tx_date,
        vat_period: month,
    };
    let c = conn.as_mut();
    diesel::sql_query("BEGIN").execute(c).await?;
    let result: Result<i64, storage::StorageError> = async {
        let eid = journal::insert_entry(c, &entry, &lines).await?;
        storage::banks::set_tx_status(c, tx.id, "posted", eid).await?;
        Ok(eid)
    }
    .await;
    match result {
        Ok(eid) => {
            diesel::sql_query("COMMIT").execute(c).await?;
            Ok(eid)
        }
        Err(e) => {
            let _ = diesel::sql_query("ROLLBACK").execute(c).await;
            Err(ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ))
        }
    }
}

pub async fn post(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let (_u, company) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let tx = storage::banks::tx_by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("транзакцията не е намерена"))?;
    let cp_number = if tx.direction == "in" { "411" } else { "401" };
    let eid = post_with_account(&mut conn, company.id, &tx, cp_number).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"ok": true, "entry_id": eid})),
    ))
}

#[derive(serde::Deserialize)]
pub struct BulkIds {
    pub ids: Vec<i64>,
}

pub async fn bulk_delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(b): Json<BulkIds>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let mut c = st.pool.get().await.map_err(|_| internal())?;
    let (mut deleted, mut failed) = (0, 0);
    for id in b.ids.iter().take(500) {
        match storage::banks::delete_tx(&mut c, company.id, *id).await {
            Ok(true) => deleted += 1,
            _ => failed += 1,
        }
    }
    Ok(Json(
        serde_json::json!({"deleted": deleted, "failed": failed}),
    ))
}
