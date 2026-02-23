//! Library connection: standard and adapter for ebook libraries.
//!
//! An **ebook library** is a collection of ebooks that can be accessed over the web or
//! locally. The library app is its own application/server; this module defines the
//! **contract** (what a library can do) and the **adapter** trait so that ebook-converter
//! can pull books from or push books to any compliant library.
//!
//! See `docs/library-standard.md` for the full design (HTTP API shape, capabilities,
//! and local/dir backend).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::document::Metadata;
use crate::error::LibraryError;
use crate::rename::format_title;

// ---------------------------------------------------------------------------
// Types: entry, capabilities, list options
// ---------------------------------------------------------------------------

/// A single entry in a library (summary for listing; full file fetched via `get`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryEntry {
    /// Opaque id assigned by the library (use for get/put/delete).
    pub id: String,
    /// Book metadata (title, authors, etc.).
    pub metadata: Metadata,
    /// Stored format (epub, txt, etc.).
    pub format: String,
    /// File size in bytes, if known.
    pub size_bytes: Option<u64>,
    /// Last updated (e.g. ISO8601 or unix ts); for sync.
    pub updated_at: Option<String>,
}

/// What a library backend supports. Used for discovery and to avoid calling unsupported ops.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryCapabilities {
    pub list: bool,
    pub get: bool,
    pub put: bool,
    pub delete: bool,
    pub search: bool,
}

/// Options when listing entries (pagination, filter).
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    /// Free-text or structured query (interpretation is backend-specific).
    pub query: Option<String>,
    /// Filter by format (e.g. "epub").
    pub format: Option<String>,
}

/// Result of a list call: entries and optional total count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
    pub entries: Vec<LibraryEntry>,
    /// Total number of entries (if backend supports it); None = unknown.
    pub total: Option<u64>,
}

// ---------------------------------------------------------------------------
// Adapter trait: any library backend implements this
// ---------------------------------------------------------------------------

/// Connection to an ebook library. Implement this for HTTP servers, local directories,
/// or any backend that follows the library standard.
pub trait LibraryConnection: Send + Sync {
    /// Human-readable name (e.g. "My Library", "~/Books").
    fn name(&self) -> &str;

    /// What this backend supports. Call before list/get/put/delete.
    fn capabilities(&self) -> Result<LibraryCapabilities, LibraryError>;

    /// List entries with optional pagination and filters.
    fn list(&self, opts: &ListOptions) -> Result<ListResult, LibraryError>;

    /// Download the file for an entry. Returns (raw bytes, format string).
    fn get(&self, id: &str) -> Result<(Vec<u8>, String), LibraryError>;

    /// Upload an ebook. Returns the assigned id (or the one used).
    fn put(&self, data: &[u8], format: &str, metadata: Option<&Metadata>) -> Result<String, LibraryError>;

    /// Remove an entry. Optional capability.
    fn delete(&self, id: &str) -> Result<(), LibraryError> {
        let _ = id;
        Err(LibraryError::NotSupported)
    }
}

// ---------------------------------------------------------------------------
// Stub / unimplemented backend (for wiring and future real backends)
// ---------------------------------------------------------------------------

/// Placeholder backend that reports no capabilities. Use when no library is configured
/// or as a base for real implementations (HttpLibrary, DirLibrary).
#[derive(Debug, Default)]
pub struct StubLibrary;

impl StubLibrary {
    pub fn new() -> Self {
        Self
    }
}

impl LibraryConnection for StubLibrary {
    fn name(&self) -> &str {
        "Stub (no library configured)"
    }

    fn capabilities(&self) -> Result<LibraryCapabilities, LibraryError> {
        Ok(LibraryCapabilities::default())
    }

    fn list(&self, _opts: &ListOptions) -> Result<ListResult, LibraryError> {
        Ok(ListResult {
            entries: Vec::new(),
            total: Some(0),
        })
    }

    fn get(&self, _id: &str) -> Result<(Vec<u8>, String), LibraryError> {
        Err(LibraryError::NotSupported)
    }

