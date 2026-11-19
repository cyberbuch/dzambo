//! Ръчни складови документи: нужда/списание (COGS) и движение между обекти.

use axum::{extract::{Extension, State}, http::StatusCode, Json};
use bigdecimal::{BigDecimal, Zero};
use domain::dto::{StockDocDto, StockDocLineDto};

use crate::{errors::ApiError, routes::common::active_company, state::{AppState, Session}};

fn internal() -> ApiError { ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка") }

fn doc_dto(d: &storage::stock_docs::StockDoc, lines: &[storage::stock_docs::StockDocLine]) -> StockDocDto {
    StockDocDto { id: d.id, number: d.number.clone(), doc_date: d.doc_date.to_string(), kind: d.kind.clone(),
        to_location_id: d.to_location_id, notes: d.notes.clone(),
        lines: lines.iter().map(|l| StockDocLineDto { product_id: l.product_id, quantity: format!("{:.4}", l.quantity),
            unit_cost: format!("{:.2}", l.unit_cost), description: l.description.clone() }).collect() }
}

pub async fn list(State(st): State<AppState>, Extension(s): Extension<Session>) -> Result<Json<Vec<StockDocDto>>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = st.pool.get().await.map_err(|_| internal())?;
    let docs = storage::stock_docs::list(&mut co, c.id).await?;
    let mut out = Vec::new();
    for d in docs {
        let lines = storage::stock_docs::lines_of(&mut co, d.id).await?;
        out.push(doc_dto(&d, &lines));
    }
    Ok(Json(out))
}

#[derive(serde::Deserialize)]
pub struct LineIn { product_id: i64, quantity: String }

#[derive(serde::Deserialize)]
pub struct DocIn {
    #[serde(default)]
    pub client_key: String,
    pub doc_date: String,
    pub kind: String,
    #[serde(default)]
    pub from_location_id: i64,
    #[serde(default)]
    pub to_location_id: i64,
    #[serde(default)]
    pub notes: String,
    pub lines: Vec<LineIn>,
}

pub async fn create(State(st): State<AppState>, Extension(s): Extension<Session>, Json(b): Json<DocIn>) -> Result<(StatusCode, Json<StockDocDto>), ApiError> {
    if !matches!(b.kind.as_str(), "issue" | "move" | "writeoff") {
        return Err(ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "kind е issue, move или writeoff"));
    }
    if b.lines.is_empty() { return Err(ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "документът няма редове")); }
    let date = storage::invoices::parse_date(&b.doc_date).ok_or_else(|| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "невалидна дата"))?;
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = st.pool.get().await.map_err(|_| internal())?;
    if !b.client_key.trim().is_empty() {
        if let Some(d) = storage::stock_docs::find_by_key(&mut co, c.id, b.client_key.trim()).await? {
            let lines = storage::stock_docs::lines_of(&mut co, d.id).await?;
            return Ok((StatusCode::OK, Json(doc_dto(&d, &lines))));
        }
    }

    // FIFO себестойност по редовете (само за issue/writeoff).
    let mut costs: Vec<BigDecimal> = Vec::new();
    if b.kind != "move" {
        for l in &b.lines {
            let qty: BigDecimal = l.quantity.trim().parse().unwrap_or_default();
            costs.push(storage::stock::consume_fifo(&mut co, c.id, l.product_id, qty, c.inventory_valuation_method == "LIFO").await?);
        }
    }

    let doc_id = storage::stock_docs::insert_document(&mut co, c.id, date, &b.kind, b.from_location_id, b.to_location_id, &b.notes, &b.client_key).await?;
    for (i, l) in b.lines.iter().enumerate() {
        let qty: BigDecimal = l.quantity.trim().parse().unwrap_or_default();
        let cost = costs.get(i).cloned().unwrap_or_default();
        let unit = if qty.is_zero() { BigDecimal::default() } else { &cost / &qty };
        storage::stock_docs::insert_line(&mut co, doc_id, l.product_id, qty, unit, "").await?;
    }

    if b.kind == "move" {
        for l in &b.lines { storage::stock_docs::move_lots(&mut co, c.id, l.product_id, b.to_location_id).await?; }
    } else {
        let prods = storage::products::list(&mut co, c.id).await?;
        let accts = storage::accounts::list(&mut co, c.id).await?;
        let inv_number = |pid: i64| -> String {
            prods.iter().find(|(p, _)| p.id == pid)
                .map(|(p, _)| p.inventory_account_id)
                .filter(|id| *id > 0)
                .and_then(|id| accts.iter().find(|a| a.id == id).map(|a| a.number.clone()))
                .unwrap_or_else(|| "302".into())
        };
        let exp = storage::accounts::get_or_create(&mut co, c.id, "602").await?;
        let mut jlines: Vec<storage::journal::NewJournalLine> = Vec::new();
        for (i, l) in b.lines.iter().enumerate() {
            let cost = costs.get(i).cloned().unwrap_or_default();
            if cost.is_zero() { continue; }
            let inv = storage::accounts::get_or_create(&mut co, c.id, &inv_number(l.product_id)).await?;
            let mk = |a: &storage::models::Account, dir: &str| storage::journal::NewJournalLine {
                entry_id: 0, account_id: a.id, account_number: a.number.clone(), account_name: a.name.clone(),
                direction: dir.into(), amount: cost.clone(), vat_amount: BigDecimal::default(),
                currency: String::new(), fx_quantity: BigDecimal::default(), fx_rate: BigDecimal::from(1),
            };
            jlines.push(mk(&exp, "debit"));
            jlines.push(mk(&inv, "credit"));
        }
        let key = storage::periods::month_key(date);
        let entry = storage::journal::NewJournalEntry { company_id: c.id, entry_date: date, document_type: "stock".into(), document_id: doc_id,
            document_number: format!("DOC-{doc_id}"), description: format!("Складов документ {}", b.kind), vat_type: "no_vat".into(),
            counterpart_id: 0, counterpart_name: String::new(), document_date: date, vat_period: key };
        storage::journal::insert_entry(&mut co, &entry, &jlines).await?;
    }

    let d = storage::stock_docs::list(&mut co, c.id).await?.into_iter().find(|d| d.id == doc_id).unwrap();
    let lines = storage::stock_docs::lines_of(&mut co, doc_id).await?;
    Ok((StatusCode::CREATED, Json(doc_dto(&d, &lines))))
}
