//! /v1/product-name-mappings — мапинг на сканирани имена към артикули.

use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::{active_company, PageQuery, PageSlice};
use crate::state::{AppState, Session};
use storage::pnmap;

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub counterpart_id: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn row_json(m: &pnmap::Mapping) -> Value {
    json!({"id": m.id, "counterpart_id": m.counterpart_id, "scanned_name": m.scanned_name, "product_id": m.product_id})
}

pub async fn index(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let mut co = conn(&st).await?;
    let rows = pnmap::list(&mut co, c.id, q.counterpart_id.unwrap_or(0)).await?;
    let items: Vec<Value> = rows.iter().map(row_json).collect();
    let page = PageQuery {
        limit: q.limit.unwrap_or(50),
        offset: q.offset.unwrap_or(0),
    };
    let slice = PageSlice::from_vec(items, &page);
    Ok(Json(
        json!({"items": slice.items, "count": slice.count(), "total": slice.total, "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more()}),
    ))
}

pub async fn save(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let counterpart = b.get("counterpart_id").and_then(Value::as_i64).unwrap_or(0);
    if counterpart <= 0 {
        return Err(unproc("counterpart_id е задължителен"));
    }
    let mut co = conn(&st).await?;
    let cp_ok = storage::counterparts::by_id(&mut co, counterpart)
        .await?
        .map(|r| r.0.company_id == c.id)
        .unwrap_or(false);
    if !cp_ok {
        return Err(unproc("контрагентът не е намерен"));
    }
    let items = b
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut pairs = Vec::new();
    for item in &items {
        let name = item
            .get("scanned_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let product = item.get("product_id").and_then(Value::as_i64).unwrap_or(0);
        if name.trim().is_empty() || product <= 0 {
            continue;
        }
        let p_ok = storage::products::by_id(&mut co, product)
            .await?
            .map(|r| r.0.company_id == c.id)
            .unwrap_or(false);
        if !p_ok {
            return Err(unproc("артикулът не е намерен"));
        }
        pairs.push((name, product));
    }
    pnmap::upsert_items(&mut co, c.id, counterpart, &pairs).await?;
    let rows = pnmap::list(&mut co, c.id, counterpart).await?;
    let items: Vec<Value> = rows.iter().map(row_json).collect();
    Ok(Json(json!({"items": items, "count": items.len()})))
}
