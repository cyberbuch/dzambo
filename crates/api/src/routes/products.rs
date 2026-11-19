//! Стоки: GET/POST /v1/products, GET/PATCH/DELETE /v1/products/{id}.
//! Правила: вид запас 10..50, услуга изключва инвентар, авто-код,
//! дедупликация по код, auto_write_off изисква сметки.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use bigdecimal::BigDecimal;
use domain::dto::ProductDto;
use storage::{
    accounts,
    models::products::{NewProduct, NewProductExtra},
    products::{self, ProductRow},
};

use crate::{
    errors::ApiError,
    mappers,
    routes::common::{active_company, PageQuery, PageSlice},
    state::{AppState, Session},
};

const PRODUCT_TYPES: [&str; 5] = ["10", "20", "30", "40", "50"];

fn dec(raw: &Option<String>, default: &str) -> BigDecimal {
    raw.as_deref()
        .unwrap_or(default)
        .trim()
        .parse()
        .unwrap_or_default()
}

/// Изважда правилата за вид запас и услуга; връща (product_type, is_inventory,
/// is_service, expense_account_id, auto_write_off, cn8_code).
#[allow(clippy::type_complexity)]
fn normalize(dto: &ProductDto) -> Result<(String, bool, bool, i64, bool, String), ApiError> {
    if dto.name.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "името е задължително",
        ));
    }
    let mut product_type = dto.product_type.trim().to_string();
    let mut is_inventory = dto.is_inventory;
    let is_service = dto.is_service;
    let mut expense = dto.expense_account_id;
    let mut auto_write_off = dto.auto_write_off;
    let mut cn8 = dto.cn8_code.trim().to_string();

    if is_service {
        is_inventory = false;
        product_type.clear();
        expense = 0;
        auto_write_off = false;
        if cn8.is_empty() {
            cn8 = "00000000".into();
        }
    } else {
        if !PRODUCT_TYPES.contains(&product_type.as_str()) {
            product_type = "30".into();
        }
        if product_type == "10" && is_inventory && auto_write_off && expense == 0 {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "auto_write_off изисква разходна сметка",
            ));
        }
        if auto_write_off && !(product_type == "10" && is_inventory) {
            auto_write_off = false;
        }
    }
    Ok((
        product_type,
        is_inventory,
        is_service,
        expense,
        auto_write_off,
        cn8,
    ))
}

async fn default_accounts(conn: &mut storage::DbConn, company_id: i64) -> (i64, i64, i64) {
    let all = match accounts::list(conn, company_id).await {
        Ok(a) => a,
        Err(_) => return (0, 0, 0),
    };
    let pick = |number: &str| {
        all.iter()
            .find(|a| a.number == number)
            .map(|a| a.id)
            .unwrap_or(0)
    };
    (pick("302"), pick("706"), pick("601"))
}

pub async fn list(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<PageQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_user, company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let rows = products::list(&mut conn, company.id).await?;
    let slice = PageSlice::from_vec(rows, &q);
    let items: Vec<_> = slice
        .items
        .iter()
        .map(|r| serde_json::to_value(mappers::product_dto(r)).unwrap())
        .collect();
    Ok(Json(serde_json::json!({
        "items": items, "count": slice.count(), "total": slice.total,
        "limit": slice.limit, "offset": slice.offset, "has_more": slice.has_more(),
    })))
}

pub async fn create(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Json(dto): Json<ProductDto>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let (_user, company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;

    let (product_type, is_inventory, is_service, expense, auto_write_off, cn8) = normalize(&dto)?;
    let (def_inv, def_rev, def_exp) = default_accounts(&mut conn, company.id).await;
    let inventory_account_id = if dto.inventory_account_id > 0 {
        dto.inventory_account_id
    } else {
        def_inv
    };
    let revenue_account_id = if dto.revenue_account_id > 0 {
        dto.revenue_account_id
    } else {
        def_rev
    };
    let expense_account_id = if expense > 0 { expense } else { def_exp };

    let code = {
        let c = dto.code.trim().to_string();
        if c.is_empty() {
            products::next_numeric_code(&mut conn, company.id).await?
        } else {
            c
        }
    };
    if products::code_taken(&mut conn, company.id, 0, &code).await? {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "артикул с този код вече съществува",
        ));
    }

    let row: ProductRow = products::insert(
        &mut conn,
        &NewProduct {
            company_id: company.id,
            name: &dto.name,
            code: &code,
            description: &dto.description,
            unit: &dto.unit,
            price: dec(&dto.price, "0"),
            cost_price: dec(&dto.cost_price, "0"),
            vat_rate: dec(&dto.vat_rate, "20"),
            is_service,
            is_inventory,
            inventory_account_id,
            revenue_account_id,
            is_active: dto.is_active,
        },
        &NewProductExtra {
            id: 0,
            product_type: &product_type,
            cn8_code: &cn8,
            cn8_name: &dto.cn8_name,
            expense_account_id,
            auto_write_off,
        },
    )
    .await?;
    let saved = products::by_id(&mut conn, row.0.id)
        .await?
        .expect("записана стока");
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(mappers::product_dto(&saved)).unwrap()),
    ))
}

pub async fn show(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_user, _company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let row = products::by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("стоката не е намерена"))?;
    Ok(Json(
        serde_json::to_value(mappers::product_dto(&row)).unwrap(),
    ))
}

pub async fn update(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
    Json(dto): Json<ProductDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (_user, company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let existing = products::by_id(&mut conn, id)
        .await?
        .ok_or_else(|| ApiError::not_found("стоката не е намерена"))?;
    let _ = existing;

    let (product_type, is_inventory, is_service, expense, auto_write_off, cn8) = normalize(&dto)?;
    let code = {
        let c = dto.code.trim().to_string();
        if c.is_empty() {
            products::next_numeric_code(&mut conn, company.id).await?
        } else {
            c
        }
    };
    if products::code_taken(&mut conn, company.id, id, &code).await? {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "артикул с този код вече съществува",
        ));
    }
    let inventory_account_id = if dto.inventory_account_id > 0 {
        dto.inventory_account_id
    } else {
        0
    };
    let revenue_account_id = if dto.revenue_account_id > 0 {
        dto.revenue_account_id
    } else {
        0
    };
    let row = products::update(
        &mut conn,
        id,
        &NewProduct {
            company_id: company.id,
            name: &dto.name,
            code: &code,
            description: &dto.description,
            unit: &dto.unit,
            price: dec(&dto.price, "0"),
            cost_price: dec(&dto.cost_price, "0"),
            vat_rate: dec(&dto.vat_rate, "20"),
            is_service,
            is_inventory,
            inventory_account_id,
            revenue_account_id,
            is_active: dto.is_active,
        },
        &NewProductExtra {
            id,
            product_type: &product_type,
            cn8_code: &cn8,
            cn8_name: &dto.cn8_name,
            expense_account_id: expense,
            auto_write_off,
        },
    )
    .await?;
    Ok(Json(
        serde_json::to_value(mappers::product_dto(&row)).unwrap(),
    ))
}

pub async fn delete(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let (_user, _company) = active_company(&st, &session.claims.sub).await?;
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    products::delete(&mut conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
