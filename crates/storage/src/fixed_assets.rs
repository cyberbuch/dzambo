//! Дълготрайни активи: списък, запис, месечна амортизация.

use bigdecimal::{BigDecimal, Zero};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    accounts, journal,
    models::journal::{NewJournalEntry, NewJournalLine},
    schema::{fixed_asset_categories, fixed_asset_events, fixed_assets},
    StorageError,
};

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = fixed_asset_categories)]
pub struct Category {
    pub id: i64,
    pub company_id: i64,
    pub name: String,
    pub cita_category: String,
    pub min_depreciation_rate: BigDecimal,
    pub max_depreciation_rate: BigDecimal,
    pub default_method: String,
    pub asset_account_id: i64,
    pub depreciation_account_id: i64,
    pub expense_account_id: i64,
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = fixed_assets)]
pub struct Asset {
    pub id: i64,
    pub company_id: i64,
    pub name: String,
    pub inventory_number: String,
    pub category_id: i64,
    pub acquisition_date: NaiveDate,
    pub put_into_service_date: Option<NaiveDate>,
    pub cost: BigDecimal,
    pub salvage_value: BigDecimal,
    pub useful_life_months: i32,
    pub depreciation_method: String,
    pub accounting_depreciation_rate: BigDecimal,
    pub tax_depreciation_rate: BigDecimal,
    pub accumulated_depreciation: BigDecimal,
    pub tax_accumulated_depreciation: BigDecimal,
    pub status: String,
    pub serial_number: String,
    pub manufacturer: String,
    pub model: String,
    pub responsible_person: String,
    pub is_conserved: bool,
    pub location_id: i64,
}

