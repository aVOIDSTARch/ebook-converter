//! Format detection via magic bytes, file extension, and content heuristics.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::DetectError;

/// The result of format detection.
#[derive(Debug, Clone)]
pub struct DetectResult {
    pub format: Format,
    pub confidence: f64,
    pub mime_type: &'static str,
}

/// Supported ebook/document formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Epub,
    Pdf,
    Mobi,
    Azw3,
    Html,
    Markdown,
    PlainText,
    Fb2,
    Docx,
    Cbz,
    Cbr,
    Ssml,
    Unknown,
}

impl Format {
    pub fn mime_type(&self) -> &'static str {
        match self {
            Format::Epub => "application/epub+zip",
            Format::Pdf => "application/pdf",
            Format::Mobi => "application/x-mobipocket-ebook",
            Format::Azw3 => "application/x-mobipocket-ebook",
            Format::Html => "text/html",
            Format::Markdown => "text/markdown",
            Format::PlainText => "text/plain",
            Format::Fb2 => "application/x-fictionbook+xml",
            Format::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Format::Cbz => "application/vnd.comicbook+zip",
            Format::Cbr => "application/vnd.comicbook-rar",
            Format::Ssml => "application/ssml+xml",
            Format::Unknown => "application/octet-stream",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Format::Epub => "epub",
            Format::Pdf => "pdf",
            Format::Mobi => "mobi",
            Format::Azw3 => "azw3",
            Format::Html => "html",
            Format::Markdown => "md",
            Format::PlainText => "txt",
            Format::Fb2 => "fb2",
            Format::Docx => "docx",
            Format::Cbz => "cbz",
            Format::Cbr => "cbr",
            Format::Ssml => "ssml",
            Format::Unknown => "bin",
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension().to_uppercase())
    }
}

const HEADER_SIZE: usize = 4096;

/// Detect the format of data by reading its first bytes and optionally using filename.
pub fn detect(header: &[u8], filename: Option<&str>) -> Result<DetectResult, DetectError> {
    // 1. Try magic bytes (highest confidence)
    if let Some(result) = detect_magic_bytes(header) {
        return Ok(result);
    }

    // 2. Try file extension
    if let Some(ext) = filename.and_then(|f| Path::new(f).extension()).and_then(|e| e.to_str()) {
        if let Some(result) = detect_by_extension(ext) {
            return Ok(result);
        }
    }

    // 3. Try content heuristics
    if let Some(result) = detect_by_content_heuristics(header) {
        return Ok(result);
    }

    Err(DetectError::Unknown(
        "Could not determine file format from magic bytes, extension, or content".to_string(),
    ))
}

/// Convenience: detect format from a file path.
pub fn detect_file(path: &Path) -> Result<DetectResult, DetectError> {
    let mut file = std::fs::File::open(path)?;
    let mut header = vec![0u8; HEADER_SIZE];
    let bytes_read = file.read(&mut header)?;
    header.truncate(bytes_read);
    file.seek(SeekFrom::Start(0))?;

    let filename = path.file_name().and_then(|f| f.to_str());
    detect(&header, filename)
}

/// Check magic bytes against known signatures.
fn detect_magic_bytes(header: &[u8]) -> Option<DetectResult> {
    if header.len() < 4 {
        return None;
    }

    // PDF: starts with %PDF-
    if header.starts_with(b"%PDF-") {
        return Some(DetectResult {
            format: Format::Pdf,
            confidence: 1.0,
            mime_type: Format::Pdf.mime_type(),
        });
    }

    // GZIP: \x1f\x8b
    if header[0] == 0x1f && header[1] == 0x8b {
        // Could be a compressed file — for now return Unknown with a note
        // In practice, we'd decompress and re-detect
        return None;
    }

    // RAR: starts with Rar!\x1a\x07
    if header.len() >= 7 && header.starts_with(b"Rar!\x1a\x07") {
        return Some(DetectResult {
            format: Format::Cbr,
            confidence: 0.7, // Could be any RAR, assume CBR if extension matches
            mime_type: Format::Cbr.mime_type(),
        });
    }

    // ZIP-based formats: PK\x03\x04
    if header.starts_with(b"PK\x03\x04") {
        return detect_zip_subformat(header);
    }

    // MOBI/PRC/AZW: check PDB header at offset 60 for BOOKMOBI
    if header.len() >= 68 && &header[60..68] == b"BOOKMOBI" {
        // Distinguish MOBI from AZW3/KF8 — for now report as MOBI
        // (Proper distinction requires reading MOBI header records)
        return Some(DetectResult {
            format: Format::Mobi,
            confidence: 0.95,
            mime_type: Format::Mobi.mime_type(),
        });
    }

    // XML-based: check for FB2 or SSML
    if let Some(result) = detect_xml_format(header) {
        return Some(result);
    }

    // HTML detection
    if let Some(result) = detect_html(header) {
        return Some(result);
    }

    None
}

