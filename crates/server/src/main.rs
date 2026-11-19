//! dzambo сървър: API + статично сервиране на SPA-та (web режим).
//! Dev портове: postgres :5434, api :8095 (8092 е зает от ytbuch).

use std::{net::SocketAddr, path::PathBuf};

use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| storage::DEFAULT_URL.to_string());
    storage::run_migrations(&url).await.expect("миграции");
    let pool = storage::connect(&url).await.expect("pool");
    storage::seed::run(&pool).await.expect("seed");

    let mut conn = pool.get().await.expect("връзка");
    storage::access::seed_roles(&mut conn).await.expect("роли");
    let jwt_secret = storage::settings::ensure_jwt_secret(&mut conn)
        .await
        .expect("jwt тайна");
    let dummy_hash = domain::auth::hash_password("dummy").expect("dummy хеш");

    let st = api::state::AppState {
        pool,
        jwt_secret,
        dummy_hash,
    };

    let dist = std::env::var("FRONTEND_DIST").unwrap_or_else(|_| "frontend/dist".to_string());
    let index = PathBuf::from(&dist).join("index.html");
    let spa = ServeDir::new(&dist).not_found_service(ServeFile::new(index));

    let port: u16 = std::env::var("DZAMBO_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8095);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    tracing::info!("dzambo-server на http://{addr} (база {url})");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("слушане");
    axum::serve(listener, api::router(st).fallback_service(spa))
        .await
        .expect("сървър");
}
