//! Начално засяване: superadmin, демо фирма YT ООД, сметкоплан НАП,
//! две фактури за таблото. Идемпотентно — проверява бройките.

use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use domain::{auth, noms};

use crate::{
    companies,
    models::{NewAccount, NewCompany, NewCompanyDetails, NewUser},
    schema::{accounts, invoices},
    users, DbPool, StorageError,
};

const SUPERADMIN_LOGIN: &str = "superadmin";

pub async fn run(pool: &DbPool) -> Result<(), StorageError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|e| StorageError::Connection(e.to_string()))?;
    ensure_superadmin(&mut conn).await?;
    ensure_jwt_secret_row(&mut conn).await?;
    ensure_demo(&mut conn).await?;
    ensure_admin_access(&mut conn).await?;
    Ok(())
}

/// Членство на superadmin във всички фирми + активна фирма по подразбиране.
/// Идемпотентно — възстановява каскадно изтрити членства при рестарт.
async fn ensure_admin_access(conn: &mut AsyncPgConnection) -> Result<(), StorageError> {
    let Some(admin) = users::by_login(conn, SUPERADMIN_LOGIN).await? else {
        return Ok(());
    };
    let all = companies::list_all(conn).await?;
    if all.is_empty() {
        return Ok(());
    }
    for (company, _) in &all {
        companies::add_member(conn, admin.id, company.id, 0).await?;
    }
    let key = crate::settings::active_company_key(SUPERADMIN_LOGIN);
    if crate::settings::get(conn, &key).await?.is_none() {
        crate::settings::put(conn, &key, &all[0].0.id.to_string()).await?;
    }
    Ok(())
}

async fn ensure_superadmin(conn: &mut AsyncPgConnection) -> Result<(), StorageError> {
    if users::count(conn).await? > 0 {
        return Ok(());
    }
    let from_env = std::env::var("DZAMBO_SUPERADMIN_PASSWORD").ok();
    let password = from_env.clone().unwrap_or_else(|| auth::random_hex(16));
    if from_env.is_none() {
        tracing::warn!("сгенерирана е еднократна парола за superadmin: {password}");
    }
    let hash = auth::hash_password(&password).map_err(StorageError::Migration)?;
    users::insert(
        conn,
        &NewUser {
            email: SUPERADMIN_LOGIN,
            name: "Superadmin",
            password_hash: &hash,
            is_super_admin: true,
        },
    )
    .await?;
    Ok(())
}

/// JWT тайната се пази в settings — рестарт не инвалидира токените.
async fn ensure_jwt_secret_row(conn: &mut AsyncPgConnection) -> Result<(), StorageError> {
    crate::settings::ensure_jwt_secret(conn).await?;
    Ok(())
}

async fn ensure_demo(conn: &mut AsyncPgConnection) -> Result<(), StorageError> {
    if companies::count(conn).await? > 0 {
        return Ok(());
    }
    let user = users::by_login(conn, SUPERADMIN_LOGIN)
        .await?
        .ok_or_else(|| StorageError::Connection("липсва superadmin".into()))?;

    let company = companies::insert(
        conn,
        &NewCompany {
            name: "YT ООД",
            eik: "207123456",
            vat_number: "BG207123456",
            is_vat_registered: true,
            vat_period: "monthly",
            currency: "EUR",
            address: "бул. Витоша 1",
            city: "София",
            post_code: "1000",
            phone: "02 000 0000",
            email: "office@yt.example",
            website: "",
            mol: "Иван Петров",
            manager_name: "Иван Петров",
            manager_eik: "",
            accountant_name: "",
            nap_office: "ТД на НАП София",
            iban: "BG80BNBG96611020345678",
            bic: "BNBGBGSD",
            inventory_valuation_method: "WAC",
            fiscal_year_start_month: 1,
            country: "BG",
            region: "София",
            created_at: "2026-10-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        },
    )
    .await?;
    let (company, _details) = company;

    companies::set_details(
        conn,
        company.id,
        &NewCompanyDetails {
            company_id: company.id,
            tax_authority: "",
            tax_accounting_basis: "A",
            street_name: "бул. Витоша",
            building_number: "1",
        },
    )
    .await?;

    companies::add_member(conn, user.id, company.id, 0).await?;
    crate::settings::put(
        conn,
        &crate::settings::active_company_key(SUPERADMIN_LOGIN),
        &company.id.to_string(),
    )
    .await?;

    fill_chart(conn, company.id).await?;
    seed_invoices(conn, company.id).await?;
    tracing::info!("демо фирма YT ООД (id={}) засеяна", company.id);
    Ok(())
}

/// Сметкопланът НАП се досева само за липсващите номера.
async fn fill_chart(conn: &mut AsyncPgConnection, company_id: i64) -> Result<i64, StorageError> {
    let mut created = 0i64;
    for (code, name) in noms::chart() {
        let affected = diesel::insert_into(accounts::table)
            .values(NewAccount {
                company_id,
                number: code,
                name,
            })
            .on_conflict((accounts::company_id, accounts::number))
            .do_nothing()
            .execute(conn)
            .await?;
        created += affected as i64;
    }
    Ok(created)
}

async fn seed_invoices(conn: &mut AsyncPgConnection, company_id: i64) -> Result<(), StorageError> {
    let rows = vec![
        (
            "0000000041",
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            "Демо Клиент ООД",
            "draft",
            "out",
            "1000.0000",
            "200.0000",
            "1200.0000",
        ),
        (
            "0000000042",
            NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
            "Доставчик Демо ООД",
            "posted",
            "in",
            "250.0000",
            "50.0000",
            "300.0000",
        ),
    ];
    for (number, date, counterpart, status, direction, net, vat, total) in rows {
        let (net, vat, total): (
            bigdecimal::BigDecimal,
            bigdecimal::BigDecimal,
            bigdecimal::BigDecimal,
        ) = (
            net.parse().unwrap(),
            vat.parse().unwrap(),
            total.parse().unwrap(),
        );
        diesel::insert_into(invoices::table)
            .values((
                invoices::company_id.eq(company_id),
                invoices::number.eq(number),
                invoices::issue_date.eq(date),
                invoices::counterpart_id.eq(2i64),
                invoices::counterpart_name.eq(counterpart),
                invoices::currency.eq("EUR"),
                invoices::net_amount.eq(net),
                invoices::vat_amount.eq(vat),
                invoices::total_amount.eq(total),
                invoices::status.eq(status),
                invoices::direction.eq(direction),
            ))
            .execute(conn)
            .await?;
    }
    Ok(())
}