pub async fn list(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<Asset>, StorageError> {
    fixed_assets::table
        .filter(fixed_assets::company_id.eq(company_id))
        .order(fixed_assets::id.asc())
        .select(Asset::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn categories(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<Category>, StorageError> {
    fixed_asset_categories::table
        .filter(fixed_asset_categories::company_id.eq(company_id))
        .order(fixed_asset_categories::id.asc())
        .select(Category::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_asset(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    name: &str,
    inventory_number: &str,
    category_id: i64,
    acquisition_date: NaiveDate,
    cost: BigDecimal,
    salvage_value: BigDecimal,
    useful_life_months: i32,
) -> Result<Asset, StorageError> {
    diesel::insert_into(fixed_assets::table)
        .values((
            fixed_assets::company_id.eq(company_id),
            fixed_assets::name.eq(name),
            fixed_assets::inventory_number.eq(inventory_number),
            fixed_assets::category_id.eq(category_id),
            fixed_assets::acquisition_date.eq(acquisition_date),
            fixed_assets::cost.eq(cost),
            fixed_assets::salvage_value.eq(salvage_value),
            fixed_assets::useful_life_months.eq(useful_life_months),
        ))
        .returning(Asset::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(fixed_assets::table.find(id))
        .execute(conn)
        .await?;
    Ok(())
}

/// Месечна линеен амортизация: (цена − остатъчна) / полезен живот; иначе от ставката.
pub fn monthly(asset: &Asset) -> BigDecimal {
    let base = asset.cost.clone() - asset.salvage_value.clone();
    if base <= BigDecimal::zero() {
        return BigDecimal::zero();
    }
    if asset.useful_life_months > 0 {
        base / BigDecimal::from(asset.useful_life_months)
    } else if asset.accounting_depreciation_rate > BigDecimal::zero() {
        base * &asset.accounting_depreciation_rate / BigDecimal::from(1200)
    } else {
        BigDecimal::zero()
    }
}

pub struct DepRow {
    pub asset_id: i64,
    pub name: String,
    pub amount: BigDecimal,
    pub expense_number: String,
    pub expense_name: String,
    pub dep_number: String,
    pub dep_name: String,
}

fn last_day(y: i32, m: i32) -> NaiveDate {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm as u32, 1).unwrap() - chrono::Duration::days(1)
}

pub async fn preview(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    _year: i32,
    _month: i32,
) -> Result<Vec<DepRow>, StorageError> {
    let assets = list(conn, company_id).await?;
    let cats = categories(conn, company_id).await?;
    let accts = accounts::list(conn, company_id).await?;
    let mut out = Vec::new();
    for a in assets
        .iter()
        .filter(|a| a.status == "active" && !a.is_conserved)
    {
        let amount = monthly(a);
        if amount.is_zero() {
            continue;
        }
        let cat = cats.iter().find(|c| c.id == a.category_id);
        let pick = |id: i64, fallback: &str| -> (String, String) {
            if let Some(acc) = accts.iter().find(|x| x.id == id) {
                return (acc.number.clone(), acc.name.clone());
            }
            let _ = fallback;
            (fallback.to_string(), String::new())
        };
        let (expense_number, expense_name) = match cat {
            Some(c) => pick(c.expense_account_id, "603"),
            None => ("603".into(), String::new()),
        };
        let (dep_number, dep_name) = match cat {
            Some(c) => pick(c.depreciation_account_id, "241"),
            None => ("241".into(), String::new()),
        };
        out.push(DepRow {
            asset_id: a.id,
            name: a.name.clone(),
            amount,
            expense_number,
            expense_name,
            dep_number,
            dep_name,
        });
    }
    Ok(out)
}

/// Записва статията за месечна амортизация: Дт разход, Кт 241 по актив.
pub async fn post_depreciation(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    year: i32,
    month: i32,
) -> Result<i64, StorageError> {
    let key0 = format!("{year:04}-{month:02}");
    let existing: i64 = crate::schema::journal_entries::table
        .filter(crate::schema::journal_entries::document_number.eq(format!("DEP-{key0}")))
        .count()
        .get_result(conn)
        .await?;
    if existing > 0 {
        return Err(StorageError::Connection(format!(
            "амортизация за {key0} вече е начислена"
        )));
    }
    if crate::periods::is_closed(conn, company_id, &key0).await? {
        return Err(StorageError::Connection(format!(
            "периодът {key0} е затворен"
        )));
    }
    let rows = preview(conn, company_id, year, month).await?;
    if rows.is_empty() {
        return Err(StorageError::Connection("няма амортизация за запис".into()));
    }
    let date = last_day(year, month);
    let mut lines: Vec<NewJournalLine> = Vec::new();
    for r in &rows {
        let exp = accounts::get_or_create(conn, company_id, &r.expense_number).await?;
        let dep = accounts::get_or_create(conn, company_id, &r.dep_number).await?;
        lines.push(NewJournalLine {
            entry_id: 0,
            account_id: exp.id,
            account_number: exp.number.clone(),
            account_name: exp.name.clone(),
            direction: "debit".into(),
            amount: r.amount.clone(),
            vat_amount: BigDecimal::default(),
            currency: String::new(),
            fx_quantity: BigDecimal::default(),
            fx_rate: BigDecimal::from(1),
        });
        lines.push(NewJournalLine {
            entry_id: 0,
            account_id: dep.id,
            account_number: dep.number.clone(),
            account_name: dep.name.clone(),
            direction: "credit".into(),
            amount: r.amount.clone(),
            vat_amount: BigDecimal::default(),
            currency: String::new(),
            fx_quantity: BigDecimal::default(),
            fx_rate: BigDecimal::from(1),
        });
    }
    let key = format!("{year:04}-{month:02}");
    let entry = NewJournalEntry {
        company_id,
        entry_date: date,
        document_type: "depreciation".into(),
        document_id: 0,
        document_number: format!("DEP-{key}"),
        description: format!("Амортизация {key}"),
        vat_type: "no_vat".into(),
        counterpart_id: 0,
        counterpart_name: String::new(),
        document_date: date,
        vat_period: key,
    };
    journal::insert_entry(conn, &entry, &lines).await
}

// ---- Операции с ДМА (преоценка, преместване, консервация, брак, събития) ----

pub async fn get(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    id: i64,
) -> Result<Option<Asset>, StorageError> {
    fixed_assets::table
        .filter(fixed_assets::company_id.eq(company_id))
        .find(id)
        .select(Asset::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn set_cost(
    conn: &mut AsyncPgConnection,
    id: i64,
    cost: &BigDecimal,
) -> Result<(), StorageError> {
    diesel::update(fixed_assets::table.find(id))
        .set(fixed_assets::cost.eq(cost))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn set_location(
    conn: &mut AsyncPgConnection,
    id: i64,
    location_id: i64,
) -> Result<(), StorageError> {
    diesel::update(fixed_assets::table.find(id))
        .set(fixed_assets::location_id.eq(location_id))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn set_conserved(
    conn: &mut AsyncPgConnection,
    id: i64,
    flag: bool,
) -> Result<(), StorageError> {
    diesel::update(fixed_assets::table.find(id))
        .set(fixed_assets::is_conserved.eq(flag))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn set_status(
    conn: &mut AsyncPgConnection,
    id: i64,
    status: &str,
) -> Result<(), StorageError> {
    diesel::update(fixed_assets::table.find(id))
        .set(fixed_assets::status.eq(status))
        .execute(conn)
        .await?;
    Ok(())
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = fixed_asset_events)]
pub struct Event {
    pub id: i64,
    pub company_id: i64,
    pub asset_id: i64,
    pub event_type: String,
    pub event_date: NaiveDate,
    pub amount: BigDecimal,
    pub prev_value: String,
    pub location: String,
    pub reason: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn add_event(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    asset_id: i64,
    kind: &str,
    date: NaiveDate,
    amount: &BigDecimal,
    prev: &str,
    location: &str,
    reason: &str,
) -> Result<(), StorageError> {
    diesel::insert_into(fixed_asset_events::table)
        .values((
            fixed_asset_events::company_id.eq(company_id),
            fixed_asset_events::asset_id.eq(asset_id),
            fixed_asset_events::event_type.eq(kind),
            fixed_asset_events::event_date.eq(date),
            fixed_asset_events::amount.eq(amount),
            fixed_asset_events::prev_value.eq(prev),
            fixed_asset_events::location.eq(location),
            fixed_asset_events::reason.eq(reason),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn events(
    conn: &mut AsyncPgConnection,
    asset_id: i64,
) -> Result<Vec<Event>, StorageError> {
    fixed_asset_events::table
        .filter(fixed_asset_events::asset_id.eq(asset_id))
        .order(fixed_asset_events::id.asc())
        .select(Event::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_category(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    name: &str,
    cita_category: &str,
    min_rate: &BigDecimal,
    max_rate: &BigDecimal,
    default_method: &str,
) -> Result<Category, StorageError> {
    diesel::insert_into(fixed_asset_categories::table)
        .values((
            fixed_asset_categories::company_id.eq(company_id),
            fixed_asset_categories::name.eq(name),
            fixed_asset_categories::cita_category.eq(cita_category),
            fixed_asset_categories::min_depreciation_rate.eq(min_rate),
            fixed_asset_categories::max_depreciation_rate.eq(max_rate),
            fixed_asset_categories::default_method.eq(default_method),
        ))
        .returning(Category::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}
