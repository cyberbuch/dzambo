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

// ---- Import preview + auto-allocate (matching по правила) ----

#[derive(serde::Deserialize)]
pub struct PreviewBody {
    pub bank_account_id: i64,
    pub file_content: String,
}

pub async fn preview(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<PreviewBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    if storage::banks::account_by_id(&mut conn, b.bank_account_id)
        .await?
        .map(|a| a.company_id != c.id)
        .unwrap_or(true)
    {
        return Err(ApiError::not_found("сметката не е намерена"));
    }
    let parsed = storage::bank_formats::parse(&b.file_content);
    let (format_name, account_iban, account_currency, txs) = match parsed {
        Some(p) => (p.format_name, p.account_iban, p.account_currency, p.txs),
        None => {
            let mut txs = Vec::new();
            for line in b.file_content.lines() {
                let cols: Vec<&str> = line
                    .split(';')
                    .map(|x| x.trim().trim_matches('"'))
                    .collect();
                if cols.len() < 2 {
                    continue;
                }
                txs.push(storage::bank_formats::Tx {
                    date: cols[0].to_string(),
                    amount: cols[1].replace(',', "."),
                    currency: String::new(),
                    description: cols.get(4).copied().unwrap_or("").to_string(),
                    contra_name: cols.get(3).copied().unwrap_or("").to_string(),
                    contra_iban: String::new(),
                    reference: cols.get(5).copied().unwrap_or("").to_string(),
                });
            }
            ("CSV".to_string(), String::new(), String::new(), txs)
        }
    };
    let refs = storage::banks::refs_for_account(&mut conn, b.bank_account_id).await?;
    let mut dup = 0;
    let transactions: Vec<serde_json::Value> = txs.iter().map(|t| {
        let is_duplicate = !t.reference.is_empty() && refs.iter().any(|r| r == &t.reference);
        if is_duplicate { dup += 1; }
        serde_json::json!({
            "date": t.date, "amount": t.amount, "currency": t.currency,
            "description": t.description, "contra_name": t.contra_name,
            "contra_iban": t.contra_iban, "reference": t.reference, "is_duplicate": is_duplicate,
        })
    }).collect();
    let total = transactions.len();
    Ok(Json(serde_json::json!({
        "format_name": format_name, "account_iban": account_iban, "account_currency": account_currency,
        "transactions": transactions, "total_count": total, "duplicate_count": dup, "new_count": total - dup,
    })))
}

async fn allocate(
    st: &AppState,
    company_id: i64,
    apply: bool,
) -> Result<serde_json::Value, ApiError> {
    let mut conn = st.pool.get().await.map_err(|_| internal())?;
    let rules = storage::bank_rules::list(&mut conn, company_id).await?;
    let accs = accounts::list(&mut conn, company_id).await?;
    let acc_label = |id: i64| {
        accs.iter()
            .find(|a| a.id == id)
            .map(|a| format!("{} {}", a.number, a.name))
            .unwrap_or_else(|| format!("#{id}"))
    };
    let acc_number = |id: i64| accs.iter().find(|a| a.id == id).map(|a| a.number.clone());
    let cps = storage::counterparts::list(&mut conn, company_id).await?;
    let txs = storage::banks::list_unallocated(&mut conn, company_id).await?;
    let (mut matched, mut unmatched, mut errors, mut counterpart_hits) = (0, 0, 0, 0);
    let mut details = Vec::new();
    for tx in &txs {
        let text = format!(
            "{} {} {}",
            tx.description, tx.counterpart_name, tx.reference
        );
        let amount = format!("{}", tx.amount);
        let Some(rule) = storage::bank_rules::find_rule(&rules, &text, &amount) else {
            unmatched += 1;
            if details.len() < 50 {
                details.push(serde_json::json!({"id": tx.id, "status": "unmatched", "text": text.chars().take(80).collect::<String>()}));
            }
            continue;
        };
        let cp_id = if rule.counterpart_id > 0 {
            rule.counterpart_id
        } else {
            cps.iter()
                .find(|cp| cp.0.name.eq_ignore_ascii_case(&tx.counterpart_name))
                .map(|cp| cp.0.id)
                .unwrap_or(0)
        };
        if cp_id > 0 {
            counterpart_hits += 1;
        }
        let cp_name = cps
            .iter()
            .find(|cp| cp.0.id == cp_id)
            .map(|cp| cp.0.name.clone())
            .unwrap_or_default();
        if !apply {
            matched += 1;
            if details.len() < 50 {
                details.push(serde_json::json!({"id": tx.id, "status": "matched", "rule": rule.name, "account": acc_label(rule.account_id), "counterpart": cp_name, "amount": amount, "text": text.chars().take(80).collect::<String>()}));
            }
            continue;
        }
        let cp_number = acc_number(rule.account_id).unwrap_or_else(|| {
            if tx.direction == "in" {
                "411".into()
            } else {
                "401".into()
            }
        });
        // Вече постната транзакция не се осчетоводява повторно — само се маркира.
        let book = if tx.status == "posted" {
            Ok(())
        } else {
            post_with_account(&mut conn, company_id, tx, &cp_number)
                .await
                .map(|_| ())
        };
        match book {
            Ok(_) => {
                let _ =
                    storage::banks::mark_allocated(&mut conn, tx.id, rule.account_id, cp_id).await;
                matched += 1;
                if details.len() < 50 {
                    details.push(serde_json::json!({"id": tx.id, "status": "matched", "rule": rule.name, "account": acc_label(rule.account_id)}));
                }
            }
            Err(e) => {
                errors += 1;
                if details.len() < 50 {
                    details.push(
                        serde_json::json!({"id": tx.id, "status": "error", "error": e.detail}),
                    );
                }
            }
        }
    }
    Ok(
        serde_json::json!({"matched": matched, "unmatched": unmatched, "errors": errors, "counterpart_hits": counterpart_hits, "details": details}),
    )
}

pub async fn auto_allocate(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    Ok(Json(allocate(&st, c.id, true).await?))
}

pub async fn auto_allocate_preview(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    Ok(Json(allocate(&st, c.id, false).await?))
}
