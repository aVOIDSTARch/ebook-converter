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

pub fn validate(doc: &Document, opts: &ValidateOptions) -> Vec<ValidationIssue> {
    let _ = (doc, opts);
    todo!()
}
