//! Вътрешни документи (протоколи за преместване) — реална логика от ytbuch idocs.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::{json, Value};

use crate::schema::{
    fixed_asset_events, fixed_assets, internal_doc_lines, internal_docs, locations,
};
use crate::StorageResult;

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = internal_docs)]
pub struct InternalDoc {
    pub id: i64,
    pub company_id: i64,
    pub number: String,
    pub doc_date: NaiveDate,
    pub from_location_id: i64,
    pub to_location_id: i64,
    pub status: String,
    pub notes: String,
    pub handed_by: String,
    pub received_by: String,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = internal_doc_lines)]
pub struct InternalLine {
    pub id: i64,
    pub doc_id: i64,
    pub line_kind: String,
    pub product_id: i64,
    pub asset_id: i64,
    pub quantity: BigDecimal,
    pub unit_cost: BigDecimal,
    pub amount: BigDecimal,
    pub description: String,
}

pub fn line_json(l: &InternalLine) -> Value {
    json!({
        "id": l.id, "line_kind": l.line_kind, "product_id": l.product_id, "asset_id": l.asset_id,
        "quantity": format!("{}", l.quantity), "unit_cost": format!("{}", l.unit_cost),
        "amount": format!("{:.2}", l.amount), "description": l.description,
    })
}

pub fn doc_json(d: &InternalDoc, lines: &[InternalLine]) -> Value {
    json!({
        "id": d.id, "number": d.number, "doc_date": d.doc_date.to_string(),
        "from_location_id": d.from_location_id, "to_location_id": d.to_location_id,
        "status": d.status, "notes": d.notes, "handed_by": d.handed_by, "received_by": d.received_by,
        "lines": lines.iter().map(line_json).collect::<Vec<_>>(),
    })
}

pub async fn next_number(conn: &mut crate::DbConn, company_id: i64) -> StorageResult<String> {
    let nums: Vec<String> = internal_docs::table
        .filter(internal_docs::company_id.eq(company_id))
        .select(internal_docs::number)
        .load(conn)
        .await?;
    let max = nums
        .iter()
        .filter_map(|s| s.parse::<i64>().ok())
        .max()
        .unwrap_or(0);
    Ok(format!("{:010}", max + 1))
}

pub async fn list(
    conn: &mut crate::DbConn,
    company_id: i64,
) -> StorageResult<Vec<(InternalDoc, Vec<InternalLine>)>> {
    let docs: Vec<InternalDoc> = internal_docs::table
        .filter(internal_docs::company_id.eq(company_id))
        .order(internal_docs::id.desc())
        .load(conn)
        .await?;
    let mut out = Vec::new();
    for d in docs {
        let lines = lines_for(conn, d.id).await?;
        out.push((d, lines));
    }
    Ok(out)
}

pub async fn lines_for(conn: &mut crate::DbConn, doc_id: i64) -> StorageResult<Vec<InternalLine>> {
    Ok(internal_doc_lines::table
        .filter(internal_doc_lines::doc_id.eq(doc_id))
        .order(internal_doc_lines::id.asc())
        .load(conn)
        .await?)
}

