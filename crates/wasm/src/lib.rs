//! WASM bindings for ebook-converter.

use std::io::Cursor;

use ebook_converter_core::convert::{parse_format, read_document, write_document};
use ebook_converter_core::detect::detect;
use ebook_converter_core::readers::ReadOptions;
use ebook_converter_core::validate::{validate, ValidateOptions, WcagLevel};
use ebook_converter_core::writers::WriteOptions;
use wasm_bindgen::prelude::*;

/// Convert ebook data between formats. Input and output are in-memory buffers.
#[wasm_bindgen]
pub fn convert(
    data: &[u8],
    input_format: &str,
    output_format: &str,
) -> Result<Vec<u8>, JsValue> {
    let input_fmt = parse_format(input_format).ok_or_else(|| JsValue::from_str("Unsupported input format"))?;
    let output_fmt = parse_format(output_format).ok_or_else(|| JsValue::from_str("Unsupported output format"))?;

    let header = if data.len() > 4096 { &data[..4096] } else { data };
    let detected = detect(header, None).map_err(|e| JsValue::from_str(&e.to_string()))?;
    if detected.format != input_fmt {
        return Err(JsValue::from_str("Input format mismatch"));
    }

    let mut cursor = Cursor::new(data);
    let doc = read_document(detected.format, &mut cursor, &ReadOptions::default(), None)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut out = Vec::new();
    write_document(output_fmt, &doc, Cursor::new(&mut out), &WriteOptions::default(), None)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(out)
}

/// Validate ebook data. Returns a JSON string of validation issues.
#[wasm_bindgen]
pub fn validate_ebook(data: &[u8], input_format: &str) -> Result<String, JsValue> {
    let _input_fmt = parse_format(input_format).ok_or_else(|| JsValue::from_str("Unsupported format"))?;
    let header = if data.len() > 4096 { &data[..4096] } else { data };
    let detected = detect(header, None).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut cursor = Cursor::new(data);
    let doc = read_document(detected.format, &mut cursor, &ReadOptions::default(), None)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let opts = ValidateOptions {
        strict: false,
        accessibility: false,
        wcag_level: WcagLevel::Aa,
    };
    let issues = validate(&doc, &opts);
    serde_json::to_string(&issues).map_err(|e| JsValue::from_str(&e.to_string()))
}
