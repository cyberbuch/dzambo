//! Публични auth flows: registration-status, register, forgot/reset-password.
//! Пренесени от ytbuch auth_flow.rs (без SMTP — токенът се логва за dev).

use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use diesel_async::RunQueryDsl;
use serde_json::{json, Value};

use crate::{errors::ApiError, state::AppState};
use domain::auth;
use storage::{
    access, companies,
    models::{NewCompany, NewUser},
    password_reset, settings, users,
};

fn internal() -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка")
}
fn unproc(m: &str) -> ApiError {
    ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, m)
}
async fn conn(st: &AppState) -> Result<storage::DbConn, ApiError> {
    st.pool.get().await.map_err(|_| internal())
}

fn reg_flag(v: Option<String>) -> i64 {
    match v.as_deref().map(str::trim) {
        Some("0") | Some("false") | Some("FALSE") => 0,
        _ => 1,
    }
}

pub async fn registration_status(State(st): State<AppState>) -> Result<Json<Value>, ApiError> {
    let mut c = conn(&st).await?;
    let en = reg_flag(settings::get(&mut c, "auth.registration_enabled").await?);
    Ok(Json(json!({"ok": 1, "registration_enabled": en})))
}

fn trimmed(b: &Value, k: &str) -> String {
    b.get(k)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn password_problem(p: &str, confirm: &str) -> Option<&'static str> {
    if p.len() < 8 {
        Some("паролата трябва да е поне 8 символа")
    } else if p != confirm {
        Some("паролите не съвпадат")
    } else {
        None
    }
}

const REGISTER_MSG: &str =
    "Регистрацията е приета. Ако email-ът е свободен, вече можете да влезете.";

pub async fn register(
    State(st): State<AppState>,
    Json(b): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let mut c = conn(&st).await?;
    if reg_flag(settings::get(&mut c, "auth.registration_enabled").await?) == 0 {
        return Err(ApiError::forbidden("Регистрацията е спряна."));
    }
    let email = trimmed(&b, "email");
    if !email.contains('@') {
        return Err(unproc("валиден email е задължителен"));
    }
    let name = trimmed(&b, "name");
    if name.is_empty() {
        return Err(unproc("името е задължително"));
    }
    let password = b
        .get("password")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let confirm = match b.get("password_confirmation") {
        None | Some(Value::Null) => password.clone(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    if let Some(m) = password_problem(&password, &confirm) {
        return Err(unproc(m));
    }
    let company_name = trimmed(&b, "company_name");
    if company_name.is_empty() {
        return Err(unproc("име на фирмата е задължително"));
    }
    let eik = trimmed(&b, "company_eik");
    if eik.is_empty() {
        return Err(unproc("ЕИК е задължителен"));
    }

    // Зает имейл → еднакъв отговор (да не се издава кои имейли имат акаунти).
    if users::by_login(&mut c, &email).await?.is_some() {
        auth::verify_password(&password, &st.dummy_hash);
        return Ok((
            StatusCode::CREATED,
            Json(json!({"ok": 1, "message": REGISTER_MSG})),
        ));
    }

    let hash = auth::hash_password(&password)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let vat = trimmed(&b, "company_vat_number");
    diesel::sql_query("BEGIN")
        .execute(&mut c)
        .await
        .map_err(internal_e)?;
    let result = register_inner(&mut c, &email, &name, &hash, &company_name, &eik, &vat, &b).await;
    match result {
        Ok(()) => {
            diesel::sql_query("COMMIT")
                .execute(&mut c)
                .await
                .map_err(internal_e)?;
            Ok((
                StatusCode::CREATED,
                Json(json!({"ok": 1, "message": REGISTER_MSG})),
            ))
        }
        Err(e) => {
            let _ = diesel::sql_query("ROLLBACK").execute(&mut c).await;
            Err(e)
        }
    }
}

fn internal_e(e: diesel::result::Error) -> ApiError {
    tracing::error!("заявка: {e}");
    internal()
}

#[allow(clippy::too_many_arguments)]
async fn register_inner(
    c: &mut storage::DbConn,
    email: &str,
    name: &str,
    hash: &str,
    company_name: &str,
    eik: &str,
    vat: &str,
    b: &Value,
) -> Result<(), ApiError> {
    let row = companies::insert(
        c,
        &NewCompany {
            name: company_name,
            eik,
            vat_number: vat,
            is_vat_registered: !vat.is_empty(),
            vat_period: "monthly",
            currency: "BGN",
            address: &trimmed(b, "company_address"),
            city: &trimmed(b, "company_city"),
            post_code: "",
            phone: "",
            email,
            website: "",
            mol: name,
            manager_eik: "",
            accountant_name: "",
            nap_office: "",
            iban: "",
            bic: "",
            inventory_valuation_method: "WAC",
            fiscal_year_start_month: 1,
            country: "BG",
            manager_name: name,
            region: "",
            created_at: Utc::now(),
        },
    )
    .await?;
    let cid = row.0.id;
    let user = users::insert(
        c,
        &NewUser {
            email,
            name,
            password_hash: hash,
            is_super_admin: false,
        },
    )
    .await?;
    settings::put(c, &settings::active_company_key(email), &cid.to_string()).await?;
    let admin = access::role_by_name(c, "Администратор на фирма").await?;
    let role_id = admin.map(|r| r.id as i32).unwrap_or(0);
    access::grant(c, user.id, cid, role_id).await?;
    Ok(())
}

const FORGOT_MSG: &str = "Ако email-ът съществува, изпратихме линк за възстановяване.";

pub async fn forgot_password(
    State(st): State<AppState>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let email = trimmed(&b, "email");
    if email.is_empty() {
        return Err(unproc("Email е задължителен"));
    }
    let mut c = conn(&st).await?;
    let answer = Json(json!({"ok": 1, "message": FORGOT_MSG}));
    let Some(user) = users::by_login(&mut c, &email).await? else {
        return Ok(answer);
    };
    let raw = auth::random_hex(32);
    let hash = auth::sha256_hex(&raw);
    let exp = auth::now_secs() as i64 + 3600;
    password_reset::create(&mut c, user.id, &hash, exp).await?;
    // Няма SMTP — логваме линка за dev/test.
    tracing::info!("reset-password линк за {email}: /reset-password?token={raw}");
    Ok(answer)
}

const BAD_TOKEN: &str = "Невалиден или изтекъл токен";

pub async fn reset_password(
    State(st): State<AppState>,
    Json(b): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let token = trimmed(&b, "token");
    if token.is_empty() {
        return Err(unproc("Невалиден или липсващ токен"));
    }
    let password = b
        .get("password")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let confirm = match b.get("password_confirmation") {
        None | Some(Value::Null) => password.clone(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    if let Some(m) = password_problem(&password, &confirm) {
        return Err(unproc(m));
    }
    let mut c = conn(&st).await?;
    let hash = auth::sha256_hex(&token);
    let now = auth::now_secs() as i64;
    let Some((tid, uid)) = password_reset::find_live(&mut c, &hash, now).await? else {
        return Err(unproc(BAD_TOKEN));
    };
    let Some(user) = users::by_id(&mut c, uid).await? else {
        return Err(unproc(BAD_TOKEN));
    };
    let new_hash = auth::hash_password(&password)
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    users::set_password(&mut c, user.id, &new_hash).await?;
    password_reset::mark_used(&mut c, tid).await?;
    Ok(Json(
        json!({"ok": 1, "message": "Паролата е променена. Можете да влезете."}),
    ))
}
