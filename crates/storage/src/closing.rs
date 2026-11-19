//! Приключвателни операции: прехвърляне на салда по карта (default 60→611).

use bigdecimal::{BigDecimal, Zero};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{accounts, journal, models::journal::{NewJournalEntry, NewJournalLine}, schema::{journal_entries, journal_lines}, periods, StorageError};

pub struct ClosingRow {
    pub source: String,
    pub source_name: String,
    pub target: String,
    pub amount: BigDecimal,
}

pub fn month_key(year: i32, month: i32) -> String {
    format!("{year:04}-{month:02}")
}

fn last_day(year: i32, month: i32) -> NaiveDate {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(ny, nm as u32, 1).unwrap() - chrono::Duration::days(1)
}

/// Карта source→target от companies.settings.closing_map; default 60→611.
fn closing_map(settings: &serde_json::Value) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    match settings.get("closing_map") {
        Some(serde_json::Value::Object(m)) => {
            for (k, v) in m {
                if let Some(t) = v.as_str() { out.push((k.clone(), t.to_string())); }
            }
        }
        Some(serde_json::Value::Array(a)) => {
            for e in a {
                if let (Some(s), Some(t)) = (e.get("source").and_then(|x| x.as_str()), e.get("target").and_then(|x| x.as_str())) {
                    out.push((s.to_string(), t.to_string()));
                }
            }
        }
        _ => out.push(("60".into(), "611".into())),
    }
    if out.is_empty() { out.push(("60".into(), "611".into())); }
    out.sort_by_key(|(s, _)| std::cmp::Reverse(s.len())); // най-дълъг префикс пръв
    out
}

fn target_for(map: &[(String, String)], number: &str) -> Option<String> {
    map.iter().find(|(s, _)| number.starts_with(s.as_str())).map(|(_, t)| t.clone())
}

/// Салда по сметка за периода (дебит − кредит).
async fn balances(conn: &mut AsyncPgConnection, company_id: i64, key: &str) -> Result<std::collections::HashMap<i64, BigDecimal>, StorageError> {
    let rows = journal_lines::table
        .inner_join(journal_entries::table.on(journal_entries::id.eq(journal_lines::entry_id)))
        .filter(journal_entries::company_id.eq(company_id))
        .filter(journal_entries::vat_period.eq(key))
        .select((journal_lines::account_id, journal_lines::direction, journal_lines::amount))
        .load::<(i64, String, BigDecimal)>(conn)
        .await?;
    let mut map = std::collections::HashMap::new();
    for (id, dir, amount) in rows {
        let e = map.entry(id).or_insert_with(BigDecimal::default);
        if dir == "debit" { *e += amount; } else { *e -= amount; }
    }
    Ok(map)
}

pub async fn preview(conn: &mut AsyncPgConnection, company_id: i64, year: i32, month: i32) -> Result<Vec<ClosingRow>, StorageError> {
    let key = month_key(year, month);
    let company = crate::companies::by_id(conn, company_id).await?.ok_or(StorageError::Connection("фирмата липсва".into()))?;
    let settings = company.1.map(|d| d.settings).unwrap_or(serde_json::Value::Null);
    let map = closing_map(&settings);
    let accts = accounts::list(conn, company_id).await?;
    let bal = balances(conn, company_id, &key).await?;
    let mut out = Vec::new();
    for a in &accts {
        let Some(target) = target_for(&map, &a.number) else { continue };
        let amount = bal.get(&a.id).cloned().unwrap_or_default();
        if amount.is_zero() { continue; }
        out.push(ClosingRow { source: a.number.clone(), source_name: a.name.clone(), target, amount });
    }
    Ok(out)
}

pub async fn already_run(conn: &mut AsyncPgConnection, company_id: i64, key: &str) -> Result<bool, StorageError> {
    let n: i64 = journal_entries::table
        .filter(journal_entries::company_id.eq(company_id))
        .filter(journal_entries::document_number.eq(format!("CLOSE-{key}")))
        .count().get_result(conn).await?;
    Ok(n > 0)
}

pub async fn run(conn: &mut AsyncPgConnection, company_id: i64, year: i32, month: i32) -> Result<i64, StorageError> {
    let key = month_key(year, month);
    if periods::is_closed(conn, company_id, &key).await? {
        return Err(StorageError::Connection(format!("периодът {key} е затворен")));
    }
    if already_run(conn, company_id, &key).await? {
        return Err(StorageError::Connection(format!("приключване за {key} вече е записано")));
    }
    let rows = preview(conn, company_id, year, month).await?;
    if rows.is_empty() { return Err(StorageError::Connection("няма салда за приключване".into())); }
    let date = last_day(year, month);
    let mut lines: Vec<NewJournalLine> = Vec::new();
    let mut total = BigDecimal::default();
    let accts = accounts::list(conn, company_id).await?;
    for r in &rows {
        let src = accts.iter().find(|a| a.number == r.source).unwrap();
        lines.push(NewJournalLine { entry_id: 0, account_id: src.id, account_number: src.number.clone(), account_name: src.name.clone(), direction: "credit".into(), amount: r.amount.clone(), vat_amount: BigDecimal::default(), currency: String::new(), fx_quantity: BigDecimal::default(), fx_rate: BigDecimal::from(1) });
        total += &r.amount;
    }
    let target_number = rows[0].target.clone();
    let tgt = accounts::get_or_create(conn, company_id, &target_number).await?;
    lines.push(NewJournalLine { entry_id: 0, account_id: tgt.id, account_number: tgt.number.clone(), account_name: tgt.name.clone(), direction: "debit".into(), amount: total, vat_amount: BigDecimal::default(), currency: String::new(), fx_quantity: BigDecimal::default(), fx_rate: BigDecimal::from(1) });
    let entry = NewJournalEntry { company_id, entry_date: date, document_type: "closing".into(), document_id: 0, document_number: format!("CLOSE-{key}"), description: format!("Приключвателни операции {key}"), vat_type: "no_vat".into(), counterpart_id: 0, counterpart_name: String::new(), document_date: date, vat_period: key };
    journal::insert_entry(conn, &entry, &lines).await
}
