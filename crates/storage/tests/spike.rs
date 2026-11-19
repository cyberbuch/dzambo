//! Жива сонда: миграция → запис на NUMERIC(18,4) + TIMESTAMPTZ → четене обратно.
//! Пуска се срещу dev базата: postgres://dzambo:dzambo@127.0.0.1:5434/dzambo

use std::str::FromStr;

use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use storage::schema::spike_entries::dsl::{amount, note, spike_entries};
use storage::{connect, models::SpikeEntry, run_migrations, DEFAULT_URL};

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
    let inserted: SpikeEntry = diesel::insert_into(spike_entries)
        .values((amount.eq(entry_amount.clone()), note.eq("сонда")))
        .returning(SpikeEntry::as_returning())
        .get_result(&mut *conn)
        .await
        .expect("запис");

    assert_eq!(inserted.amount, entry_amount);
    assert_eq!(inserted.json_amount(), "1234.5678");
    assert!(inserted.created_at.timestamp() > 1_700_000_000);

    let read_back: SpikeEntry = spike_entries
        .find(inserted.id)
        .select(SpikeEntry::as_select())
        .first(&mut *conn)
        .await
        .expect("четене");
    assert_eq!(read_back.amount, entry_amount);
}