    fn put(&self, _data: &[u8], _format: &str, _metadata: Option<&Metadata>) -> Result<String, LibraryError> {
        Err(LibraryError::NotSupported)
    }
}

// ---------------------------------------------------------------------------
// Directory backend: treat a local folder as a library (no server)
// ---------------------------------------------------------------------------

/// Ebook extensions to consider when listing a directory.
const EBOOK_EXTENSIONS: &[&str] = &["epub", "txt", "pdf", "html", "md", "mobi", "azw3", "fb2"];

/// Backend that uses a local directory as the library. List = scan for ebooks;
/// Get = read file; Put = write file (optional naming from template).
pub struct DirLibrary {
    root: PathBuf,
    /// Optional template for put (e.g. "{author} - {title}.{ext}").
    put_template: Option<String>,
}

impl DirLibrary {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            put_template: None,
        }
    }

    pub fn with_put_template(mut self, template: String) -> Self {
        self.put_template = Some(template);
        self
    }

    fn format_from_path(&self, p: &Path) -> String {
        p.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_string()
    }
}

impl LibraryConnection for DirLibrary {
    fn name(&self) -> &str {
        self.root.to_str().unwrap_or("(directory)")
    }

    fn capabilities(&self) -> Result<LibraryCapabilities, LibraryError> {
        Ok(LibraryCapabilities {
            list: true,
            get: true,
            put: true,
            delete: true,
            search: false,
        })
    }

