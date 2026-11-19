//! Печат на фактура: PDF (ironpress) / DOCX / ODT (ferrodoc), както ytbuch.

use crate::errors::ApiError;
use axum::http::StatusCode;
use storage::invoices::InvoiceRow;

const FONT: &[u8] = include_bytes!("../data/DejaVuSans.ttf");
const FONT_NAME: &str = "DejaVu";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn invoice_html(row: &InvoiceRow) -> String {
    let (h, lines) = (&row.0, &row.1);
    let mut body = String::new();
    body.push_str(&format!("<h1>Фактура № {}</h1>", esc(&h.number)));
    body.push_str(&format!(
        "<p>Дата: {} · Падеж: {} · Валута: {}</p>",
        h.issue_date,
        h.due_date.map(|d| d.to_string()).unwrap_or_default(),
        esc(&h.currency)
    ));
    body.push_str(&format!(
        "<p>Получател: {} (ЕИК {})</p>",
        esc(&h.counterpart_name),
        h.counterpart_id
    ));
    body.push_str("<table><tr><th>Описание</th><th>К-во</th><th>М.ед.</th><th>Ед.цена</th><th>ДДС%</th><th>Сума</th></tr>");
    for l in lines {
        body.push_str(&format!("<tr><td>{}</td><td>{:.2}</td><td>{}</td><td>{:.2}</td><td>{:.0}</td><td>{:.2}</td></tr>",
            esc(&l.description), l.quantity, esc(&l.unit), l.unit_price, l.vat_rate, l.total_amount));
    }
    body.push_str("</table>");
    body.push_str(&format!(
        "<p>Данъчна основа: {:.2} · ДДС: {:.2} · Общо: {:.2}</p>",
        h.net_amount, h.vat_amount, h.total_amount
    ));
    format!("<!doctype html><html lang=\"bg\"><head><meta charset=\"utf-8\"><style>\
        body {{ font-family: {FONT_NAME}, sans-serif; font-size: 10pt; }} \
        table {{ border-collapse: collapse; width: 100%; }} td, th {{ border: 1px solid #999; padding: 4px; }}</style>\
        </head><body>{body}</body></html>")
}

pub fn render(row: &InvoiceRow, format: &str) -> Result<(Vec<u8>, &'static str), ApiError> {
    let html = invoice_html(row);
    match format {
        "pdf" => {
            let bytes = ironpress::HtmlConverter::new()
                .add_font(FONT_NAME, FONT.to_vec())
                .sanitize(false)
                .convert(&html)
                .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok((bytes, "application/pdf"))
        }
        "docx" | "odt" => {
            let to = if format == "docx" {
                ferrodoc::Format::Docx
            } else {
                ferrodoc::Format::Odt
            };
            let bytes = ferrodoc::convert(html.as_bytes(), ferrodoc::Format::Html, to)
                .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok((
                bytes,
                if format == "docx" {
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                } else {
                    "application/vnd.oasis.opendocument.text"
                },
            ))
        }
        _ => Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "format трябва да е pdf, docx или odt",
        )),
    }
}
