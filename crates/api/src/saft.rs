//! SAF-T експорт (месечен режим) — BG declaration v1, namespace mf:nra:dgti.

use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Response},
};
use bigdecimal::BigDecimal;
use chrono;

use crate::errors::ApiError;
use storage::{accounts, counterparts, invoices, journal, products, related};

const NS: &str = "mf:nra:dgti:dxxxx:declaration:v1";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn t(tag: &str, v: &str) -> String {
    format!("<{tag}>{}</{tag}>", esc(v))
}
fn tn(tag: &str, v: &str) -> String {
    format!("<{tag}>{v}</{tag}>")
}
fn money(v: &BigDecimal) -> String {
    format!("{v:.2}")
}
fn qty(v: &BigDecimal) -> String {
    format!("{v:.4}")
}

fn month_range(year: i32, month: i32) -> (String, String) {
    let start = format!("{year:04}-{month:02}-01");
    let next = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let end = format!("{:04}-{:02}-01", next.0, next.1);
    (start, end)
}

/// Месечен SAF-T файл за YYYY-MM.
pub async fn monthly_xml(
    conn: &mut storage::DbConn,
    company_id: i64,
    year: i32,
    month: i32,
) -> Result<String, ApiError> {
    use storage::models::invoices::Invoice;
    let (company_row, company_details) = storage::companies::by_id(conn, company_id)
        .await?
        .ok_or_else(|| ApiError::not_found("фирмата не е намерена"))?;
    let c = &company_row;
    let accts = accounts::list(conn, company_id).await?;
    let cps = counterparts::list(conn, company_id).await?;
    let owners = related::owners(conn, company_id).await?;
    let parents = related::parents(conn, company_id).await?;
    let prods = products::list(conn, company_id).await?;

    let (start, end) = month_range(year, month);
    let inv_all = invoices::list(conn, company_id).await?;
    let inv_month: Vec<Invoice> = inv_all
        .into_iter()
        .filter(|i| {
            i.status == "posted"
                && i.issue_date.to_string() >= start
                && i.issue_date.to_string() < end
        })
        .collect();
    let entries = journal::list(conn, company_id).await?;
    let entries_month: Vec<_> = entries
        .into_iter()
        .filter(|e| e.entry_date.to_string() >= start && e.entry_date.to_string() < end)
        .collect();

    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    x.push_str(&format!("<nsSAFT:AuditFile xmlns:nsSAFT=\"{NS}\">\n"));
    // ---------- Header ----------
    x.push_str("\t<nsSAFT:Header>\n");
    x.push_str(&format!("\t\t{}\n", t("nsSAFT:AuditFileVersion", "007")));
    x.push_str(&format!("\t\t{}\n", t("nsSAFT:AuditFileCountry", "BG")));
    x.push_str(&format!("\t\t{}\n", t("nsSAFT:AuditFileRegion", "")));
    x.push_str(&format!(
        "\t\t{}\n",
        t("nsSAFT:AuditFileDateCreated", &today())
    ));
    x.push_str(&format!(
        "\t\t{}\n",
        t("nsSAFT:SoftwareCompanyName", "dzambo")
    ));
    x.push_str(&format!("\t\t{}\n", t("nsSAFT:SoftwareID", "dzambo")));
    x.push_str(&format!(
        "\t\t{}\n",
        t("nsSAFT:SoftwareVersion", env!("CARGO_PKG_VERSION"))
    ));
    x.push_str("\t\t<nsSAFT:Company>\n");
    x.push_str(&format!(
        "\t\t\t{}\n",
        t("nsSAFT:RegistrationNumber", c.eik.trim())
    ));
    x.push_str(&format!("\t\t\t{}\n", t("nsSAFT:Name", &c.name)));
    x.push_str("\t\t\t<nsSAFT:Address>\n");
    for (tag, val) in [
        ("StreetName", c.address.as_str()),
        (
            "Number",
            company_details
                .as_ref()
                .map(|d| d.building_number.as_str())
                .unwrap_or(""),
        ),
        ("AdditionalAddressDetail", ""),
        ("Building", ""),
        ("City", c.city.as_str()),
        ("PostalCode", c.post_code.as_str()),
        ("Region", ""),
        ("Country", "BG"),
        ("AddressType", "StreetAddress"),
    ] {
        x.push_str(&format!("\t\t\t\t{}\n", t(&format!("nsSAFT:{tag}"), val)));
    }
    x.push_str("\t\t\t</nsSAFT:Address>\n");
    x.push_str("\t\t\t<nsSAFT:Contact>\n");
    x.push_str("\t\t\t\t<nsSAFT:ContactPerson>\n");
    x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:Title", "")));
    x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:FirstName", &c.mol)));
    x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:LastName", &c.mol)));
    x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:OtherTitles", "")));
    x.push_str("\t\t\t\t</nsSAFT:ContactPerson>\n");
    x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:Telephone", &c.phone)));
    x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:Email", &c.email)));
    x.push_str("\t\t\t</nsSAFT:Contact>\n");
    x.push_str("\t\t\t<nsSAFT:TaxRegistration>\n");
    x.push_str(&format!(
        "\t\t\t\t{}\n",
        t("nsSAFT:TaxRegistrationNumber", c.eik.trim())
    ));
    x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:TaxType", "100010")));
    x.push_str(&format!(
        "\t\t\t\t{}\n",
        t("nsSAFT:TaxNumber", &c.vat_number)
    ));
    x.push_str(&format!(
        "\t\t\t\t{}\n",
        t("nsSAFT:TaxVerificationDate", &today())
    ));
    x.push_str("\t\t\t</nsSAFT:TaxRegistration>\n");
    x.push_str("\t\t\t<nsSAFT:BankAccount>\n");
    x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:IBANNumber", &c.iban)));
    x.push_str("\t\t\t</nsSAFT:BankAccount>\n");
    x.push_str("\t\t</nsSAFT:Company>\n");
    x.push_str("\t\t<nsSAFT:Ownership>\n");
    x.push_str(&format!("\t\t\t{}\n", t("nsSAFT:IsPartOfGroup", "1")));
    let bo = owners.first();
    let up = parents.first();
    x.push_str(&format!(
        "\t\t\t{}\n",
        t(
            "nsSAFT:BeneficialOwnerNameCyrillicBG",
            &bo.map(|o| format!("{} {}", o.first_name_bg, o.last_name_bg))
                .unwrap_or_default()
        )
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        t(
            "nsSAFT:BeneficialOwnerEGN",
            &bo.map(|o| o.egn.clone()).unwrap_or_default()
        )
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        t("nsSAFT:BeneficialOwnerNameLatinForeign", "")
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        t("nsSAFT:BeneficialCountryForeign", "BG")
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        t("nsSAFT:BeneficialCountryForeignCode", "BG")
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        t(
            "nsSAFT:UltimateOwnerNameCyrillicBG",
            &up.map(|p| p.name_bg.clone()).unwrap_or_default()
        )
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        t(
            "nsSAFT:UltimateOwnerUICBG",
            &up.map(|p| p.uic.clone()).unwrap_or_default()
        )
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        t("nsSAFT:UltimateOwnerNameCyrillicForeign", "")
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        t("nsSAFT:UltimateOwnerNameLatinForeign", "")
    ));
    x.push_str(&format!("\t\t\t{}\n", t("nsSAFT:CountryForeign", "BG")));
    x.push_str("\t\t</nsSAFT:Ownership>\n");
    x.push_str(&format!(
        "\t\t{}\n",
        t("nsSAFT:DefaultCurrencyCode", &c.currency)
    ));
    x.push_str("\t\t<nsSAFT:SelectionCriteria>\n");
    x.push_str(&format!(
        "\t\t\t{}\n",
        t("nsSAFT:TaxReportingJurisdiction", "NRA")
    ));
    x.push_str(&format!("\t\t\t{}\n", t("nsSAFT:CompanyEntity", "")));
    x.push_str(&format!(
        "\t\t\t{}\n",
        tn("nsSAFT:PeriodStart", &month.to_string())
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        tn("nsSAFT:PeriodStartYear", &year.to_string())
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        tn("nsSAFT:PeriodEnd", &month.to_string())
    ));
    x.push_str(&format!(
        "\t\t\t{}\n",
        tn("nsSAFT:PeriodEndYear", &year.to_string())
    ));
    x.push_str(&format!("\t\t\t{}\n", t("nsSAFT:DocumentType", "")));
    x.push_str(&format!("\t\t\t{}\n", t("nsSAFT:OtherCriteria", "")));
    x.push_str("\t\t</nsSAFT:SelectionCriteria>\n");
    x.push_str(&format!("\t\t{}\n", t("nsSAFT:HeaderComment", "M")));
    x.push_str(&format!("\t\t{}\n", t("nsSAFT:TaxAccountingBasis", "A")));
    x.push_str(&format!("\t\t{}\n", t("nsSAFT:TaxEntity", "Company")));
    x.push_str("\t</nsSAFT:Header>\n");

    // ---------- MasterFilesMonthly ----------
    x.push_str("\t<nsSAFT:MasterFilesMonthly>\n");
    x.push_str("\t\t<nsSAFT:GeneralLedgerAccounts>\n");
    for a in &accts {
        x.push_str("\t\t\t<nsSAFT:Account>\n");
        let nap_code = if a.saft_account_type.trim().is_empty() {
            a.number.clone()
        } else {
            a.saft_account_type.clone()
        };
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:AccountID", nap_code.trim())
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:AccountDescription", &a.name)
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:TaxpayerAccountID", &a.number)
        ));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:GroupingCategory", "")));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:GroupingCode", "")));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:AccountType", "Bifunctional")
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:AccountCreationDate", "2020-01-01")
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:OpeningDebitBalance", "0.00")
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:ClosingDebitBalance", "0.00")
        ));
        x.push_str("\t\t\t</nsSAFT:Account>\n");
    }
    x.push_str("\t\t</nsSAFT:GeneralLedgerAccounts>\n");

    let mut customers: Vec<&storage::models::counterparts::Counterpart> = Vec::new();
    let mut suppliers: Vec<&storage::models::counterparts::Counterpart> = Vec::new();
    for (cp, _e) in &cps {
        match cp.counterpart_type.as_str() {
            "customer" => customers.push(cp),
            "supplier" => suppliers.push(cp),
            _ => {
                customers.push(cp);
                suppliers.push(cp);
            }
        }
    }
    x.push_str("\t\t<nsSAFT:Customers>\n");
    for cp in &customers {
        x.push_str("\t\t\t<nsSAFT:Customer>\n");
        x.push_str("\t\t\t\t<nsSAFT:CompanyStructure>\n");
        x.push_str(&format!(
            "\t\t\t\t\t{}\n",
            t("nsSAFT:RegistrationNumber", cp.eik.trim())
        ));
        x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:NameLatin", &cp.name)));
        x.push_str("\t\t\t\t\t<nsSAFT:Address>\n");
        x.push_str(&format!("\t\t\t\t\t\t{}\n", t("nsSAFT:City", &cp.city)));
        x.push_str(&format!(
            "\t\t\t\t\t\t{}\n",
            t("nsSAFT:PostalCode", &cp.post_code)
        ));
        x.push_str(&format!(
            "\t\t\t\t\t\t{}\n",
            t("nsSAFT:Country", &cp.country)
        ));
        x.push_str(&format!(
            "\t\t\t\t\t\t{}\n",
            t("nsSAFT:AddressType", "StreetAddress")
        ));
        x.push_str("\t\t\t\t\t</nsSAFT:Address>\n");
        x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:RelatedParty", "N")));
        x.push_str("\t\t\t\t</nsSAFT:CompanyStructure>\n");
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:CustomerID", cp.eik.trim())
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:SelfBillingIndicator", "N")
        ));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:AccountID", "411")));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:OpeningDebitBalance", "0.00")
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:ClosingDebitBalance", "0.00")
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:ClosingDebitBalance", "0.00")
        ));
        x.push_str("\t\t\t</nsSAFT:Customer>\n");
    }
    x.push_str("\t\t</nsSAFT:Customers>\n");

    x.push_str("\t\t<nsSAFT:Suppliers>\n");
    for cp in &suppliers {
        x.push_str("\t\t\t<nsSAFT:Supplier>\n");
        x.push_str("\t\t\t\t<nsSAFT:CompanyStructure>\n");
        x.push_str(&format!(
            "\t\t\t\t\t{}\n",
            t("nsSAFT:RegistrationNumber", cp.eik.trim())
        ));
        x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:NameLatin", &cp.name)));
        x.push_str("\t\t\t\t\t<nsSAFT:Address>\n");
        x.push_str(&format!("\t\t\t\t\t\t{}\n", t("nsSAFT:City", &cp.city)));
        x.push_str(&format!(
            "\t\t\t\t\t\t{}\n",
            t("nsSAFT:Country", &cp.country)
        ));
        x.push_str(&format!(
            "\t\t\t\t\t\t{}\n",
            t("nsSAFT:AddressType", "StreetAddress")
        ));
        x.push_str("\t\t\t\t\t</nsSAFT:Address>\n");
        x.push_str("\t\t\t\t</nsSAFT:CompanyStructure>\n");
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:SupplierID", cp.eik.trim())
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:SelfBillingIndicator", "N")
        ));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:AccountID", "401")));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:OpeningCreditBalance", "0.00")
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:ClosingCreditBalance", "0.00")
        ));
        x.push_str("\t\t\t</nsSAFT:Supplier>\n");
    }
    x.push_str("\t\t</nsSAFT:Suppliers>\n");

    // Статична данъчна таблица (ДДС 20/9/0 + освобени).
    x.push_str("\t\t<nsSAFT:TaxTable><nsSAFT:TaxTableEntry>\n");
    x.push_str(&format!("\t\t\t{}\n", t("nsSAFT:TaxType", "100010")));
    x.push_str(&format!("\t\t\t{}\n", t("nsSAFT:Description", "ДДС")));
    for (code, desc, pct) in [
        ("110010", "Стандартна ставка на ДДС", "20"),
        ("110020", "Намалена ставка на ДДС", "9"),
        ("110030", "Нулева ставка на ДДС", "0"),
        ("110040", "Освободени доставки и придобивания", "0"),
    ] {
        x.push_str("\t\t\t<nsSAFT:TaxCodeDetails>\n");
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:TaxCode", code)));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:EffectiveDate", "2020-01-01")
        ));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:ExpirationDate", "2100-01-01")
        ));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:Description", desc)));
        x.push_str(&format!("\t\t\t\t{}\n", tn("nsSAFT:TaxPercentage", pct)));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:Country", "BG")));
        x.push_str("\t\t\t</nsSAFT:TaxCodeDetails>\n");
    }
    x.push_str("\t\t</nsSAFT:TaxTableEntry></nsSAFT:TaxTable>\n");

    x.push_str("\t\t<nsSAFT:UOMTable>\n");
    let mut units: Vec<String> = vec!["C62".to_string()];
    for (p, _) in &prods {
        if !units.contains(&p.unit) {
            units.push(p.unit.clone());
        }
    }
    for u in units {
        x.push_str("\t\t\t<nsSAFT:UOMTableEntry>\n");
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:UnitOfMeasure", &u)));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:Description", &u)));
        x.push_str("\t\t\t</nsSAFT:UOMTableEntry>\n");
    }
    x.push_str("\t\t</nsSAFT:UOMTable>\n");

    x.push_str("\t\t<nsSAFT:Products>\n");
    for (p, e) in &prods {
        x.push_str("\t\t\t<nsSAFT:Product>\n");
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:ProductCode", &p.code)));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:GoodsServicesID", "01")));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:ProductGroup", &e.product_type)
        ));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:Description", &p.name)));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:ProductCommodityCode", &e.cn8_code)
        ));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:ProductNumberCode", "")));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:ValuationMethod", "Средна")
        ));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:UOMBase", &p.unit)));
        x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:UOMStandard", &p.unit)));
        x.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:UOMToUOMBaseConversionFactor", "1")
        ));
        x.push_str("\t\t\t\t<nsSAFT:Tax>\n");
        x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:TaxType", "100010")));
        x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:TaxCode", "110010")));
        x.push_str("\t\t\t\t</nsSAFT:Tax>\n");
        x.push_str("\t\t\t</nsSAFT:Product>\n");
    }
    x.push_str("\t\t</nsSAFT:Products>\n");
    x.push_str("\t</nsSAFT:MasterFilesMonthly>\n");

    // ---------- GeneralLedgerEntries ----------
    let mut tot_debit = BigDecimal::default();
    let mut tot_credit = BigDecimal::default();
    let mut body = String::new();
    for e in &entries_month {
        let lines = journal::entry_with_lines(conn, e.id)
            .await?
            .map(|(_, l)| l)
            .unwrap_or_default();
        if lines.is_empty() {
            continue;
        }
        body.push_str("\t\t\t<nsSAFT:Transaction>\n");
        body.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:TransactionID", &e.id.to_string())
        ));
        body.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:Period", &month.to_string())
        ));
        body.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:PeriodYear", &year.to_string())
        ));
        body.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:TransactionDate", &e.entry_date.to_string())
        ));
        body.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:TransactionType", "Normal")
        ));
        body.push_str(&format!(
            "\t\t\t\t{}\n",
            t("nsSAFT:Description", &e.description)
        ));
        body.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:SystemEntryDate", &e.entry_date.to_string())
        ));
        body.push_str(&format!(
            "\t\t\t\t{}\n",
            tn("nsSAFT:GLPostingDate", &e.entry_date.to_string())
        ));
        body.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:CustomerID", "0")));
        body.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:SupplierID", "0")));
        body.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:SystemID", "dzambo")));
        for l in &lines {
            if l.direction == "debit" {
                tot_debit += &l.amount;
            } else {
                tot_credit += &l.amount;
            }
            body.push_str("\t\t\t\t<nsSAFT:TransactionLine>\n");
            body.push_str(&format!(
                "\t\t\t\t\t{}\n",
                tn("nsSAFT:RecordID", &l.id.to_string())
            ));
            let nap = accts
                .iter()
                .find(|a| a.id == l.account_id)
                .map(|a| a.saft_account_type.clone())
                .unwrap_or_else(|| l.account_number.clone());
            body.push_str(&format!(
                "\t\t\t\t\t{}\n",
                t("nsSAFT:AccountID", nap.trim())
            ));
            body.push_str(&format!(
                "\t\t\t\t\t{}\n",
                t("nsSAFT:TaxpayerAccountID", &l.account_number)
            ));
            body.push_str(&format!(
                "\t\t\t\t\t{}\n",
                t("nsSAFT:Description", &l.account_name)
            ));
            if l.direction == "debit" {
                body.push_str("\t\t\t\t\t<nsSAFT:DebitAmount>\n");
                body.push_str(&format!(
                    "\t\t\t\t\t\t{}\n",
                    tn("nsSAFT:Amount", &money(&l.amount))
                ));
                body.push_str(&format!(
                    "\t\t\t\t\t\t{}\n",
                    t("nsSAFT:CurrencyCode", "EUR")
                ));
                body.push_str(&format!(
                    "\t\t\t\t\t\t{}\n",
                    tn("nsSAFT:CurrencyAmount", &money(&l.amount))
                ));
                body.push_str("\t\t\t\t\t</nsSAFT:DebitAmount>\n");
            } else {
                body.push_str("\t\t\t\t\t<nsSAFT:CreditAmount>\n");
                body.push_str(&format!(
                    "\t\t\t\t\t\t{}\n",
                    tn("nsSAFT:Amount", &money(&l.amount))
                ));
                body.push_str(&format!(
                    "\t\t\t\t\t\t{}\n",
                    t("nsSAFT:CurrencyCode", "EUR")
                ));
                body.push_str(&format!(
                    "\t\t\t\t\t\t{}\n",
                    tn("nsSAFT:CurrencyAmount", &money(&l.amount))
                ));
                body.push_str("\t\t\t\t\t</nsSAFT:CreditAmount>\n");
            }
            body.push_str("\t\t\t\t</nsSAFT:TransactionLine>\n");
        }
        body.push_str("\t\t\t</nsSAFT:Transaction>\n");
    }
    let n_entries = body.matches("<nsSAFT:Transaction>").count();
    x.push_str("\t<nsSAFT:GeneralLedgerEntries>\n");
    x.push_str(&format!(
        "\t\t{}\n",
        tn("nsSAFT:NumberOfEntries", &n_entries.to_string())
    ));
    x.push_str(&format!(
        "\t\t{}\n",
        tn("nsSAFT:TotalDebit", &money(&tot_debit))
    ));
    x.push_str(&format!(
        "\t\t{}\n",
        tn("nsSAFT:TotalCredit", &money(&tot_credit))
    ));
    x.push_str("\t\t<nsSAFT:Journal>\n");
    x.push_str(&format!("\t\t\t{}\n", t("nsSAFT:JournalID", "GL")));
    x.push_str(&format!(
        "\t\t\t{}\n",
        t("nsSAFT:Description", "Счетоводни записи")
    ));
    x.push_str(&format!("\t\t\t{}\n", t("nsSAFT:Type", "GLEntry")));
    x.push_str(&body);
    x.push_str("\t\t</nsSAFT:Journal>\n\t</nsSAFT:GeneralLedgerEntries>\n");

    // ---------- SourceDocumentsMonthly ----------
    x.push_str("\t<nsSAFT:SourceDocumentsMonthly>\n");
    let sales: Vec<&Invoice> = inv_month.iter().filter(|i| i.direction == "out").collect();
    let purchases: Vec<&Invoice> = inv_month.iter().filter(|i| i.direction == "in").collect();
    for (section, list, counterpart_kind, account_id) in [
        ("SalesInvoices", &sales, "Customer", "411"),
        ("PurchaseInvoices", &purchases, "Supplier", "401"),
    ] {
        let td: BigDecimal = list.iter().map(|i| i.net_amount.clone()).sum();
        let tc: BigDecimal = td.clone();
        x.push_str(&format!("\t\t<nsSAFT:{section}>\n"));
        x.push_str(&format!(
            "\t\t\t{}\n",
            tn("nsSAFT:NumberOfEntries", &list.len().to_string())
        ));
        x.push_str(&format!("\t\t\t{}\n", tn("nsSAFT:TotalDebit", &money(&td))));
        x.push_str(&format!(
            "\t\t\t{}\n",
            tn("nsSAFT:TotalCredit", &money(&tc))
        ));
        for i in list.iter() {
            x.push_str("\t\t\t<nsSAFT:Invoice>\n");
            x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:InvoiceNo", &i.number)));
            x.push_str(&format!("\t\t\t\t<nsSAFT:{}Info>\n", counterpart_kind));
            x.push_str(&format!(
                "\t\t\t\t\t{}\n",
                t(
                    &format!("nsSAFT:{}ID", counterpart_kind),
                    i.counterpart_id.to_string().as_str()
                )
            ));
            x.push_str(&format!(
                "\t\t\t\t\t{}\n",
                t("nsSAFT:Name", &i.counterpart_name)
            ));
            x.push_str("\t\t\t\t\t<nsSAFT:BillingAddress><nsSAFT:City/><nsSAFT:Country>BG</nsSAFT:Country></nsSAFT:BillingAddress>\n");
            x.push_str(&format!("\t\t\t\t</nsSAFT:{}Info>\n", counterpart_kind));
            x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:AccountID", account_id)));
            x.push_str(&format!(
                "\t\t\t\t{}\n",
                tn("nsSAFT:BranchStoreNumber", "0")
            ));
            x.push_str(&format!(
                "\t\t\t\t{}\n",
                tn("nsSAFT:Period", &month.to_string())
            ));
            x.push_str(&format!(
                "\t\t\t\t{}\n",
                tn("nsSAFT:PeriodYear", &year.to_string())
            ));
            x.push_str(&format!(
                "\t\t\t\t{}\n",
                tn("nsSAFT:InvoiceDate", &i.issue_date.to_string())
            ));
            x.push_str(&format!(
                "\t\t\t\t{}\n",
                t("nsSAFT:InvoiceType", &i.document_type)
            ));
            x.push_str(&format!(
                "\t\t\t\t{}\n",
                t("nsSAFT:SelfBillingIndicator", "N")
            ));
            x.push_str(&format!("\t\t\t\t{}\n", t("nsSAFT:SourceID", "dzambo")));
            x.push_str(&format!(
                "\t\t\t\t{}\n",
                tn("nsSAFT:GLPostingDate", &i.issue_date.to_string())
            ));
            let row = invoices::by_id(conn, i.id).await?;
            if let Some((_, lines)) = row {
                for (ln, l) in lines.iter().enumerate() {
                    x.push_str("\t\t\t\t<nsSAFT:InvoiceLine>\n");
                    x.push_str(&format!(
                        "\t\t\t\t\t{}\n",
                        tn("nsSAFT:LineNumber", &(ln + 1).to_string())
                    ));
                    x.push_str(&format!(
                        "\t\t\t\t\t{}\n",
                        t("nsSAFT:GoodsServicesID", "01")
                    ));
                    let code = prods
                        .iter()
                        .find(|(p, _)| p.id == l.product_id)
                        .map(|(p, _)| p.code.clone())
                        .unwrap_or_default();
                    x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:ProductCode", &code)));
                    x.push_str(&format!(
                        "\t\t\t\t\t{}\n",
                        t("nsSAFT:ProductDescription", &l.description)
                    ));
                    x.push_str(&format!(
                        "\t\t\t\t\t{}\n",
                        t("nsSAFT:Quantity", &qty(&l.quantity))
                    ));
                    x.push_str(&format!("\t\t\t\t\t{}\n", t("nsSAFT:InvoiceUOM", &l.unit)));
                    x.push_str(&format!(
                        "\t\t\t\t\t{}\n",
                        tn("nsSAFT:UnitPrice", &money(&l.unit_price))
                    ));
                    x.push_str("\t\t\t\t\t<nsSAFT:InvoiceLineAmount>\n");
                    x.push_str(&format!(
                        "\t\t\t\t\t\t{}\n",
                        tn("nsSAFT:Amount", &money(&l.net_amount))
                    ));
                    x.push_str(&format!(
                        "\t\t\t\t\t\t{}\n",
                        t("nsSAFT:CurrencyCode", "EUR")
                    ));
                    x.push_str(&format!(
                        "\t\t\t\t\t\t{}\n",
                        tn("nsSAFT:CurrencyAmount", &money(&l.net_amount))
                    ));
                    x.push_str("\t\t\t\t\t</nsSAFT:InvoiceLineAmount>\n");
                    x.push_str(&format!(
                        "\t\t\t\t\t{}\n",
                        t("nsSAFT:DebitCreditIndicator", "C")
                    ));
                    x.push_str("\t\t\t\t</nsSAFT:InvoiceLine>\n");
                }
            }
            x.push_str("\t\t\t</nsSAFT:Invoice>\n");
        }
        x.push_str(&format!("\t\t</nsSAFT:{section}>\n"));
    }
    x.push_str("\t</nsSAFT:SourceDocumentsMonthly>\n");
    x.push_str("</nsSAFT:AuditFile>\n");
    Ok(x)
}

fn today() -> String {
    chrono::Utc::now().date_naive().to_string()
}

/// GET /v1/saft/export?period=YYYY-MM&mode=monthly
pub async fn export(
    State(st): State<crate::state::AppState>,
    axum::extract::Extension(session): axum::extract::Extension<crate::state::Session>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, ApiError> {
    let period = q.get("period").cloned().unwrap_or_default();
    let parts: Vec<&str> = period.split('-').collect();
    if parts.len() != 2 || parts[0].parse::<i32>().is_err() || parts[1].parse::<i32>().is_err() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "period трябва да е YYYY-MM",
        ));
    }
    let (year, month) = (
        parts[0].parse::<i32>().unwrap(),
        parts[1].parse::<i32>().unwrap(),
    );
    let (_u, company) = crate::routes::common::active_company(&st, &session.claims.sub).await?;
    let mut conn = st.pool.get().await.map_err(|_| {
        ApiError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "вътрешна грешка",
        )
    })?;
    let xml = monthly_xml(&mut conn, company.id, year, month).await?;
    let filename = format!("saft_{year:04}-{month:02}_monthly.xml");
    Ok((
        axum::http::StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "application/xml; charset=utf-8".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        xml,
    )
        .into_response())
}
