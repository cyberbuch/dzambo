//! /v1/scans/extract (Mistral OCR) + /v1/scans/ubl (UBL XML → преглед).

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::Json;
use base64::Engine;
use serde_json::{json, Value};
use std::time::Duration;

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use storage::bank_formats::common::{xml_blocks, xml_tag};
use storage::settings;

fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}
fn bad_gateway(m: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_GATEWAY, m)
}
fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}

async fn mistral_key(st: &AppState) -> Option<String> {
    if let Ok(k) = std::env::var("MISTRAL_API_KEY") {
        if !k.trim().is_empty() {
            return Some(k);
        }
    }
    let mut conn = st.pool.get().await.ok()?;
    let k = settings::get(&mut conn, "mistral_key").await.ok()??;
    let k = k.trim().to_string();
    if k.is_empty() {
        None
    } else {
        Some(k)
    }
}

fn clean_b64(raw: &str) -> String {
    let head: String = raw.chars().take(80).collect();
    let body = if head.contains("base64") {
        raw.find(',').map(|i| &raw[i + 1..]).unwrap_or(raw)
    } else {
        raw
    };
    body.chars().filter(|c| !c.is_whitespace()).collect()
}

fn str_field(v: &Value, key: &str, default: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_string()
}

async fn post_mistral(key: &str, url: &str, body: &Value, secs: u64) -> Result<Value, ApiError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(secs))
        .build()
        .map_err(|_| internal())?;
    let resp = client
        .post(url)
        .bearer_auth(key)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|_| bad_gateway("Mistral: връзката се скъса (TLS/таймаут)"))?;
    if !resp.status().is_success() {
        return Err(bad_gateway(&format!(
            "Mistral върна статус {}",
            resp.status()
        )));
    }
    let txt = resp
        .text()
        .await
        .map_err(|_| bad_gateway("Mistral: невалиден отговор"))?;
    serde_json::from_str(&txt).map_err(|_| bad_gateway("невалиден JSON от Mistral"))
}

fn mime_of(filename: &str, mime: &str) -> String {
    let m = mime.trim();
    if !m.is_empty() && m != "application/octet-stream" {
        return m.to_string();
    }
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".pdf") {
        "application/pdf".into()
    } else if lower.ends_with(".png") {
        "image/png".into()
    } else if lower.ends_with(".webp") {
        "image/webp".into()
    } else {
        "image/jpeg".into()
    }
}

