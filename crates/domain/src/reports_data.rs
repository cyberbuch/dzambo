//! Общи данни за справките: дневник, салда и обороти (сума в стотинки i128).
//! Крайното салдо е начално плюс оборот, без нетиране. Порт на ytbuch reports_data.

use std::collections::HashMap;

use serde_json::{json, Value};

#[derive(Clone)]
pub struct Acc {
    pub id: i64,
    pub number: String,
    pub name: String,
}

#[derive(Clone)]
pub struct Entry {
    pub id: i64,
    pub day: String,
    pub doc: String,
    pub desc: String,
    pub cp_id: i64,
    pub cp_name: String,
}

#[derive(Clone)]
pub struct Line {
    pub entry_id: i64,
    pub account_id: i64,
    pub debit: bool,
    pub amount: i128,
}

#[derive(Clone)]
pub struct Open {
    pub account_id: i64,
    pub cp_id: i64,
    pub debit: i128,
    pub credit: i128,
}

pub struct Books {
    pub company_id: i64,
    pub accounts: Vec<Acc>,
    pub entries: Vec<Entry>,
    pub lines: Vec<Line>,
    pub openings: Vec<Open>,
    pub from: String,
    pub to: String,
    pub fiscal: String,
    pub account: String,
    pub cpid: i64,
    pub firm_name: String,
    pub firm_eik: String,
}

pub struct Turn {
    pub od: HashMap<i64, i128>,
    pub oc: HashMap<i64, i128>,
    pub td: HashMap<i64, i128>,
    pub tc: HashMap<i64, i128>,
}

pub struct Built {
    pub body: Value,
    pub slug: String,
    pub grid: Vec<Vec<String>>,
}

/// Сума в стотинки → низ с 2 знака.
pub fn money(v: i128) -> String {
    let neg = v < 0;
    let a = v.abs();
    format!("{}{}.{:02}", if neg { "-" } else { "" }, a / 100, a % 100)
}

pub fn day_ok(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

pub fn in_range(day: &str, from: &str, to: &str, exclusive_end: bool) -> bool {
    if day.len() < 10 || day < from {
        return false;
    }
    if exclusive_end {
        day < to
    } else {
        day <= to
    }
}

pub fn starts(number: &str, prefix: &str) -> bool {
    prefix.is_empty() || number.starts_with(prefix)
}

pub fn acc_match(account: &str, pattern: &str) -> bool {
    let prefix = pattern.strip_suffix('*').unwrap_or(pattern);
    !prefix.is_empty() && account.starts_with(prefix)
}

fn add(map: &mut HashMap<i64, i128>, key: i64, amount: i128) {
    if amount == 0 {
        return;
    }
    let slot = map.entry(key).or_insert(0i128);
    *slot = slot.saturating_add(amount);
}

pub fn get(map: &HashMap<i64, i128>, key: i64) -> i128 {
    map.get(&key).copied().unwrap_or(0)
}

pub fn turns(b: &Books) -> Turn {
    let mut t = Turn {
        od: HashMap::new(),
        oc: HashMap::new(),
        td: HashMap::new(),
        tc: HashMap::new(),
    };
    for row in &b.openings {
        if b.cpid > 0 && row.cp_id != b.cpid {
            continue;
        }
        add(&mut t.od, row.account_id, row.debit);
        add(&mut t.oc, row.account_id, row.credit);
    }
    let days: HashMap<i64, &str> = b.entries.iter().map(|e| (e.id, e.day.as_str())).collect();
    let cps: HashMap<i64, i64> = b.entries.iter().map(|e| (e.id, e.cp_id)).collect();
    let from_is_fiscal = b.from == b.fiscal;
    for line in &b.lines {
        if b.cpid > 0 && cps.get(&line.entry_id).copied().unwrap_or(0) != b.cpid {
            continue;
        }
        let day = days.get(&line.entry_id).copied().unwrap_or("");
        if !from_is_fiscal && in_range(day, &b.fiscal, &b.from, true) {
            if line.debit {
                add(&mut t.od, line.account_id, line.amount);
            } else {
                add(&mut t.oc, line.account_id, line.amount);
            }
        }
        if in_range(day, &b.from, &b.to, false) {
            if line.debit {
                add(&mut t.td, line.account_id, line.amount);
            } else {
                add(&mut t.tc, line.account_id, line.amount);
            }
        }
    }
    t
}

pub fn numbers(b: &Books) -> HashMap<i64, String> {
    b.accounts
        .iter()
        .map(|a| (a.id, a.number.clone()))
        .collect()
}

pub fn names(b: &Books) -> HashMap<String, String> {
    b.accounts
        .iter()
        .map(|a| (a.number.clone(), a.name.clone()))
        .collect()
}

pub fn by_number(b: &Books, t: &Turn) -> [HashMap<String, i128>; 4] {
    let nums = numbers(b);
    let mut out = [
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    ];
    let src = [&t.od, &t.oc, &t.td, &t.tc];
    for (i, map) in src.iter().enumerate() {
        for (id, amount) in *map {
            let Some(num) = nums.get(id) else { continue };
            let slot = out[i].entry(num.clone()).or_insert(0i128);
            *slot = slot.saturating_add(*amount);
        }
    }
    out
}

pub struct Side {
    pub num: String,
    pub rem: i128,
}

pub fn pair(debits: &mut [Side], credits: &mut [Side]) -> Vec<(String, String, i128)> {
    let mut out = Vec::new();
    let (mut di, mut ci) = (0, 0);
    while di < debits.len() && ci < credits.len() {
        if debits[di].rem == 0 {
            di += 1;
            continue;
        }
        if credits[ci].rem == 0 {
            ci += 1;
            continue;
        }
        let amount = debits[di].rem.min(credits[ci].rem);
        out.push((debits[di].num.clone(), credits[ci].num.clone(), amount));
        debits[di].rem -= amount;
        credits[ci].rem -= amount;
    }
    out
}

pub fn correspondences(b: &Books) -> HashMap<(String, String), i128> {
    let nums = numbers(b);
    let mut by_entry: HashMap<i64, Vec<&Line>> = HashMap::new();
    for line in &b.lines {
        by_entry.entry(line.entry_id).or_default().push(line);
    }
    let mut pairs: HashMap<(String, String), i128> = HashMap::new();
    for entry in &b.entries {
        if !in_range(&entry.day, &b.from, &b.to, false) {
            continue;
        }
        let Some(lines) = by_entry.get(&entry.id) else {
            continue;
        };
        let mut debits = Vec::new();
        let mut credits = Vec::new();
        for line in lines {
            let num = nums.get(&line.account_id).cloned().unwrap_or_default();
            let side = Side {
                num,
                rem: line.amount,
            };
            if line.debit {
                debits.push(side);
            } else {
                credits.push(side);
            }
        }
        for (d, c, amount) in pair(&mut debits, &mut credits) {
            if amount == 0 {
                continue;
            }
            let slot = pairs.entry((d, c)).or_insert(0i128);
            *slot = slot.saturating_add(amount);
        }
    }
    pairs
}

pub fn title(kind: &str) -> &'static str {
    match kind {
        "trial_balance" => "Оборотна ведомост",
        "counterpart_trial" => "Оборотна ведомост по контрагент",
        "general_ledger" => "Главна книга",
        "chronological" => "Хронологичен журнал",
        "counterpart_chrono" => "Хронологичен журнал по контрагент",
        "correspondence_ledger" => "Главна книга по кореспонденции",
        "balance_sheet" => "Счетоводен баланс",
        "income_statement" => "Отчет за приходите и разходите",
        "cash_flow" => "Отчет за паричните потоци",
        "equity" => "Отчет за собствения капитал",
        _ => "Справка",
    }
}

