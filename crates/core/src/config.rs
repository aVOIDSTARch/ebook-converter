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
    #[serde(default = "default_library_format")]
    pub format: String,
    pub output_dir: Option<String>,
    #[serde(default = "default_library_template")]
    pub template: String,
}

fn default_library_format() -> String {
    "epub3".to_string()
}
fn default_library_template() -> String {
    "{author} - {title}.{ext}".to_string()
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
    let config_path = match dirs::config_dir() {
        Some(mut p) => {
            p.push("ebook-converter");
            p.push("config.toml");
            p
        }
        None => return AppConfig::default(),
    };

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return AppConfig::default(),
    };

    match toml::from_str::<AppConfig>(&content) {
        Ok(cfg) => cfg,
        Err(_) => AppConfig::default(),
    }
}

/// Return the default config file path (for init and show).
pub fn config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|mut p| {
        p.push("ebook-converter");
        p.push("config.toml");
        p
    })
}
