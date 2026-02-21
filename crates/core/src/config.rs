//! Config file parsing for `~/.config/ebook-converter/config.toml`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub library: LibraryConfig,
    #[serde(default)]
    pub lookup: LookupConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub encoding: EncodingConfig,
    #[serde(default)]
    pub watch: WatchConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            library: LibraryConfig::default(),
            lookup: LookupConfig::default(),
            security: SecurityConfig::default(),
            encoding: EncodingConfig::default(),
            watch: WatchConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    pub format: String,
    pub output_dir: Option<String>,
    pub template: String,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            format: "epub3".to_string(),
            output_dir: None,
            template: "{author} - {title}.{ext}".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LookupConfig {
    pub default_provider: Option<String>,
    pub cache_dir: Option<String>,
    pub cache_ttl_hours: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub max_file_size_mb: Option<u64>,
    pub max_compression_ratio: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodingConfig {
    pub unicode_form: String,
    pub smart_quotes: bool,
    pub normalize_ligatures: bool,
    pub fix_macos_nfd: bool,
}

impl Default for EncodingConfig {
    fn default() -> Self {
        Self {
            unicode_form: "NFC".to_string(),
            smart_quotes: false,
            normalize_ligatures: false,
            fix_macos_nfd: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WatchConfig {
    pub debounce_ms: Option<u64>,
    pub ignored_patterns: Vec<String>,
}

/// Load config from the default path (`~/.config/ebook-converter/config.toml`).
pub fn load_config() -> AppConfig {
    // TODO: Load from file, falling back to defaults
    AppConfig::default()
}
