//! Контрагенти: списък, четене, запис, изтриване, дедупликация.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    models::counterparts::{Counterpart, CounterpartExtra, NewCounterpart, NewCounterpartExtra},
    schema::{counterpart_extra, counterparts},
    StorageError,
};

pub type CounterpartRow = (Counterpart, CounterpartExtra);

type Joined = (Counterpart, Option<CounterpartExtra>);

pub async fn list(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<CounterpartRow>, StorageError> {
    let rows: Vec<Joined> = counterparts::table
        .filter(counterparts::company_id.eq(company_id))
        .order(counterparts::id.asc())
        .left_join(counterpart_extra::table.on(counterpart_extra::id.eq(counterparts::id)))
        .select((
            Counterpart::as_select(),
            Option::<CounterpartExtra>::as_select(),
        ))
        .load(conn)
        .await?;
    Ok(rows.into_iter().map(row_of).collect())
}

pub async fn by_id(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<CounterpartRow>, StorageError> {
    let row: Option<Joined> = counterparts::table
        .find(id)
        .left_join(counterpart_extra::table.on(counterpart_extra::id.eq(counterparts::id)))
        .select((
            Counterpart::as_select(),
            Option::<CounterpartExtra>::as_select(),
        ))
        .first(conn)
        .await
        .optional()?;
    Ok(row.map(row_of))
}

fn row_of(j: Joined) -> CounterpartRow {
    (j.0, j.1.unwrap_or_default())
}

pub async fn insert(
    conn: &mut AsyncPgConnection,
    new: &NewCounterpart<'_>,
    extra: &NewCounterpartExtra<'_>,
) -> Result<CounterpartRow, StorageError> {
    let cp: Counterpart = diesel::insert_into(counterparts::table)
        .values(new)
        .returning(Counterpart::as_returning())
        .get_result(conn)
        .await?;
    let extra_row = save_extra(conn, cp.id, extra).await?;
    Ok((cp, extra_row))
}

async fn save_extra(
    conn: &mut AsyncPgConnection,
    id: i64,
    e: &NewCounterpartExtra<'_>,
) -> Result<CounterpartExtra, StorageError> {
    diesel::insert_into(counterpart_extra::table)
        .values(NewCounterpartExtra {
            id,
            vies_address: e.vies_address,
            street_name: e.street_name,
            building_number: e.building_number,
            address_building: e.address_building,
            additional_address_detail: e.additional_address_detail,
            address_type: e.address_type,
            region: e.region,
            email: e.email,
            phone: e.phone,
        })
        .on_conflict(counterpart_extra::id)
        .do_update()
        .set((
            counterpart_extra::vies_address.eq(e.vies_address),
            counterpart_extra::street_name.eq(e.street_name),
            counterpart_extra::building_number.eq(e.building_number),
            counterpart_extra::address_building.eq(e.address_building),
            counterpart_extra::additional_address_detail.eq(e.additional_address_detail),
            counterpart_extra::address_type.eq(e.address_type),
            counterpart_extra::region.eq(e.region),
            counterpart_extra::email.eq(e.email),
            counterpart_extra::phone.eq(e.phone),
        ))
        .returning(CounterpartExtra::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

/// Обновява всички полета на контрагент (PATCH с пълно тяло),
/// без company_id — фирмата не се сменя.
pub async fn update(
    conn: &mut AsyncPgConnection,
    id: i64,
    new: &NewCounterpart<'_>,
    extra: &NewCounterpartExtra<'_>,
) -> Result<CounterpartRow, StorageError> {
    let cp: Counterpart = diesel::update(counterparts::table.find(id))
        .set((
            counterparts::name.eq(new.name),
            counterparts::counterpart_type.eq(new.counterpart_type),
            counterparts::eik.eq(new.eik),
            counterparts::vat_number.eq(new.vat_number),
            counterparts::address.eq(new.address),
            counterparts::city.eq(new.city),
            counterparts::post_code.eq(new.post_code),
            counterparts::country.eq(new.country),
            counterparts::contact_person.eq(new.contact_person),
        ))
        .returning(Counterpart::as_returning())
        .get_result(conn)
        .await?;
    let extra_row = save_extra(conn, id, extra).await?;
    Ok((cp, extra_row))
}

pub async fn delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(counterparts::table.find(id))
        .execute(conn)
        .await?;
    Ok(())
}

/// Дедупликация по ДДС/ЕИК — правилата от ytbuch. Връща текста на грешката.
pub async fn duplicate_error(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    exclude_id: i64,
    vat: &str,
    eik: &str,
) -> Result<Option<String>, StorageError> {
    let rows = counterparts::table
        .filter(counterparts::company_id.eq(company_id))
        .filter(counterparts::id.ne(exclude_id))
        .select((counterparts::vat_number, counterparts::eik))
        .load::<(String, String)>(conn)
        .await?;
    let vat = vat.trim();
    let eik = eik.trim();
    let eik_placeholder = eik.is_empty() || eik.starts_with("999");
    for (other_vat, other_eik) in rows {
        if !vat.is_empty() && other_vat.trim().eq_ignore_ascii_case(vat) {
            return Ok(Some("контрагент с този ДДС номер вече съществува".into()));
        }
        let other_placeholder = other_eik.trim().is_empty() || other_eik.trim().starts_with("999");
        if !eik_placeholder && !other_placeholder && other_eik.trim().eq_ignore_ascii_case(eik) {
            return Ok(Some("контрагент с този ЕИК вече съществува".into()));
        }
    }
    Ok(None)
}