/// Disambiguate ZIP-based formats by inspecting archive structure.
fn detect_zip_subformat(header: &[u8]) -> Option<DetectResult> {
    // Try to read ZIP with the zip crate for proper disambiguation
    let cursor = std::io::Cursor::new(header);
    if let Ok(mut archive) = zip::ZipArchive::new(cursor) {
        // EPUB: check for mimetype file containing "application/epub+zip"
        if let Ok(mut mimetype) = archive.by_name("mimetype") {
            let mut content = String::new();
            if mimetype.read_to_string(&mut content).is_ok()
                && content.trim() == "application/epub+zip"
            {
                return Some(DetectResult {
                    format: Format::Epub,
                    confidence: 1.0,
                    mime_type: Format::Epub.mime_type(),
                });
            }
        }

        // EPUB: also check for META-INF/container.xml (some EPUBs lack mimetype)
        if archive.by_name("META-INF/container.xml").is_ok() {
            return Some(DetectResult {
                format: Format::Epub,
                confidence: 0.9,
                mime_type: Format::Epub.mime_type(),
            });
        }

        // DOCX: check for [Content_Types].xml with word MIME
        if let Ok(mut ct) = archive.by_name("[Content_Types].xml") {
            let mut content = String::new();
            if ct.read_to_string(&mut content).is_ok()
                && content.contains("application/vnd.openxmlformats-officedocument.wordprocessingml")
            {
                return Some(DetectResult {
                    format: Format::Docx,
                    confidence: 1.0,
                    mime_type: Format::Docx.mime_type(),
                });
            }
        }

        // CBZ: ZIP with only image files
        let file_names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
            .collect();
        let all_images = !file_names.is_empty()
            && file_names.iter().all(|name| {
                let lower = name.to_lowercase();
                lower.ends_with(".jpg")
                    || lower.ends_with(".jpeg")
                    || lower.ends_with(".png")
                    || lower.ends_with(".gif")
                    || lower.ends_with(".webp")
                    || lower.ends_with(".bmp")
                    || lower.ends_with('/')  // directory entries
            });
        if all_images {
            return Some(DetectResult {
                format: Format::Cbz,
                confidence: 0.8,
                mime_type: Format::Cbz.mime_type(),
            });
        }
    }

    // Fallback: It's a ZIP but we can't determine the subformat from just the header.
    // This happens when the header doesn't contain the full ZIP central directory.
    // We'll fall through to extension-based detection.
    None
}

/// Check if content looks like an XML-based format (FB2, SSML).
fn detect_xml_format(header: &[u8]) -> Option<DetectResult> {
    // Skip BOM if present
    let text = skip_bom_and_decode(header)?;

    if text.contains("<FictionBook") {
        return Some(DetectResult {
            format: Format::Fb2,
            confidence: 0.95,
            mime_type: Format::Fb2.mime_type(),
        });
    }

    if text.contains("<speak") && text.contains("xmlns") {
        return Some(DetectResult {
            format: Format::Ssml,
            confidence: 0.9,
            mime_type: Format::Ssml.mime_type(),
        });
    }

    None
}

/// Check if content looks like HTML.
fn detect_html(header: &[u8]) -> Option<DetectResult> {
    let text = skip_bom_and_decode(header)?;
    let lower = text.to_lowercase();

    if lower.contains("<!doctype html") || lower.contains("<html") {
        return Some(DetectResult {
            format: Format::Html,
            confidence: 0.85,
            mime_type: Format::Html.mime_type(),
        });
    }

    None
}

/// Detect format by file extension alone (lower confidence).
fn detect_by_extension(ext: &str) -> Option<DetectResult> {
    let (format, confidence) = match ext.to_lowercase().as_str() {
        "epub" => (Format::Epub, 0.6),
        "pdf" => (Format::Pdf, 0.6),
        "mobi" | "prc" => (Format::Mobi, 0.6),
        "azw" | "azw3" | "kf8" | "kfx" => (Format::Azw3, 0.6),
        "html" | "htm" | "xhtml" => (Format::Html, 0.5),
        "md" | "markdown" => (Format::Markdown, 0.7),
        "txt" | "text" => (Format::PlainText, 0.5),
        "fb2" => (Format::Fb2, 0.6),
        "docx" => (Format::Docx, 0.6),
        "cbz" => (Format::Cbz, 0.6),
        "cbr" => (Format::Cbr, 0.6),
        "ssml" => (Format::Ssml, 0.6),
        _ => return None,
    };

    Some(DetectResult {
        format,
        confidence,
        mime_type: format.mime_type(),
    })
}

