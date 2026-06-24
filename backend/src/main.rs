// Web migration backend entrypoint for llm_wiki
// Replaces Tauri runtime with standalone HTTP server

use std::net::SocketAddr;

mod api;
mod graph;
mod memory;
mod vector;

#[tokio::main]
async fn main() {
    // Bind to 0.0.0.0 for external access (requirement #4)
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();

    println!("llm_wiki web backend starting on {}", addr);

    // TODO: replace with real router (axum recommended)
    llm_wiki_server::start(addr).await;
}

mod llm_wiki_server {
    use std::net::SocketAddr;

    pub async fn start(_addr: SocketAddr) {
        println!("Server running (placeholder)");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }
}