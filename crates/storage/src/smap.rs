//! SAF-T Дт/Кт кореспонденции към тип движение — по фирма.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::schema::saft_account_mappings;
use crate::StorageResult;

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = saft_account_mappings)]
pub struct Smap {
    pub id: i64,
    pub company_id: i64,
    pub kind: String,
    pub type_code: String,
    pub debit_account: String,
    pub credit_account: String,
    pub debit_analytical: String,
    pub credit_analytical: String,
    pub description: String,
    pub is_active: bool,
}

pub async fn list(
    conn: &mut crate::DbConn,
    company_id: i64,
    kind: &str,
) -> StorageResult<Vec<Smap>> {
    Ok(saft_account_mappings::table
        .filter(saft_account_mappings::company_id.eq(company_id))
        .filter(saft_account_mappings::kind.eq(kind))
        .order(saft_account_mappings::id.asc())
        .load(conn)
        .await?)
}

pub async fn get(
    conn: &mut crate::DbConn,
    company_id: i64,
    id: i64,
) -> StorageResult<Option<Smap>> {
    Ok(saft_account_mappings::table
        .filter(saft_account_mappings::company_id.eq(company_id))
        .find(id)
        .first(conn)
        .await
        .optional()?)
}

#[allow(clippy::too_many_arguments)]
pub async fn create(
    conn: &mut crate::DbConn,
    company_id: i64,
    kind: &str,
    type_code: &str,
    debit: &str,
    credit: &str,
    dan: &str,
    kan: &str,
    desc: &str,
) -> StorageResult<Smap> {
    Ok(diesel::insert_into(saft_account_mappings::table)
        .values((
            saft_account_mappings::company_id.eq(company_id),
            saft_account_mappings::kind.eq(kind),
            saft_account_mappings::type_code.eq(type_code),
            saft_account_mappings::debit_account.eq(debit),
            saft_account_mappings::credit_account.eq(credit),
            saft_account_mappings::debit_analytical.eq(dan),
            saft_account_mappings::credit_analytical.eq(kan),
            saft_account_mappings::description.eq(desc),
        ))
        .get_result(conn)
        .await?)
}

pub async fn update(conn: &mut crate::DbConn, id: i64, s: &Smap) -> StorageResult<()> {
    diesel::update(saft_account_mappings::table.find(id))
        .set((
            saft_account_mappings::type_code.eq(&s.type_code),
            saft_account_mappings::debit_account.eq(&s.debit_account),
            saft_account_mappings::credit_account.eq(&s.credit_account),
            saft_account_mappings::debit_analytical.eq(&s.debit_analytical),
            saft_account_mappings::credit_analytical.eq(&s.credit_analytical),
            saft_account_mappings::description.eq(&s.description),
            saft_account_mappings::is_active.eq(s.is_active),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn delete(conn: &mut crate::DbConn, company_id: i64, id: i64) -> StorageResult<bool> {
    let n = diesel::delete(
        saft_account_mappings::table
            .filter(saft_account_mappings::company_id.eq(company_id))
            .find(id),
    )
    .execute(conn)
    .await?;
    Ok(n > 0)
}
