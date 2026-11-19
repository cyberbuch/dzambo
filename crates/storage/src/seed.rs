//! Начално засяване: superadmin, демо фирма YT ООД, сметкоплан НАП,
//! две фактури за таблото. Идемпотентно — проверява бройките.

use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use domain::{auth, noms};

use crate::{
    companies,
    models::{
        counterparts::{NewCounterpart, NewCounterpartExtra},
        products::{NewProduct, NewProductExtra},
        NewAccount, NewCompany, NewCompanyDetails, NewUser,
    },
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
    ensure_demo_extras(&mut conn).await?;
    if let Some((company, _)) = companies::list_all(&mut conn)
        .await?
        .into_iter()
        .find(|(c, _)| c.name == "YT ООД")
    {
        seed_invoices(&mut conn, company.id).await?;
    }
    Ok(())
}

/// Демо контрагенти и стока — идемпотентно по име/код.
async fn ensure_demo_extras(conn: &mut AsyncPgConnection) -> Result<(), StorageError> {
    let Some((company, _)) = companies::list_all(conn)
        .await?
        .into_iter()
        .find(|(c, _)| c.name == "YT ООД")
    else {
        return Ok(());
    };
    let cid = company.id;

    let cps = crate::counterparts::list(conn, cid).await?;
    if !cps.iter().any(|(c, _)| c.name == "Демо Клиент ООД") {
        crate::counterparts::insert(
            conn,
            &NewCounterpart {
                company_id: cid,
                name: "Демо Клиент ООД",
                counterpart_type: "customer",
                eik: "123456789",
                vat_number: "BG123456789",
                address: "бул. Демо 1",
                city: "София",
                post_code: "1000",
                country: "BG",
                contact_person: "",
            },
            &empty_cp_extra(),
        )
        .await?;
    }
    if !cps.iter().any(|(c, _)| c.name == "Доставчик Демо ООД") {
        crate::counterparts::insert(
            conn,
            &NewCounterpart {
                company_id: cid,
                name: "Доставчик Демо ООД",
                counterpart_type: "supplier",
                eik: "987654321",
                vat_number: "BG987654321",
                address: "ул. Демо 2",
                city: "София",
                post_code: "1000",
                country: "BG",
                contact_person: "",
            },
            &empty_cp_extra(),
        )
        .await?;
    }

    let products = crate::products::list(conn, cid).await?;
    if !products.iter().any(|(p, _)| p.code == "1") {
        let def = default_product_accounts(conn, cid).await;
        crate::products::insert(
            conn,
            &NewProduct {
                company_id: cid,
                name: "Демо стока",
                code: "1",
                description: "",
                unit: "C62",
                price: "10".parse().unwrap(),
                cost_price: "6".parse().unwrap(),
                vat_rate: "20".parse().unwrap(),
                is_service: false,
                is_inventory: true,
                inventory_account_id: def.0,
                revenue_account_id: def.1,
                is_active: true,
            },
            &NewProductExtra {
                id: 0,
                product_type: "30",
                cn8_code: "",
                cn8_name: "",
                expense_account_id: def.2,
                auto_write_off: false,
            },
        )
        .await?;
    }
    Ok(())
}

fn empty_cp_extra() -> NewCounterpartExtra<'static> {
    NewCounterpartExtra {
        id: 0,
        vies_address: "",
        street_name: "",
        building_number: "",
        address_building: "",
        additional_address_detail: "",
        address_type: "",
        region: "",
        email: "",
        phone: "",
    }
}

async fn default_product_accounts(conn: &mut AsyncPgConnection, cid: i64) -> (i64, i64, i64) {
    let Ok(all) = crate::accounts::list(conn, cid).await else {
        return (0, 0, 0);
    };
    let pick = |n: &str| {
        all.iter()
            .find(|a| a.number == n)
            .map(|a| a.id)
            .unwrap_or(0)
    };
    (pick("302"), pick("706"), pick("601"))
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
    tracing::info!("демо фирма YT ООД (id={}) засеяна", company.id);
    Ok(())
}

/// Сметкопланът НАП се досева само за липсващите номера.
pub async fn fill_chart(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<i64, StorageError> {
    let mut created = 0i64;
    for (code, name) in noms::chart() {
        let affected = diesel::insert_into(accounts::table)
            .values(NewAccount {
                company_id,
                number: code,
                name,
                saft_account_type: code,
                is_currency: code.starts_with('4') || code.starts_with('5'),
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
    // Идемпотентно: ако вече има демо фактури за фирмата, не се дублира.
    let existing = crate::invoices::list(conn, company_id).await?;
    if existing.iter().any(|h| h.number == "0000000041") {
        return Ok(());
    }
    let rows = [
        (
            "0000000041",
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            "Демо Клиент ООД",
            "draft",
            "out",
            "1000",
            "200",
            "1200",
        ),
        (
            "0000000042",
            NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
            "Доставчик Демо ООД",
            "posted",
            "in",
            "250",
            "50",
            "300",
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
                invoices::direction.eq(direction),
                invoices::document_type.eq("01"),
                invoices::number.eq(number),
                invoices::issue_date.eq(date),
                invoices::counterpart_id.eq(2i64),
                invoices::counterpart_name.eq(counterpart),
                invoices::currency.eq("EUR"),
                invoices::net_amount.eq(net),
                invoices::vat_amount.eq(vat),
                invoices::total_amount.eq(total),
                invoices::status.eq(status),
                invoices::accounting_month.eq("2026-10"),
            ))
            .execute(conn)
            .await?;
    }
    Ok(())
}
