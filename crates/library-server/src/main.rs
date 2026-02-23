//! Binary entry point for the ebook library server.
//!
//! Serves the library HTTP API (see docs/library-standard.md) so that websites
//! and the ebook-converter CLI can list, get, put, and delete ebooks.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use ebook_converter_library_server::{api_routes, config::ServerConfig, AppState};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive("ebook_converter_library_server=info".parse().unwrap()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = ServerConfig::from_env();
    let state: AppState = AppState::new(config).await;

    let app = api_routes(state.clone()).layer(tower_http::cors::CorsLayer::permissive());

    let addr = state.config.bind_addr();
    tracing::info!("Library server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
