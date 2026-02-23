//! Ebook library HTTP server implementing the library standard.
//!
//! This server exposes a REST API that the ebook-converter project's
//! HttpLibrary adapter (and any website) can use to list, get, put, and
//! delete ebooks. Storage is directory-backed; metadata is read from
//! EPUB/TXT when possible via ebook-converter-core.

pub mod api;
pub mod config;
pub mod storage;

use std::sync::Arc;

use axum::Router;

use crate::config::ServerConfig;
use crate::storage::DirStore;

/// Shared application state (storage and config).
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<DirStore>,
    pub config: ServerConfig,
}

impl AppState {
    pub async fn new(config: ServerConfig) -> Self {
        std::fs::create_dir_all(&config.library_path).ok();
        let store = Arc::new(DirStore::new(config.library_path.clone()));
        Self { store, config }
    }
}

/// Build API routes (under /api) with state.
pub fn api_routes(state: AppState) -> Router {
    api::routes(state)
}
