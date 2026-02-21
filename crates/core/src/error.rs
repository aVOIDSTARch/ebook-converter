/// Top-level error type. All public API functions return this.
#[derive(Debug, thiserror::Error)]
pub enum EbookError {
    #[error("Format detection failed: {0}")]
    Detect(#[from] DetectError),

    #[error("Read error: {0}")]
    Read(#[from] ReadError),

    #[error("Write error: {0}")]
    Write(#[from] WriteError),

    #[error("Validation error: {0}")]
    Validate(#[from] ValidateError),

    #[error("Repair error: {0}")]
    Repair(#[from] RepairError),

    #[error("Optimization error: {0}")]
    Optimize(#[from] OptimizeError),

    #[error("Security violation: {0}")]
    Security(#[from] SecurityError),

    #[error("Metadata lookup error: {0}")]
    Lookup(#[from] LookupError),

    #[error("Title format error: {0}")]
    Format(#[from] FormatError),

    #[error("Merge error: {0}")]
    Merge(#[from] MergeError),

    #[error("Split error: {0}")]
    Split(#[from] SplitError),

    #[error("Metadata edit error: {0}")]
    Meta(#[from] MetaError),

    #[error("Duplicate detection error: {0}")]
    Dedup(#[from] DedupError),

    #[error("Transform error: {0}")]
    Transform(#[from] TransformError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("ZIP bomb detected: decompression ratio {ratio}:1 exceeds limit {limit}:1")]
    ZipBomb { ratio: u64, limit: u64 },

    #[error("Path traversal detected in archive entry: {path}")]
    PathTraversal { path: String },

    #[error("Archive contains {count} files, exceeding limit of {limit}")]
    TooManyFiles { count: u64, limit: u64 },

    #[error("Resource {name} is {size_mb}MB, exceeding limit of {limit_mb}MB")]
    OversizedResource {
        name: String,
        size_mb: u64,
        limit_mb: u64,
    },

    #[error("XML/HTML nesting depth {depth} exceeds limit of {limit}")]
    ExcessiveNesting { depth: u32, limit: u32 },

    #[error("Parse timeout after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("DRM protected file ({drm_type} on {format})")]
    DrmProtected { format: String, drm_type: String },
}

#[derive(Debug, thiserror::Error)]
pub enum DetectError {
    #[error("Could not determine format: {0}")]
    Unknown(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Malformed {format} file: {detail}")]
    MalformedFile { format: String, detail: String },

    #[error("Missing required content: {0}")]
    MissingContent(String),

    #[error(transparent)]
    Security(#[from] SecurityError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("Write failed for format {format}: {detail}")]
    WriteFailed { format: String, detail: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ValidateError {
    #[error("Validation failed: {0}")]
    Failed(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum RepairError {
    #[error("Repair failed: {0}")]
    Failed(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum OptimizeError {
    #[error("Optimization failed: {0}")]
    Failed(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Provider {provider} returned error: {message}")]
    ProviderError { provider: String, message: String },

    #[error("No results found for query")]
    NotFound,

    #[error("Rate limited by {provider}, retry after {retry_after_ms}ms")]
    RateLimited {
        provider: String,
        retry_after_ms: u64,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("Invalid format string: {0}")]
    InvalidFormatString(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

#[derive(Debug, thiserror::Error)]
pub enum MergeError {
    #[error("Merge failed: {0}")]
    Failed(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum SplitError {
    #[error("Split failed: {0}")]
    Failed(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum MetaError {
    #[error("Metadata operation failed: {0}")]
    Failed(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DedupError {
    #[error("Duplicate detection failed: {0}")]
    Failed(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    #[error("Transform '{name}' failed: {detail}")]
    Failed { name: String, detail: String },
}
