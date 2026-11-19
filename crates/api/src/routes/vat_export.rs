//! /v1/vat/export и /v1/vat/registers — точен НАП файл от дневника.

use axum::extract::{Extension, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use crate::vatfile::{self, Line, Packet};
use storage::schema::{
    companies, company_details, counterparts, invoice_lines, invoices, journal_entries,
    journal_lines,
};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
fn bad(m: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, m)
}

#[derive(Deserialize)]
pub struct ExportQuery {
    pub period: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

fn parse_period(s: &str) -> Result<(String, String), ApiError> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 6 {
        let iso = format!("{}-{}", &digits[0..4], &digits[4..6]);
        return Ok((iso, digits));
    }
    if s.len() == 7 && s.as_bytes()[4] == b'-' {
        return Ok((s.to_string(), s.replace('-', "")));
    }
    Err(bad("period трябва да е YYYY-MM или YYYYMM"))
}

fn doc_type_code(t: &str) -> String {
    match t {
        "invoice" | "proforma" => "01".into(),
        "debit_note" => "02".into(),
        "credit_note" => "03".into(),
        _ if t.len() == 1 => format!("0{t}"),
        _ => t.chars().take(2).collect(),
    }
}

fn cin(vat: &str, eik: &str) -> String {
    let v: String = vat.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if !v.is_empty() {
        if v.chars().all(|c| c.is_ascii_digit()) {
            return format!("BG{v}");
        }
        return v;
    }
    let e: String = eik.chars().filter(|c| c.is_ascii_digit()).collect();
    if !e.is_empty() {
        return e;
    }
    "999999999".into()
}

fn ddate(d: chrono::NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

async fn build_packet(
    conn: &mut storage::DbConn,
    company_id: i64,
    iso: &str,
    nap: &str,
) -> Result<Packet, ApiError> {
    let comp: (String, String, String) = companies::table
        .find(company_id)
        .select((companies::name, companies::vat_number, companies::eik))
        .first(conn)
        .await
        .map_err(|_| internal())?;
    let branch: String = company_details::table
        .find(company_id)
        .select(company_details::vat_branch_number)
        .first(conn)
        .await
        .unwrap_or_default();
    let accountant: String = companies::table
        .find(company_id)
        .select(companies::accountant_name)
        .first(conn)
        .await
        .unwrap_or_default();
    let manager: String = companies::table
        .find(company_id)
        .select(companies::manager_name)
        .first(conn)
        .await
        .unwrap_or_default();
    let submitter = if !accountant.is_empty() {
        accountant
    } else {
        manager
    };
    let company_vat = cin(&comp.1, &comp.2);

    let mut p = Packet {
        period_nap: nap.to_string(),
        company_vat,
        company_name: comp.0,
        branch,
        submitter,
        sales: Vec::new(),
        purchases: Vec::new(),
    };

    let entries: Vec<(i64, String, String, i64, chrono::NaiveDate)> = journal_entries::table
        .filter(journal_entries::company_id.eq(company_id))
        .filter(journal_entries::vat_period.eq(iso))
        .filter(journal_entries::vat_type.eq_any(vec!["purchase", "sales"]))
        .order((journal_entries::entry_date.asc(), journal_entries::id.asc()))
        .select((
            journal_entries::id,
            journal_entries::vat_type,
            journal_entries::document_type,
            journal_entries::document_id,
            journal_entries::entry_date,
        ))
        .load(conn)
        .await
        .map_err(|_| internal())?;

    for (eid, vtype, dtype, doc_id, edate) in entries {
        let jl: Vec<(String, bigdecimal::BigDecimal, bigdecimal::BigDecimal)> =
            journal_lines::table
                .filter(journal_lines::entry_id.eq(eid))
                .select((
                    journal_lines::direction,
                    journal_lines::amount,
                    journal_lines::vat_amount,
                ))
                .load(conn)
                .await
                .map_err(|_| internal())?;
        let total: f64 = jl
            .iter()
            .filter(|(dir, _, _)| dir == "debit")
            .map(|(_, a, _)| a.to_string().parse::<f64>().unwrap_or(0.0))
            .sum();
        let vat: f64 = jl
            .iter()
            .map(|(_, _, v)| v.to_string().parse::<f64>().unwrap_or(0.0))
            .sum();
        let base = total - vat;

        let mut line = Line::default();
        let mut date = ddate(edate);
        let mut goods = String::new();
        let mut cin_s = "999999999".to_string();
        let mut name = String::new();
        let mut number = String::new();
        let mut code = doc_type_code(&dtype);
        let mut reason = String::new();
        let mut distributed = false;

        if doc_id > 0 {
            let inv: Option<(String, String, chrono::NaiveDate, i64, String, String)> =
                invoices::table
                    .find(doc_id)
                    .select((
                        invoices::number,
                        invoices::document_type,
                        invoices::issue_date,
                        invoices::counterpart_id,
                        invoices::counterpart_name,
                        invoices::vat_exemption_reason,
                    ))
                    .first(conn)
                    .await
                    .optional()
                    .map_err(|_| internal())?;
            if let Some((num, dt, idate, cid, cname, ex)) = inv {
                number = num;
                code = doc_type_code(&dt);
                date = ddate(idate);
                name = cname;
                reason = ex;
                let cp: Option<(String, String)> = counterparts::table
                    .find(cid)
                    .select((counterparts::vat_number, counterparts::eik))
                    .first(conn)
                    .await
                    .optional()
                    .map_err(|_| internal())?;
                if let Some((v, e)) = cp {
                    cin_s = cin(&v, &e);
                }
                let il: Vec<(
                    String,
                    bigdecimal::BigDecimal,
                    bigdecimal::BigDecimal,
                    bigdecimal::BigDecimal,
                )> = invoice_lines::table
                    .filter(invoice_lines::invoice_id.eq(doc_id))
                    .select((
                        invoice_lines::description,
                        invoice_lines::net_amount,
                        invoice_lines::vat_amount,
                        invoice_lines::vat_rate,
                    ))
                    .load(conn)
                    .await
                    .map_err(|_| internal())?;
                if !il.is_empty() {
                    goods = il[0].0.clone();
                    for (desc, net, vamt, rate) in &il {
                        if goods.is_empty() {
                            goods = desc.clone();
                        }
                        let b = net.to_string().parse::<f64>().unwrap_or(0.0);
                        let v = vamt.to_string().parse::<f64>().unwrap_or(0.0);
                        let r = rate.to_string().parse::<f64>().unwrap_or(0.0);
                        if vtype == "sales" {
                            line.add_sale(b, v, r, &reason);
                        } else {
                            line.add_purch(b, v);
                        }
                    }
                    distributed = true;
                }
            }
        }
        if !distributed {
            if vtype == "sales" {
                line.add_sale(base, vat, if vat == 0.0 { 0.0 } else { 20.0 }, &reason);
            } else {
                line.add_purch(base, vat);
            }
        }
        line.doc_type = code;
        line.number = number;
        line.date = date;
        line.cin = cin_s;
        line.name = name;
        line.goods = goods;
        line.flip_if_credit();
        if vtype == "sales" {
            p.sales.push(line);
        } else {
            p.purchases.push(line);
        }
    }
    Ok(p)
}

pub async fn registers(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<ExportQuery>,
) -> Result<Json<Value>, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let period = q
        .period
        .ok_or_else(|| bad("period е задължителен (YYYY-MM)"))?;
    let (iso, nap) = parse_period(&period)?;
    let mut c = st.pool.get().await.map_err(|_| internal())?;
    let p = build_packet(&mut c, company.id, &iso, &nap).await?;
    Ok(Json(vatfile::packet_json(&p)))
}

