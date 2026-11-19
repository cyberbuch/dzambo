//! Сонда на MT940 и camt.053 парсерите.

use storage::bank_formats;

const MT940: &str = ":25:BG80BNBG96611020345678\n:60F:C261101EUR1000,00\n:61:2611011101C1200,00NTRFREF43\n:86:Плащане фактура 43\n:61:2611051105D300,00NTRFREF42\n:86:Плащане доставчик\n";

const CAMT: &str = r#"<?xml version="1.0"?>
<Document><BkToCstmrStmt><Stmt>
<Acct><Id><IBAN>BG80BNBG96611020345678</IBAN></Id><Ccy>EUR</Ccy></Acct>
<Ntry><Amt Ccy="EUR">1200.00</Amt><CdtDbtInd>CRDT</CdtDbtInd><BookgDt><Dt>2026-11-01</Dt></BookgDt><AcctSvcrRef>REF43</AcctSvcrRef><NtryDtls><TxDtls><RmtInf><Ustrd>Плащане фактура 43</Ustrd></RmtInf><RltdPties><Dbtr><Nm>Демо Клиент ООД</Nm></Dbtr></RltdPties></TxDtls></NtryDtls></Ntry>
<Ntry><Amt Ccy="EUR">300.00</Amt><CdtDbtInd>DBIT</CdtDbtInd><BookgDt><Dt>2026-11-05</Dt></BookgDt><AcctSvcrRef>REF42</AcctSvcrRef></Ntry>
</Stmt></BkToCstmrStmt></Document>"#;

#[test]
fn mt940_parses() {
    let p = bank_formats::parse(MT940).expect("mt940");
    assert_eq!(p.format_name, "MT940");
    assert_eq!(p.account_iban, "BG80BNBG96611020345678");
    assert_eq!(p.txs.len(), 2);
    assert_eq!(p.txs[0].amount, "1200.00");
    assert_eq!(p.txs[0].date, "2026-11-01");
    assert_eq!(p.txs[1].amount, "-300.00");
    assert!(p.txs[0].description.contains("фактура 43"));
}

#[test]
fn camt053_parses() {
    let p = bank_formats::parse(CAMT).expect("camt053");
    assert_eq!(p.format_name, "ISO camt.053");
    assert_eq!(p.txs.len(), 2);
    assert_eq!(p.txs[0].amount, "1200.00");
    assert_eq!(p.txs[0].contra_name, "Демо Клиент ООД");
    assert_eq!(p.txs[1].amount, "-300.00");
    assert_eq!(p.txs[1].reference, "REF42");
}
