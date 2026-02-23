//! C-ABI FFI bindings for ebook-converter.

use std::ffi::CStr;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::c_char;
use std::path::Path;

use ebook_converter_core::convert::{convert_path, parse_format, read_document};
use ebook_converter_core::detect::detect;
use ebook_converter_core::readers::ReadOptions;
use ebook_converter_core::validate::{validate, ValidateOptions, WcagLevel};
use ebook_converter_core::writers::WriteOptions;

/// Convert an ebook file. Returns 0 on success, non-zero on error.
#[no_mangle]
pub extern "C" fn ebook_convert(
    input_path: *const c_char,
    output_path: *const c_char,
    output_format: *const c_char,
) -> i32 {
    if input_path.is_null() || output_path.is_null() {
        return -1;
    }
    let input = match unsafe { CStr::from_ptr(input_path).to_str() } {
        Ok(s) => s,
        Err(_) => return -2,
    };
    let output = match unsafe { CStr::from_ptr(output_path).to_str() } {
        Ok(s) => s,
        Err(_) => return -2,
    };
    let format_str = if output_format.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(output_format).to_str().ok() }
    };
    let format = format_str
        .and_then(parse_format)
        .unwrap_or(ebook_converter_core::detect::Format::Epub);
    let read_opts = ReadOptions::default();
    let write_opts = WriteOptions::default();
    match convert_path(
        Path::new(input),
        Path::new(output),
        format,
        &read_opts,
        &write_opts,
    ) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

/// Validate an ebook file. Returns 0 if valid, non-zero if invalid or error.
#[no_mangle]
pub extern "C" fn ebook_validate(input_path: *const c_char) -> i32 {
    if input_path.is_null() {
        return -1;
    }
    let input = match unsafe { CStr::from_ptr(input_path).to_str() } {
        Ok(s) => s,
        Err(_) => return -2,
    };
    let path = Path::new(input);
    let doc = match std::fs::File::open(path) {
        Ok(f) => {
            let mut r = std::io::BufReader::new(f);
            let mut header = vec![0u8; 4096];
            let n = match r.read(&mut header) {
                Ok(n) => n,
                Err(_) => return -3,
            };
            header.truncate(n);
            r.seek(std::io::SeekFrom::Start(0)).ok();
            let filename = path.file_name().and_then(|p| p.to_str());
            let detected = match detect(&header, filename) {
                Ok(d) => d,
                Err(_) => return -4,
            };
            match read_document(
                detected.format,
                r,
                &ReadOptions::default(),
                None,
            ) {
                Ok(d) => d,
                Err(_) => return -5,
            }
        }
        Err(_) => return -3,
    };
    let opts = ValidateOptions {
        strict: false,
        accessibility: false,
        wcag_level: WcagLevel::Aa,
    };
    let issues = validate(&doc, &opts);
    let has_errors = issues.iter().any(|i| matches!(i.severity, ebook_converter_core::validate::Severity::Error));
    if has_errors {
        1
    } else {
        0
    }
}