pub fn slug(kind: &str) -> &'static str {
    match kind {
        "trial_balance" => "oborotna",
        "counterpart_trial" => "oborotna-kontragent",
        "general_ledger" => "glavna-kniga",
        "chronological" => "hronologichen",
        "counterpart_chrono" => "hronologichen-kontragent",
        "correspondence_ledger" => "glavna-kniga-korespondencii",
        "balance_sheet" => "balans",
        "income_statement" => "opr",
        "cash_flow" => "parichen-potok",
        "equity" => "sobstven-kapital",
        _ => "spravka",
    }
}

pub fn kind_ok(kind: &str) -> bool {
    matches!(
        kind,
        "trial_balance"
            | "counterpart_trial"
            | "general_ledger"
            | "chronological"
            | "counterpart_chrono"
            | "correspondence_ledger"
            | "balance_sheet"
            | "income_statement"
            | "cash_flow"
            | "equity"
    )
}

pub fn fs_statement(kind: &str) -> Option<&'static str> {
    match kind {
        "balance_sheet" => Some("balance"),
        "income_statement" => Some("pl"),
        "cash_flow" => Some("cashflow"),
        "equity" => Some("equity"),
        _ => None,
    }
}

pub fn head(b: &Books, kind: &str) -> Value {
    json!({"kind": kind, "from": b.from, "to": b.to, "title": title(kind), "company_name": b.firm_name, "company_eik": b.firm_eik})
}

pub fn firm_line(name: &str, eik: &str) -> String {
    match (name.is_empty(), eik.is_empty()) {
        (true, true) => String::new(),
        (false, true) => name.to_string(),
        (true, false) => format!("ЕИК {eik}"),
        (false, false) => format!("{name} — ЕИК {eik}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_format() {
        assert_eq!(money(123456), "1234.56");
        assert_eq!(money(-500), "-5.00");
        assert_eq!(money(0), "0.00");
    }

    #[test]
    fn range_and_prefix() {
        assert!(day_ok("2026-10-01"));
        assert!(!day_ok("2026-10-1"));
        assert!(in_range("2026-01-15", "2026-01-01", "2026-10-01", true));
        assert!(!in_range("2026-10-01", "2026-01-01", "2026-10-01", true));
        assert!(acc_match("4111", "411"));
        assert!(acc_match("501", "50*"));
        assert!(!acc_match("601", "50*"));
    }
}
