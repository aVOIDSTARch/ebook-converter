# Ebook Converter — Project Plan v1

## Table of Contents

- [Ebook Converter — Project Plan v1](#ebook-converter--project-plan-v1)
  - [Table of Contents](#table-of-contents)
  - [1. Overview](#1-overview)
  - [2. Language Choice](#2-language-choice)
  - [3. Architecture](#3-architecture)
  - [4. Supported Formats](#4-supported-formats)
    - [Phase 1 (MVP)](#phase-1-mvp)
    - [Phase 2](#phase-2)
    - [Phase 3](#phase-3)
  - [5. Core Modules](#5-core-modules)
    - [5.1 Format Detector](#51-format-detector)
    - [5.2 Reader Pipeline](#52-reader-pipeline)
    - [5.3 Intermediate Representation (`Document`)](#53-intermediate-representation-document)
    - [5.4 Writer Pipeline](#54-writer-pipeline)
    - [5.5 Validator](#55-validator)
    - [5.6 Repair Engine](#56-repair-engine)
    - [5.7 Optimizer](#57-optimizer)
  - [6. Public API Surface](#6-public-api-surface)
    - [Rust (native)](#rust-native)
    - [C-ABI (for FFI consumers)](#c-abi-for-ffi-consumers)
    - [WASM (for browser / Node.js)](#wasm-for-browser--nodejs)
  - [7. CLI Interface](#7-cli-interface)
  - [8. File Validation \& Repair](#8-file-validation--repair)
    - [Validation Checks](#validation-checks)
    - [Repair Actions](#repair-actions)
  - [9. Size Optimization](#9-size-optimization)
  - [10. Portability \& Integration](#10-portability--integration)
    - [Compile Targets](#compile-targets)
    - [Language Bindings (generated from C-ABI)](#language-bindings-generated-from-c-abi)
    - [Integration Patterns](#integration-patterns)
  - [11. Project Structure](#11-project-structure)
  - [12. Dependency Strategy](#12-dependency-strategy)
  - [13. Build \& Distribution](#13-build--distribution)
  - [14. Implementation Phases](#14-implementation-phases)
    - [Phase 1 — Foundation (Core IR + EPUB + TXT)](#phase-1--foundation-core-ir--epub--txt)
    - [Phase 2 — HTML, Markdown, Repair, Optimize](#phase-2--html-markdown-repair-optimize)
    - [Phase 3 — PDF + Proprietary Formats](#phase-3--pdf--proprietary-formats)
    - [Phase 4 — Bindings \& Distribution](#phase-4--bindings--distribution)
  - [15. Open Questions](#15-open-questions)

---

## 1. Overview

A library-first ebook toolkit that converts, validates, repairs, and optimizes ebook files across common formats. Designed as a portable core library with C-ABI FFI bindings so any language (Python, Node, Swift, C#, etc.) can call it, plus a standalone CLI for direct use.

---

## 2. Language Choice

**Recommendation: Rust**

| Criteria | Rust | Go |
|---|---|---|
| Binary size | Small, static | Larger (runtime) |
| FFI / C-ABI exports | First-class (`#[no_mangle]`, cbindgen) | Requires cgo, less ergonomic |
| WASM target | Excellent (wasm32 target built-in) | Experimental |
| Memory safety | Compile-time guarantees | GC-based |
| Cross-compile | Straightforward via `cross` crate | Built-in but FFI complicates it |
| Ebook ecosystem | `epub-builder`, `pdf-rs`, `lopdf` | Fewer mature crates |

Rust wins on FFI ergonomics and WASM portability — the two pillars of "fast and portable use by many applications."

---

## 3. Architecture

```
┌─────────────────────────────────────────────────┐
│                  Consumers                      │
│  CLI  │  Python  │  Node  │  Swift  │  WASM/Web │
└───┬───┴────┬─────┴───┬────┴───┬────┴─────┬─────┘
    │        │         │        │          │
    ▼        ▼         ▼        ▼          ▼
┌─────────────────────────────────────────────────┐
│              C-ABI / FFI Boundary               │
│         (libebook_converter.so/.dylib/.dll)      │
└───────────────────┬─────────────────────────────┘
                    ▼
┌─────────────────────────────────────────────────┐
│              ebook-converter-core               │
│                                                 │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐│
│  │  Reader   │ │  Writer  │ │   Validator      ││
│  │ (parse)   │ │ (emit)   │ │   & Repair       ││
│  └────┬──────┘ └────┬─────┘ └────┬─────────────┘│
│       │             │            │               │
│       ▼             ▼            ▼               │
│  ┌──────────────────────────────────────────┐   │
│  │         Intermediate Representation      │   │
│  │  (unified document model: chapters,      │   │
│  │   metadata, TOC, images, styles)         │   │
│  └──────────────────────────────────────────┘   │
│                                                 │
│  ┌──────────┐ ┌──────────────┐                  │
│  │ Optimizer│ │  Format      │                  │
│  │ (shrink) │ │  Detector    │                  │
│  └──────────┘ └──────────────┘                  │
└─────────────────────────────────────────────────┘
```

The core concept: **every format is parsed into a single Intermediate Representation (IR), then emitted to the target format.** This means N readers + M writers = N×M conversions without N×M codepaths.

---

## 4. Supported Formats

### Phase 1 (MVP)
| Format | Read | Write |
|--------|------|-------|
| EPUB (.epub) | Yes | Yes |
| Plain Text (.txt) | Yes | Yes |

### Phase 2
| Format | Read | Write |
|--------|------|-------|
| HTML (.html) | Yes | Yes |
| Markdown (.md) | Yes | Yes |
| SSML (.ssml) | No | Yes (TTS-optimized output) |

### Phase 3
| Format | Read | Write |
|--------|------|-------|
| PDF (.pdf) | Yes | Yes |
| MOBI (.mobi/.azw) | Yes | No (Amazon proprietary) |
| AZW3 / KF8 | Yes | No |

### Phase 4
| Format | Read | Write |
|--------|------|-------|
| DOCX (.docx) | Yes | Yes |
| FB2 (.fb2) | Yes | Yes |
| CBZ/CBR (comics) | Yes | Yes |

---

## 5. Core Modules

### 5.1 Format Detector

Magic-byte detection (not just file extension) with MIME type reporting and confidence scoring.

**Magic bytes table:**
| Format | Magic Bytes / Signature | Offset | Notes |
|--------|------------------------|--------|-------|
| EPUB | `PK` (ZIP) + `mimetype` entry = `application/epub+zip` | 0 | Check ZIP, then first entry |
| PDF | `%PDF-` | 0 | |
| MOBI/PRC | `BOOKMOBI` | 60 | PDB header at offset 60 |
| AZW3/KF8 | `BOOKMOBI` + KF8 header record | 60 | Same PDB, different internal record |
| ZIP (generic) | `PK\x03\x04` | 0 | Fallback for EPUB detection |
| GZIP | `\x1f\x8b` | 0 | For compressed inputs |
| HTML | `<!DOCTYPE html` or `<html` | 0 (skip BOM/whitespace) | Case-insensitive |
| Markdown | No magic bytes | — | Detect by extension + heuristic (headings, links) |
| Plain Text | No magic bytes | — | Fallback: valid UTF-8 with no binary bytes |
| FB2 | `<?xml` + `<FictionBook` | 0 | XML-based |
| DOCX | `PK` (ZIP) + `[Content_Types].xml` with word MIME | 0 | ZIP-based Office format |
| CBZ | `PK` (ZIP) + image files only | 0 | ZIP of images |

**Detection flow:**
1. Read first 4KB of file
2. Check magic bytes against table (fast path)
3. If ZIP: inspect archive contents to disambiguate EPUB vs DOCX vs CBZ
4. If no magic match: try extension, then content heuristics
5. Return `DetectResult { format: Format, confidence: f64, mime_type: &str }`

**DRM detection** (checked during read, before parsing content):
- EPUB: look for `META-INF/encryption.xml` with Adobe/Apple DRM namespace URIs
- MOBI/AZW: check DRM flag in PDB header (byte offset 0x0C)
- If DRM detected: return `Err(ConvertError::DrmProtected { format, drm_type })` — never attempt to process

### 5.2 Reader Pipeline

Each format implements a `Reader` trait. Readers support both in-memory and streaming input to handle large files.

```rust
pub trait FormatReader: Send + Sync {
    /// Check if this reader can handle the given input. Called with first 4KB.
    fn detect(header: &[u8]) -> DetectResult;

    /// Read from a byte source into the IR. Uses `Read + Seek` for streaming.
    fn read<R: std::io::Read + std::io::Seek>(
        input: R,
        opts: &ReadOptions,
        progress: Option<&dyn ProgressHandler>,
    ) -> Result<Document, ConvertError>;
}

pub struct ReadOptions {
    pub security: SecurityLimits,       // from config or defaults
    pub extract_cover: bool,            // default: true
    pub parse_toc: bool,                // default: true
    pub encoding: EncodingOptions,      // from config or defaults
}
```

**EPUB version handling:**
- Reader auto-detects EPUB2 vs EPUB3 from the OPF `<package version="...">` attribute
- EPUB2: parse NCX for navigation, Dublin Core metadata from OPF
- EPUB3: parse NAV document for navigation, extended metadata properties
- Both versions produce the same `Document` IR — version is stored as `epub_version: Option<EpubVersion>` in metadata for the writer to use
- Writer defaults to EPUB3 output; `--epub-version 2` flag forces EPUB2 output (downgrades NAV → NCX, strips EPUB3-only features)

### 5.3 Intermediate Representation (`Document`)

The unified document model. Every format is parsed into this struct, and every writer emits from it.

```rust
pub struct Document {
    pub metadata: Metadata,
    pub toc: Vec<TocEntry>,                // table of contents (nested tree)
    pub content: Vec<Chapter>,             // ordered chapters/sections
    pub resources: ResourceMap,            // embedded images, fonts, stylesheets
    pub text_direction: TextDirection,     // document-level default
    pub epub_version: Option<EpubVersion>, // if read from EPUB, tracks 2 vs 3
}

pub struct Metadata {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,              // ordered, first = primary
    pub language: Option<String>,          // BCP 47 language tag (e.g., "en-US")
    pub publisher: Option<String>,
    pub publish_date: Option<String>,      // ISO 8601 or just year
    pub isbn_10: Option<String>,
    pub isbn_13: Option<String>,
    pub description: Option<String>,
    pub subjects: Vec<String>,             // genres/tags
    pub series: Option<SeriesInfo>,
    pub cover_image_id: Option<String>,    // references ResourceMap key
    pub page_count: Option<u32>,
    pub rights: Option<String>,            // copyright notice
    pub custom: HashMap<String, String>,   // format-specific metadata overflow
}

pub struct SeriesInfo {
    pub name: String,
    pub position: Option<f32>,             // e.g., 3.0, or 1.5 for novellas
}

pub struct TocEntry {
    pub title: String,
    pub href: String,                      // chapter ref + optional fragment
    pub children: Vec<TocEntry>,           // nested sub-entries
}

pub struct Chapter {
    pub id: String,                        // unique identifier
    pub title: Option<String>,
    pub content: Vec<ContentNode>,
    pub text_direction: Option<TextDirection>, // per-chapter override
}

/// Content nodes — the core building blocks of document content.
pub enum ContentNode {
    Paragraph {
        children: Vec<InlineNode>,
    },
    Heading {
        level: u8,                         // 1-6
        children: Vec<InlineNode>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<ContentNode>>,      // each item can contain block-level content
    },
    Table {
        headers: Vec<Vec<InlineNode>>,
        rows: Vec<Vec<Vec<InlineNode>>>,
    },
    BlockQuote {
        children: Vec<ContentNode>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Image {
        resource_id: String,               // references ResourceMap key
        alt_text: Option<String>,
        caption: Option<String>,
    },
    HorizontalRule,
    RawHtml(String),                       // passthrough for format-specific content
}

pub enum InlineNode {
    Text(String),
    Emphasis(Vec<InlineNode>),             // italic
    Strong(Vec<InlineNode>),               // bold
    Code(String),                          // inline code
    Link { href: String, children: Vec<InlineNode> },
    Superscript(Vec<InlineNode>),
    Subscript(Vec<InlineNode>),
    Ruby { base: String, annotation: String }, // for CJK furigana
    LineBreak,
}

pub enum TextDirection { Ltr, Rtl, Auto }
pub enum EpubVersion { V2, V3 }

pub struct ResourceMap {
    resources: HashMap<String, Resource>,  // keyed by unique ID
}

pub struct Resource {
    pub id: String,
    pub media_type: String,                // MIME type
    pub data: Vec<u8>,                     // raw bytes
    pub filename: Option<String>,          // original filename if known
}
```

**Computed statistics** (lazily calculated, cached — not stored in struct, computed on demand):
```rust
pub struct DocumentStats {
    pub word_count: u64,
    pub character_count: u64,
    pub sentence_count: u64,
    pub chapter_count: u32,
    pub image_count: u32,
    pub resource_size_bytes: u64,
    pub estimated_reading_time_minutes: f32,  // at configurable WPM (default 250)
    pub flesch_kincaid_grade: Option<f32>,     // None for non-English text
}

impl Document {
    pub fn stats(&self) -> DocumentStats { /* ... */ }
}
```

### 5.4 Writer Pipeline

Each format implements a `Writer` trait. Writers support both in-memory and streaming output.

```rust
pub trait FormatWriter: Send + Sync {
    /// Write the document to a byte sink. Uses `Write` for streaming.
    fn write<W: std::io::Write>(
        doc: &Document,
        output: W,
        opts: &WriteOptions,
        progress: Option<&dyn ProgressHandler>,
    ) -> Result<(), ConvertError>;
}

pub struct WriteOptions {
    pub image_quality: u8,                 // 1-100, default 80
    pub epub_version: Option<EpubVersion>, // force EPUB2 or EPUB3 (default: EPUB3)
    pub embed_fonts: bool,                 // default: true
    pub minify: bool,                      // minify HTML/CSS in output, default: false
    pub transforms: Vec<Box<dyn Transform>>, // transforms to apply before writing
}
```

### 5.5 Validator

Per-format structural validation.

```rust
pub fn validate(doc: &Document, opts: &ValidateOptions) -> Vec<ValidationIssue>;
pub fn validate_file<R: Read + Seek>(input: R, opts: &ValidateOptions) -> Result<Vec<ValidationIssue>, ConvertError>;

pub struct ValidateOptions {
    pub strict: bool,                      // treat warnings as errors
    pub accessibility: bool,               // run accessibility checks (5.18)
    pub wcag_level: WcagLevel,             // A, AA, AAA (default: AA)
}

pub struct ValidationIssue {
    pub severity: Severity,                // Error, Warning, Info
    pub code: &'static str,               // machine-readable code, e.g. "EPUB-001"
    pub message: String,                   // human-readable description
    pub location: Option<String>,          // file/chapter/line if applicable
    pub auto_fixable: bool,                // can the repair engine fix this?
}

pub enum Severity { Error, Warning, Info }
pub enum WcagLevel { A, Aa, Aaa }
```

### 5.6 Repair Engine

Auto-fix common issues. Each repair action maps to a `ValidationIssue.code`.

```rust
pub fn repair(doc: &mut Document, opts: &RepairOptions) -> RepairReport;

pub struct RepairOptions {
    pub fix_metadata: bool,                // fill missing required fields with defaults
    pub fix_links: bool,                   // remove or remap broken internal links
    pub fix_xml: bool,                     // attempt to repair malformed XML/HTML
    pub fix_encoding: bool,                // apply encoding normalization (5.20)
    pub generate_toc: bool,                // generate TOC from heading structure if missing
    pub fix_zip: bool,                     // rebuild ZIP structure if damaged
}

pub struct RepairReport {
    pub fixes_applied: Vec<RepairAction>,
    pub fixes_failed: Vec<(RepairAction, String)>, // action + failure reason
    pub issues_remaining: Vec<ValidationIssue>,
}

pub struct RepairAction {
    pub code: &'static str,                // maps to ValidationIssue.code
    pub description: String,
}
```

Repairs are applied in a transaction-like manner: the document is cloned before repair. If any critical repair fails, the original is preserved and the error is reported.

### 5.7 Error Type Hierarchy

All errors use `thiserror` and are organized by module. The top-level `EbookError` wraps all module errors.

```rust
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

/// Security-specific errors — always halt processing.
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("ZIP bomb detected: decompression ratio {ratio}:1 exceeds limit {limit}:1")]
    ZipBomb { ratio: u64, limit: u64 },

    #[error("Path traversal detected in archive entry: {path}")]
    PathTraversal { path: String },

    #[error("Archive contains {count} files, exceeding limit of {limit}")]
    TooManyFiles { count: u64, limit: u64 },

    #[error("Resource {name} is {size_mb}MB, exceeding limit of {limit_mb}MB")]
    OversizedResource { name: String, size_mb: u64, limit_mb: u64 },

    #[error("XML/HTML nesting depth {depth} exceeds limit of {limit}")]
    ExcessiveNesting { depth: u32, limit: u32 },

    #[error("Parse timeout after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("DRM protected file ({drm_type} on {format})")]
    DrmProtected { format: String, drm_type: String },
}

/// Read-specific errors.
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

/// Lookup-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Provider {provider} returned error: {message}")]
    ProviderError { provider: String, message: String },

    #[error("No results found for query")]
    NotFound,

    #[error("Rate limited by {provider}, retry after {retry_after_ms}ms")]
    RateLimited { provider: String, retry_after_ms: u64 },
}
```

Other error types (`WriteError`, `ValidateError`, `RepairError`, `OptimizeError`, `FormatError`, `MergeError`, `SplitError`, `MetaError`, `DedupError`, `TransformError`) follow the same pattern: descriptive variants with structured fields, `thiserror` derives, and `#[from]` conversions for common sub-errors like `std::io::Error`.

### 5.8 Optimizer
- Image downscaling / recompression (configurable quality, JPEG/PNG only)
- Font subsetting (remove unused glyphs)
- Strip unnecessary metadata
- Minify embedded HTML/CSS
- Deduplicate embedded resources

### 5.8 Title Formatter
A filename templating engine that parses filenames into semantic parts and reassembles them using a format string.

**Format string syntax** — uses `{placeholder}` tokens (Python/Rust `str::format` style, universally understood):

| Token | Source | Example |
|-------|--------|---------|
| `{title}` | Book title from metadata or filename | `The Great Gatsby` |
| `{author}` | Author name | `F. Scott Fitzgerald` |
| `{author_last}` | Author surname only | `Fitzgerald` |
| `{author_first}` | Author first name only | `F. Scott` |
| `{series}` | Series name | `Harry Potter` |
| `{series_num}` | Series number | `3` |
| `{year}` | Publication year | `1925` |
| `{ext}` | Target file extension | `epub` |
| `{isbn}` | ISBN if available | `9780743273565` |
| `{lang}` | Language code | `en` |

**Modifiers** — applied with pipe syntax inside braces:

| Modifier | Effect | Example |
|----------|--------|---------|
| `lower` | Lowercase | `{title\|lower}` → `the great gatsby` |
| `upper` | Uppercase | `{title\|upper}` → `THE GREAT GATSBY` |
| `snake` | Snake_case | `{title\|snake}` → `the_great_gatsby` |
| `kebab` | Kebab-case | `{title\|kebab}` → `the-great-gatsby` |
| `camel` | camelCase | `{title\|camel}` → `theGreatGatsby` |
| `trim` | Remove leading/trailing whitespace | `{title\|trim}` |
| `truncN` | Truncate to N chars | `{title\|trunc30}` |

**Part extraction from raw filenames** — when metadata isn't available, the formatter parses common filename conventions:
- `Author - Title.epub`
- `Title (Series #3).epub`
- `Author - Series 03 - Title.epub`
- `Title [Year].epub`
- `Last, First - Title.epub`

**Rust API:**
```rust
pub fn format_title(
    filename: &str,
    format_str: &str,
    metadata: Option<&Metadata>,
) -> Result<String, FormatError>;
```

When `metadata` is provided (e.g., during conversion), values come from the document IR. When `None`, values are parsed from the input filename string.

### 5.9 Metadata Lookup

A pluggable, provider-based module that queries book metadata APIs to enrich a document. The active provider is set in `config.toml` (`lookup.provider`) and can be overridden per-call with `--provider <name>`.

**Architecture:**
```rust
/// Each provider implements this trait.
pub trait MetadataProvider: Send + Sync {
    fn name(&self) -> &str;
    fn search(&self, query: &MetadataQuery) -> Result<Vec<MetadataResult>, LookupError>;
    fn lookup_isbn(&self, isbn: &str) -> Result<MetadataResult, LookupError>;
    fn fetch_cover(&self, result: &MetadataResult) -> Result<Option<Vec<u8>>, LookupError>;
}
```

**Built-in providers:**

| Provider | API Key | Coverage | Default |
|----------|---------|----------|---------|
| Open Library | Not required | Broad, community-driven | Yes |
| Google Books | Optional (higher quota with key) | Very broad | No |
| Custom | User-defined in config | User-controlled | No |

Users select the active provider in `config.toml` or override per-command:
```bash
ebook-converter lookup book.epub                         # uses config default (openlibrary)
ebook-converter lookup book.epub --provider google_books # override for this call
```

**Lookup strategy (waterfall, provider-agnostic):**
1. If ISBN exists in metadata → exact ISBN lookup
2. If title+author exist → search query
3. If only title exists → title search with fuzzy matching
4. If only filename → parse filename (via 5.8), then search

**What gets populated:**
| Field | Source | Overwrite existing? |
|-------|--------|---------------------|
| Title | API | No (only if missing) |
| Author(s) | API | No |
| ISBN-10/13 | API | Yes (fill if missing) |
| Description/Summary | API | Yes (fill if missing) |
| Cover image | API (downloads image) | Yes (fill if missing) |
| Subjects/Tags | API | Merge with existing |
| Publisher | API | No |
| Publish year | API | No |
| Series name/number | API | Yes (fill if missing) |
| Page count | API | Yes (fill if missing) |

**Behavior:**
- Always opt-in via `--lookup` flag — never phones home without explicit request
- Caches API responses locally (`~/.cache/ebook-converter/lookup/`) keyed by provider + query
- When multiple results returned, picks the best match by similarity score
- Reports what was found/updated via structured output
- Respects per-provider `rate_limit_ms` to avoid throttling

**Rust API:**
```rust
pub async fn lookup_metadata(
    query: &MetadataQuery,
    provider: &dyn MetadataProvider,
    opts: &LookupOptions,
) -> Result<MetadataResult, LookupError>;

pub fn enrich_document(
    doc: &mut Document,
    result: &MetadataResult,
    policy: &EnrichPolicy,  // which fields to overwrite
) -> EnrichReport;
```

### 5.10 Library Copy & Configuration

A feature that outputs an additional metadata-enriched copy in a canonical format to a library directory, alongside the primary conversion output.

**Config file: `~/.config/ebook-converter/config.toml`**
```toml
[library]
# Directory where library copies are stored
path = "~/Books/library"

# Format for library copies (default: epub3)
format = "epub3"

# Auto-lookup metadata when creating library copies (default: true)
auto_lookup = true

# Filename template for library copies
naming = "{author_last}, {author_first} - {title}.{ext}"

# Organize into subdirectories
organize_by = "author"  # "author" | "genre" | "year" | "flat"

[lookup]
# Active provider (use this for all lookups)
provider = "openlibrary"

# Cache directory for API responses
cache_dir = "~/.cache/ebook-converter/lookup"

# Cache TTL in days (0 = forever)
cache_ttl = 30

# --- Provider configurations ---
# Each provider has its own section. Add new providers by creating a
# [lookup.providers.<name>] block. The "provider" field above selects
# which one is active.

[lookup.providers.openlibrary]
name = "Open Library"
base_url = "https://openlibrary.org"
search_endpoint = "/search.json"
isbn_endpoint = "/isbn/{isbn}.json"
covers_url = "https://covers.openlibrary.org/b/id/{cover_id}-L.jpg"
api_key = ""                    # not required for Open Library
rate_limit_ms = 100             # minimum ms between requests

[lookup.providers.google_books]
name = "Google Books"
base_url = "https://www.googleapis.com/books/v1"
search_endpoint = "/volumes?q={query}"
isbn_endpoint = "/volumes?q=isbn:{isbn}"
covers_url = ""                 # included in response
api_key = ""                    # optional, higher quota with key
rate_limit_ms = 200

# Users can add custom providers:
# [lookup.providers.my_api]
# name = "My Custom API"
# base_url = "https://api.example.com"
# search_endpoint = "/books/search?q={query}"
# isbn_endpoint = "/books/isbn/{isbn}"
# covers_url = ""
# api_key = "my-secret-key"
# rate_limit_ms = 500
```

**Behavior of `--library-copy`:**
1. Perform the requested conversion as normal (e.g., EPUB → PDF)
2. Enrich the IR with metadata lookup (if `auto_lookup = true` or `--lookup` also passed)
3. Write a second copy in the configured library format (default EPUB3) to the library directory
4. Apply the library naming template to the output filename
5. Optionally organize into subdirectories based on `organize_by` setting

**Defaults (zero-config):**
- Format: EPUB3
- Directory: `~/Books/ebook-converter-library/`
- Naming: `{author} - {title}.{ext}`
- Organize: flat (no subdirectories)
- Auto-lookup: true

The config file is created on first use of `--library-copy` with sensible defaults if it doesn't exist. Users can also run `ebook-converter config init` to generate a commented config file.

### 5.11 Merge & Split

Operations to combine or divide ebooks at the document level.

**Split:**
- Split a single ebook into multiple files by chapter, part, or custom delimiter
- Each output file inherits the parent's metadata (with updated title: "Book Title — Chapter N")
- Preserves TOC entries for the extracted portion
- Configurable split strategy: `chapter` (default), `part`, `heading-level-N`, `page-count-N`

**Merge:**
- Combine multiple ebooks into a single document
- Concatenate in file-argument order
- Auto-generate a unified TOC from individual TOCs
- Metadata taken from first file (or overridden via flags)
- Handle conflicting styles/CSS by namespacing

**Rust API:**
```rust
pub fn split(doc: &Document, strategy: SplitStrategy) -> Result<Vec<Document>, SplitError>;
pub fn merge(docs: &[Document], opts: &MergeOptions) -> Result<Document, MergeError>;
```

### 5.12 Metadata Editor

Standalone metadata read/write/strip operations without format conversion.

**Operations:**
- `get` — read specific metadata fields
- `set` — write/overwrite specific fields (title, author, description, cover, ISBN, etc.)
- `strip` — remove all metadata (for privacy) or remove specific fields
- `copy` — copy metadata from one ebook to another

**Rust API:**
```rust
pub fn meta_get(doc: &Document, field: &str) -> Option<MetadataValue>;
pub fn meta_set(doc: &mut Document, field: &str, value: MetadataValue) -> Result<(), MetaError>;
pub fn meta_strip(doc: &mut Document, fields: Option<&[&str]>) -> StrippedReport;
```

### 5.13 Duplicate Detection

Identifies duplicate or near-duplicate books in a directory or library.

**Detection strategies (layered, fast to slow):**
1. **Exact hash** — SHA-256 of file bytes (catches identical files)
2. **ISBN match** — same ISBN across different files/formats
3. **Fuzzy metadata** — title + author similarity score (using `strsim`) above configurable threshold (default 0.85)
4. **Content fingerprint** — hash of first N paragraphs of text content (catches reformatted duplicates)

**Output:**
- Groups of duplicates with match reason and confidence
- `--json` output for programmatic consumption
- `--interactive` mode for CLI to choose which to keep

**Rust API:**
```rust
pub fn find_duplicates(
    paths: &[PathBuf],
    strategy: DuplicateStrategy,
    threshold: f64,
) -> Result<Vec<DuplicateGroup>, DupError>;
```

### 5.14 Security Hardening

Safety measures for processing untrusted ebook files. Critical for a library that other applications embed.

**Protections:**
| Threat | Mitigation | Default Limit |
|--------|-----------|---------------|
| ZIP bomb | Track decompression ratio, abort if ratio > threshold | 100:1 ratio, 1GB max |
| Path traversal | Sanitize all ZIP entry paths, reject `..` components | Always on |
| Resource exhaustion | Max file count in archive | 10,000 files |
| Oversized resources | Max single resource size | 100MB per resource |
| Malformed XML/HTML | Use memory-bounded parsers, abort on excessive nesting | 256 nesting depth |
| Infinite loops | Timeout on all parsing operations | 60s per file |

**Config:**
```toml
[security]
max_decompress_ratio = 100
max_decompressed_size_mb = 1024
max_file_count = 10000
max_resource_size_mb = 100
max_parse_depth = 256
parse_timeout_secs = 60
```

All limits are configurable but ship with safe defaults. Violations produce `SecurityError` with a clear description of what was blocked and why.

### 5.15 Progress Reporting

Callback-based progress reporting for long-running operations. Essential for FFI consumers building GUIs.

**Design:**
```rust
/// Progress callback trait — implementors receive updates during operations.
pub trait ProgressHandler: Send {
    fn on_progress(&self, event: ProgressEvent);
}

pub struct ProgressEvent {
    pub operation: &'static str,  // "reading", "writing", "optimizing", etc.
    pub current: u64,             // bytes processed / items completed
    pub total: Option<u64>,       // total bytes / items (None if unknown)
    pub message: Option<String>,  // human-readable status
}
```

**Integration:**
- All public API functions accept an optional `&dyn ProgressHandler`
- CLI uses this to drive a terminal progress bar (`indicatif` crate)
- FFI exposes a C function pointer callback: `typedef void (*progress_cb)(const char* op, uint64_t current, uint64_t total);`
- WASM exposes a JS callback

### 5.16 Watch Mode

Directory monitoring that auto-processes new/modified ebook files using a configured pipeline.

**Design:**
```bash
ebook-converter watch ./inbox --format epub --outdir ./library/ --lookup --library-copy
```

**Behavior:**
- Uses filesystem events (not polling) via `notify` crate
- Processes new files that match a configurable glob filter (default: `*.{epub,pdf,mobi,txt,html,md}`)
- Applies the full pipeline: detect → read → (optional lookup) → convert → (optional library-copy) → write
- Moves/deletes processed originals based on config (`keep`, `move-to-processed`, `delete`)
- Logs all actions; `--json` streams newline-delimited JSON events
- Graceful shutdown on SIGINT

**Config:**
```toml
[watch]
filter = "*.{epub,pdf,mobi,txt,html,md}"
on_complete = "keep"  # "keep" | "move" | "delete"
move_to = "./processed"
debounce_ms = 1000
```

### 5.17 Plugin / Transform Hooks

A system for users and consumers to register custom transformations on the IR before writing.

**Design — composable transform functions:**
```rust
/// A transform receives a mutable Document and can modify it in any way.
pub trait Transform: Send + Sync {
    fn name(&self) -> &str;
    fn apply(&self, doc: &mut Document) -> Result<(), TransformError>;
}
```

**Built-in transforms (also serve as examples):**
| Transform | Description |
|-----------|-------------|
| `StripImages` | Remove all images from the document |
| `StripStyles` | Remove all CSS/styling |
| `InjectWatermark` | Add a watermark string to each chapter header/footer |
| `ReplaceFont` | Swap all font references to a specified font family |
| `NormalizeUnicode` | Apply NFC/NFD normalization across all text content |
| `SmartQuotes` | Convert straight quotes to curly quotes (or vice versa) |

**CLI usage:**
```bash
# Apply transforms via --transform flag (comma-separated, applied in order)
ebook-converter convert input.epub -o output.epub --transform strip-images,normalize-unicode
ebook-converter convert input.epub -o output.epub --transform "inject-watermark:text=REVIEW COPY"
```

**Programmatic (Rust):**
```rust
let mut doc = ebook_converter::read_file("book.epub")?;
doc.apply_transform(&StripImages)?;
doc.apply_transform(&NormalizeUnicode { form: UnicodeForm::Nfc })?;
let bytes = doc.write_to(Format::Epub, &WriteOptions::default())?;
```

### 5.18 Accessibility Validation

EPUB Accessibility 1.0 / WCAG compliance checking, increasingly a legal requirement for published ebooks.

**Checks:**
| Check | Severity | Standard |
|-------|----------|----------|
| All images have alt text | Error | WCAG 2.1 1.1.1 |
| Reading order is logical (DOM order matches visual) | Warning | EPUB Accessibility 1.0 |
| Language declared on document and content changes | Warning | WCAG 2.1 3.1.1/3.1.2 |
| Semantic markup used (headings, lists, tables) | Warning | EPUB Accessibility 1.0 |
| Color contrast (if CSS is parseable) | Info | WCAG 2.1 1.4.3 |
| Navigation (TOC) matches heading structure | Warning | EPUB Accessibility 1.0 |
| ARIA roles present where appropriate | Info | WAI-ARIA |
| Accessibility metadata in OPF | Warning | EPUB Accessibility 1.0 |

**CLI:**
```bash
ebook-converter validate book.epub --accessibility
ebook-converter validate book.epub --accessibility --wcag-level AA
```

### 5.19 RTL & CJK Text Handling

Explicit support for right-to-left scripts (Arabic, Hebrew, Persian) and CJK (Chinese, Japanese, Korean) text during format conversion.

**Concerns addressed:**
| Issue | Solution |
|-------|----------|
| Text direction loss on conversion | Preserve `dir="rtl"` / `writing-mode` in IR, propagate to all writers |
| Bidirectional text mixing (e.g., English in Arabic paragraph) | Preserve Unicode bidi control characters; generate proper `<bdo>` / `<bdi>` in HTML-based outputs |
| CJK line breaking | Respect line break opportunities per UAX #14 (Unicode Line Breaking Algorithm) |
| CJK punctuation spacing | Preserve fullwidth punctuation; don't normalize to ASCII |
| Ruby/Furigana annotations (Japanese) | Model as IR content node; render in formats that support it (EPUB3, HTML) |
| Vertical writing mode | Preserve in IR; apply in EPUB3/CSS (`writing-mode: vertical-rl`) |
| Font fallback | Detect CJK/RTL content and warn if no appropriate font is embedded |

Stored in the IR as a `text_direction` field on `Document` and individual content sections, so mixed-direction documents are supported.

### 5.20 Encoding Normalization

Explicit Unicode and character encoding handling as part of the repair/transform pipeline.

**Operations:**
| Operation | Description | Default |
|-----------|-------------|---------|
| UTF-8 enforcement | Convert all content to UTF-8, replacing invalid sequences | Always on |
| Unicode normalization | NFC or NFD form | NFC (configurable) |
| Smart quote conversion | `"` → `\u201c`/`\u201d`, `'` → `\u2018`/`\u2019` | Off (opt-in) |
| Ligature normalization | `ﬁ` → `fi`, `ﬂ` → `fl` | Off |
| Dash normalization | Hyphens, en-dashes, em-dashes consistency | Off |
| Whitespace normalization | Collapse runs, normalize line endings, strip BOM | On |
| macOS NFD filename fix | Normalize filenames in ZIP archives from NFD → NFC | On |

**Config:**
```toml
[encoding]
unicode_form = "NFC"          # "NFC" | "NFD" | "NFKC" | "NFKD" | "none"
smart_quotes = false
normalize_ligatures = false
normalize_dashes = false
normalize_whitespace = true
fix_macos_nfd = true
```

---

## 6. Public API Surface

### Rust (native)

```rust
use ebook_converter::prelude::*;
use std::fs::File;

// --- Core conversion ---
let input = File::open("book.epub")?;
let doc = ebook_converter::read(input, &ReadOptions::default())?;

let mut output = File::create("book.pdf")?;
ebook_converter::write::<PdfWriter>(&doc, &mut output, &WriteOptions::default())?;

// Convenience: file-path based (reads format from extension)
let doc = ebook_converter::read_file("book.epub", &ReadOptions::default())?;
ebook_converter::write_file(&doc, "book.pdf", &WriteOptions::default())?;

// --- Format detection ---
let result = ebook_converter::detect_file("mystery-file")?;
println!("{:?} (confidence: {:.0}%)", result.format, result.confidence * 100.0);

// --- Validation ---
let issues = ebook_converter::validate(&doc, &ValidateOptions::default());
let a11y_issues = ebook_converter::validate(&doc, &ValidateOptions {
    accessibility: true,
    wcag_level: WcagLevel::Aa,
    ..Default::default()
});

// --- Repair ---
let report = ebook_converter::repair(&mut doc, &RepairOptions::default());
println!("Fixed {} issues", report.fixes_applied.len());

// --- Optimize ---
let report = ebook_converter::optimize(&mut doc, &OptimizeOptions {
    image_quality: 75,
    subset_fonts: true,
    ..Default::default()
});

// --- Metadata editing ---
let title = doc.metadata.title.as_deref().unwrap_or("Unknown");
doc.metadata.authors = vec!["New Author".to_string()];
ebook_converter::meta_strip(&mut doc, None); // strip all

// --- Statistics ---
let stats = doc.stats();
println!("{} words, ~{:.0} min read", stats.word_count, stats.estimated_reading_time_minutes);

// --- Metadata lookup ---
let provider = ebook_converter::lookup::OpenLibraryProvider::new();
let result = ebook_converter::lookup::lookup_metadata(
    &MetadataQuery::from_document(&doc),
    &provider,
    &LookupOptions::default(),
).await?;
let report = ebook_converter::lookup::enrich_document(&mut doc, &result, &EnrichPolicy::default());

// --- Merge & Split ---
let doc2 = ebook_converter::read_file("chapter2.epub", &ReadOptions::default())?;
let merged = ebook_converter::merge(&[doc, doc2], &MergeOptions::default())?;
let chapters = ebook_converter::split(&merged, SplitStrategy::Chapter)?;

// --- Title formatting ---
let name = ebook_converter::format_title(
    "Fitzgerald, F Scott - The Great Gatsby.epub",
    "{author} - {title}.{ext}",
    Some(&doc.metadata),
)?;

// --- Transforms ---
doc.apply_transform(&StripImages)?;
doc.apply_transform(&NormalizeUnicode { form: UnicodeForm::Nfc })?;

// --- Duplicate detection ---
let groups = ebook_converter::find_duplicates(
    &paths,
    DuplicateStrategy::Fuzzy,
    0.85,
)?;

// --- Progress reporting ---
struct MyProgress;
impl ProgressHandler for MyProgress {
    fn on_progress(&self, event: ProgressEvent) {
        println!("[{}] {}/{}", event.operation, event.current, event.total.unwrap_or(0));
    }
}
let doc = ebook_converter::read_with_progress(input, &ReadOptions::default(), &MyProgress)?;
```

### C-ABI (for FFI consumers)
```c
#include "ebook_converter.h"

// Open and convert
EbookHandle* handle = ebook_open("book.epub", NULL);  // NULL = default options
int result = ebook_convert(handle, "output.pdf", NULL);

// Validate
EbookValidationList* issues = ebook_validate(handle, NULL);
int count = ebook_validation_count(issues);
for (int i = 0; i < count; i++) {
    const char* msg = ebook_validation_message(issues, i);
    int severity = ebook_validation_severity(issues, i);
    printf("[%d] %s\n", severity, msg);
}
ebook_validation_free(issues);

// Repair
EbookRepairReport* report = ebook_repair(handle, NULL);
ebook_repair_report_free(report);

// Metadata
const char* title = ebook_meta_get(handle, "title");
ebook_meta_set(handle, "title", "New Title");

// Statistics
EbookStats stats;
ebook_stats(handle, &stats);
printf("Words: %llu, Reading time: %.0f min\n", stats.word_count, stats.reading_time_minutes);

// Progress callback
void my_progress(const char* op, uint64_t current, uint64_t total, void* user_data) {
    printf("[%s] %llu/%llu\n", op, current, total);
}
EbookHandle* handle2 = ebook_open_with_progress("large.pdf", NULL, my_progress, NULL);

// Cleanup
ebook_free(handle);
```

### WASM (for browser / Node.js)
```js
import {
  readEbook, writeEbook, validate, repair, optimize,
  detect, lookup, merge, split, formatTitle, stats
} from '@ebook-converter/wasm';

// Convert
const doc = await readEbook(epubArrayBuffer);
const pdfBytes = await writeEbook(doc, 'pdf');

// Validate
const issues = validate(doc);
const a11yIssues = validate(doc, { accessibility: true });

// Repair & optimize
const repairReport = repair(doc);
const optimizeReport = optimize(doc, { imageQuality: 75 });

// Metadata
console.log(doc.metadata.title);
doc.metadata.authors = ['New Author'];

// Stats
const s = stats(doc);
console.log(`${s.wordCount} words, ~${s.readingTimeMinutes} min read`);

// Lookup (browser fetch)
const lookupResult = await lookup(doc, { provider: 'openlibrary' });

// Progress (callback)
const doc2 = await readEbook(largeBuffer, {
  onProgress: (op, current, total) => console.log(`${op}: ${current}/${total}`)
});
```

---

## 7. CLI Interface

```bash
# Convert
ebook-converter convert input.epub -o output.pdf
ebook-converter convert input.txt -o output.epub --title "My Book" --author "Jane Doe"

# Batch convert
ebook-converter convert *.epub --format pdf --outdir ./pdfs/

# Convert with rename (--rename flag applies format string to output filename)
ebook-converter convert input.epub -o ./out/ --format pdf --rename "{author_last} - {title}.{ext}"

# Batch convert with rename
ebook-converter convert *.epub --format epub --outdir ./clean/ --rename "{author} - {title} ({year}).{ext}"

# Standalone rename (rename files without converting)
ebook-converter rename "Fitzgerald, F Scott - The Great Gatsby.epub" --template "{author} - {title}.{ext}"
ebook-converter rename *.epub --template "{author_last} - {title|kebab}.{ext}" --dry-run
ebook-converter rename *.epub --template "{series} {series_num} - {title}.{ext}" --outdir ./renamed/

# Validate
ebook-converter validate book.epub
ebook-converter validate book.epub --strict

# Repair
ebook-converter repair broken.epub -o fixed.epub

# Optimize
ebook-converter optimize large.epub -o smaller.epub --image-quality 60

# Inspect metadata
ebook-converter info book.epub --json

# Detect format
ebook-converter detect mystery-file

# Metadata lookup (enrich file with web data)
ebook-converter lookup book.epub                          # print found metadata
ebook-converter lookup book.epub --apply -o enriched.epub # write enriched copy
ebook-converter lookup book.epub --apply --in-place       # update file in place

# --lookup flag works on any command to enrich metadata before processing
ebook-converter convert input.epub -o output.pdf --lookup

# Library copy (convert + dump enriched library copy)
ebook-converter convert input.epub -o output.pdf --library-copy
ebook-converter convert *.epub --format pdf --outdir ./pdfs/ --library-copy --lookup

# Library copy with all the trimmings
ebook-converter convert input.txt -o output.epub --lookup --library-copy --rename "{author} - {title}.{ext}"

# Metadata editing (standalone, no conversion)
ebook-converter meta get book.epub                        # print all metadata
ebook-converter meta get book.epub --field title           # print specific field
ebook-converter meta set book.epub --title "New Title" --author "New Author"
ebook-converter meta strip book.epub -o clean.epub         # remove all metadata (privacy)
ebook-converter meta strip book.epub --fields description,isbn -o clean.epub
ebook-converter meta copy source.epub target.epub          # copy metadata between files

# Cover extraction
ebook-converter cover book.epub -o cover.jpg               # extract cover image
ebook-converter cover book.epub --format png -o cover.png  # extract in specific format

# Merge & Split
ebook-converter merge ch1.epub ch2.epub ch3.epub -o combined.epub --title "Full Book"
ebook-converter merge *.txt -o anthology.epub --title "Collected Stories"
ebook-converter split book.epub --by chapter --outdir ./chapters/
ebook-converter split book.epub --by heading-level-1 --outdir ./parts/
ebook-converter split book.epub --by page-count-50 --outdir ./segments/

# Duplicate detection
ebook-converter dedup ./library/                           # find duplicates in directory
ebook-converter dedup ./library/ --strategy fuzzy --threshold 0.9
ebook-converter dedup ./library/ --json                    # machine-readable output
ebook-converter dedup ./library/ --interactive             # choose which to keep

# Watch mode (auto-process new files in a directory)
ebook-converter watch ./inbox --format epub --outdir ./library/
ebook-converter watch ./inbox --format epub --lookup --library-copy
ebook-converter watch ./inbox --format epub --on-complete move --move-to ./processed/

# Transforms (applied during any conversion)
ebook-converter convert input.epub -o output.epub --transform strip-images,normalize-unicode
ebook-converter convert input.epub -o output.epub --transform "inject-watermark:text=REVIEW COPY"
ebook-converter convert input.epub -o output.epub --transform smart-quotes

# Accessibility validation
ebook-converter validate book.epub --accessibility
ebook-converter validate book.epub --accessibility --wcag-level AA

# Info with reading statistics
ebook-converter info book.epub                             # includes word count, reading time, chapter count
ebook-converter info book.epub --json                      # full stats as JSON
ebook-converter info book.epub --stats-only                # just the numbers

# Config management
ebook-converter config init            # generate default ~/.config/ebook-converter/config.toml
ebook-converter config show            # print current config
ebook-converter config set library.path ~/MyLibrary
ebook-converter config set library.format epub3
```

Exit codes: 0 = success, 1 = error, 2 = validation warnings found.
All commands support `--json` for machine-readable output.
The `rename` command supports `--dry-run` to preview changes without moving files.
The `--lookup` flag can be combined with any command that reads ebook files.
The `--transform` flag can be combined with any command that writes ebook files.

---

## 8. File Validation & Repair

### Validation Checks
| Check | Formats | Severity |
|-------|---------|----------|
| Valid ZIP structure | EPUB | Error |
| Required metadata present | EPUB, MOBI | Warning |
| Valid XML/XHTML | EPUB | Error |
| Internal links resolve | EPUB | Warning |
| Image references valid | EPUB, MOBI | Warning |
| TOC present and valid | EPUB, MOBI | Warning |
| Character encoding | All | Warning |
| File size anomalies | All | Info |

### Repair Actions
Each validation issue maps to zero or more auto-repair actions. Repairs are applied in a transaction-like manner: if any repair fails, the original is preserved.

---

## 9. Size Optimization

| Technique | Potential Savings | Default |
|-----------|------------------|---------|
| Image recompression (JPEG/PNG) | 30-70% of image size | On (quality 80) |
| Font subsetting | 50-90% of font size | On |
| Strip unused CSS | 10-30% of CSS size | On |
| Minify HTML | 5-15% of HTML size | Off |
| Remove duplicate resources | Variable | On |
| Strip metadata bloat | Variable | Off |

---

## 10. Portability & Integration

### Compile Targets
| Target | Method | Output |
|--------|--------|--------|
| macOS (x86/ARM) | Native | `.dylib` + CLI binary |
| Linux (x86/ARM) | `cross` | `.so` + CLI binary |
| Windows (x86) | `cross` | `.dll` + CLI binary |
| WASM (browser) | `wasm-pack` | `.wasm` + JS glue |
| iOS | `cargo-lipo` | `.a` static lib |
| Android | NDK | `.so` |

### Language Bindings (generated from C-ABI)
- **Python**: `cffi` or `PyO3` wrapper → publish to PyPI
- **Node.js**: `napi-rs` wrapper → publish to npm
- **Swift**: C header import via bridging header
- **C#/.NET**: P/Invoke wrapper
- **Ruby**: FFI gem

### Integration Patterns
- Streaming API for large files (don't load entire file into memory)
- Async-compatible (non-blocking I/O for the CLI, sync core for FFI simplicity)
- Thread-safe: `Document` is `Send + Sync`
- No global state, no singletons

---

## 11. Project Structure

```
ebook-converter/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── core/                   # ebook-converter-core (the library)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── document.rs     # IR types
│   │   │   ├── detect.rs       # Format detection
│   │   │   ├── error.rs        # Error types
│   │   │   ├── optimize.rs     # Size optimizer
│   │   │   ├── validate.rs     # Validation engine
│   │   │   ├── repair.rs       # Repair engine
│   │   │   ├── rename.rs       # Title formatter / filename templating
│   │   │   ├── lookup.rs       # Metadata lookup (Open Library API)
│   │   │   ├── library.rs      # Library copy logic & config
│   │   │   ├── config.rs       # Config file parsing (~/.config/ebook-converter/config.toml)
│   │   │   ├── merge.rs        # Merge multiple documents
│   │   │   ├── split.rs        # Split document by chapter/heading/page
│   │   │   ├── meta.rs         # Standalone metadata editing
│   │   │   ├── dedup.rs        # Duplicate detection
│   │   │   ├── security.rs     # ZIP bomb, path traversal, resource limits
│   │   │   ├── progress.rs     # Progress reporting trait + events
│   │   │   ├── transform.rs    # Plugin/transform hook system
│   │   │   ├── accessibility.rs # EPUB Accessibility / WCAG validation
│   │   │   ├── encoding.rs     # Unicode normalization, smart quotes, etc.
│   │   │   ├── stats.rs        # Reading statistics (word count, reading time)
│   │   │   ├── watch.rs        # Directory watch mode
│   │   │   ├── readers/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── epub.rs
│   │   │   │   ├── txt.rs
│   │   │   │   ├── html.rs
│   │   │   │   ├── markdown.rs
│   │   │   │   └── pdf.rs
│   │   │   └── writers/
│   │   │       ├── mod.rs
│   │   │       ├── epub.rs
│   │   │       ├── txt.rs
│   │   │       ├── html.rs
│   │   │       ├── markdown.rs
│   │   │       ├── ssml.rs
│   │   │       └── pdf.rs
│   │   └── Cargo.toml
│   ├── ffi/                    # C-ABI FFI crate
│   │   ├── src/lib.rs
│   │   ├── cbindgen.toml
│   │   └── Cargo.toml
│   ├── cli/                    # CLI binary
│   │   ├── src/main.rs
│   │   └── Cargo.toml
│   └── wasm/                   # WASM bindings
│       ├── src/lib.rs
│       └── Cargo.toml
├── bindings/
│   ├── python/                 # PyO3 wrapper
│   ├── node/                   # napi-rs wrapper
│   └── swift/                  # Swift package
├── schemas/
│   ├── cli-response.schema.json
│   ├── document-info.schema.json
│   ├── validation-report.schema.json
│   ├── repair-report.schema.json
│   ├── optimize-report.schema.json
│   ├── lookup-result.schema.json
│   ├── dedup-report.schema.json
│   ├── stats.schema.json
│   ├── metadata.schema.json
│   └── providers/
│       ├── openapi-provider-contract.yaml
│       ├── openlibrary.yaml
│       └── google-books.yaml
├── tests/
│   ├── fixtures/               # Sample ebook files (see §14 for full listing)
│   └── integration/            # Integration test files (see §14 for full listing)
├── docs/                       # Generated documentation (HTML)
├── ai-docs/                    # Project planning documents
└── README.md
```

---

## 12. Dependency Strategy

| Purpose | Crate | Why |
|---------|-------|-----|
| EPUB parsing | `zip`, `quick-xml` | Mature, no heavy deps |
| HTML parsing | `scraper` / `html5ever` | Standards-compliant |
| Markdown | `pulldown-cmark` | Fast, CommonMark-compliant |
| PDF read | `lopdf` or `pdf-rs` | Pure Rust |
| PDF write | `printpdf` | Pure Rust |
| Image processing | `image` | Comprehensive, pure Rust |
| CLI | `clap` | De facto standard |
| Serialization | `serde`, `serde_json` | For JSON output / config |
| Error handling | `thiserror` | Ergonomic error types |
| WASM glue | `wasm-bindgen` | Standard |
| Logging | `tracing` | Structured, composable |
| HTTP client | `reqwest` (with `rustls`) | For Open Library API calls, pure Rust TLS |
| Config parsing | `toml` | For `config.toml` |
| Fuzzy matching | `strsim` | Title/author similarity scoring for lookup |
| Async runtime | `tokio` (minimal features) | For HTTP requests in lookup |
| Directories | `dirs` | Cross-platform `~/.config`, `~/.cache` paths |
| File watching | `notify` | Cross-platform filesystem events for watch mode |
| Progress bars | `indicatif` | Terminal progress bar for CLI |
| Hashing | `sha2` | SHA-256 for duplicate detection, pure Rust |
| Unicode | `unicode-normalization` | NFC/NFD/NFKC/NFKD normalization |
| Unicode segmentation | `unicode-segmentation` | Word count, grapheme boundaries |
| Readability | (custom) | Flesch-Kincaid scoring from word/sentence counts |
| JSON Schema | `jsonschema` | Validate CLI `--json` output against schemas in CI |
| JSON path | `serde_json_path` | Extract fields from custom provider responses via `field_map` config |

**Dev/test dependencies** (not shipped in release):

| Purpose | Crate | Why |
|---------|-------|-----|
| CLI testing | `assert_cmd` | Test CLI binary as subprocess |
| Temp files | `tempfile` | Isolated temp dirs per test |
| Assertions | `pretty_assertions` | Readable diff on struct equality failures |
| Property testing | `proptest` | Fuzz-style generative tests |
| HTTP mocking | `wiremock` | Mock API responses for lookup tests |

Avoid: any crate that links to system C libraries (keeps cross-compilation clean).

---

## 13. Build & Distribution

- **CI**: GitHub Actions matrix (Linux, macOS, Windows × x86, ARM)
- **Release binaries**: Publish pre-built binaries via GitHub Releases
- **Crate**: Publish `ebook-converter-core` to crates.io
- **WASM**: Publish to npm as `@ebook-converter/wasm`
- **Python**: Publish to PyPI as `ebook-converter`
- **Docs**: `cargo doc` → GitHub Pages

---

## 14. Test Strategy

### Test Organization
```
tests/
├── fixtures/
│   ├── epub2/                    # Valid EPUB2 files
│   │   ├── minimal.epub          # Bare minimum valid EPUB2 (1 chapter, no images)
│   │   ├── full-featured.epub    # TOC, images, fonts, CSS, multiple chapters
│   │   └── metadata-rich.epub    # All metadata fields populated
│   ├── epub3/                    # Valid EPUB3 files
│   │   ├── minimal.epub
│   │   ├── full-featured.epub
│   │   ├── nav-document.epub     # EPUB3 NAV instead of NCX
│   │   └── media-overlays.epub   # Audio sync
│   ├── txt/
│   │   ├── utf8.txt              # Standard UTF-8
│   │   ├── latin1.txt            # Non-UTF-8 encoding
│   │   └── bom.txt               # UTF-8 with BOM
│   ├── html/
│   │   ├── simple.html
│   │   └── styled.html           # With CSS, images
│   ├── markdown/
│   │   ├── simple.md
│   │   └── gfm.md                # GitHub-flavored markdown
│   ├── malformed/                # Broken files for repair/security testing
│   │   ├── broken-zip.epub       # Corrupt ZIP structure
│   │   ├── invalid-xml.epub      # Malformed XHTML content
│   │   ├── missing-opf.epub      # No OPF package file
│   │   ├── missing-toc.epub      # No NCX or NAV
│   │   ├── bad-encoding.epub     # Wrong character encoding
│   │   ├── broken-links.epub     # Internal links to nonexistent targets
│   │   └── path-traversal.epub   # ZIP entries with ../ paths
│   ├── security/
│   │   ├── zip-bomb.epub         # Extreme compression ratio
│   │   ├── too-many-files.epub   # 50,000 entries
│   │   ├── oversized-image.epub  # 500MB image resource
│   │   └── deep-nesting.epub     # 1000-level nested XML
│   ├── drm/
│   │   ├── adobe-drm.epub        # Adobe DRM encryption.xml
│   │   └── apple-drm.epub        # Apple FairPlay DRM
│   ├── i18n/
│   │   ├── arabic-rtl.epub       # Right-to-left Arabic text
│   │   ├── hebrew-rtl.epub       # Right-to-left Hebrew text
│   │   ├── japanese-vertical.epub # Vertical writing mode
│   │   ├── chinese-cjk.epub      # CJK text with ruby annotations
│   │   └── mixed-bidi.epub       # Mixed LTR/RTL in same document
│   └── filenames/                # For title formatter testing
│       ├── "Author - Title.epub"
│       ├── "Title (Series #3).epub"
│       ├── "Author - Series 03 - Title.epub"
│       ├── "Title [2024].epub"
│       └── "Last, First - Title.epub"
└── integration/
    ├── convert_test.rs           # Round-trip conversion tests
    ├── validate_test.rs          # Validation + accessibility tests
    ├── repair_test.rs            # Repair engine tests
    ├── optimize_test.rs          # Size optimization tests
    ├── security_test.rs          # Security hardening tests
    ├── rename_test.rs            # Title formatter tests
    ├── lookup_test.rs            # Metadata lookup tests (with mock HTTP)
    ├── merge_split_test.rs       # Merge & split tests
    ├── meta_test.rs              # Metadata editor tests
    ├── dedup_test.rs             # Duplicate detection tests
    ├── encoding_test.rs          # Unicode normalization tests
    ├── stats_test.rs             # Reading statistics tests
    ├── transform_test.rs         # Transform pipeline tests
    └── cli_test.rs               # End-to-end CLI tests (assert_cmd)
```

### Test Categories

**Unit tests** (in each module's source file):
- IR struct construction and manipulation
- Format string parsing and token replacement
- Magic byte detection for each format
- Security limit checking
- Unicode normalization operations
- Statistics calculations

**Integration tests** (`tests/integration/`):
- **Round-trip fidelity**: Read format A → IR → write format B → read format B → IR → assert IR equality (within format limitations). Key pairs: EPUB↔HTML, EPUB↔TXT, EPUB↔Markdown.
- **Fixture validation**: Every fixture file must load without panicking. Valid fixtures must produce zero validation errors. Malformed fixtures must produce specific expected error codes.
- **Repair idempotency**: `repair(repair(doc))` must equal `repair(doc)` — no infinite fix loops.
- **Security rejection**: Every file in `fixtures/security/` must be rejected with the correct `SecurityError` variant before any content is parsed.
- **DRM rejection**: Every file in `fixtures/drm/` must return `SecurityError::DrmProtected` with correct `drm_type`.
- **Encoding round-trip**: Apply NFC normalization → write → read → assert text is still NFC.
- **Stats accuracy**: Known fixture with manually counted words/sentences → assert stats match.

**CLI tests** (using `assert_cmd` crate):
- Each subcommand runs without error on a valid fixture
- `--json` output is valid JSON matching expected schema
- Exit codes match documented values (0/1/2)
- `--dry-run` produces no file system changes
- Error messages include actionable information

**Property-based tests** (using `proptest` crate):
- Arbitrary `Document` → write → read → assert structural equality
- Arbitrary format strings → `format_title` never panics
- Arbitrary byte slices → `detect` never panics (may return `Unknown`)

**Mock HTTP for lookup tests**:
- Use `wiremock` crate to mock Open Library / Google Books responses
- Test cache hit/miss behavior
- Test rate limiting behavior
- Test network error handling
- Never make real HTTP calls in CI

### Dependency additions for testing

| Purpose | Crate | Why |
|---------|-------|-----|
| CLI testing | `assert_cmd` | Test CLI binary as subprocess |
| Temp files | `tempfile` | Isolated temp dirs per test |
| Assertions | `pretty_assertions` | Readable diff on struct equality failures |
| Property testing | `proptest` | Fuzz-style generative tests |
| HTTP mocking | `wiremock` | Mock API responses for lookup tests |

---

## 15. OpenAPI & JSON Schema Standards

All structured output from the CLI and all provider API interactions follow OpenAPI / JSON Schema conventions.

### CLI JSON Output Schema

Every command that supports `--json` outputs a response conforming to a consistent envelope:

```json
{
  "version": "1.0.0",
  "command": "convert",
  "status": "success",
  "data": { /* command-specific payload */ },
  "errors": [],
  "warnings": []
}
```

**Schemas are defined as JSON Schema files** in the project:
```
schemas/
├── cli-response.schema.json       # Top-level envelope
├── document-info.schema.json      # Output of `info` command
├── validation-report.schema.json  # Output of `validate`
├── repair-report.schema.json      # Output of `repair`
├── optimize-report.schema.json    # Output of `optimize`
├── lookup-result.schema.json      # Output of `lookup`
├── dedup-report.schema.json       # Output of `dedup`
├── stats.schema.json              # Reading statistics
└── metadata.schema.json           # Metadata structure (shared)
```

All schemas use **JSON Schema Draft 2020-12**. CLI `--json` output is validated against these schemas in CI tests.

### Metadata Provider API Contracts

Each metadata provider's interaction is documented as an **OpenAPI 3.1 spec** so custom providers have a clear contract:

```
schemas/
└── providers/
    ├── openapi-provider-contract.yaml   # Abstract provider interface
    ├── openlibrary.yaml                 # Open Library implementation
    └── google-books.yaml                # Google Books implementation
```

**Provider contract (abstract):**
```yaml
openapi: "3.1.0"
info:
  title: Ebook Converter Metadata Provider
  version: "1.0.0"
paths:
  /search:
    get:
      summary: Search for books by title/author
      parameters:
        - name: q
          in: query
          required: true
          schema: { type: string }
        - name: limit
          in: query
          schema: { type: integer, default: 5 }
      responses:
        "200":
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/SearchResponse"
  /isbn/{isbn}:
    get:
      summary: Look up a book by ISBN
      parameters:
        - name: isbn
          in: path
          required: true
          schema: { type: string }
      responses:
        "200":
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/BookResult"
components:
  schemas:
    SearchResponse:
      type: object
      properties:
        results:
          type: array
          items: { $ref: "#/components/schemas/BookResult" }
        total: { type: integer }
    BookResult:
      type: object
      properties:
        title: { type: string }
        authors: { type: array, items: { type: string } }
        isbn_10: { type: string, nullable: true }
        isbn_13: { type: string, nullable: true }
        description: { type: string, nullable: true }
        cover_url: { type: string, format: uri, nullable: true }
        publisher: { type: string, nullable: true }
        publish_date: { type: string, nullable: true }
        subjects: { type: array, items: { type: string } }
        series: { type: string, nullable: true }
        series_number: { type: number, nullable: true }
        page_count: { type: integer, nullable: true }
        language: { type: string, nullable: true }
```

**Custom provider mapping in `config.toml`:**
Users who add custom providers configure endpoint paths and a **response mapping** that tells the library which JSON fields in their API response correspond to the `BookResult` schema:

```toml
[lookup.providers.my_api]
name = "My Custom API"
base_url = "https://api.example.com"
search_endpoint = "/books/search?q={query}"
isbn_endpoint = "/books/{isbn}"
api_key = "my-key"
rate_limit_ms = 500

# Response field mappings (JSONPath-like) — maps provider response to BookResult schema
[lookup.providers.my_api.field_map]
title = "$.data.book_title"
authors = "$.data.writers"
isbn_13 = "$.data.identifiers.isbn13"
description = "$.data.summary"
cover_url = "$.data.images.large"
publish_date = "$.data.pub_year"
subjects = "$.data.categories"
```

This means users can integrate any REST API that returns JSON without writing code — they only need to describe the response shape in the config.

### Rust-side Schema Validation

The `MetadataProvider` trait implementations normalize external API responses into the canonical `MetadataResult` struct (which mirrors the `BookResult` schema). For built-in providers, this mapping is hardcoded. For custom providers, the `field_map` config drives a generic JSON-path extractor at runtime.

---

## 16. Implementation Phases

### Phase 1 — Foundation (Core IR + EPUB + TXT + Safety)
1. Set up Cargo workspace with `core`, `cli`, `ffi`, `wasm` crates
2. Define `Document` IR types and `FormatReader` / `FormatWriter` traits
3. Implement security hardening (`security.rs`) — ZIP bomb protection, path traversal guards, resource limits
4. Implement progress reporting trait (`progress.rs`) — callback system for all operations
5. Implement `detect` module (magic-byte format detection)
6. Implement EPUB reader (parse ZIP → OPF → content documents → IR) with security checks
7. Implement EPUB writer (IR → content documents → OPF → ZIP)
8. Implement plain text reader/writer
9. Implement basic validator for EPUB
10. Implement title formatter / filename templating engine (`rename.rs`)
11. Implement reading statistics (`stats.rs`) — word count, chapter count, estimated reading time, readability score
12. Implement standalone metadata editor (`meta.rs`) — get/set/strip/copy
13. Implement cover extraction from IR
14. Define JSON Schema files for CLI output envelope and `metadata.schema.json` (§15)
15. Build CLI with `convert`, `validate`, `info`, `rename`, `meta`, `cover` commands
16. Create test fixtures (valid EPUB2, EPUB3, TXT, malformed, security, DRM — see §14)
17. Integration tests with fixture EPUB files
18. Set up CI pipeline (including JSON schema validation of `--json` output)

### Phase 2 — Formats, Enrichment, Repair, Optimize
1. HTML reader/writer
2. Markdown reader/writer
3. SSML writer (TTS-optimized output — map chapters to breaks, headings to announcements, emphasis to `<emphasis>`, numbers/dates to `<say-as>`)
4. Encoding normalization module (`encoding.rs`) — UTF-8 enforcement, Unicode NFC/NFD, smart quotes, ligatures, macOS NFD filename fix
5. Metadata lookup module (`lookup.rs`) — pluggable provider system, Open Library + Google Books, caching, fuzzy matching
6. Library copy module (`library.rs`) — config-driven enriched copy output
7. Config system (`config.rs`) — `~/.config/ebook-converter/config.toml` parsing, `config init/show/set` commands
8. Repair engine (XML fixes, metadata fills, broken links, encoding normalization)
9. Image optimizer (recompress, downscale — JPEG/PNG only)
10. Font subsetter
11. Merge & split (`merge.rs`, `split.rs`) — combine/divide ebooks by chapter, heading, or page count
12. Expand CLI with `repair`, `optimize`, `lookup`, `config`, `merge`, `split` commands and `--lookup`, `--library-copy` flags
13. `--json` output for all commands, validated against JSON Schema files (§15)
14. OpenAPI 3.1 specs for built-in metadata providers (`openlibrary.yaml`, `google-books.yaml`)
15. Custom provider `field_map` JSON-path extractor for user-defined providers

### Phase 3 — PDF, Proprietary Formats, Advanced Features
1. PDF reader (extract text + chapter structure, paragraph separations)
2. PDF writer (generate from IR)
3. MOBI/AZW3 reader
4. Expand validator for all formats
5. Batch conversion mode in CLI
6. Duplicate detection (`dedup.rs`) — hash, ISBN, fuzzy metadata, content fingerprint
7. Accessibility validation (`accessibility.rs`) — EPUB Accessibility 1.0, WCAG checks
8. RTL & CJK text handling — bidirectional text, CJK line breaking, vertical writing mode, ruby annotations
9. Transform / plugin hook system (`transform.rs`) — composable IR transforms, built-in transforms (strip-images, smart-quotes, watermark, normalize-unicode)
10. Watch mode (`watch.rs`) — filesystem monitoring, auto-process pipeline, configurable on-complete behavior

### Phase 4 — Bindings & Distribution
1. FFI crate with C-ABI exports + cbindgen header (including progress callback as C function pointer)
2. Node.js bindings (napi-rs) — highest priority binding
3. WASM build + JS wrapper — second priority
4. Additional bindings (Python, Swift, etc.) — TBD based on demand
5. Publish to package registries (crates.io, npm)
6. Documentation site

---

## 17. Decisions (Resolved)

| # | Topic | Decision |
|---|-------|----------|
| 1 | **Language** | Rust confirmed |
| 2 | **PDF fidelity** | Preserve chapter structure, paragraph separations, and book-like traits. No fancy layout preservation. |
| 3 | **DRM handling** | Detect and alert the user. No DRM removal. |
| 4 | **Image formats** | JPEG/PNG only (maximum compatibility). No WebP. |
| 5 | **EPUB versions** | Support both EPUB2 and EPUB3 |
| 6 | **Binding priority** | CLI → Node.js → WASM → (remaining TBD) |
| 7 | **Licensing** | MIT / Apache-2.0 dual license |