    fn list(&self, opts: &ListOptions) -> Result<ListResult, LibraryError> {
        let read_dir = std::fs::read_dir(&self.root).map_err(LibraryError::from)?;
        let mut entries = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !EBOOK_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                continue;
            }
            let id = path
                .strip_prefix(&self.root)
                .ok()
                .and_then(|p| p.to_str())
                .map(String::from)
                .unwrap_or_else(|| path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string());
            if let Some(ref q) = opts.query {
                let name_lower = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                if !name_lower.contains(&q.to_lowercase()) {
                    continue;
                }
            }
            if let Some(ref fmt) = opts.format {
                if !ext.eq_ignore_ascii_case(fmt) {
                    continue;
                }
            }
            let size_bytes = std::fs::metadata(&path).ok().map(|m| m.len());
            let metadata = crate::document::Metadata::default();
            entries.push(LibraryEntry {
                id: id.clone(),
                metadata,
                format: self.format_from_path(&path),
                size_bytes,
                updated_at: None,
            });
        }
        let total = entries.len() as u64;
        let offset = opts.offset.unwrap_or(0) as usize;
        let limit = opts.limit.unwrap_or(100) as usize;
        let entries = entries.into_iter().skip(offset).take(limit).collect();
        Ok(ListResult {
            entries,
            total: Some(total),
        })
    }

    fn get(&self, id: &str) -> Result<(Vec<u8>, String), LibraryError> {
        let path = self.root.join(id);
        if !path.is_file() {
            return Err(LibraryError::NotFound(id.to_string()));
        }
        let data = std::fs::read(&path).map_err(LibraryError::from)?;
        let format = self.format_from_path(&path);
        Ok((data, format))
    }

    fn put(&self, data: &[u8], format: &str, metadata: Option<&Metadata>) -> Result<String, LibraryError> {
        let ext = format.to_lowercase();
        let filename = if let Some(template) = &self.put_template {
            let dummy_filename = format!("book.{}", ext);
            format_title(&dummy_filename, template, metadata).map_err(|e| LibraryError::Failed(e.to_string()))?
        } else {
            let title = metadata.and_then(|m| m.title.as_deref()).unwrap_or("book");
            let author = metadata.and_then(|m| m.authors.first().map(|s| s.as_str())).unwrap_or("Unknown");
            format!("{} - {}.{}", author, title, ext)
        };
        let safe_name = filename.replace(std::path::MAIN_SEPARATOR, "-");
        let path = self.root.join(&safe_name);
        std::fs::write(&path, data).map_err(LibraryError::from)?;
        Ok(safe_name)
    }

    fn delete(&self, id: &str) -> Result<(), LibraryError> {
        let path = self.root.join(id);
        if path.is_file() {
            std::fs::remove_file(&path).map_err(LibraryError::from)?;
            Ok(())
        } else {
            Err(LibraryError::NotFound(id.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_library_name_and_capabilities() {
        let stub = StubLibrary::new();
        assert_eq!(stub.name(), "Stub (no library configured)");
        let cap = stub.capabilities().unwrap();
        assert!(!cap.list && !cap.get && !cap.put && !cap.delete && !cap.search);
    }

    #[test]
    fn stub_library_list_empty() {
        let stub = StubLibrary::new();
        let r = stub.list(&ListOptions::default()).unwrap();
        assert_eq!(r.entries.len(), 0);
        assert_eq!(r.total, Some(0));
    }

    #[test]
    fn stub_library_get_not_supported() {
        let stub = StubLibrary::new();
        let err = stub.get("any").unwrap_err();
        assert!(matches!(err, LibraryError::NotSupported));
    }

    #[test]
    fn stub_library_put_not_supported() {
        let stub = StubLibrary::new();
        let err = stub.put(b"data", "epub", None).unwrap_err();
        assert!(matches!(err, LibraryError::NotSupported));
    }

    #[test]
    fn dir_library_list_get_put_delete() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        std::fs::write(path.join("book.epub"), b"epub content").unwrap();
        std::fs::write(path.join("other.txt"), b"text content").unwrap();
        std::fs::write(path.join("ignore.xyz"), b"ignored").unwrap();

        let lib = DirLibrary::new(&path);
        assert!(lib.name().len() > 0);
        let cap = lib.capabilities().unwrap();
        assert!(cap.list && cap.get && cap.put && cap.delete);

        let list = lib.list(&ListOptions::default()).unwrap();
        assert_eq!(list.entries.len(), 2);
        assert_eq!(list.total, Some(2));
        let ids: Vec<_> = list.entries.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"book.epub"));
        assert!(ids.contains(&"other.txt"));

        let (data, fmt) = lib.get("book.epub").unwrap();
        assert_eq!(data, b"epub content");
        assert_eq!(fmt, "epub");

        let meta = crate::document::Metadata {
            title: Some("New Book".to_string()),
            authors: vec!["Author".to_string()],
            ..Default::default()
        };
        let id = lib.put(b"new content", "epub", Some(&meta)).unwrap();
        assert!(id.contains("Author") && id.contains("New Book") && id.ends_with(".epub"));
        let (data, _) = lib.get(&id).unwrap();
        assert_eq!(data, b"new content");

        lib.delete(&id).unwrap();
        assert!(lib.get(&id).is_err());
    }

    #[test]
    fn dir_library_list_filter_query_and_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        std::fs::write(path.join("alpha.epub"), b"a").unwrap();
        std::fs::write(path.join("beta.epub"), b"b").unwrap();
        std::fs::write(path.join("alpha.txt"), b"c").unwrap();

        let lib = DirLibrary::new(path);
        let list = lib.list(&ListOptions { query: Some("alpha".to_string()), ..Default::default() }).unwrap();
        assert_eq!(list.entries.len(), 2);

        let list = lib.list(&ListOptions { format: Some("epub".to_string()), ..Default::default() }).unwrap();
        assert_eq!(list.entries.len(), 2);
        assert!(list.entries.iter().all(|e| e.format == "epub"));

        let list = lib.list(&ListOptions { offset: Some(1), limit: Some(1), ..Default::default() }).unwrap();
        assert_eq!(list.entries.len(), 1);
    }

    #[test]
    fn dir_library_get_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let lib = DirLibrary::new(dir.path());
        let err = lib.get("nonexistent.epub").unwrap_err();
        assert!(matches!(err, LibraryError::NotFound(_)));
    }
}
