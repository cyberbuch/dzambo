//! dzambo сървър. Фаза 0: само health/ready на :8095 (8092 е зает от ytbuch).
//! Фаза 1: storage + auth + SPA static serving.

use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let addr = SocketAddr::from(([127, 0, 0, 1], 8095));
    let listener = tokio::net::TcpListener::bind(addr).await.expect("слушане");
    tracing::info!("dzambo-server на http://{addr}");
    axum::serve(listener, api::router()).await.expect("сървър");
}
