//! Домейн на dzambo: auth (argon2/JWT), DTO-та (serde + ts-rs),
//! номенклатури на НАП, счетоводни правила и печатни модели (фази 3+).

pub mod auth;
pub mod cn8;
pub mod dto;
pub mod fsform;
pub mod fsnom;
pub mod noms;
pub mod reports_book;
pub mod reports_corr;
pub mod reports_data;
pub mod reports_fs;
pub mod reports_sheet;
pub mod reports_trial;
pub mod smap_def;
pub mod totp;

#[cfg(test)]
mod tests {
    #[test]
    fn harness() {
        // Тестовият ранър задейства ts-rs export на DTO-тата.
    }
}

pub mod vat_ex;
