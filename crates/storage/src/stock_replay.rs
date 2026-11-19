//! Партиди: rebuild на наличности (WAC/FIFO/LIFO), презапис на journal-а на
//! ръчните изписвания и месечно изписване на стоки — както ytbuch stock_replay/writeoff.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::{json, Value};
use std::str::FromStr;

use crate::schema::{
    companies, inventory_lots, invoice_lines, invoices, journal_entries, journal_lines,
    material_issue_lines, material_issues, products,
};
use crate::stockops::{issue_lines, list_rules, pair, Issue};
use crate::StorageResult;

fn d(v: &BigDecimal) -> f64 {
    v.to_string().parse().unwrap_or(0.0)
}
fn bd(v: f64) -> BigDecimal {
    BigDecimal::from_str(&format!("{v:.6}")).unwrap_or_else(|_| BigDecimal::from(0))
}

// ---- Партиди / rebuild ----

#[derive(Clone)]
struct Lot {
    location: i64,
    qty: f64,
    cost: f64,
}

#[derive(Clone)]
struct Mov {
    date: NaiveDate,
    rank: i32,
    seq: i64,
    product: i64,
    location: i64,
    qty: f64,
    kind: &'static str, // receipt | move | issue | sale
    to: i64,
    unit_cost: f64,
    doc: String,
}

pub async fn rebuild(conn: &mut crate::DbConn, company_id: i64) -> StorageResult<Value> {
    let method: String = companies::table
        .find(company_id)
        .select(companies::inventory_valuation_method)
        .first(conn)
        .await
        .unwrap_or_else(|_| "WAC".into());
    let mut movs = load_movs(conn, company_id).await?;
    movs.sort_by_key(|m| (m.date, m.rank, m.seq));

    let mut lots: std::collections::HashMap<i64, Vec<Lot>> = Default::default();
    let mut warnings: Vec<String> = Vec::new();
    let mut issue_cost: std::collections::HashMap<(i64, i64), f64> = Default::default(); // (doc,line)->cost
    let mut sale_cost: std::collections::HashMap<(i64, String), f64> = Default::default(); // (product, month)->cost

    for m in &movs {
        let entry = lots.entry(m.product).or_default();
        match m.kind {
            "receipt" => entry.push(Lot {
                location: m.location,
                qty: m.qty,
                cost: m.unit_cost,
            }),
            "move" => {
                let cost = consume(entry, m.location, m.qty, &method);
                entry.push(Lot {
                    location: m.to,
                    qty: m.qty,
                    cost,
                });
            }
            "issue" | "sale" => {
                let cost = consume(entry, m.location, m.qty, &method);
                if m.kind == "issue" {
                    issue_cost.insert((m.seq, m.product), cost);
                } else {
                    let month = m.date.format("%Y-%m").to_string();
                    *sale_cost.entry((m.product, month)).or_insert(0.0) += cost;
                }
                if cost <= 0.0 && m.qty > 0.0 {
                    warnings.push(format!(
                        "няма наличност за артикул {} на {}",
                        m.product, m.date
                    ));
                }
            }
            _ => {}
        }
    }

    // Персист: партиди + наличности по артикул.
    diesel::delete(inventory_lots::table.filter(inventory_lots::company_id.eq(company_id)))
        .execute(conn)
        .await?;
    let mut balances = 0;
    for (product, ls) in &lots {
        let mut qty = 0.0;
        let mut val = 0.0;
        for l in ls {
            if l.qty <= 0.0 {
                continue;
            }
            qty += l.qty;
            val += l.qty * l.cost;
            diesel::insert_into(inventory_lots::table)
                .values((
                    inventory_lots::company_id.eq(company_id),
                    inventory_lots::product_id.eq(*product),
                    inventory_lots::invoice_id.eq(0),
                    inventory_lots::invoice_line_id.eq(0),
                    inventory_lots::received_date.eq(NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()),
                    inventory_lots::quantity.eq(bd(l.qty)),
                    inventory_lots::remaining.eq(bd(l.qty)),
                    inventory_lots::unit_cost.eq(bd(l.cost)),
                    inventory_lots::location_id.eq(l.location),
                ))
                .execute(conn)
                .await?;
            balances += 1;
        }
        diesel::update(products::table.find(*product))
            .set((
                products::stock_quantity.eq(bd(qty)),
                products::stock_value.eq(bd(val)),
                products::cost_price.eq(bd(if qty > 0.0 { val / qty } else { 0.0 })),
            ))
            .execute(conn)
            .await?;
    }

    // Презапис на journal-а на ръчните изписвания.
    rewrite_issues(conn, company_id, &issue_cost).await?;

    Ok(
        json!({"warnings": warnings, "balances": balances, "sale_cost": sale_cost.iter().map(|((p, m), c)| json!({"product": p, "month": m, "cost": format!("{c:.2}")})).collect::<Vec<_>>()}),
    )
}