pub async fn get(
    conn: &mut crate::DbConn,
    company_id: i64,
    id: i64,
) -> StorageResult<Option<(InternalDoc, Vec<InternalLine>)>> {
    let d: Option<InternalDoc> = internal_docs::table
        .filter(internal_docs::company_id.eq(company_id))
        .find(id)
        .first(conn)
        .await
        .optional()?;
    match d {
        Some(d) => Ok(Some((d.clone(), lines_for(conn, d.id).await?))),
        None => Ok(None),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn create(
    conn: &mut crate::DbConn,
    company_id: i64,
    number: &str,
    doc_date: NaiveDate,
    from: i64,
    to: i64,
    notes: &str,
    handed_by: &str,
    received_by: &str,
    lines: &[Value],
) -> StorageResult<(InternalDoc, Vec<InternalLine>)> {
    let d: InternalDoc = diesel::insert_into(internal_docs::table)
        .values((
            internal_docs::company_id.eq(company_id),
            internal_docs::number.eq(number),
            internal_docs::doc_date.eq(doc_date),
            internal_docs::from_location_id.eq(from),
            internal_docs::to_location_id.eq(to),
            internal_docs::notes.eq(notes),
            internal_docs::handed_by.eq(handed_by),
            internal_docs::received_by.eq(received_by),
        ))
        .get_result(conn)
        .await?;
    insert_lines(conn, d.id, lines).await?;
    Ok((d.clone(), lines_for(conn, d.id).await?))
}

pub async fn insert_lines(
    conn: &mut crate::DbConn,
    doc_id: i64,
    lines: &[Value],
) -> StorageResult<()> {
    for l in lines {
        let qty = dec(l["quantity"].as_str().unwrap_or("0"));
        let cost = dec(l["unit_cost"].as_str().unwrap_or("0"));
        let amount = if l["amount"].as_str().is_some() {
            dec(l["amount"].as_str().unwrap())
        } else {
            &qty * &cost
        };
        diesel::insert_into(internal_doc_lines::table)
            .values((
                internal_doc_lines::doc_id.eq(doc_id),
                internal_doc_lines::line_kind.eq(l["line_kind"].as_str().unwrap_or("product")),
                internal_doc_lines::product_id.eq(l["product_id"].as_i64().unwrap_or(0)),
                internal_doc_lines::asset_id.eq(l["asset_id"].as_i64().unwrap_or(0)),
                internal_doc_lines::quantity.eq(qty),
                internal_doc_lines::unit_cost.eq(cost),
                internal_doc_lines::amount.eq(amount),
                internal_doc_lines::description.eq(l["description"].as_str().unwrap_or("")),
            ))
            .execute(conn)
            .await?;
    }
    Ok(())
}

pub async fn replace_lines(
    conn: &mut crate::DbConn,
    doc_id: i64,
    lines: &[Value],
) -> StorageResult<()> {
    diesel::delete(internal_doc_lines::table.filter(internal_doc_lines::doc_id.eq(doc_id)))
        .execute(conn)
        .await?;
    insert_lines(conn, doc_id, lines).await
}

pub async fn delete(conn: &mut crate::DbConn, company_id: i64, id: i64) -> StorageResult<bool> {
    let n = diesel::delete(
        internal_docs::table
            .filter(internal_docs::company_id.eq(company_id))
            .find(id),
    )
    .execute(conn)
    .await?;
    if n > 0 {
        diesel::delete(internal_doc_lines::table.filter(internal_doc_lines::doc_id.eq(id)))
            .execute(conn)
            .await?;
    }
    Ok(n > 0)
}

pub async fn set_status(conn: &mut crate::DbConn, id: i64, status: &str) -> StorageResult<()> {
    diesel::update(internal_docs::table.find(id))
        .set(internal_docs::status.eq(status))
        .execute(conn)
        .await?;
    Ok(())
}

/// Потвърждение: премества ДМА (location + event); продуктовите редове се
/// движат от stock rebuild през потвърдените документи.
pub async fn confirm_assets(
    conn: &mut crate::DbConn,
    company_id: i64,
    doc: &InternalDoc,
    lines: &[InternalLine],
) -> StorageResult<()> {
    let to_name = location_name(conn, company_id, doc.to_location_id).await?;
    let from_name = location_name(conn, company_id, doc.from_location_id).await?;
    for l in lines
        .iter()
        .filter(|l| l.line_kind == "fixed_asset" && l.asset_id > 0)
    {
        diesel::update(fixed_assets::table.find(l.asset_id))
            .set(fixed_assets::location_id.eq(doc.to_location_id))
            .execute(conn)
            .await?;
        diesel::insert_into(fixed_asset_events::table)
            .values((
                fixed_asset_events::company_id.eq(company_id),
                fixed_asset_events::asset_id.eq(l.asset_id),
                fixed_asset_events::event_type.eq("movement"),
                fixed_asset_events::event_date.eq(doc.doc_date),
                fixed_asset_events::amount.eq(BigDecimal::from(0)),
                fixed_asset_events::location.eq(&to_name),
                fixed_asset_events::reason.eq(format!(
                    "протокол {}: {} → {}",
                    doc.number, from_name, to_name
                )),
            ))
            .execute(conn)
            .await?;
    }
    Ok(())
}

pub async fn location_name(
    conn: &mut crate::DbConn,
    company_id: i64,
    id: i64,
) -> StorageResult<String> {
    if id == 0 {
        return Ok(String::new());
    }
    let n: Option<String> = locations::table
        .filter(locations::company_id.eq(company_id))
        .find(id)
        .select(locations::name)
        .first(conn)
        .await
        .optional()?;
    Ok(n.unwrap_or_default())
}

fn dec(s: &str) -> BigDecimal {
    s.replace(',', ".")
        .parse()
        .unwrap_or_else(|_| BigDecimal::from(0))
}
