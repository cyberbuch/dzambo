//! Мапинг на сканирани имена на артикули към наши продукти (по контрагент).

use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::schema::product_name_mappings;
use crate::StorageResult;

/// Нормализация: trim + свити интервали + lowercase (за съвпадение при сканиране).
pub fn norm(raw: &str) -> String {
    let mut out = String::new();
    let mut space = false;
    for ch in raw.trim().chars() {
        if ch.is_whitespace() {
            if !space && !out.is_empty() {
                out.push(' ');
                space = true;
            }
        } else {
            space = false;
            out.push(ch.to_lowercase().next().unwrap_or(ch));
        }
    }
    out.trim().to_string()
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = product_name_mappings)]
pub struct Mapping {
    pub id: i64,
    pub company_id: i64,
    pub counterpart_id: i64,
    pub scanned_name: String,
    pub product_id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list(
    conn: &mut crate::DbConn,
    company_id: i64,
    counterpart_id: i64,
) -> StorageResult<Vec<Mapping>> {
    let mut q = product_name_mappings::table
        .filter(product_name_mappings::company_id.eq(company_id))
        .into_boxed();
    if counterpart_id > 0 {
        q = q.filter(product_name_mappings::counterpart_id.eq(counterpart_id));
    }
    Ok(q.order(product_name_mappings::id.asc()).load(conn).await?)
}

/// Upsert по нормализирано име. Връща броя записани.
pub async fn upsert_items(
    conn: &mut crate::DbConn,
    company_id: i64,
    counterpart_id: i64,
    items: &[(String, i64)],
) -> StorageResult<usize> {
    let mut n = 0;
    for (name, product_id) in items {
        let name = norm(name);
        if name.is_empty() || *product_id <= 0 {
            continue;
        }
        diesel::insert_into(product_name_mappings::table)
            .values((
                product_name_mappings::company_id.eq(company_id),
                product_name_mappings::counterpart_id.eq(counterpart_id),
                product_name_mappings::scanned_name.eq(&name),
                product_name_mappings::product_id.eq(*product_id),
            ))
            .on_conflict((
                product_name_mappings::company_id,
                product_name_mappings::counterpart_id,
                product_name_mappings::scanned_name,
            ))
            .do_update()
            .set(product_name_mappings::product_id.eq(*product_id))
            .execute(conn)
            .await?;
        n += 1;
    }
    Ok(n)
}
