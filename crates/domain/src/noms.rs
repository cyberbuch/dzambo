//! Каталогът на НАП — сметкоплан (360 сметки). Данните са от ytbuch
//! (BSD-3-Clause), data/saft_noms.json.

use std::sync::OnceLock;

pub struct NapAccount {
    pub code: String,
    pub name: String,
}

fn accounts() -> &'static Vec<NapAccount> {
    static ACCOUNTS: OnceLock<Vec<NapAccount>> = OnceLock::new();
    ACCOUNTS.get_or_init(|| {
        let raw = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/saft_noms.json"));
        let parsed: serde_json::Value =
            serde_json::from_str(raw).expect("saft_noms.json е валиден JSON");
        parsed["accounts"]
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