fn consume(lots: &mut [Lot], location: i64, qty: f64, method: &str) -> f64 {
    let mut need = qty;
    let mut cost = 0.0;
    let same: Vec<usize> = lots
        .iter()
        .enumerate()
        .filter(|(_, l)| l.location == location && l.qty > 0.0)
        .map(|(i, _)| i)
        .collect();
    let other: Vec<usize> = lots
        .iter()
        .enumerate()
        .filter(|(_, l)| l.location != location && l.qty > 0.0)
        .map(|(i, _)| i)
        .collect();
    let order: Vec<usize> = match method {
        "LIFO" => same
            .into_iter()
            .rev()
            .chain(other.into_iter().rev())
            .collect(),
        "FIFO" => same.into_iter().chain(other).collect(),
        _ => same.into_iter().chain(other).collect(),
    };
    if method == "WAC" {
        let total_qty: f64 = lots.iter().map(|l| l.qty).sum();
        let total_val: f64 = lots.iter().map(|l| l.qty * l.cost).sum();
        let unit = if total_qty > 0.0 {
            total_val / total_qty
        } else {
            0.0
        };
        cost = unit * qty.min(total_qty);
        for i in order {
            if need <= 0.0 {
                break;
            }
            let take = need.min(lots[i].qty);
            lots[i].qty -= take;
            need -= take;
        }
        return cost;
    }
    for i in order {
        if need <= 0.0 {
            break;
        }
        let take = need.min(lots[i].qty);
        cost += take * lots[i].cost;
        lots[i].qty -= take;
        need -= take;
    }
    cost
}

async fn load_movs(conn: &mut crate::DbConn, company_id: i64) -> StorageResult<Vec<Mov>> {
    let mut movs = Vec::new();
    // Доставки и продажби от постнати фактури.
    let invs: Vec<(i64, String, String, NaiveDate, i64, String)> = invoices::table
        .filter(invoices::company_id.eq(company_id))
        .filter(invoices::status.eq("posted"))
        .select((
            invoices::id,
            invoices::direction,
            invoices::document_type,
            invoices::issue_date,
            invoices::location_id,
            invoices::number,
        ))
        .load(conn)
        .await?;
    for (id, dir, dtype, date, loc, number) in invs {
        if !matches!(dtype.as_str(), "01" | "11" | "02" | "03") {
            continue;
        }
        let lines: Vec<(i64, BigDecimal, BigDecimal)> = invoice_lines::table
            .filter(invoice_lines::invoice_id.eq(id))
            .select((
                invoice_lines::product_id,
                invoice_lines::quantity,
                invoice_lines::unit_price,
            ))
            .load(conn)
            .await?;
        for (n, (pid, qty, price)) in lines.into_iter().enumerate() {
            if pid == 0 {
                continue;
            }
            let kind = if dir == "in" { "receipt" } else { "sale" };
            movs.push(Mov {
                date,
                rank: if dir == "in" { 1 } else { 6 },
                seq: id * 1000 + n as i64,
                product: pid,
                location: loc,
                qty: d(&qty),
                kind,
                to: 0,
                unit_cost: d(&price),
                doc: number.clone(),
            });
        }
    }
    // Премествания от потвърдени протоколи.
    let docs: Vec<(i64, NaiveDate, i64, i64, String)> = crate::schema::internal_docs::table
        .filter(crate::schema::internal_docs::company_id.eq(company_id))
        .filter(crate::schema::internal_docs::status.eq("confirmed"))
        .select((
            crate::schema::internal_docs::id,
            crate::schema::internal_docs::doc_date,
            crate::schema::internal_docs::from_location_id,
            crate::schema::internal_docs::to_location_id,
            crate::schema::internal_docs::number,
        ))
        .load(conn)
        .await?;
    for (id, date, from, to, number) in docs {
        let lines: Vec<(i64, BigDecimal)> = crate::schema::internal_doc_lines::table
            .filter(crate::schema::internal_doc_lines::doc_id.eq(id))
            .filter(crate::schema::internal_doc_lines::line_kind.eq("product"))
            .select((
                crate::schema::internal_doc_lines::product_id,
                crate::schema::internal_doc_lines::quantity,
            ))
            .load(conn)
            .await?;
        for (n, (pid, qty)) in lines.into_iter().enumerate() {
            movs.push(Mov {
                date,
                rank: 2,
                seq: id * 1000 + n as i64,
                product: pid,
                location: from,
                qty: d(&qty),
                kind: "move",
                to,
                unit_cost: 0.0,
                doc: number.clone(),
            });
        }
    }
    // Ръчни изписвания.
    let issues: Vec<(i64, NaiveDate, i64, String)> = material_issues::table
        .filter(material_issues::company_id.eq(company_id))
        .filter(material_issues::status.eq("confirmed"))
        .select((
            material_issues::id,
            material_issues::doc_date,
            material_issues::location_id,
            material_issues::number,
        ))
        .load(conn)
        .await?;
    for (id, date, loc, number) in issues {
        let lines: Vec<(i64, i64, BigDecimal)> = material_issue_lines::table
            .filter(material_issue_lines::doc_id.eq(id))
            .select((
                material_issue_lines::id,
                material_issue_lines::product_id,
                material_issue_lines::quantity,
            ))
            .load(conn)
            .await?;
        for (lid, pid, qty) in lines {
            movs.push(Mov {
                date,
                rank: 5,
                seq: lid,
                product: pid,
                location: loc,
                qty: d(&qty),
                kind: "issue",
                to: 0,
                unit_cost: 0.0,
                doc: number.clone(),
            });
        }
    }
    Ok(movs)
}

