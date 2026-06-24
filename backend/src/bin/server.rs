use std::net::SocketAddr;
use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();

    let app = Router::new().route("/health", get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}