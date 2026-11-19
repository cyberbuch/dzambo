//! Модели на сондата. Фаза 1 заменя с истинската схема.

use std::str::FromStr;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable, Debug, PartialEq)]
#[diesel(table_name = crate::schema::spike_entries)]
pub struct SpikeEntry {
    pub id: i32,
    pub amount: BigDecimal,
    pub note: String,
    pub created_at: DateTime<Utc>,
}

impl SpikeEntry {
    /// Сумата тръгва към API-то като низ — договорът от ytbuch.
    pub fn json_amount(&self) -> String {
        self.amount.to_string()
    }
}

pub fn parse_money(raw: &str) -> Result<BigDecimal, <BigDecimal as FromStr>::Err> {
    raw.parse::<BigDecimal>()
}
