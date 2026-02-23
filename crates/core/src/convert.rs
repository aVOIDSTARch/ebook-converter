//! Conversion pipeline: detect → read → transform → write.
//!
//! **Supported format matrix (read × write):**
//!
//! | Input  | Output |
//! |--------|--------|
//! | EPUB   | EPUB, TXT |
//! | TXT    | EPUB, TXT |
//!
//! Other formats (HTML, MD, PDF, SSML, etc.) are detected but readers/writers
//! may not be implemented yet; see PROJECT-TODO-AND-IMPROVEMENTS.md.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom};
use std::path::Path;

use crate::detect::{detect, Format};
use crate::document::Document;
use crate::error::{EbookError, ReadError};
use crate::readers::epub::EpubReader;
use crate::readers::txt::TxtReader;
use crate::readers::{FormatReader, ReadOptions};
use crate::writers::epub::EpubWriter;
use crate::writers::txt::TxtWriter;
use crate::writers::{FormatWriter, WriteOptions};

/// Convert an input file to the given output path and format.
pub fn convert_path(
    input_path: &Path,
    output_path: &Path,
    output_format: Format,
    read_opts: &ReadOptions,
    write_opts: &WriteOptions,
) -> Result<(), EbookError> {
    let mut input_file = File::open(input_path)?;
    let mut header = vec![0u8; 4096];
    let n = input_file.read(&mut header)?;
    header.truncate(n);
    input_file.seek(SeekFrom::Start(0))?;

    let filename = input_path.file_name().and_then(|p| p.to_str());
    let detected = detect(&header, filename)?;
    let input_format = detected.format;

    let input = BufReader::new(input_file);
    let doc = read_document(input_format, input, read_opts, None)?;

    // Apply transforms
    let mut doc = doc;
    for transform in &write_opts.transforms {
        transform.apply(&mut doc)?;
    }

    let output_file = File::create(output_path)?;
    let output = BufWriter::new(output_file);

    write_document(output_format, &doc, output, write_opts, None)?;

    Ok(())
}

/// Read a document from a byte source given the detected format.
pub fn read_document<R: std::io::Read + std::io::Seek>(
    format: Format,
    input: R,
    opts: &ReadOptions,
    progress: Option<&dyn crate::progress::ProgressHandler>,
) -> Result<Document, ReadError> {
    match format {
        Format::Epub => EpubReader::read(input, opts, progress),
        Format::PlainText => TxtReader::read(input, opts, progress),
        _ => Err(ReadError::UnsupportedFormat(format!(
            "reading {} is not yet supported; supported input formats: epub, txt",
            format
        ))),
    }
}

/// Write a document to a byte sink in the given format.
pub fn write_document<W: std::io::Write>(
    format: Format,
    doc: &Document,
    output: W,
    opts: &WriteOptions,
    progress: Option<&dyn crate::progress::ProgressHandler>,
) -> Result<(), crate::error::WriteError> {
    match format {
        Format::Epub => EpubWriter::write(doc, output, opts, progress),
        Format::PlainText => TxtWriter::write(doc, output, opts, progress),
        _ => Err(crate::error::WriteError::WriteFailed {
            format: format!("{:?}", format),
            detail: "Format not supported for writing".to_string(),
        }),
    }
}

/// Parse format from string. Returns a format that has a reader/writer implemented.
/// Additional strings (html, md, ssml, pdf) may be added for CLI/API consistency
/// before their readers/writers exist.
pub fn parse_format(s: &str) -> Option<Format> {
    match s.to_lowercase().as_str() {
        "epub" => Some(Format::Epub),
        "txt" | "text" => Some(Format::PlainText),
        "html" => Some(Format::Html),
        "md" | "markdown" => Some(Format::Markdown),
        "ssml" => Some(Format::Ssml),
        "pdf" => Some(Format::Pdf),
        _ => None,
    }
}
