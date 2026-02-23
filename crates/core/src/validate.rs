//! Per-format structural validation.

use serde::{Deserialize, Serialize};

use crate::document::Document;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateOptions {
    pub strict: bool,
    pub accessibility: bool,
    pub wcag_level: WcagLevel,
}

impl Default for ValidateOptions {
    fn default() -> Self {
        Self {
            strict: false,
            accessibility: false,
            wcag_level: WcagLevel::Aa,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub location: Option<String>,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WcagLevel {
    A,
    Aa,
    Aaa,
}

impl WcagLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "A" => WcagLevel::A,
            "AA" => WcagLevel::Aa,
            "AAA" => WcagLevel::Aaa,
            _ => WcagLevel::Aa,
        }
    }
}

pub fn validate(doc: &Document, opts: &ValidateOptions) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if doc.content.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            code: "empty-content".to_string(),
            message: "Document has no chapters".to_string(),
            location: None,
            auto_fixable: false,
        });
    }

    for (i, chapter) in doc.content.iter().enumerate() {
        if chapter.id.is_empty() {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                code: "empty-chapter-id".to_string(),
                message: format!("Chapter {} has empty id", i + 1),
                location: Some(format!("chapter[{}]", i)),
                auto_fixable: true,
            });
        }
    }

    for (id, _) in doc.resources.iter() {
        if id.is_empty() {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                code: "empty-resource-id".to_string(),
                message: "Resource has empty id".to_string(),
                location: None,
                auto_fixable: false,
            });
        }
    }

    if doc.metadata.language.is_none() || doc.metadata.language.as_deref() == Some("") {
        issues.push(ValidationIssue {
            severity: if opts.strict { Severity::Error } else { Severity::Warning },
            code: "missing-language".to_string(),
            message: "Document language is not set".to_string(),
            location: Some("metadata".to_string()),
            auto_fixable: true,
        });
    }

    if opts.accessibility {
        issues.extend(crate::accessibility::check_accessibility(doc, opts.wcag_level));
    }

    issues
}
