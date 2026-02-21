//! EPUB Accessibility 1.0 / WCAG compliance checking.

use crate::document::Document;
use crate::validate::{ValidationIssue, WcagLevel};

pub fn check_accessibility(
    doc: &Document,
    wcag_level: WcagLevel,
) -> Vec<ValidationIssue> {
    let _ = (doc, wcag_level);
    todo!()
}
