//! Format writers — each format implements FormatWriter to emit from the Document IR.

pub mod epub;
pub mod txt;

// Phase 2
// pub mod html;
// pub mod markdown;
// pub mod ssml;

// Phase 3
// pub mod pdf;

use crate::document::{Document, EpubVersion};
use crate::error::WriteError;
use crate::progress::ProgressHandler;
use crate::transform::Transform;

pub trait FormatWriter: Send + Sync {
    /// Write the document to a byte sink.
    fn write<W: std::io::Write>(
        doc: &Document,
        output: W,
        opts: &WriteOptions,
        progress: Option<&dyn ProgressHandler>,
    ) -> Result<(), WriteError>
    where
        Self: Sized;
}

pub struct WriteOptions {
    pub image_quality: u8,
    pub epub_version: Option<EpubVersion>,
    pub embed_fonts: bool,
    pub minify: bool,
    pub transforms: Vec<Box<dyn Transform>>,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            image_quality: 80,
            epub_version: None,
            embed_fonts: true,
            minify: false,
            transforms: Vec::new(),
        }
    }
}
