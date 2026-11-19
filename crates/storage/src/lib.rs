//! Diesel + diesel-async + PostgreSQL слой: pool, миграции, seed,
//! и repo модули по домейн.

pub mod access;
pub mod accounts;
pub mod bank_formats;
pub mod bank_rules;
pub mod banks;
pub mod closing;
pub mod companies;
pub mod counterparts;
pub mod dashboard;
pub mod dividends;
pub mod fixed_assets;
pub mod idocs;
pub mod invoices;
pub mod journal;
pub mod mfg;
pub mod models;
pub mod opening;
pub mod password_reset;
pub mod periods;
pub mod products;
pub mod related;
pub mod schema;
pub mod seed;
pub mod settings;
pub mod stock;
pub mod stock_docs;
pub mod stock_replay;
pub mod stockops;
pub mod users;

use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Адресът на dev базата (контейнер dzambo-postgres).
pub const DEFAULT_URL: &str = "postgres://dzambo:dzambo@127.0.0.1:5434/dzambo";

pub type DbConn = Object<AsyncPgConnection>;
pub type DbPool = Pool<AsyncPgConnection>;
pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("pool: {0}")]
    Pool(#[from] diesel_async::pooled_connection::deadpool::BuildError),
    #[error("връзка: {0}")]
    Connection(String),
    #[error("заявка: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("миграции: {0}")]
    Migration(String),
}

pub async fn connect(url: &str) -> Result<DbPool, StorageError> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);
    let pool = Pool::builder(manager).build()?;
    Ok(pool)
}

/// Миграциите се пускат през sync wrapper в spawn_blocking — diesel_migrations
/// е синхронен, а обвивката я прави Send. Връзката се отваря директно, за да
/// не се дърпа конекция от pool-а.
pub async fn run_migrations(url: &str) -> Result<(), StorageError> {
    let conn = AsyncPgConnection::establish(url)
        .await
        .map_err(|e| StorageError::Migration(e.to_string()))?;
    let mut wrapper = AsyncConnectionWrapper::<AsyncPgConnection>::from(conn);
    tokio::task::spawn_blocking(move || {
        wrapper
            .run_pending_migrations(MIGRATIONS)
            .map(|_| ())
            .map_err(|e| StorageError::Migration(e.to_string()))
    })
    .await
    .map_err(|e| StorageError::Migration(e.to_string()))?
}
