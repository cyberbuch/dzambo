//! Каталогът на НАП — сметкоплан (360 сметки) и мерни единици (127).
//! Данните са от ytbuch (BSD-3-Clause), data/saft_noms.json.

use std::sync::OnceLock;

fn catalog() -> &'static serde_json::Value {
    static CATALOG: OnceLock<serde_json::Value> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let raw = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/saft_noms.json"));
        serde_json::from_str(raw).expect("saft_noms.json е валиден JSON")
    })
}

pub struct NapAccount {
    pub code: String,
    pub name: String,
}

fn accounts() -> &'static Vec<NapAccount> {
    static ACCOUNTS: OnceLock<Vec<NapAccount>> = OnceLock::new();
    ACCOUNTS.get_or_init(|| {
        catalog()["accounts"]
            .as_array()
            .expect("accounts е масив")
            .iter()
            .map(|entry| {
                let parts: Vec<&str> = entry.as_str().unwrap_or_default().split('|').collect();
                NapAccount {
                    code: parts.first().unwrap_or(&"").to_string(),
                    name: parts.get(1).unwrap_or(&"").to_string(),
                }
            })
            .collect()
    })
}

pub fn chart() -> impl Iterator<Item = (&'static str, &'static str)> {
    accounts()
        .iter()
        .map(|a| (a.code.as_str(), a.name.as_str()))
}

/// Мерни единици: {code, name, name_en, symbol, category, source}.
#[derive(Clone)]
pub struct NapUnit {
    pub code: String,
    pub name: String,
    pub name_en: String,
    pub symbol: String,
    pub category: String,
    pub source: String,
}

fn all_units() -> &'static Vec<NapUnit> {
    static UNITS: OnceLock<Vec<NapUnit>> = OnceLock::new();
    UNITS.get_or_init(|| {
        catalog()["units"]
            .as_array()
            .expect("units е масив")
            .iter()
            .map(|entry| {
                let p: Vec<&str> = entry.as_str().unwrap_or_default().split('|').collect();
                let get = |i: usize| p.get(i).unwrap_or(&"").to_string();
                NapUnit {
                    code: get(0),
                    name: get(1),
                    name_en: get(2),
                    symbol: get(3),
                    category: get(4),
                    source: get(5),
                }
            })
            .collect()
    })
}

pub fn units(search: Option<&str>) -> Vec<NapUnit> {
    let all = all_units();
    let Some(q) = search.map(str::trim).filter(|q| !q.is_empty()) else {
        return all.clone();
    };
    let q = q.to_lowercase();
    all.iter()
        .filter(|u| {
            u.code.to_lowercase().contains(&q)
                || u.name.to_lowercase().contains(&q)
                || u.name_en.to_lowercase().contains(&q)
        })
        .cloned()
        .collect()
}

/// Всички видове номенклатури от каталога на НАП.
pub const KINDS: [&str; 11] = [
    "stock_movements",
    "asset_movements",
    "tax_regimes",
    "payment_methods",
    "invoice_types",
    "tax_types",
    "tax_codes",
    "product_types",
    "units",
    "regions",
    "accounts",
];

/// Сурови записи (pipe-разделени) за даден вид.
pub fn entries(kind: &str) -> &'static Vec<String> {
    static CACHE: OnceLock<std::collections::HashMap<&'static str, Vec<String>>> = OnceLock::new();
    let map = CACHE.get_or_init(|| {
        let mut m = std::collections::HashMap::new();
        for k in KINDS {
            m.insert(
                k,
                catalog()[k]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
            );
        }
        m
    });
    map.get(kind)
        .unwrap_or_else(|| panic!("непознат вид номенклатура: {kind}"))
}

/// Мапинг на сметка към НОМЕНКЛАТУРА (НАП) код: accounts.kind.
pub fn nap_account_code(number: &str) -> Option<String> {
    accounts()
        .iter()
        .find(|a| a.code == number)
        .map(|a| a.code.clone())
}
