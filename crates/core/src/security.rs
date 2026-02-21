//! Security hardening: ZIP bomb protection, path traversal guards, resource limits.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityLimits {
    /// Maximum decompression ratio before flagging as ZIP bomb.
    pub max_compression_ratio: u64,
    /// Maximum number of files allowed in an archive.
    pub max_file_count: u64,
    /// Maximum size of a single resource in bytes.
    pub max_resource_size_bytes: u64,
    /// Maximum total decompressed size in bytes.
    pub max_total_size_bytes: u64,
    /// Maximum XML/HTML nesting depth.
    pub max_nesting_depth: u32,
    /// Maximum parse time in seconds.
    pub max_parse_seconds: u64,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_compression_ratio: 100,
            max_file_count: 10_000,
            max_resource_size_bytes: 200 * 1024 * 1024, // 200 MB
            max_total_size_bytes: 1024 * 1024 * 1024,   // 1 GB
            max_nesting_depth: 200,
            max_parse_seconds: 300,
        }
    }
}
