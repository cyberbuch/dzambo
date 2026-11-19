//! Минимален UBL 2.1 експорт на фактура/кредитно известие.

use domain::dto::fmt_money;
use storage::invoices::InvoiceRow;

use crate::errors::ApiError;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Заглавие на документа според типа и посоката.
fn type_code(doc_type: &str, direction: &str) -> &'static str {
    match doc_type {
        "03" | "13" | "23" => "381",
        "09" | "07" => "325",
        _ => {
            if direction == "in" {
                "386"
            } else {
                "380"
            }
        }
    }
}

pub fn render(row: &InvoiceRow) -> Result<String, ApiError> {
    let (inv, lines) = (&row.0, &row.1);
    let is_credit = matches!(inv.document_type.as_str(), "03" | "13" | "23");
    let root = if is_credit { "CreditNote" } else { "Invoice" };
    let ns_root = if is_credit {
        "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2"
    } else {
        "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
    };
    let net = fmt_money(&inv.net_amount);
    let vat = fmt_money(&inv.vat_amount);
    let total = fmt_money(&inv.total_amount);

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!(
        "<{root} xmlns=\"{ns_root}\" \
         xmlns:cac=\"urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2\" \
         xmlns:cbc=\"urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2\">\n"
    ));
    xml.push_str("<cbc:UBLVersionID>2.1</cbc:UBLVersionID>\n");
    xml.push_str("<cbc:CustomizationID>EN16931</cbc:CustomizationID>\n");
    xml.push_str(&format!("<cbc:ID>{}</cbc:ID>\n", esc(&inv.number)));
    xml.push_str(&format!(
        "<cbc:IssueDate>{}</cbc:IssueDate>\n",
        inv.issue_date
    ));
    xml.push_str(&format!(
        "<cbc:{}TypeCode>{}</{}TypeCode>\n",
        root,
        type_code(&inv.document_type, &inv.direction),
        root
    ));
    xml.push_str(&format!(
        "<cbc:DocumentCurrencyCode>{}</cbc:DocumentCurrencyCode>\n",
        esc(&inv.currency)
    ));

    // Продавач — фирмата на потребителя (взимаме от counterpart на входящите, иначе празн).
    xml.push_str("<cac:AccountingSupplierParty><cac:Party>");
    xml.push_str(&format!(
        "<cbc:Name>{}</cbc:Name>",
        esc(&if inv.direction == "in" {
            inv.counterpart_name.clone()
        } else {
            String::new()
        })
    ));
    xml.push_str("</cac:Party></cac:AccountingSupplierParty>\n");

    xml.push_str("<cac:AccountingCustomerParty><cac:Party>");
    xml.push_str(&format!(
        "<cbc:Name>{}</cbc:Name>",
        esc(&if inv.direction == "out" {
            inv.counterpart_name.clone()
        } else {
            String::new()
        })
    ));
    xml.push_str("</cac:Party></cac:AccountingCustomerParty>\n");

    xml.push_str("<cac:TaxTotal><cbc:TaxAmount currencyID=\"EUR\">");
    xml.push_str(&vat);
    xml.push_str("</cbc:TaxAmount></cac:TaxTotal>\n");

    xml.push_str("<cac:LegalMonetaryTotal>");
    xml.push_str(&format!(
        "<cbc:LineExtensionAmount currencyID=\"EUR\">{net}</cbc:LineExtensionAmount>"
    ));
    xml.push_str(&format!(
        "<cbc:TaxExclusiveAmount currencyID=\"EUR\">{net}</cbc:TaxExclusiveAmount>"
    ));
    xml.push_str(&format!(
        "<cbc:TaxInclusiveAmount currencyID=\"EUR\">{total}</cbc:TaxInclusiveAmount>"
    ));
    xml.push_str(&format!(
        "<cbc:PayableAmount currencyID=\"EUR\">{total}</cbc:PayableAmount>"
    ));
    xml.push_str("</cac:LegalMonetaryTotal>\n");

    for (i, l) in lines.iter().enumerate() {
        let tag = if is_credit {
            "CreditNoteLine"
        } else {
            "InvoiceLine"
        };
        let qty_tag = if is_credit {
            "CreditedQuantity"
        } else {
            "InvoicedQuantity"
        };
        xml.push_str(&format!("<cac:{tag}>"));
        xml.push_str(&format!("<cbc:ID>{}</cbc:ID>", i + 1));
        xml.push_str(&format!(
            "<cbc:{qty_tag} unitCode=\"{}\">{:.4}</cbc:{qty_tag}>",
            esc(&l.unit),
            l.quantity
        ));
        xml.push_str(&format!(
            "<cbc:LineExtensionAmount currencyID=\"EUR\">{}</cbc:LineExtensionAmount>",
            fmt_money(&l.net_amount)
        ));
        xml.push_str("<cac:Item>");
        xml.push_str(&format!("<cbc:Name>{}</cbc:Name>", esc(&l.description)));
        xml.push_str("<cac:ClassifiedTaxCategory>");
        let rate_is_zero = l.vat_rate == 0;
        xml.push_str(&format!(
            "<cbc:ID>{}</cbc:ID>",
            if rate_is_zero { "Z" } else { "S" }
        ));
        xml.push_str(&format!("<cbc:Percent>{:.2}</cbc:Percent>", l.vat_rate));
        xml.push_str("<cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>");
        xml.push_str("</cac:ClassifiedTaxCategory>");
        xml.push_str("</cac:Item>");
        xml.push_str(&format!(
            "<cac:Price><cbc:PriceAmount currencyID=\"EUR\">{}</cbc:PriceAmount></cac:Price>",
            fmt_money(&l.unit_price)
        ));
        xml.push_str(&format!("</cac:{tag}>\n"));
    }

    xml.push_str(&format!("</{root}>\n"));
    Ok(xml)
}
