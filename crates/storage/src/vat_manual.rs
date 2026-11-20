//! Ръчни клетки на ДДС декларацията (по фирма и период).

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::Value;

use crate::schema::vat_manual;
use crate::StorageResult;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = vat_manual)]
pub struct Manual {
    pub id: i64,
    pub company_id: i64,
    pub period: String,
    pub cell_33: String,
    pub cell_70: String,
    pub cell_71: String,
    pub cell_80: String,
    pub cell_81: String,
    pub cell_82: String,
}

pub async fn get(
    conn: &mut crate::DbConn,
    company_id: i64,
    period: &str,
) -> StorageResult<Option<Manual>> {
    Ok(vat_manual::table
        .filter(vat_manual::company_id.eq(company_id))
        .filter(vat_manual::period.eq(period))
        .first(conn)
        .await
        .optional()?)
}

fn cell(b: &Value, k: &str) -> String {
    b.get(k)
        .and_then(Value::as_str)
        .unwrap_or("0.00")
        .trim()
        .replace(',', ".")
}

pub async fn save(
    conn: &mut crate::DbConn,
    company_id: i64,
    period: &str,
    b: &Value,
) -> StorageResult<()> {
    let (c33, c70, c71, c80, c81, c82) = (
        cell(b, "cell_33"),
        cell(b, "cell_70"),
        cell(b, "cell_71"),
        cell(b, "cell_80"),
        cell(b, "cell_81"),
        cell(b, "cell_82"),
    );
    diesel::insert_into(vat_manual::table)
        .values((
            vat_manual::company_id.eq(company_id),
            vat_manual::period.eq(period),
            vat_manual::cell_33.eq(&c33),
            vat_manual::cell_70.eq(&c70),
            vat_manual::cell_71.eq(&c71),
            vat_manual::cell_80.eq(&c80),
            vat_manual::cell_81.eq(&c81),
            vat_manual::cell_82.eq(&c82),
        ))
        .on_conflict((vat_manual::company_id, vat_manual::period))
        .do_update()
        .set((
            vat_manual::cell_33.eq(&c33),
            vat_manual::cell_70.eq(&c70),
            vat_manual::cell_71.eq(&c71),
            vat_manual::cell_80.eq(&c80),
            vat_manual::cell_81.eq(&c81),
            vat_manual::cell_82.eq(&c82),
        ))
        .execute(conn)
        .await?;
    Ok(())
}
