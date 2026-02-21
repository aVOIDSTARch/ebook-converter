//! Format readers — each format implements FormatReader to parse into the Document IR.

pub mod epub;
pub mod txt;

// Phase 2
// pub mod html;
// pub mod markdown;

// Phase 3
// pub mod pdf;

use crate::document::Document;
use crate::encoding::EncodingOptions;
use crate::error::ReadError;
use crate::progress::ProgressHandler;
use crate::security::SecurityLimits;

pub trait FormatReader: Send + Sync {
    /// Check if this reader can handle the given input. Called with first 4KB.
    fn detect(header: &[u8]) -> crate::detect::DetectResult
    where
        Self: Sized;

    /// Read from a byte source into the IR.
    fn read<R: std::io::Read + std::io::Seek>(
        input: R,
        opts: &ReadOptions,
        progress: Option<&dyn ProgressHandler>,
    ) -> Result<Document, ReadError>
    where
        Self: Sized;
}

#[derive(Debug, Clone)]
pub struct ReadOptions {
    pub security: SecurityLimits,
    pub extract_cover: bool,
    pub parse_toc: bool,
    pub encoding: EncodingOptions,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            security: SecurityLimits::default(),
            extract_cover: true,
            parse_toc: true,
            encoding: EncodingOptions::default(),
        }
    }
}
