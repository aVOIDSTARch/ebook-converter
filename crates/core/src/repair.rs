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

#[derive(Debug, Clone, serde::Serialize)]
pub struct RepairReport {
    pub fixes_applied: Vec<RepairAction>,
    pub fixes_failed: Vec<(RepairAction, String)>,
    pub issues_remaining: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RepairAction {
    pub code: String,
    pub description: String,
}

pub fn repair(doc: &mut Document, opts: &RepairOptions) -> RepairReport {
    let mut fixes_applied = Vec::new();
    let fixes_failed = Vec::new();

    if opts.fix_encoding {
        crate::encoding::normalize_encoding(doc, &crate::encoding::EncodingOptions::default());
        fixes_applied.push(RepairAction {
            code: "encoding".to_string(),
            description: "Normalized text encoding".to_string(),
        });
    }

    if opts.fix_metadata && doc.metadata.language.as_deref().unwrap_or("").is_empty() {
        doc.metadata.language = Some("en".to_string());
        fixes_applied.push(RepairAction {
            code: "metadata-language".to_string(),
            description: "Set default language".to_string(),
        });
    }

    let issues_remaining = crate::validate::validate(doc, &crate::validate::ValidateOptions::default());

    RepairReport {
        fixes_applied,
        fixes_failed,
        issues_remaining,
    }
}
