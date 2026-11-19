//! Изчисление на финансовите отчети по формули. Порт на ytbuch reports_fs::build.
//! Празен низ е нула, освен сборът на група; предходната година още е нула.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::fsform::{parse, Piece};
use crate::fsnom::{self, Line};
use crate::reports_data::{
    self, acc_match, by_number, correspondences, money, turns, Books, Built,
};

fn amount_of(which: &str, num: &str, maps: &[HashMap<String, i128>; 4]) -> i128 {
    let g = |i: usize| maps[i].get(num).copied().unwrap_or(0);
    match which {
        "D" => g(0).saturating_add(g(2)),
        "C" => g(1).saturating_add(g(3)),
        "TD" => g(2),
        "TC" => g(3),
        "OD" => g(0),
        "OC" => g(1),
        _ => 0,
    }
}

fn sum_accounts(pat: &str, which: &str, books: &Books, maps: &[HashMap<String, i128>; 4]) -> i128 {
    let mut seen = HashSet::new();
    let mut total = 0i128;
    for acc in &books.accounts {
        if !seen.insert(acc.number.clone()) {
            continue;
        }
        if acc_match(&acc.number, pat) {
            total = total.saturating_add(amount_of(which, &acc.number, maps));
        }
    }
    total
}

fn sum_corr(debit: &str, credit: &str, pairs: &HashMap<(String, String), i128>) -> i128 {
    let mut total = 0i128;
    for ((d, c), amount) in pairs {
        if acc_match(d, debit) && acc_match(c, credit) {
            total = total.saturating_add(*amount);
        }
    }
    total
}

fn child_sum(
    cat: &[Line],
    parent: &str,
    self_code: &str,
    done: &HashSet<String>,
    val: &HashMap<String, i128>,
) -> Option<i128> {
    let mut total = 0i128;
    for line in cat {
        if line.parent != parent || line.code == self_code {
            continue;
        }
        if line.kind != "line" && line.kind != "total" {
            continue;
        }
        if !done.contains(&line.code) {
            return None;
        }
        total = total.saturating_add(val.get(&line.code).copied().unwrap_or(0));
    }
    Some(total)
}

pub fn build(kind: &str, books: &Books, formulas: &HashMap<String, String>) -> Built {
    let statement = reports_data::fs_statement(kind).unwrap_or("");
    let cat = fsnom::lines(statement);
    let turn = turns(books);
    let maps = by_number(books, &turn);
    let pairs = correspondences(books);
    let mut done = HashSet::new();
    let mut val: HashMap<String, i128> = HashMap::new();
    for _ in 0..cat.len().saturating_add(4) {
        let mut progress = false;
        for line in &cat {
            if done.contains(&line.code) {
                continue;
            }
            let formula = formulas.get(&line.code).map(String::as_str).unwrap_or("");
            if formula.is_empty() {
                if line.kind == "total" {
                    if let Some(sum) = child_sum(&cat, &line.parent, &line.code, &done, &val) {
                        val.insert(line.code.clone(), sum);
                        done.insert(line.code.clone());
                        progress = true;
                    }
                } else {
                    val.insert(line.code.clone(), 0);
                    done.insert(line.code.clone());
                    progress = true;
                }
                continue;
            }
            let toks = match parse(formula) {
                Ok(t) => t,
                Err(_) => {
                    val.insert(line.code.clone(), 0);
                    done.insert(line.code.clone());
                    progress = true;
                    continue;
                }
            };
            let waiting = toks
                .iter()
                .any(|t| matches!(&t.piece, Piece::At(code) if !done.contains(code)));
            if waiting {
                continue;
            }
            let mut acc = 0i128;
            for tok in &toks {
                let piece = match &tok.piece {
                    Piece::At(code) => val.get(code).copied().unwrap_or(0),
                    Piece::Acct { pat, which } => sum_accounts(pat, which, books, &maps),
                    Piece::Corr { debit, credit } => sum_corr(debit, credit, &pairs),
                };
                acc = if tok.neg {
                    acc.saturating_sub(piece)
                } else {
                    acc.saturating_add(piece)
                };
            }
            val.insert(line.code.clone(), acc);
            done.insert(line.code.clone());
            progress = true;
        }
        if !progress {
            break;
        }
    }
    let mut rows = Vec::new();
    let mut grid = vec![
        vec![reports_data::title(kind).to_string()],
        vec![format!("{} — {}", books.from, books.to)],
        vec!["Статия".into(), "Текуща".into(), "Предходна".into()],
    ];
    for line in &cat {
        let amt = val.get(&line.code).copied().unwrap_or(0);
        rows.push(json!({
            "code": line.code, "parent": line.parent, "kind": line.kind, "side": line.side,
            "name": line.name, "current": money(amt), "prior": money(0),
        }));
        if line.kind == "header" {
            grid.push(vec![line.name.clone(), String::new(), String::new()]);
        } else {
            grid.push(vec![line.name.clone(), money(amt), money(0)]);
        }
    }
    let mut body = reports_data::head(books, kind);
    body["statement"] = json!(statement);
    body["rows"] = Value::Array(rows);
    Built {
        body,
        slug: reports_data::slug(kind).into(),
        grid,
    }
}
