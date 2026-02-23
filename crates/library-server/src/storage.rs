//! Directory-backed storage: list, get, put, delete ebooks.
//! Optionally extracts metadata via ebook-converter-core for list/get.

use std::path::{Path, PathBuf};

use ebook_converter_core::document::Metadata;
use ebook_converter_core::library::{LibraryCapabilities, LibraryEntry, ListOptions, ListResult};

const EBOOK_EXTENSIONS: &[&str] = &["epub", "txt", "pdf", "html", "md", "mobi", "azw3", "fb2"];

/// Directory-backed store. Files are stored as-is; ids are relative paths (filename or subpath).
#[derive(Clone)]
pub struct DirStore {
    root: PathBuf,
}

impl DirStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn capabilities(&self) -> LibraryCapabilities {
        LibraryCapabilities {
            list: true,
            get: true,
            put: true,
            delete: true,
            search: true,
        }
    }

    fn format_from_path(&self, p: &Path) -> String {
        p.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_string()
    }

    /// List entries with optional query/format/offset/limit. Search is by filename stem.
    pub fn list(&self, opts: &ListOptions) -> std::io::Result<ListResult> {
        let read_dir = std::fs::read_dir(&self.root)?;
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
            let metadata = self.metadata_for_path(&path).unwrap_or_default();
            let updated_at = std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| format!("{:?}", t));
            entries.push(LibraryEntry {
                id: id.clone(),
                metadata,
                format: self.format_from_path(&path),
                size_bytes,
                updated_at,
            });
        }
        let total = entries.len() as u64;
        let offset = opts.offset.unwrap_or(0) as usize;
        let limit = opts.limit.unwrap_or(100).min(500) as usize;
        let entries = entries.into_iter().skip(offset).take(limit).collect();
        Ok(ListResult {
            entries,
            total: Some(total),
        })
    }

    /// Get file bytes and format by id.
    pub fn get(&self, id: &str) -> std::io::Result<(Vec<u8>, String)> {
        let path = self.safe_path(id)?;
        let data = std::fs::read(&path)?;
        let format = self.format_from_path(&path);
        Ok((data, format))
    }

    /// Store bytes with optional suggested id. Returns assigned id.
    pub fn put(&self, data: &[u8], suggested_id: Option<&str>, format_hint: Option<&str>) -> std::io::Result<String> {
        std::fs::create_dir_all(&self.root)?;
        let id = suggested_id
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty() && !s.contains(".."))
            .unwrap_or_else(|| {
                let ext = format_hint.unwrap_or("bin");
                format!("{}.{}", uuid::Uuid::new_v4().as_simple(), ext)
            });
        let path = self.safe_path(&id)?;
        std::fs::write(&path, data)?;
        Ok(id)
    }

    pub fn delete(&self, id: &str) -> std::io::Result<()> {
        let path = self.safe_path(id)?;
        if path.is_file() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Resolve id to path, ensuring it stays under root (no path traversal).
    fn safe_path(&self, id: &str) -> std::io::Result<PathBuf> {
        if id.contains("..") || Path::new(id).is_absolute() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid id",
            ));
        }
        let path = self.root.join(id);
        if path.exists() {
            let canonical_root = self.root.canonicalize().unwrap_or_else(|_| self.root.clone());
            let canonical_path = path.canonicalize().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, "path escapes library root")
            })?;
            if !canonical_path.starts_with(&canonical_root) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "path escapes library root",
                ));
            }
        }
        Ok(path)
    }

    /// Try to read metadata from file (EPUB/TXT) for list display.
    fn metadata_for_path(&self, path: &Path) -> Option<Metadata> {
        let ext = path.extension().and_then(|e| e.to_str())?;
        if ext.eq_ignore_ascii_case("epub") || ext.eq_ignore_ascii_case("txt") {
            let data = std::fs::read(path).ok()?;
            let header: Vec<u8> = data.iter().copied().take(4096).collect();
            let filename = path.file_name().and_then(|s| s.to_str());
            let detected = ebook_converter_core::detect::detect(&header, filename).ok()?;
            let mut cursor = std::io::Cursor::new(data);
            let opts = ebook_converter_core::readers::ReadOptions::default();
            let doc = ebook_converter_core::convert::read_document(
                detected.format,
                &mut cursor,
                &opts,
                None,
            ).ok()?;
            Some(doc.metadata)
        } else {
            None
        }
    }
}
