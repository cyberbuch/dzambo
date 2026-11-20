//! Склад: партиди и оценка (WAC/FIFO).

use bigdecimal::{BigDecimal, Zero};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    models::invoices::InvoiceLine,
    models::products::Product,
    schema::{inventory_lots, products},
    StorageError,
};

#[allow(clippy::too_many_arguments)]
pub async fn add_lot(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    product_id: i64,
    invoice_id: i64,
    line_id: i64,
    date: NaiveDate,
    quantity: BigDecimal,
    unit_cost: BigDecimal,
) -> Result<(), StorageError> {
    diesel::insert_into(inventory_lots::table)
        .values((
            inventory_lots::company_id.eq(company_id),
            inventory_lots::product_id.eq(product_id),
            inventory_lots::invoice_id.eq(invoice_id),
            inventory_lots::invoice_line_id.eq(line_id),
            inventory_lots::received_date.eq(date),
            inventory_lots::quantity.eq(quantity.clone()),
            inventory_lots::remaining.eq(quantity),
            inventory_lots::unit_cost.eq(unit_cost),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

/// При пост на входяща фактура редовете със стока стават партиди.
pub async fn lots_from_invoice(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    invoice_id: i64,
    lines: &[InvoiceLine],
    date: NaiveDate,
) -> Result<(), StorageError> {
    for l in lines {
        if l.product_id <= 0 {
            continue;
        }
        let qty = l.quantity.clone();
        if qty.is_zero() {
            continue;
        }
        add_lot(
            conn,
            company_id,
            l.product_id,
            invoice_id,
            l.id,
            date,
            qty,
            l.unit_price.clone(),
        )
        .await?;
    }
    Ok(())
}

pub struct LotRow {
    pub product_id: i64,
    pub remaining: BigDecimal,
    pub unit_cost: BigDecimal,
}

pub async fn open_lots(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<LotRow>, StorageError> {
    inventory_lots::table
        .filter(inventory_lots::company_id.eq(company_id))
        .select((
            inventory_lots::product_id,
            inventory_lots::remaining,
            inventory_lots::unit_cost,
        ))
        .load::<(i64, BigDecimal, BigDecimal)>(conn)
        .await
        .map(|rows| {
            rows.into_iter()
                .filter(|(_, r, _)| !r.is_zero())
                .map(|(product_id, remaining, unit_cost)| LotRow {
                    product_id,
                    remaining,
                    unit_cost,
                })
                .collect()
        })
        .map_err(StorageError::from)
}

pub struct StockItem {
    pub product_id: i64,
    pub name: String,
    pub code: String,
    pub unit: String,
    pub quantity: BigDecimal,
    pub avg_unit_cost: BigDecimal,
    pub value: BigDecimal,
}

/// Справка по WAC (средно претеглена): стойност = Σ остатък × единична цена.
pub async fn report_wac(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<StockItem>, StorageError> {
    let lots = open_lots(conn, company_id).await?;
    let prods: Vec<Product> = products::table
        .filter(products::company_id.eq(company_id))
        .select(Product::as_select())
        .load(conn)
        .await?;
    let mut agg: std::collections::HashMap<i64, (BigDecimal, BigDecimal)> =
        std::collections::HashMap::new();
    for l in lots {
        let e = agg
            .entry(l.product_id)
            .or_insert_with(|| (BigDecimal::default(), BigDecimal::default()));
        e.0 += &l.remaining;
        e.1 += &l.remaining * &l.unit_cost;
    }
    let mut out = Vec::new();
    for (pid, (qty, value)) in agg {
        let Some(p) = prods.iter().find(|p| p.id == pid) else {
            continue;
        };
        let avg = if qty.is_zero() {
            BigDecimal::zero()
        } else {
            &value / &qty
        };
        out.push(StockItem {
            product_id: pid,
            name: p.name.clone(),
            code: p.code.clone(),
            unit: p.unit.clone(),
            quantity: qty,
            avg_unit_cost: avg,
            value,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Консумира партиди по FIFO (най-старите първи); връща общата себестойност.
pub async fn consume_fifo(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    product_id: i64,
    qty: BigDecimal,
    lifo: bool,
) -> Result<BigDecimal, StorageError> {
    let mut remaining = qty.clone();
    let mut cost = BigDecimal::default();
    if qty.is_zero() {
        return Ok(cost);
    }
    let lots = if lifo {
        inventory_lots::table
            .filter(inventory_lots::company_id.eq(company_id))
            .filter(inventory_lots::product_id.eq(product_id))
            .order((
                inventory_lots::received_date.desc(),
                inventory_lots::id.desc(),
            ))
            .select((
                inventory_lots::id,
                inventory_lots::remaining,
                inventory_lots::unit_cost,
            ))
            .load::<(i64, BigDecimal, BigDecimal)>(conn)
            .await?
    } else {
        inventory_lots::table
            .filter(inventory_lots::company_id.eq(company_id))
            .filter(inventory_lots::product_id.eq(product_id))
            .order((
                inventory_lots::received_date.asc(),
                inventory_lots::id.asc(),
            ))
            .select((
                inventory_lots::id,
                inventory_lots::remaining,
                inventory_lots::unit_cost,
            ))
            .load::<(i64, BigDecimal, BigDecimal)>(conn)
            .await?
    };
    for (id, lot_remaining, unit_cost) in lots {
        if remaining.is_zero() {
            break;
        }
        let take = if remaining >= lot_remaining {
            lot_remaining.clone()
        } else {
            remaining.clone()
        };
        let new_remaining = &lot_remaining - &take;
        diesel::update(inventory_lots::table.find(id))
            .set(inventory_lots::remaining.eq(new_remaining))
            .execute(conn)
            .await?;
        cost += &take * &unit_cost;
        remaining -= &take;
    }
    Ok(cost)
}

/// Наличности по обект: агрегирани партиди (remaining) по артикул.
pub async fn at_location(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    location_id: i64,
) -> Result<Vec<(i64, String, String, String, String, String)>, StorageError> {
    let lots: Vec<(i64, BigDecimal, BigDecimal)> = inventory_lots::table
        .filter(inventory_lots::company_id.eq(company_id))
        .filter(inventory_lots::location_id.eq(location_id))
        .select((
            inventory_lots::product_id,
            inventory_lots::remaining,
            inventory_lots::unit_cost,
        ))
        .load(conn)
        .await?;
    let mut agg: std::collections::HashMap<i64, (BigDecimal, BigDecimal)> = Default::default();
    for (pid, rem, cost) in lots {
        if rem.is_zero() {
            continue;
        }
        let e = agg
            .entry(pid)
            .or_insert_with(|| (BigDecimal::from(0), BigDecimal::from(0)));
        e.0 += &rem;
        e.1 += &rem * &cost;
    }
    let mut out = Vec::new();
    for (pid, (qty, val)) in agg {
        if qty.is_zero() {
            continue;
        }
        let p: Option<(String, String, String)> = products::table
            .find(pid)
            .select((products::name, products::code, products::unit))
            .first(conn)
            .await
            .optional()?;
        let (name, code, unit) = p.unwrap_or_default();
        let avg = &val / &qty;
        out.push((
            pid,
            name,
            code,
            unit,
            format!("{qty:.4}"),
            format!("{avg:.2}"),
        ));
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}
