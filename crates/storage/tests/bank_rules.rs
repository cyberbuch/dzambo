//! Сонда на нормализация и съвпадение на правила.

use storage::bank_rules::{direction_matches, fold, matches_keywords};

#[test]
fn fold_transliterates() {
    assert!(fold("ПОС покупка").contains("pos"));
    assert!(fold("Теглене на каса").contains("teg"));
    assert_eq!(fold("  Банка   Такса  "), "banka taksa");
}

#[test]
fn keywords_match_cyr_and_lat() {
    assert!(matches_keywords("POS покупка с карта", "POS,ПОС"));
    assert!(matches_keywords(
        "месечна такса поддръжка",
        "bank fee,месечна такса"
    ));
    assert!(!matches_keywords("получен превод", "теглене,atm"));
}

#[test]
fn direction_by_sign() {
    assert!(direction_matches("debit", "-300.00"));
    assert!(direction_matches("credit", "1200.00"));
    assert!(direction_matches("any", "0.00"));
    assert!(!direction_matches("credit", "-5.00"));
}