pub async fn export(
    State(st): State<AppState>,
    Extension(session): Extension<Session>,
    Query(q): Query<ExportQuery>,
) -> Result<axum::response::Response, ApiError> {
    let (_u, company) = active_company(&st, &session.claims.sub).await?;
    let period = q
        .period
        .ok_or_else(|| bad("period е задължителен (YYYY-MM)"))?;
    let (iso, nap) = parse_period(&period)?;
    let mut c = st.pool.get().await.map_err(|_| internal())?;
    let p = build_packet(&mut c, company.id, &iso, &nap).await?;
    let manual: Value = json!({});
    let kind = q.kind.unwrap_or_default();
    let (bytes, mime, filename) = match kind.as_str() {
        "pokupki" => (
            vatfile::pokupki(&p),
            "text/plain; charset=windows-1251",
            "POKUPKI.TXT".to_string(),
        ),
        "prodagbi" | "prodajbi" => (
            vatfile::prodagbi(&p),
            "text/plain; charset=windows-1251",
            "PRODAGBI.TXT".to_string(),
        ),
        "vies" => (
            vatfile::vies(&p, &[]),
            "text/plain; charset=windows-1251",
            format!("VIES_{nap}.TXT"),
        ),
        "zip" => {
            let z = vatfile::zip(&[
                ("DEKLAR.TXT", vatfile::deklar(&p, &manual)),
                ("POKUPKI.TXT", vatfile::pokupki(&p)),
                ("PRODAGBI.TXT", vatfile::prodagbi(&p)),
            ]);
            (z, "application/zip", format!("PPDDS_{nap}.zip"))
        }
        _ => (
            vatfile::deklar(&p, &manual),
            "text/plain; charset=windows-1251",
            "DEKLAR.TXT".to_string(),
        ),
    };
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}
