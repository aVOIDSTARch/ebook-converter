//! Auto-fix common ebook issues. Each repair action maps to a ValidationIssue code.

use crate::document::Document;
use crate::validate::ValidationIssue;

#[derive(Debug, Clone)]
pub struct RepairOptions {
    pub fix_metadata: bool,
    pub fix_links: bool,
    pub fix_xml: bool,
    pub fix_encoding: bool,
    pub generate_toc: bool,
    pub fix_zip: bool,
}

impl Default for RepairOptions {
    fn default() -> Self {
        Self {
            fix_metadata: true,
            fix_links: true,
            fix_xml: true,
            fix_encoding: true,
            generate_toc: true,
            fix_zip: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RepairReport {
    pub fixes_applied: Vec<RepairAction>,
    pub fixes_failed: Vec<(RepairAction, String)>,
    pub issues_remaining: Vec<ValidationIssue>,
}

#[derive(Debug, Clone)]
pub struct RepairAction {
    pub code: String,
    pub description: String,
}

pub fn repair(doc: &mut Document, opts: &RepairOptions) -> RepairReport {
    let _ = (doc, opts);
    todo!()
}
