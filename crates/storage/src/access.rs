//! Роли, членства и одит — реална логика от ytbuch (access/members/audit).

use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::{json, Value};

use crate::schema::{audit_logs, roles, user_companies};
use crate::StorageResult;

pub const PERM_KEYS: &[&str] = &[
    "companies",
    "users",
    "accounts",
    "counterparts",
    "products",
    "invoices",
    "scans",
    "journal",
    "periods",
    "vat",
    "banking",
    "assets",
    "stock",
    "production",
    "dividends",
    "reports",
    "saft",
    "settings",
    "rates",
];

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = roles)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub permissions: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = roles)]
struct NewRole<'a> {
    name: &'a str,
    description: &'a str,
    permissions: Value,
}

pub fn role_json(r: &Role) -> Value {
    json!({
        "id": r.id, "name": r.name, "description": r.description,
        "permissions": r.permissions, "created_at": r.created_at.to_rfc3339(),
    })
}

fn norm_perms(v: Option<Value>) -> Value {
    let arr = match v {
        Some(Value::Array(a)) => a,
        Some(Value::String(s)) => serde_json::from_str(&s)
            .unwrap_or(Value::Array(vec![]))
            .as_array()
            .cloned()
            .unwrap_or_default(),
        _ => vec![],
    };
    let clean: Vec<Value> = arr
        .into_iter()
        .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .map(Value::String)
        .collect();
    Value::Array(clean)
}

pub async fn list_roles(conn: &mut crate::DbConn) -> StorageResult<Vec<Role>> {
    Ok(roles::table.order(roles::id.asc()).load(conn).await?)
}

pub async fn create_role(
    conn: &mut crate::DbConn,
    name: &str,
    description: &str,
    permissions: Option<Value>,
) -> StorageResult<Role> {
    Ok(diesel::insert_into(roles::table)
        .values(NewRole {
            name,
            description,
            permissions: norm_perms(permissions),
        })
        .get_result(conn)
        .await?)
}

pub async fn update_role(
    conn: &mut crate::DbConn,
    id: i64,
    name: Option<&str>,
    description: Option<&str>,
    permissions: Option<Value>,
) -> StorageResult<Option<Role>> {
    let existing: Option<Role> = roles::table.find(id).first(conn).await.optional()?;
    let Some(mut r) = existing else {
        return Ok(None);
    };
    if let Some(n) = name {
        diesel::update(roles::table.find(id))
            .set(roles::name.eq(n))
            .execute(conn)
            .await?;
        r.name = n.to_string();
    }
    if let Some(d) = description {
        diesel::update(roles::table.find(id))
            .set(roles::description.eq(d))
            .execute(conn)
            .await?;
        r.description = d.to_string();
    }
    if let Some(p) = permissions {
        let np = norm_perms(Some(p));
        diesel::update(roles::table.find(id))
            .set(roles::permissions.eq(&np))
            .execute(conn)
            .await?;
        r.permissions = np;
    }
    Ok(Some(r))
}

pub async fn delete_role(conn: &mut crate::DbConn, id: i64) -> StorageResult<bool> {
    let n = diesel::delete(roles::table.find(id)).execute(conn).await?;
    Ok(n > 0)
}

pub async fn seed_roles(conn: &mut crate::DbConn) -> StorageResult<()> {
    let count: i64 = roles::table.count().get_result(conn).await?;
    if count > 0 {
        return Ok(());
    }
    let admin: Vec<Value> = PERM_KEYS.iter().map(|k| json!(k)).collect();
    let accountant: Vec<Value> = PERM_KEYS
        .iter()
        .filter(|k| **k != "users")
        .map(|k| json!(k))
        .collect();
    let invoicer: Vec<Value> = ["invoices", "counterparts", "products", "scans"]
        .iter()
        .map(|k| json!(k))
        .collect();
    for (name, perms) in [
        ("Администратор на фирма", admin),
        ("Счетоводител", accountant),
        ("Фактурист", invoicer),
    ] {
        diesel::insert_into(roles::table)
            .values(NewRole {
                name,
                description: "",
                permissions: Value::Array(perms),
            })
            .execute(conn)
            .await?;
    }
    Ok(())
}