/// Heuristic content detection for formats without magic bytes.
fn detect_by_content_heuristics(header: &[u8]) -> Option<DetectResult> {
    let text = skip_bom_and_decode(header)?;

    // Markdown heuristics: headings, links, emphasis
    let md_signals = [
        text.contains("\n# ") || text.starts_with("# "),
        text.contains("\n## ") || text.starts_with("## "),
        text.contains("]("),   // markdown link
        text.contains("```"),  // code fence
        text.contains("**"),   // bold
    ];
    let md_score: usize = md_signals.iter().filter(|&&s| s).count();
    if md_score >= 2 {
        return Some(DetectResult {
            format: Format::Markdown,
            confidence: 0.4 + (md_score as f64 * 0.1),
            mime_type: Format::Markdown.mime_type(),
        });
    }

    // Plain text fallback: valid UTF-8 with no binary bytes
    if is_likely_text(header) {
        return Some(DetectResult {
            format: Format::PlainText,
            confidence: 0.3,
            mime_type: Format::PlainText.mime_type(),
        });
    }

    None
}

/// Skip UTF-8 BOM and whitespace, decode to string for content inspection.
fn skip_bom_and_decode(bytes: &[u8]) -> Option<String> {
    let bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    };
    std::str::from_utf8(bytes).ok().map(|s| s.to_string())
}

/// Check if the byte slice looks like text (no binary control characters).
fn is_likely_text(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }

    // Check if valid UTF-8
    if std::str::from_utf8(bytes).is_err() {
        return false;
    }

    // Check for binary control characters (allow tab, newline, carriage return)
    let binary_count = bytes
        .iter()
        .filter(|&&b| b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r')
        .count();

    // Allow up to 0.1% binary bytes (for the occasional form feed etc.)
    binary_count * 1000 < bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_pdf() {
        let header = b"%PDF-1.4 some content here";
        let result = detect(header, None).unwrap();
        assert_eq!(result.format, Format::Pdf);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn test_detect_epub_by_extension() {
        let result = detect(b"", Some("book.epub")).unwrap();
        assert_eq!(result.format, Format::Epub);
        assert!(result.confidence < 1.0); // extension-only = lower confidence
    }

    #[test]
    fn test_detect_mobi_magic() {
        let mut header = vec![0u8; 100];
        header[60..68].copy_from_slice(b"BOOKMOBI");
        let result = detect(&header, None).unwrap();
        assert_eq!(result.format, Format::Mobi);
    }

    #[test]
    fn test_detect_markdown_heuristics() {
        let content = b"# My Title\n\nSome text with **bold** and a [link](http://example.com).\n\n## Section\n";
        let result = detect(content, None).unwrap();
        assert_eq!(result.format, Format::Markdown);
    }

    #[test]
    fn test_detect_plain_text() {
        let content = b"Just some plain text content.\nWith multiple lines.\nNothing special.";
        let result = detect(content, None).unwrap();
        assert_eq!(result.format, Format::PlainText);
    }

    #[test]
    fn test_detect_html() {
        let content = b"<!DOCTYPE html>\n<html><head><title>Test</title></head><body>Hello</body></html>";
        let result = detect(content, None).unwrap();
        assert_eq!(result.format, Format::Html);
    }

    #[test]
    fn test_detect_fb2() {
        let content = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\">";
        let result = detect(content, None).unwrap();
        assert_eq!(result.format, Format::Fb2);
    }

    #[test]
    fn test_detect_by_extension_txt() {
        let result = detect(b"", Some("notes.txt")).unwrap();
        assert_eq!(result.format, Format::PlainText);
    }

    #[test]
    fn test_detect_by_extension_md() {
        let result = detect(b"", Some("README.md")).unwrap();
        assert_eq!(result.format, Format::Markdown);
    }

    #[test]
    fn test_detect_unknown() {
        let binary = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert!(detect(&binary, None).is_err());
    }

    #[test]
    fn test_format_display() {
        assert_eq!(format!("{}", Format::Epub), "EPUB");
        assert_eq!(format!("{}", Format::Pdf), "PDF");
        assert_eq!(format!("{}", Format::PlainText), "TXT");
    }

    #[test]
    fn test_format_extension() {
        assert_eq!(Format::Epub.extension(), "epub");
        assert_eq!(Format::Pdf.extension(), "pdf");
        assert_eq!(Format::Markdown.extension(), "md");
    }

    #[test]
    fn test_is_likely_text() {
        assert!(is_likely_text(b"Hello, world!"));
        assert!(is_likely_text(b"Line 1\nLine 2\tTabbed"));
        assert!(!is_likely_text(b"\x00\x01\x02\x03"));
        assert!(!is_likely_text(b""));
    }

    #[test]
    fn test_skip_bom() {
        let with_bom = [0xEF, 0xBB, 0xBF, b'H', b'i'];
        let result = skip_bom_and_decode(&with_bom).unwrap();
        assert_eq!(result, "Hi");
    }
}
