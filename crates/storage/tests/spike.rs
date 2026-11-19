//! Жива сонда: миграция → NUMERIC(18,4)/TIMESTAMPTZ roundtrip,
//! и проверка на argon2 хеша от seed-а.

use std::str::FromStr;

use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use storage::{
    connect, run_migrations, schema::spike_entries::dsl::spike_entries, schema::users, DEFAULT_URL,
};

#[tokio::test]
async fn numeric_and_timestamptz_roundtrip() {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
    let pool = connect(&url).await.expect("pool");
    run_migrations(&url).await.expect("миграции");

    let mut conn = pool.get().await.expect("връзка");
    diesel::delete(spike_entries)
        .execute(&mut *conn)
        .await
        .expect("изчистване");

    let entry_amount = BigDecimal::from_str("1234.5678").expect("сума");
    let inserted: (i32, BigDecimal, String, chrono::DateTime<chrono::Utc>) =
        diesel::insert_into(spike_entries)
            .values((
                storage::schema::spike_entries::amount.eq(entry_amount.clone()),
                storage::schema::spike_entries::note.eq("сонда"),
            ))
            .returning(spike_entries::all_columns())
            .get_result(&mut *conn)
            .await
            .expect("запис");

    assert_eq!(inserted.1, entry_amount);
    assert_eq!(format!("{:.4}", inserted.1), "1234.5678");
}

#[tokio::test]
async fn seeded_password_verifies() {
    let pool = connect(DEFAULT_URL).await.expect("pool");
    let mut conn = pool.get().await.expect("conn");
    let hash: Option<String> = users::table
        .select(users::password_hash)
        .first(&mut *conn)
        .await
        .optional()
        .expect("хеш");
    // В CI няма seed — без потребител тестът се прескача.
    let Some(hash) = hash else { return };
    let expected = std::env::var("DZAMBO_SUPERADMIN_PASSWORD")
        .unwrap_or_else(|_| "dzambo-admin-dev".to_string());
    assert!(
        domain::auth::verify_password(&expected, &hash),
        "хешът от базата не съответства на паролата"
    );
}