pub async fn role_by_name(conn: &mut crate::DbConn, name: &str) -> StorageResult<Option<Role>> {
    Ok(roles::table
        .filter(roles::name.eq(name))
        .first(conn)
        .await
        .optional()?)
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = user_companies)]
pub struct UserCompany {
    pub id: i64,
    pub user_id: i64,
    pub company_id: i64,
    pub role_id: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn grant(
    conn: &mut crate::DbConn,
    user_id: i64,
    company_id: i64,
    role_id: i32,
) -> StorageResult<()> {
    diesel::insert_into(user_companies::table)
        .values((
            user_companies::user_id.eq(user_id),
            user_companies::company_id.eq(company_id),
            user_companies::role_id.eq(role_id),
        ))
        .on_conflict((user_companies::user_id, user_companies::company_id))
        .do_update()
        .set(user_companies::role_id.eq(role_id))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn memberships(
    conn: &mut crate::DbConn,
    user_id: i64,
) -> StorageResult<Vec<UserCompany>> {
    Ok(user_companies::table
        .filter(user_companies::user_id.eq(user_id))
        .load(conn)
        .await?)
}

pub async fn permissions_for(
    conn: &mut crate::DbConn,
    user_id: i64,
    company_id: i64,
) -> StorageResult<Vec<String>> {
    let m: Option<UserCompany> = user_companies::table
        .filter(user_companies::user_id.eq(user_id))
        .filter(user_companies::company_id.eq(company_id))
        .first(conn)
        .await
        .optional()?;
    let Some(m) = m else { return Ok(vec![]) };
    if m.role_id == 0 {
        return Ok(vec![]);
    }
    let r: Option<Role> = roles::table
        .find(m.role_id as i64)
        .first(conn)
        .await
        .optional()?;
    Ok(r.and_then(|r| r.permissions.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect())
}

// ---- Одит ----

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = audit_logs)]
pub struct AuditLog {
    pub id: i64,
    pub company_id: i64,
    pub user_id: i64,
    pub user_login: String,
    pub user_name: String,
    pub entity_type: String,
    pub entity_id: i64,
    pub entity_number: String,
    pub action: String,
    pub summary: String,
    pub old_amount: String,
    pub new_amount: String,
    pub old_data: Option<Value>,
    pub new_data: Option<Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[allow(clippy::too_many_arguments)]
pub async fn record(
    conn: &mut crate::DbConn,
    company_id: i64,
    user_id: i64,
    user_login: &str,
    user_name: &str,
    entity_type: &str,
    entity_id: i64,
    entity_number: &str,
    action: &str,
    summary: &str,
    old_amount: &str,
    new_amount: &str,
    old_data: Option<Value>,
    new_data: Option<Value>,
) -> StorageResult<()> {
    diesel::insert_into(audit_logs::table)
        .values((
            audit_logs::company_id.eq(company_id),
            audit_logs::user_id.eq(user_id),
            audit_logs::user_login.eq(user_login),
            audit_logs::user_name.eq(user_name),
            audit_logs::entity_type.eq(entity_type),
            audit_logs::entity_id.eq(entity_id),
            audit_logs::entity_number.eq(entity_number),
            audit_logs::action.eq(action),
            audit_logs::summary.eq(summary),
            audit_logs::old_amount.eq(old_amount),
            audit_logs::new_amount.eq(new_amount),
            audit_logs::old_data.eq(old_data),
            audit_logs::new_data.eq(new_data),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

pub fn audit_json(a: &AuditLog) -> Value {
    let amount_changed =
        !a.old_amount.is_empty() && !a.new_amount.is_empty() && a.old_amount != a.new_amount;
    json!({
        "id": a.id, "company_id": a.company_id, "user_id": a.user_id,
        "user_login": a.user_login, "user_name": a.user_name,
        "entity_type": a.entity_type, "entity_id": a.entity_id,
        "entity_number": a.entity_number, "action": a.action, "summary": a.summary,
        "old_amount": a.old_amount, "new_amount": a.new_amount,
        "amount_changed": amount_changed,
        "old_data": a.old_data, "new_data": a.new_data,
        "created_at": a.created_at.to_rfc3339(),
    })
}

pub async fn list_audit(
    conn: &mut crate::DbConn,
    company_id: i64,
    entity_type: Option<&str>,
    entity_id: Option<i64>,
) -> StorageResult<Vec<AuditLog>> {
    let mut q = audit_logs::table
        .filter(audit_logs::company_id.eq(company_id))
        .into_boxed();
    if let Some(t) = entity_type {
        q = q.filter(audit_logs::entity_type.eq(t));
    }
    if let Some(id) = entity_id {
        q = q.filter(audit_logs::entity_id.eq(id));
    }
    Ok(q.order(audit_logs::id.desc()).load(conn).await?)
}

pub fn money_str(v: &BigDecimal) -> String {
    format!("{v:.2}")
}
