//! Склад: правила, обекти и ръчно изписване (material issues).
//! Партидите и месечното изписване са в stock_replay.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::{json, Value};

use crate::schema::{locations, material_issue_lines, material_issues, stock_account_rules};
use crate::StorageResult;

pub use crate::stock_replay::{rebuild, write_journal, writeoff};

fn dec(s: &str) -> BigDecimal {
    s.replace(',', ".")
        .parse()
        .unwrap_or_else(|_| BigDecimal::from(0))
}

// ---- Правила ----

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = stock_account_rules)]
pub struct Rule {
    pub id: i64,
    pub company_id: i64,
    pub kind: String,
    pub inventory_account_id: i64,
    pub offset_account_id: i64,
    pub clearing_account_id: i64,
}

pub fn rule_json(r: &Rule) -> Value {
    json!({"id": r.id, "kind": r.kind, "inventory_account_id": r.inventory_account_id,
        "offset_account_id": r.offset_account_id, "clearing_account_id": r.clearing_account_id})
}

pub async fn list_rules(conn: &mut crate::DbConn, company_id: i64) -> StorageResult<Vec<Rule>> {
    Ok(stock_account_rules::table
        .filter(stock_account_rules::company_id.eq(company_id))
        .order(stock_account_rules::id.asc())
        .load(conn)
        .await?)
}

pub async fn create_rule(
    conn: &mut crate::DbConn,
    company_id: i64,
    kind: &str,
    inventory: i64,
    offset: i64,
    clearing: i64,
) -> StorageResult<Rule> {
    let clearing = if kind == "production" { clearing } else { 0 };
    Ok(diesel::insert_into(stock_account_rules::table)
        .values((
            stock_account_rules::company_id.eq(company_id),
            stock_account_rules::kind.eq(kind),
            stock_account_rules::inventory_account_id.eq(inventory),
            stock_account_rules::offset_account_id.eq(offset),
            stock_account_rules::clearing_account_id.eq(clearing),
        ))
        .get_result(conn)
        .await?)
}

pub async fn delete_rule(
    conn: &mut crate::DbConn,
    company_id: i64,
    id: i64,
) -> StorageResult<bool> {
    let n = diesel::delete(
        stock_account_rules::table
            .filter(stock_account_rules::company_id.eq(company_id))
            .find(id),
    )
    .execute(conn)
    .await?;
    Ok(n > 0)
}

pub fn pair<'a>(rules: &'a [Rule], kind: &str, inventory: i64, offset: i64) -> Option<&'a Rule> {
    rules.iter().find(|r| {
        r.kind == kind && r.inventory_account_id == inventory && r.offset_account_id == offset
    })
}

// ---- Обекти ----

pub async fn list_locations(
    conn: &mut crate::DbConn,
    company_id: i64,
) -> StorageResult<Vec<Value>> {
    let rows: Vec<(i64, String, bool)> = locations::table
        .filter(locations::company_id.eq(company_id))
        .select((locations::id, locations::name, locations::is_active))
        .load(conn)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, active)| json!({"id": id, "name": name, "is_active": active}))
        .collect())
}

pub async fn create_location(
    conn: &mut crate::DbConn,
    company_id: i64,
    name: &str,
) -> StorageResult<Value> {
    let id: i64 = diesel::insert_into(locations::table)
        .values((
            locations::company_id.eq(company_id),
            locations::name.eq(name),
        ))
        .returning(locations::id)
        .get_result(conn)
        .await?;
    Ok(json!({"id": id, "name": name, "is_active": true}))
}

// ---- Ръчно изписване ----

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = material_issues)]
pub struct Issue {
    pub id: i64,
    pub company_id: i64,
    pub number: String,
    pub doc_date: NaiveDate,
    pub location_id: i64,
    pub status: String,
    pub notes: String,
    pub journal_entry_id: i64,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = material_issue_lines)]
pub struct IssueLine {
    pub id: i64,
    pub doc_id: i64,
    pub product_id: i64,
    pub quantity: BigDecimal,
    pub unit_cost: BigDecimal,
    pub amount: BigDecimal,
    pub account_id: i64,
    pub inventory_account_id: i64,
    pub line_no: i32,
}