async fn rewrite_issues(
    conn: &mut crate::DbConn,
    company_id: i64,
    issue_cost: &std::collections::HashMap<(i64, i64), f64>,
) -> StorageResult<()> {
    let issues: Vec<Issue> = material_issues::table
        .filter(material_issues::company_id.eq(company_id))
        .filter(material_issues::status.eq("confirmed"))
        .load(conn)
        .await?;
    for i in issues {
        let lines = issue_lines(conn, i.id).await?;
        // Изтрий стария journal за този документ.
        let old: Vec<i64> = journal_entries::table
            .filter(journal_entries::company_id.eq(company_id))
            .filter(journal_entries::document_type.eq("material_issue"))
            .filter(journal_entries::document_id.eq(i.id))
            .select(journal_entries::id)
            .load(conn)
            .await?;
        for oid in old {
            diesel::delete(journal_lines::table.filter(journal_lines::entry_id.eq(oid)))
                .execute(conn)
                .await?;
            diesel::delete(journal_entries::table.find(oid))
                .execute(conn)
                .await?;
        }
        let mut jl: Vec<(i64, String, BigDecimal)> = Vec::new();
        for l in &lines {
            let cost = issue_cost
                .get(&(l.id, l.product_id))
                .copied()
                .unwrap_or(0.0);
            let amount = bd(cost);
            let unit = if d(&l.quantity) > 0.0 {
                bd(cost / d(&l.quantity))
            } else {
                BigDecimal::from(0)
            };
            diesel::update(material_issue_lines::table.find(l.id))
                .set((
                    material_issue_lines::unit_cost.eq(unit),
                    material_issue_lines::amount.eq(&amount),
                ))
                .execute(conn)
                .await?;
            if l.account_id > 0 {
                jl.push((l.account_id, "debit".into(), amount.clone()));
            }
            if l.inventory_account_id > 0 {
                jl.push((l.inventory_account_id, "credit".into(), amount));
            }
        }
        if jl.is_empty() {
            continue;
        }
        let eid = write_journal(
            conn,
            company_id,
            i.doc_date,
            "material_issue",
            i.id,
            &i.number,
            &format!("Изписване {}", i.number),
            &jl,
        )
        .await?;
        diesel::update(material_issues::table.find(i.id))
            .set(material_issues::journal_entry_id.eq(eid))
            .execute(conn)
            .await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn write_journal(
    conn: &mut crate::DbConn,
    company_id: i64,
    date: NaiveDate,
    dtype: &str,
    doc_id: i64,
    number: &str,
    description: &str,
    lines: &[(i64, String, BigDecimal)],
) -> StorageResult<i64> {
    let month = date.format("%Y-%m").to_string();
    let eid: i64 = diesel::insert_into(journal_entries::table)
        .values((
            journal_entries::company_id.eq(company_id),
            journal_entries::entry_date.eq(date),
            journal_entries::document_type.eq(dtype),
            journal_entries::document_id.eq(doc_id),
            journal_entries::document_number.eq(number),
            journal_entries::description.eq(description),
            journal_entries::vat_type.eq("no_vat"),
            journal_entries::vat_period.eq(&month),
            journal_entries::document_date.eq(date),
        ))
        .returning(journal_entries::id)
        .get_result(conn)
        .await?;
    for (acc, dir, amount) in lines {
        let accnum: Option<(String, String)> = crate::schema::accounts::table
            .find(*acc)
            .select((
                crate::schema::accounts::number,
                crate::schema::accounts::name,
            ))
            .first(conn)
            .await
            .optional()?;
        let (num, name) = accnum.unwrap_or_default();
        diesel::insert_into(journal_lines::table)
            .values((
                journal_lines::entry_id.eq(eid),
                journal_lines::account_id.eq(*acc),
                journal_lines::account_number.eq(num),
                journal_lines::account_name.eq(name),
                journal_lines::direction.eq(dir),
                journal_lines::amount.eq(amount),
            ))
            .execute(conn)
            .await?;
    }
    Ok(eid)
}

// ---- Месечно изписване ----

pub async fn writeoff(
    conn: &mut crate::DbConn,
    company_id: i64,
    month: &str,
) -> StorageResult<Value> {
    let res = rebuild(conn, company_id).await?;
    let rules = list_rules(conn, company_id).await?;
    let sale_cost = res["sale_cost"].as_array().cloned().unwrap_or_default();
    let mut entries = Vec::new();
    let mut warnings: Vec<String> = res["warnings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    let mut total = 0.0;
    for sc in sale_cost {
        if sc["month"].as_str() != Some(month) {
            continue;
        }
        let pid = sc["product"].as_i64().unwrap_or(0);
        let cost = sc["cost"]
            .as_str()
            .unwrap_or("0")
            .parse::<f64>()
            .unwrap_or(0.0);
        if cost <= 0.0 {
            continue;
        }
        let prod: Option<(i64, i64, String)> = products::table
            .find(pid)
            .select((
                products::inventory_account_id,
                products::revenue_account_id,
                products::name,
            ))
            .first(conn)
            .await
            .optional()?;
        let (inv, rev, name) = prod.unwrap_or((0, 0, String::new()));
        let extra_exp: i64 = crate::schema::product_extra::table
            .find(pid)
            .select(crate::schema::product_extra::expense_account_id)
            .first(conn)
            .await
            .unwrap_or(0);
        let debit = if extra_exp > 0 {
            extra_exp
        } else if let Some(r) = pair(&rules, "goods", inv, rev) {
            r.offset_account_id
        } else if let Some(r) = pair(&rules, "production", inv, rev) {
            r.offset_account_id
        } else {
            warnings.push(format!("няма правило за {name}"));
            continue;
        };
        if inv == 0 {
            continue;
        }
        let date = NaiveDate::from_str(&format!("{month}-01")).unwrap();
        let amount = bd(cost);
        total += cost;
        let doc_id = month.replace('-', "").parse::<i64>().unwrap_or(0);
        // Идемпотентно: изтрий стария запис за месеца.
        let old: Vec<i64> = journal_entries::table
            .filter(journal_entries::company_id.eq(company_id))
            .filter(journal_entries::document_type.eq("stock_writeoff"))
            .filter(journal_entries::document_id.eq(doc_id))
            .select(journal_entries::id)
            .load(conn)
            .await?;
        for oid in old {
            diesel::delete(journal_lines::table.filter(journal_lines::entry_id.eq(oid)))
                .execute(conn)
                .await?;
            diesel::delete(journal_entries::table.find(oid))
                .execute(conn)
                .await?;
        }
        let eid = write_journal(
            conn,
            company_id,
            date,
            "stock_writeoff",
            doc_id,
            month,
            &format!("Изписване {month}"),
            &[
                (debit, "debit".into(), amount.clone()),
                (inv, "credit".into(), amount),
            ],
        )
        .await?;
        entries.push(eid);
    }
    Ok(
        json!({"months": [month], "entries": entries, "amount": format!("{total:.2}"), "warnings": warnings}),
    )
}

// ---- Справка за склада (explain): приход/разход/наличност, без запис ----

#[derive(Clone)]
pub struct Flow {
    pub date: String,
    pub doc: String,
    pub kind: String,
    pub reason: String,
    pub product_id: i64,
    pub product_name: String,
    pub location_id: i64,
    pub quantity: String,
    pub value: String,
}

#[derive(Clone)]
pub struct Balance {
    pub product_id: i64,
    pub product_name: String,
    pub location_id: i64,
    pub quantity: String,
    pub value: String,
}

pub struct Explain {
    pub flows: Vec<Flow>,
    pub balances: Vec<Balance>,
    pub warnings: Vec<String>,
}

fn q4(v: f64) -> String {
    format!("{v:.4}")
}
fn m2(v: f64) -> String {
    format!("{v:.2}")
}

pub async fn explain(conn: &mut crate::DbConn, company_id: i64) -> StorageResult<Explain> {
    let method: String = companies::table
        .find(company_id)
        .select(companies::inventory_valuation_method)
        .first(conn)
        .await
        .unwrap_or_else(|_| "WAC".into());
    let names: std::collections::HashMap<i64, String> = products::table
        .filter(products::company_id.eq(company_id))
        .select((products::id, products::name))
        .load::<(i64, String)>(conn)
        .await?
        .into_iter()
        .collect();
    let pname = |pid: i64| names.get(&pid).cloned().unwrap_or_default();

    let mut movs = load_movs(conn, company_id).await?;
    movs.sort_by_key(|m| (m.date, m.rank, m.seq));
    let mut lots: std::collections::HashMap<i64, Vec<Lot>> = Default::default();
    let mut flows = Vec::new();
    let mut warnings = Vec::new();

    for m in &movs {
        let entry = lots.entry(m.product).or_default();
        let day = m.date.to_string();
        match m.kind {
            "receipt" => {
                entry.push(Lot {
                    location: m.location,
                    qty: m.qty,
                    cost: m.unit_cost,
                });
                flows.push(Flow {
                    date: day,
                    doc: m.doc.clone(),
                    kind: "in".into(),
                    reason: "Доставка".into(),
                    product_id: m.product,
                    product_name: pname(m.product),
                    location_id: m.location,
                    quantity: q4(m.qty),
                    value: m2(m.qty * m.unit_cost),
                });
            }
            "move" => {
                let cost = consume(entry, m.location, m.qty, &method);
                entry.push(Lot {
                    location: m.to,
                    qty: m.qty,
                    cost,
                });
                flows.push(Flow {
                    date: day.clone(),
                    doc: m.doc.clone(),
                    kind: "out".into(),
                    reason: "Преместване".into(),
                    product_id: m.product,
                    product_name: pname(m.product),
                    location_id: m.location,
                    quantity: q4(m.qty),
                    value: m2(cost),
                });
                flows.push(Flow {
                    date: day,
                    doc: m.doc.clone(),
                    kind: "in".into(),
                    reason: "Преместване".into(),
                    product_id: m.product,
                    product_name: pname(m.product),
                    location_id: m.to,
                    quantity: q4(m.qty),
                    value: m2(cost),
                });
            }
            "issue" | "sale" => {
                let cost = consume(entry, m.location, m.qty, &method);
                if cost <= 0.0 && m.qty > 0.0 {
                    warnings.push(format!(
                        "няма наличност за артикул {} на {}",
                        m.product, m.date
                    ));
                }
                flows.push(Flow {
                    date: day,
                    doc: m.doc.clone(),
                    kind: "out".into(),
                    reason: if m.kind == "issue" {
                        "Изписване".into()
                    } else {
                        "Продажба".into()
                    },
                    product_id: m.product,
                    product_name: pname(m.product),
                    location_id: m.location,
                    quantity: q4(m.qty),
                    value: m2(cost),
                });
            }
            _ => {}
        }
    }

    let mut balances = Vec::new();
    let mut keys: Vec<(i64, i64)> = Vec::new();
    let mut agg: std::collections::HashMap<(i64, i64), (f64, f64)> = Default::default();
    for (product, ls) in &lots {
        for l in ls {
            if l.qty <= 0.0 {
                continue;
            }
            let key = (*product, l.location);
            if !agg.contains_key(&key) {
                keys.push(key);
            }
            let e = agg.entry(key).or_insert((0.0, 0.0));
            e.0 += l.qty;
            e.1 += l.qty * l.cost;
        }
    }
    keys.sort();
    for (product, location) in keys {
        let (qty, val) = agg[&(product, location)];
        balances.push(Balance {
            product_id: product,
            product_name: pname(product),
            location_id: location,
            quantity: q4(qty),
            value: m2(val),
        });
    }
    Ok(Explain {
        flows,
        balances,
        warnings,
    })
}
