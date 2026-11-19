//! Настройки на фирмата: обекти, собственици, крайни майки, серии.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    models::related::{
        BeneficialOwner, DocumentSeries, Location, NewBeneficialOwner, NewDocumentSeries,
        NewLocation, NewUltimateParent, UltimateParent,
    },
    schema::{beneficial_owners, company_locations, document_series, ultimate_parents},
    StorageError,
};

pub async fn locations(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<Location>, StorageError> {
    company_locations::table
        .filter(company_locations::company_id.eq(company_id))
        .order(company_locations::id.asc())
        .select(Location::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn location_by_id(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<Location>, StorageError> {
    company_locations::table
        .find(id)
        .select(Location::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn location_insert(
    conn: &mut AsyncPgConnection,
    new: &NewLocation<'_>,
) -> Result<Location, StorageError> {
    diesel::insert_into(company_locations::table)
        .values(new)
        .returning(Location::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn location_update(
    conn: &mut AsyncPgConnection,
    id: i64,
    new: &NewLocation<'_>,
) -> Result<Location, StorageError> {
    diesel::update(company_locations::table.find(id))
        .set((
            company_locations::company_id.eq(new.company_id),
            company_locations::name.eq(new.name),
            company_locations::location_type.eq(new.location_type),
            company_locations::street_name.eq(new.street_name),
            company_locations::building_number.eq(new.building_number),
            company_locations::city.eq(new.city),
            company_locations::post_code.eq(new.post_code),
            company_locations::region.eq(new.region),
            company_locations::country.eq(new.country),
            company_locations::is_main.eq(new.is_main),
        ))
        .returning(Location::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn location_delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(company_locations::table.find(id))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn owners(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<BeneficialOwner>, StorageError> {
    beneficial_owners::table
        .filter(beneficial_owners::company_id.eq(company_id))
        .order(beneficial_owners::id.asc())
        .select(BeneficialOwner::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn owner_by_id(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<BeneficialOwner>, StorageError> {
    beneficial_owners::table
        .find(id)
        .select(BeneficialOwner::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn owner_insert(
    conn: &mut AsyncPgConnection,
    new: &NewBeneficialOwner<'_>,
) -> Result<BeneficialOwner, StorageError> {
    diesel::insert_into(beneficial_owners::table)
        .values(new)
        .returning(BeneficialOwner::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn owner_update(
    conn: &mut AsyncPgConnection,
    id: i64,
    new: &NewBeneficialOwner<'_>,
) -> Result<BeneficialOwner, StorageError> {
    diesel::update(beneficial_owners::table.find(id))
        .set((
            beneficial_owners::company_id.eq(new.company_id),
            beneficial_owners::first_name_bg.eq(new.first_name_bg),
            beneficial_owners::last_name_bg.eq(new.last_name_bg),
            beneficial_owners::egn.eq(new.egn),
            beneficial_owners::first_name_latin.eq(new.first_name_latin),
            beneficial_owners::last_name_latin.eq(new.last_name_latin),
            beneficial_owners::country.eq(new.country),
            beneficial_owners::ownership_percentage.eq(new.ownership_percentage),
        ))
        .returning(BeneficialOwner::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn owner_delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(beneficial_owners::table.find(id))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn parents(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<UltimateParent>, StorageError> {
    ultimate_parents::table
        .filter(ultimate_parents::company_id.eq(company_id))
        .order(ultimate_parents::id.asc())
        .select(UltimateParent::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn parent_by_id(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<UltimateParent>, StorageError> {
    ultimate_parents::table
        .find(id)
        .select(UltimateParent::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn parent_insert(
    conn: &mut AsyncPgConnection,
    new: &NewUltimateParent<'_>,
) -> Result<UltimateParent, StorageError> {
    diesel::insert_into(ultimate_parents::table)
        .values(new)
        .returning(UltimateParent::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn parent_update(
    conn: &mut AsyncPgConnection,
    id: i64,
    new: &NewUltimateParent<'_>,
) -> Result<UltimateParent, StorageError> {
    diesel::update(ultimate_parents::table.find(id))
        .set((
            ultimate_parents::company_id.eq(new.company_id),
            ultimate_parents::name_bg.eq(new.name_bg),
            ultimate_parents::uic.eq(new.uic),
            ultimate_parents::name_latin.eq(new.name_latin),
            ultimate_parents::country.eq(new.country),
        ))
        .returning(UltimateParent::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn parent_delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(ultimate_parents::table.find(id))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn series_list(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<DocumentSeries>, StorageError> {
    document_series::table
        .filter(document_series::company_id.eq(company_id))
        .order(document_series::id.asc())
        .select(DocumentSeries::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn series_by_id(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<DocumentSeries>, StorageError> {
    document_series::table
        .find(id)
        .select(DocumentSeries::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn series_insert(
    conn: &mut AsyncPgConnection,
    new: &NewDocumentSeries<'_>,
) -> Result<DocumentSeries, StorageError> {
    diesel::insert_into(document_series::table)
        .values(new)
        .returning(DocumentSeries::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn series_update(
    conn: &mut AsyncPgConnection,
    id: i64,
    new: &NewDocumentSeries<'_>,
) -> Result<DocumentSeries, StorageError> {
    diesel::update(document_series::table.find(id))
        .set((
            document_series::company_id.eq(new.company_id),
            document_series::name.eq(new.name),
            document_series::document_types.eq(new.document_types),
            document_series::pad_width.eq(new.pad_width),
            document_series::start_number.eq(new.start_number),
            document_series::next_number.eq(new.next_number),
        ))
        .returning(DocumentSeries::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn series_delete(conn: &mut AsyncPgConnection, id: i64) -> Result<(), StorageError> {
    diesel::delete(document_series::table.find(id))
        .execute(conn)
        .await?;
    Ok(())
}
