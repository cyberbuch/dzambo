//! /v1/products/cn8-suggest — КН8 класификация (локален каталог + optional Mistral).

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use crate::errors::ApiError;
use crate::routes::common::active_company;
use crate::state::{AppState, Session};
use domain::cn8::{self, Cn8Entry};
use storage::settings;

const MISTRAL_URL: &str = "https://api.mistral.ai/v1/chat/completions";
const MISTRAL_MODEL: &str = "mistral-small-latest";

fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}

#[derive(Deserialize)]
struct AiReply {
    cn8_code: Option<String>,
    hs4: Option<String>,
    keywords: Option<Vec<String>>,
    unit: Option<String>,
    product_type: Option<String>,
    is_service: Option<bool>,
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

async fn ask_mistral(
    key: &str,
    name: &str,
    desc: &str,
    candidates: &[&Cn8Entry],
) -> Option<AiReply> {
    let mut cand_lines = String::new();
    for c in candidates.iter().take(35) {
        cand_lines.push_str(&format!("{} | {}\n", c.code, c.full_name));
    }
    let prompt = format!(
        "Ти си експерт по Комбинираната митническа номенклатура на ЕС и България (КН8 / NC8_TARIC) и SAF-T BG.\n\
         За подадения артикул избери най-подходящия 8-цифрен код по КН8, 4-цифрената позиция по ХС/КН (hs4), \
         ключови думи на български от митническата номенклатура (keywords), подходяща SAF-T мерна единица (unit: H87 за брой/парче, C62 за единица, KGM за кг, GRM за грам, TNE за тон, LTR за литър, MTR за метър, MTK за м2, MTQ за м3, PR за чифт, SET за комплект/пакет, KWH за електроенергия, HUR за час, MON за месец) \
         и вид материален запас по SAF-T (product_type: \"10\" Материали, \"20\" Продукция, \"30\" Стоки, \"40\" Незавършено производство, \"50\" Инвестиция в материален запас, или \"\" за услуга).\n\
         За услуги кодът по КН8 винаги е \"00000000\".\n\
         Върни САМО валиден JSON обект: {{\"cn8_code\":\"...\",\"hs4\":\"...\",\"keywords\":[\"...\"],\"unit\":\"...\",\"product_type\":\"...\",\"is_service\":false}}.\n\n\
         Артикул: {name}\n\
         Описание: {desc}\n\n\
         Кандидати от каталога NC8_TARIC:\n{cand_lines}"
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .ok()?;
    let body = json!({
        "model": MISTRAL_MODEL, "temperature": 0.0,
        "response_format": {"type": "json_object"},
        "messages": [{"role": "user", "content": prompt}],
    });
    let resp = client
        .post(MISTRAL_URL)
        .bearer_auth(key)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let txt = resp.text().await.ok()?;
    let v: Value = serde_json::from_str(&txt).ok()?;
    let content = v["choices"][0]["message"]["content"].as_str()?;
    let trimmed = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(trimmed).ok()
}

pub async fn suggest(
    State(st): State<AppState>,
    Extension(s): Extension<Session>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let (_u, _c) = active_company(&st, &s.claims.sub).await?;
    let name = body["name"].as_str().unwrap_or("").trim();
    let desc = body["description"].as_str().unwrap_or("").trim();
    let cur_unit = body["unit"].as_str().unwrap_or("").trim();
    let cur_ptype = body["product_type"].as_str().unwrap_or("").trim();
    let flag_srv =
        body["is_service"].as_bool().unwrap_or(false) || body["is_service"].as_i64() == Some(1);
    let combined = format!("{name} {desc}").trim().to_string();
    if combined.is_empty() {
        return Err(unproc("въведете име или описание на артикула"));
    }
    let mut is_service = cn8::guess_is_service(&combined, flag_srv);
    let (_, mut cands) = cn8::rank_entries(&combined, is_service, 35);
    let cat = cn8::catalog();
    if cands.is_empty() {
        let fallback_code = if is_service { "00000000" } else { "0" };
        if let Some(&idx) = cat.by_code.get(fallback_code) {
            cands.push(&cat.entries[idx]);
        }
    }
    let mut chosen = cands[0];
    let mut unit = cn8::guess_saft_unit(&combined, is_service, cur_unit);
    let mut product_type = cn8::guess_product_type(&combined, is_service, cur_ptype);
    let mut source = "catalog";

    if let Some(key) = mistral_key(&st).await {
        if let Some(ai) = ask_mistral(&key, name, desc, &cands).await {
            source = "ai";
            if let Some(srv) = ai.is_service {
                is_service = srv;
            }
            if let Some(u) = ai.unit.as_deref() {
                let u = u.trim().to_uppercase();
                if !u.is_empty() {
                    unit = u;
                }
            }
            if let Some(pt) = ai.product_type.as_deref() {
                if matches!(pt.trim(), "10" | "20" | "30" | "40" | "50") {
                    product_type = pt.trim().to_string();
                }
            }
            let kw = ai.keywords.unwrap_or_default().join(" ");
            let hs4: String = ai
                .hs4
                .as_deref()
                .unwrap_or("")
                .chars()
                .filter(|c| c.is_ascii_digit())
                .take(4)
                .collect();
            let raw_code: String = ai
                .cn8_code
                .as_deref()
                .unwrap_or("")
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect();
            if let Some(&idx) = cat.by_code.get(&raw_code) {
                chosen = &cat.entries[idx];
            } else {
                let prefix = if hs4.len() == 4 {
                    hs4
                } else if raw_code.len() >= 4 {
                    raw_code[..4].to_string()
                } else {
                    String::new()
                };
                let (_, drilled) =
                    cn8::rank_entries(&format!("{combined} {prefix} {kw}"), is_service, 20);
                if let Some(&first) = drilled.first() {
                    chosen = first;
                    cands = drilled;
                }
            }
        }
    }
    if is_service {
        product_type.clear();
    }
    if !cn8::known_unit(&unit) {
        unit = cn8::guess_saft_unit(&combined, is_service, "");
    }
    let mut out_cands = vec![cn8::entry_json(chosen)];
    for c in cands.into_iter().take(15) {
        if c.code != chosen.code {
            out_cands.push(cn8::entry_json(c));
        }
    }
    Ok(Json(json!({
        "cn8_code": chosen.code, "cn8_name": chosen.full_name, "unit": unit,
        "product_type": product_type, "is_service": is_service, "source": source,
        "candidates": out_cands,
    })))
}
