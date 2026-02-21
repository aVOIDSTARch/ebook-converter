//! Format detection via magic bytes, file extension, and content heuristics.

use crate::error::DetectError;

#[derive(Debug, Clone)]
pub struct DetectResult {
    pub format: Format,
    pub confidence: f64,
    pub mime_type: &'static str,
}

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

/// Detect the format of a file by reading its first bytes and checking magic signatures.
pub fn detect(header: &[u8], filename: Option<&str>) -> Result<DetectResult, DetectError> {
    let _ = (header, filename);
    todo!()
}

/// Convenience: detect format from a file path.
pub fn detect_file(path: &std::path::Path) -> Result<DetectResult, DetectError> {
    let _ = path;
    todo!()
}
