//! НОМЕНКЛАТУРИ НАП — сиров каталог по видове; SAF-T се води от мапинги към тук.

use axum::{extract::Query, Json};
use serde_json::json;
use std::collections::HashMap;

pub async fn index(Query(q): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    let kind = q.get("kind").cloned().unwrap_or_else(|| "accounts".into());
    let raw = domain::noms::entries(&kind);
    let items: Vec<serde_json::Value> = raw
        .iter()
        .map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            json!({"code": parts.first().copied().unwrap_or(""), "name": parts.get(1).copied().unwrap_or(""), "parts": parts})
        })
        .collect();
    Json(json!({"kind": kind, "items": items, "count": items.len()}))
}

pub async fn kinds() -> Json<serde_json::Value> {
    Json(json!({"kinds": domain::noms::KINDS}))
}