pub fn issue_json(i: &Issue, lines: &[IssueLine]) -> Value {
    json!({
        "id": i.id, "number": i.number, "doc_date": i.doc_date.to_string(),
        "location_id": i.location_id, "status": i.status, "notes": i.notes,
        "journal_entry_id": i.journal_entry_id,
        "lines": lines.iter().map(|l| json!({
            "id": l.id, "product_id": l.product_id, "quantity": format!("{}", l.quantity),
            "unit_cost": format!("{}", l.unit_cost), "amount": format!("{:.2}", l.amount),
            "account_id": l.account_id, "inventory_account_id": l.inventory_account_id, "line_no": l.line_no,
        })).collect::<Vec<_>>(),
    })
}

pub async fn list_issues(
    conn: &mut crate::DbConn,
    company_id: i64,
) -> StorageResult<Vec<(Issue, Vec<IssueLine>)>> {
    let docs: Vec<Issue> = material_issues::table
        .filter(material_issues::company_id.eq(company_id))
        .order(material_issues::id.desc())
        .load(conn)
        .await?;
    let mut out = Vec::new();
    for i in docs {
        out.push((i.clone(), issue_lines(conn, i.id).await?));
    }
    Ok(out)
}

pub async fn issue_lines(conn: &mut crate::DbConn, doc_id: i64) -> StorageResult<Vec<IssueLine>> {
    Ok(material_issue_lines::table
        .filter(material_issue_lines::doc_id.eq(doc_id))
        .order(material_issue_lines::line_no.asc())
        .load(conn)
        .await?)
}

pub async fn get_issue(
    conn: &mut crate::DbConn,
    company_id: i64,
    id: i64,
) -> StorageResult<Option<(Issue, Vec<IssueLine>)>> {
    let i: Option<Issue> = material_issues::table
        .filter(material_issues::company_id.eq(company_id))
        .find(id)
        .first(conn)
        .await
        .optional()?;
    match i {
        Some(i) => Ok(Some((i.clone(), issue_lines(conn, i.id).await?))),
        None => Ok(None),
    }
}

pub async fn create_issue(
    conn: &mut crate::DbConn,
    company_id: i64,
    doc_date: NaiveDate,
    location_id: i64,
    notes: &str,
    lines: &[Value],
) -> StorageResult<(Issue, Vec<IssueLine>)> {
    let i: Issue = diesel::insert_into(material_issues::table)
        .values((
            material_issues::company_id.eq(company_id),
            material_issues::number.eq(""),
            material_issues::doc_date.eq(doc_date),
            material_issues::location_id.eq(location_id),
            material_issues::notes.eq(notes),
        ))
        .get_result(conn)
        .await?;
    let number = format!("МИ-{}", i.id);
    diesel::update(material_issues::table.find(i.id))
        .set(material_issues::number.eq(&number))
        .execute(conn)
        .await?;
    insert_issue_lines(conn, i.id, lines).await?;
    let mut i = i;
    i.number = number;
    Ok((i.clone(), issue_lines(conn, i.id).await?))
}

pub async fn insert_issue_lines(
    conn: &mut crate::DbConn,
    doc_id: i64,
    lines: &[Value],
) -> StorageResult<()> {
    for (n, l) in lines.iter().enumerate() {
        diesel::insert_into(material_issue_lines::table)
            .values((
                material_issue_lines::doc_id.eq(doc_id),
                material_issue_lines::product_id.eq(l["product_id"].as_i64().unwrap_or(0)),
                material_issue_lines::quantity.eq(dec(l["quantity"].as_str().unwrap_or("0"))),
                material_issue_lines::account_id.eq(l["account_id"].as_i64().unwrap_or(0)),
                material_issue_lines::inventory_account_id
                    .eq(l["inventory_account_id"].as_i64().unwrap_or(0)),
                material_issue_lines::line_no.eq(n as i32),
            ))
            .execute(conn)
            .await?;
    }
    Ok(())
}

pub async fn delete_issue(
    conn: &mut crate::DbConn,
    company_id: i64,
    id: i64,
) -> StorageResult<bool> {
    let n = diesel::delete(
        material_issues::table
            .filter(material_issues::company_id.eq(company_id))
            .find(id),
    )
    .execute(conn)
    .await?;
    if n > 0 {
        diesel::delete(material_issue_lines::table.filter(material_issue_lines::doc_id.eq(id)))
            .execute(conn)
            .await?;
    }
    Ok(n > 0)
}
