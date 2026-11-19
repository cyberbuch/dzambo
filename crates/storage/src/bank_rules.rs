//! Правила за банкови транзакции: ключови думи → сметка (пренесено от ytbuch).

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{accounts, schema::bank_matching_rules, StorageError};

const CYR: [(&str, &str); 33] = [
    ("а","a"),("б","b"),("в","v"),("г","g"),("д","d"),("е","e"),("ж","zh"),("з","z"),("и","i"),("й","y"),("к","k"),("л","l"),("м","m"),("н","n"),("о","o"),("п","p"),("р","r"),("с","s"),("т","t"),("у","u"),("ф","f"),("х","h"),("ц","ts"),("ч","ch"),("ш","sh"),("щ","sht"),("ъ","a"),("ь",""),("ю","yu"),("я","ya"),("э","e"),("ы","y"),("ё","e"),
];

pub fn fold(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    for ch in lower.chars() {
        let c = ch.to_string();
        if let Some((_, lat)) = CYR.iter().find(|(cyr, _)| *cyr == c) {
            out.push_str(lat);
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn contains_keyword(haystack: &str, keyword: &str) -> bool {
    let needle = fold(keyword);
    !needle.is_empty() && fold(haystack).contains(&needle)
}

pub fn matches_keywords(text: &str, keywords: &str) -> bool {
    keywords.split(',').any(|raw| {
        let kw = raw.trim();
        !kw.is_empty() && contains_keyword(text, kw)
    })
}

pub fn direction_matches(direction: &str, amount: &str) -> bool {
    let v: f64 = amount.parse().unwrap_or(0.0);
    match direction { "debit" => v < 0.0, "credit" => v > 0.0, _ => true }
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = bank_matching_rules)]
pub struct Rule {
    pub id: i64,
    pub company_id: i64,
    pub name: String,
    pub keywords: String,
    pub direction: String,
    pub priority: i32,
    pub is_active: bool,
    pub account_id: i64,
    pub counterpart_id: i64,
}

pub async fn list(conn: &mut AsyncPgConnection, company_id: i64) -> Result<Vec<Rule>, StorageError> {
    bank_matching_rules::table.filter(bank_matching_rules::company_id.eq(company_id))
        .order((bank_matching_rules::priority.desc(), bank_matching_rules::id.asc()))
        .select(Rule::as_select()).load(conn).await.map_err(StorageError::from)
}

/// Първото активно правило, което съвпада по посока и ключови думи (най-висок приоритет).
pub fn find_rule<'a>(rules: &'a [Rule], text: &str, amount: &str) -> Option<&'a Rule> {
    rules.iter()
        .filter(|r| r.is_active && direction_matches(&r.direction, amount) && matches_keywords(text, &r.keywords))
        .max_by_key(|r| r.priority)
}

#[allow(clippy::too_many_arguments)]
pub async fn create(conn: &mut AsyncPgConnection, company_id: i64, name: &str, keywords: &str, direction: &str, priority: i32, is_active: bool, account_id: i64, counterpart_id: i64) -> Result<Rule, StorageError> {
    diesel::insert_into(bank_matching_rules::table)
        .values((
            bank_matching_rules::company_id.eq(company_id),
            bank_matching_rules::name.eq(name),
            bank_matching_rules::keywords.eq(keywords),
            bank_matching_rules::direction.eq(direction),
            bank_matching_rules::priority.eq(priority),
            bank_matching_rules::is_active.eq(is_active),
            bank_matching_rules::account_id.eq(account_id),
            bank_matching_rules::counterpart_id.eq(counterpart_id),
        )).returning(Rule::as_returning()).get_result(conn).await.map_err(StorageError::from)
}

pub async fn delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(bank_matching_rules::table.find(id)).execute(conn).await?;
    Ok(())
}

const DEFAULTS: &[(&str, &str, &str, &[&str], i32)] = &[
    ("POS покупки", "POS,ПОС,pos трансакция,покупка на пос,pos purchase,card purchase,покупка с карта,покупка с дк", "debit", &["447","446","50902"], 100),
    ("ATM теглене", "ATM,АТМ,atm трансакция,теглене,withdrawal,cash withdrawal,теглене на каса,теглене на пари", "debit", &["501"], 90),
    ("Банкови такси", "автоматична такса,такса теглене,такса за поддръжка,комисионна,bank fee,commission,месечна такса,мес. такса,monthly fee", "debit", &["629"], 80),
    ("Получени преводи от клиенти", "получен директен превод,получен превод,sct instant получен,incoming credit,credit transfer received", "credit", &["411"], 70),
    ("Издадени преводи към доставчици", "издаден превод,sct instant издаден,изходящ превод,outgoing credit,issued transfer", "debit", &["401"], 60),
    ("Аванси", "аванс,avans,advance", "debit", &["402"], 50),
];

pub async fn seed_defaults(conn: &mut AsyncPgConnection, company_id: i64) -> Result<i64, StorageError> {
    let existing = list(conn, company_id).await?;
    let mut n = 0i64;
    for (name, keywords, direction, codes, priority) in DEFAULTS {
        if existing.iter().any(|r| r.name == *name) { continue; }
        let mut account_id = 0i64;
        if let Some(code) = codes.first() {
            account_id = accounts::get_or_create(conn, company_id, code).await?.id;
        }
        create(conn, company_id, name, keywords, direction, *priority, true, account_id, 0).await?;
        n += 1;
    }
    Ok(n)
}
