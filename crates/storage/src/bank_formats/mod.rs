//! Банкови формати: разпознаване и парсване (MT940, camt.053).

pub mod camt053;
pub mod common;
pub mod mt940;

#[derive(Debug, Clone)]
pub struct Tx {
    pub date: String,
    pub amount: String,
    pub currency: String,
    pub description: String,
    pub contra_name: String,
    pub contra_iban: String,
    pub reference: String,
}

#[derive(Debug, Clone)]
pub struct Parsed {
    pub format_name: String,
    pub account_iban: String,
    pub account_currency: String,
    pub txs: Vec<Tx>,
}

/// Разпознава формата по съдържанието. CSV се обработва отделно.
pub fn detect(content: &str) -> Option<&'static str> {
    if content.contains(":61:") || content.contains(":25:") { return Some("mt940"); }
    if content.contains("<Ntry") || content.contains("camt.053") { return Some("camt053"); }
    None
}

pub fn parse(content: &str) -> Option<Parsed> {
    match detect(content)? {
        "mt940" => Some(mt940::parse(content)),
        "camt053" => Some(camt053::parse(content)),
        _ => None,
    }
}