async fn ocr(
    key: &str,
    filename: &str,
    mime: &str,
    b64: &str,
    multipage: bool,
) -> Result<(String, usize), ApiError> {
    let raw = clean_b64(b64);
    if raw.is_empty() {
        return Err(unproc("празен файл"));
    }
    let mt = mime_of(filename, mime);
    let data = format!("data:{mt};base64,{raw}");
    let pdf = mt == "application/pdf" || filename.to_ascii_lowercase().ends_with(".pdf");
    let document = if pdf {
        json!({"type": "document_url", "document_url": data})
    } else {
        json!({"type": "image_url", "image_url": data})
    };
    let mut body =
        json!({"model": "mistral-ocr-latest", "document": document, "include_image_base64": false});
    if pdf && !multipage {
        body["pages"] = json!([0]);
    }
    let doc = post_mistral(key, "https://api.mistral.ai/v1/ocr", &body, 120).await?;
    let pages = doc
        .get("pages")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let total = pages.len();
    let limit = if multipage { total } else { 1 };
    let text = pages
        .iter()
        .take(limit)
        .enumerate()
        .filter_map(|(i, p)| {
            let md = p.get("markdown").and_then(|v| v.as_str())?.trim();
            if md.is_empty() {
                None
            } else if multipage && total > 1 {
                Some(format!("--- PAGE {} ---\n{md}", i + 1))
            } else {
                Some(md.to_string())
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    if text.is_empty() {
        return Err(bad_gateway("OCR не върна текст от документа"));
    }
    Ok((text, limit.min(total)))
}

fn prompt(direction: &str, multipage: bool) -> String {
    let who = if direction == "in" {
        "This is a PURCHASE: counterpart is the supplier (issuer)."
    } else {
        "This is a SALE: counterpart is the customer (recipient)."
    };
    let page_hint = if multipage {
        " MULTI-PAGE: combine continuation line items across pages, totals from last page; ignore duplicate copies/warranty cards/protocols."
    } else {
        " Only the first page is scanned."
    };
    format!("Extract a Bulgarian invoice as JSON only. Keys: invoice_number, issue_date (YYYY-MM-DD), due_date, document_type (01/02/03), counterpart_name, counterpart_eik (digits), counterpart_vat_number (with prefix e.g. BG), currency, currency_rate, payment_method, payment_note, vat_exemption_reason, tax_event_date, original_invoice_number, total_net_amount, total_vat_amount, total_amount, lines (array of description, quantity, unit, unit_price, vat_rate, net_amount, vat_amount, total_amount). Amounts as strings 2 decimals. unit_price net without VAT. unit UN/CEFACT (C62/KGM/LTR/MTR) else C62. vat_rate 20/9/0. Empty if unknown. {who}{page_hint}")
}

async fn extract_json(
    key: &str,
    direction: &str,
    multipage: bool,
    text: &str,
) -> Result<Value, ApiError> {
    let body = json!({
        "model": "mistral-small-latest", "temperature": 0.1, "max_tokens": 4096,
        "response_format": {"type": "json_object"},
        "messages": [
            {"role": "system", "content": "You extract Bulgarian invoices. Return only JSON."},
            {"role": "user", "content": format!("{}\n\nOCR text:\n{text}", prompt(direction, multipage))}
        ]
    });
    let doc = post_mistral(key, "https://api.mistral.ai/v1/chat/completions", &body, 60).await?;
    let content = doc
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let start = content
        .find('{')
        .ok_or_else(|| bad_gateway("Mistral не върна JSON на фактурата"))?;
    let end = content
        .rfind('}')
        .ok_or_else(|| bad_gateway("Mistral не върна JSON на фактурата"))?;
    serde_json::from_str(&content[start..=end])
        .map_err(|_| bad_gateway("невалиден JSON от Mistral"))
}

pub async fn extract(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let filename = b
        .get("filename")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("document.pdf")
        .to_string();
    let mime = b.get("mime").and_then(Value::as_str).unwrap_or("");
    let b64 = b
        .get("content_base64")
        .and_then(Value::as_str)
        .unwrap_or("");
    if b64.trim().is_empty() {
        return Err(unproc("content_base64 е задължителен"));
    }
    let direction = b
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("in")
        .trim()
        .to_string();
    if direction != "in" && direction != "out" {
        return Err(unproc("direction must be 'in' or 'out'"));
    }
    let multipage = b.get("multipage").and_then(Value::as_i64).unwrap_or(0) != 0
        || b.get("multipage").and_then(Value::as_bool).unwrap_or(false);
    let Some(key) = mistral_key(&st).await else {
        return Err(unproc(
            "Няма Mistral ключ (задайте MISTRAL_API_KEY или Настройки → Интеграции).",
        ));
    };
    let (text, pages_scanned) = ocr(&key, &filename, mime, b64, multipage).await?;
    let doc = extract_json(&key, &direction, multipage, &text).await?;
    let vat_registered = c.is_vat_registered;
    let mut lines: Vec<Value> = doc.get("lines").and_then(|v| v.as_array()).cloned().unwrap_or_default().iter().map(|l| json!({
        "description": str_field(l, "description", ""), "quantity": str_field(l, "quantity", "1"),
        "unit": str_field(l, "unit", "C62"), "unit_price": str_field(l, "unit_price", "0"),
        "vat_rate": str_field(l, "vat_rate", "20"), "net_amount": str_field(l, "net_amount", ""),
        "vat_amount": str_field(l, "vat_amount", ""), "total_amount": str_field(l, "total_amount", "0"),
    })).collect();
    let tot_net = str_field(&doc, "total_net_amount", "0");
    let mut tot_vat = str_field(&doc, "total_vat_amount", "0");
    let mut tot_all = str_field(&doc, "total_amount", "0");
    let mut vatex = str_field(&doc, "vat_exemption_reason", "");
    if !vat_registered {
        for line in &mut lines {
            line["vat_rate"] = json!("0");
            line["vat_amount"] = json!("0.00");
            line["total_amount"] = line["net_amount"].clone();
        }
        tot_vat = "0.00".into();
        tot_all = tot_net.clone();
        if vatex.trim().is_empty() {
            vatex = "VATEX-EU-SM".into();
        }
    }
    Ok(Json(json!({
        "direction": direction, "filename": filename, "multipage": multipage, "pages_scanned": pages_scanned,
        "vat_registered": vat_registered, "invoice_number": str_field(&doc, "invoice_number", ""),
        "issue_date": str_field(&doc, "issue_date", ""), "due_date": str_field(&doc, "due_date", ""),
        "tax_event_date": str_field(&doc, "tax_event_date", ""), "document_type": str_field(&doc, "document_type", "01"),
        "counterpart_name": str_field(&doc, "counterpart_name", ""), "counterpart_eik": str_field(&doc, "counterpart_eik", ""),
        "counterpart_vat_number": str_field(&doc, "counterpart_vat_number", ""), "currency": str_field(&doc, "currency", "EUR"),
        "currency_rate": str_field(&doc, "currency_rate", "1"), "payment_method": str_field(&doc, "payment_method", "банков превод"),
        "payment_note": str_field(&doc, "payment_note", ""), "vat_exemption_reason": vatex,
        "original_invoice_number": str_field(&doc, "original_invoice_number", ""),
        "total_net_amount": tot_net, "total_vat_amount": tot_vat, "total_amount": tot_all, "lines": lines,
    })))
}

// ---- UBL XML ----

fn decode_xml(raw: &str, b64: &str) -> Result<String, ApiError> {
    let mut xml = raw.trim().trim_start_matches('\u{feff}').to_string();
    if xml.is_empty() {
        if b64.trim().is_empty() {
            return Err(unproc("празен файл"));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(clean_b64(b64))
            .map_err(|_| unproc("Файлът не е XML."))?;
        if bytes.len() > 8_000_000 {
            return Err(unproc("Файлът е над 8 МБ."));
        }
        xml = String::from_utf8_lossy(&bytes)
            .trim_start_matches('\u{feff}')
            .trim()
            .to_string();
    }
    Ok(xml)
}

fn party(xml: &str, tag: &str) -> (String, String, String) {
    let block = xml_blocks(xml, tag).first().copied().unwrap_or("");
    let p = xml_blocks(block, "cac:Party")
        .first()
        .copied()
        .unwrap_or(block);
    let name = {
        let n = xml_tag(p, "cbc:Name");
        if n.is_empty() {
            xml_tag(p, "cbc:RegistrationName")
        } else {
            n
        }
    };
    let vat = xml_tag(p, "cbc:EndpointID");
    let vat = if vat.is_empty() {
        xml_tag(
            xml_blocks(p, "cac:PartyTaxScheme")
                .first()
                .copied()
                .unwrap_or(""),
            "cbc:CompanyID",
        )
    } else {
        vat
    };
    let eik = xml_tag(
        xml_blocks(p, "cac:PartyLegalEntity")
            .first()
            .copied()
            .unwrap_or(""),
        "cbc:CompanyID",
    );
    let eik = if eik.is_empty() {
        xml_tag(
            xml_blocks(p, "cac:PartyIdentification")
                .first()
                .copied()
                .unwrap_or(""),
            "cbc:ID",
        )
    } else {
        eik
    };
    (name, vat, eik)
}

fn digits(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_digit()).collect()
}
fn same(a: &str, b: &str) -> bool {
    let x = a.trim().to_ascii_lowercase();
    let y = b.trim().to_ascii_lowercase();
    !x.is_empty() && x == y
}

pub async fn ubl(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, c) = active_company(&st, &s.claims.sub).await?;
    let filename = b
        .get("filename")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("invoice.xml")
        .to_string();
    let direction = b
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("in")
        .trim()
        .to_string();
    if direction != "in" && direction != "out" {
        return Err(unproc("direction must be 'in' or 'out'"));
    }
    let xml_in = b.get("xml").and_then(Value::as_str).unwrap_or("");
    let b64 = b
        .get("content_base64")
        .and_then(Value::as_str)
        .unwrap_or("");
    let xml = decode_xml(xml_in, b64)?;

    let (sup_name, sup_vat, sup_eik) = party(&xml, "cac:AccountingSupplierParty");
    let (cus_name, cus_vat, cus_eik) = party(&xml, "cac:AccountingCustomerParty");
    let cvat = c.vat_number.to_ascii_lowercase();
    let ceik = c.eik.to_ascii_lowercase();
    let sup_us = same(&sup_vat, &cvat) || same(&sup_eik, &ceik) || same(&sup_eik, &digits(&cvat));
    let cus_us = same(&cus_vat, &cvat) || same(&cus_eik, &ceik) || same(&cus_eik, &digits(&cvat));
    let (p_name, p_vat, p_eik, dir) = if sup_us && !cus_us {
        (cus_name, cus_vat, cus_eik, "out")
    } else if cus_us && !sup_us {
        (sup_name, sup_vat, sup_eik, "in")
    } else if direction == "out" {
        (cus_name, cus_vat, cus_eik, "out")
    } else {
        (sup_name, sup_vat, sup_eik, direction.as_str())
    };

    let number = xml_tag(&xml, "cbc:ID");
    let issue_date = xml_tag(&xml, "cbc:IssueDate");
    let due_date = xml_tag(&xml, "cbc:DueDate");
    let currency = {
        let cur = xml_tag(&xml, "cbc:DocumentCurrencyCode");
        if cur.is_empty() {
            "EUR".to_string()
        } else {
            cur
        }
    };
    let monetary = xml_blocks(&xml, "cac:LegalMonetaryTotal")
        .first()
        .copied()
        .unwrap_or("");
    let tot_net = xml_tag(monetary, "cbc:LineExtensionAmount");
    let tot_all = xml_tag(monetary, "cbc:PayableAmount");
    let tax_total = xml_blocks(&xml, "cac:TaxTotal")
        .first()
        .copied()
        .unwrap_or("");
    let tot_vat = xml_tag(tax_total, "cbc:TaxAmount");

    let mut lines = Vec::new();
    for ln in xml_blocks(&xml, "cac:InvoiceLine") {
        let qty = xml_tag(ln, "cbc:InvoicedQuantity");
        let unit = xml_attr(ln, "cbc:InvoicedQuantity", "unitCode");
        let net = xml_tag(ln, "cbc:LineExtensionAmount");
        let price = xml_tag(
            xml_blocks(ln, "cac:Price").first().copied().unwrap_or(""),
            "cbc:PriceAmount",
        );
        let item = xml_blocks(ln, "cac:Item").first().copied().unwrap_or("");
        let desc = xml_tag(item, "cbc:Name");
        let seller = xml_tag(
            xml_blocks(item, "cac:SellersItemIdentification")
                .first()
                .copied()
                .unwrap_or(""),
            "cbc:ID",
        );
        let pct = xml_tag(
            xml_blocks(item, "cac:ClassifiedTaxCategory")
                .first()
                .copied()
                .unwrap_or(""),
            "cbc:Percent",
        );
        lines.push(json!({
            "description": desc, "seller_code": seller,
            "quantity": if qty.is_empty() { "1" } else { qty.as_str() },
            "unit": if unit.is_empty() { "C62" } else { unit.as_str() },
            "unit_price": if price.is_empty() { "0" } else { price.as_str() },
            "vat_rate": if pct.is_empty() { "20" } else { pct.as_str() },
            "net_amount": net, "vat_amount": "", "total_amount": "",
        }));
    }
    let type_code = xml_tag(&xml, "cbc:InvoiceTypeCode");
    let document_type = if type_code.is_empty() {
        "01".to_string()
    } else if type_code.len() == 1 {
        format!("0{type_code}")
    } else {
        type_code.chars().take(2).collect()
    };
    Ok(Json(json!({
        "direction": dir, "filename": filename, "vat_registered": c.is_vat_registered,
        "invoice_number": number, "issue_date": issue_date, "due_date": due_date,
        "tax_event_date": issue_date, "document_type": document_type,
        "counterpart_name": p_name, "counterpart_eik": p_eik, "counterpart_vat_number": p_vat,
        "currency": currency, "currency_rate": "1", "payment_method": "банков превод", "payment_note": "",
        "vat_exemption_reason": "", "original_invoice_number": "",
        "total_net_amount": tot_net, "total_vat_amount": tot_vat, "total_amount": tot_all, "lines": lines,
    })))
}

fn xml_attr(block: &str, name: &str, attr: &str) -> String {
    storage::bank_formats::common::xml_attr(block, name, attr)
}
