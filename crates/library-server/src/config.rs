//! Server configuration (library path, bind address).

use std::path::PathBuf;

/// Configuration for the library server.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// Directory where ebooks are stored (and scanned for list).
    pub library_path: PathBuf,
    /// Host:port to bind (e.g. "127.0.0.1:3030" or "0.0.0.0:3030").
    pub bind: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            library_path: dirs::data_local_dir()
                .map(|p| p.join("ebook-converter").join("library"))
                .unwrap_or_else(|| PathBuf::from("./library")),
            bind: "127.0.0.1:3030".to_string(),
        }
    }
}

impl ServerConfig {
    /// Build config from environment (and defaults).
    /// - `EBOOK_LIBRARY_PATH`: directory for ebooks (default: platform data dir or ./library)
    /// - `EBOOK_LIBRARY_BIND`: host:port (default: 127.0.0.1:3030)
    pub fn from_env() -> Self {
        let mut c = Self::default();
        if let Ok(p) = std::env::var("EBOOK_LIBRARY_PATH") {
            c.library_path = PathBuf::from(p);
        }
        if let Ok(b) = std::env::var("EBOOK_LIBRARY_BIND") {
            c.bind = b;
        }
        c
    }

    pub fn bind_addr(&self) -> &str {
        &self.bind
    }
}
