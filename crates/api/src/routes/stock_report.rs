//! /v1/stock/report — приход/разход/наличност към дата (чете движенията, не пише).

use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use domain::reports_data::{firm_line, Built};
use storage::stock_replay::{self, Balance, Flow};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}

#[derive(Deserialize, Default)]
pub struct Q {
    pub from: Option<String>,
    pub to: Option<String>,
    pub location_id: Option<String>,
}

fn iso(s: &str) -> bool {
    s.len() == 10 && s.as_bytes().get(4) == Some(&b'-') && s.as_bytes().get(7) == Some(&b'-')
}

fn flow_json(f: &Flow) -> Value {
    json!({
        "date": f.date, "document": f.doc, "kind": f.kind, "reason": f.reason,
        "product_id": f.product_id, "product_name": f.product_name,
        "location_id": f.location_id, "quantity": f.quantity, "value": f.value,
    })
}

fn bal_json(b: &Balance) -> Value {
    json!({
        "product_id": b.product_id, "product_name": b.product_name,
        "location_id": b.location_id, "quantity": b.quantity, "value": b.value,
    })
}

struct Filtered {
    receipts: Vec<Flow>,
    issues: Vec<Flow>,
    balances: Vec<Balance>,
    warnings: Vec<String>,
}

async fn gather(
    st: &AppState,
    company_id: i64,
    from: &str,
    to: &str,
    loc: i64,
) -> Result<(Filtered, String, String), ApiError> {
    let mut c = st.pool.get().await.map_err(|_| internal())?;
    let out = stock_replay::explain(&mut c, company_id).await?;
    let in_range = |f: &Flow| {
        f.date.as_str() >= from && f.date.as_str() <= to && (loc <= 0 || f.location_id == loc)
    };
    let receipts: Vec<Flow> = out
        .flows
        .iter()
        .filter(|f| f.kind == "in" && in_range(f))
        .cloned()
        .collect();
    let issues: Vec<Flow> = out
        .flows
        .iter()
        .filter(|f| f.kind == "out" && in_range(f))
        .cloned()
        .collect();
    let balances: Vec<Balance> = out
        .balances
        .iter()
        .filter(|b| loc <= 0 || b.location_id == loc)
        .cloned()
        .collect();
    let firm: Option<(String, String)> = storage::companies::by_id(&mut c, company_id)
        .await?
        .map(|(c, _)| (c.name, c.eik));
    let (name, eik) = firm.unwrap_or_default();
    Ok((
        Filtered {
            receipts,
            issues,
            balances,
            warnings: out.warnings,
        },
        name,
        eik,
    ))
}

pub async fn report(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<Q>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let to = q.to.unwrap_or_default();
    let to = if iso(&to) {
        to
    } else {
        "9999-12-31".to_string()
    };
    let from = q.from.unwrap_or_default();
    let from = if iso(&from) {
        from
    } else {
        "0001-01-01".to_string()
    };
    let loc: i64 = q
        .location_id
        .unwrap_or_default()
        .trim()
        .parse()
        .unwrap_or(0);
    let (f, _name, _eik) = gather(&st, company.id, &from, &to, loc).await?;
    Ok(Json(json!({
        "from": from, "to": if to == "9999-12-31" { "" } else { &to },
        "receipts": f.receipts.iter().map(flow_json).collect::<Vec<_>>(),
        "issues": f.issues.iter().map(flow_json).collect::<Vec<_>>(),
        "balances": f.balances.iter().map(bal_json).collect::<Vec<_>>(),
        "warnings": f.warnings,
    })))
}

/// За export през /v1/reports?kind=stock.
pub async fn sheet(
    st: &AppState,
    company_id: i64,
    from: &str,
    to: &str,
    loc: i64,
) -> Result<Built, ApiError> {
    let (f, name, eik) = gather(st, company_id, from, to, loc).await?;
    let mut grid = Vec::new();
    let line = firm_line(&name, &eik);
    if !line.is_empty() {
        grid.push(vec![line]);
    }
    grid.push(vec!["Склад".into(), from.into(), to.into()]);
    push_flows(&mut grid, "Приход", &f.receipts);
    push_flows(&mut grid, "Разход", &f.issues);
    grid.push(vec![]);
    grid.push(vec!["Наличности".into()]);
    grid.push(vec![
        "Артикул".into(),
        "Обект".into(),
        "Количество".into(),
        "Стойност".into(),
    ]);
    for b in &f.balances {
        grid.push(vec![
            b.product_name.clone(),
            b.location_id.to_string(),
            b.quantity.clone(),
            b.value.clone(),
        ]);
    }
    Ok(Built {
        body: json!({
            "from": from, "to": to,
            "receipts": f.receipts.iter().map(flow_json).collect::<Vec<_>>(),
            "issues": f.issues.iter().map(flow_json).collect::<Vec<_>>(),
            "balances": f.balances.iter().map(bal_json).collect::<Vec<_>>(),
            "warnings": f.warnings,
        }),
        slug: "sklad".into(),
        grid,
    })
}

fn push_flows(grid: &mut Vec<Vec<String>>, title: &str, rows: &[Flow]) {
    grid.push(vec![]);
    grid.push(vec![title.into()]);
    grid.push(vec![
        "Дата".into(),
        "Документ".into(),
        "Артикул".into(),
        "Обект".into(),
        "Количество".into(),
        "Стойност".into(),
        "Основание".into(),
    ]);
    for f in rows {
        grid.push(vec![
            f.date.clone(),
            f.doc.clone(),
            f.product_name.clone(),
            f.location_id.to_string(),
            f.quantity.clone(),
            f.value.clone(),
            f.reason.clone(),
        ]);
    }
}
