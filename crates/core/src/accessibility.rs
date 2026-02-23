//! EPUB Accessibility 1.0 / WCAG compliance checking.

use crate::document::{ContentNode, Document, InlineNode};
use crate::validate::{ValidationIssue, WcagLevel};

pub fn check_accessibility(
    doc: &Document,
    _wcag_level: WcagLevel,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if doc.metadata.language.is_none() {
        issues.push(ValidationIssue {
            severity: crate::validate::Severity::Warning,
            code: "wcag-missing-lang".to_string(),
            message: "Document should have a language (dc:language) for screen readers".to_string(),
            location: Some("metadata".to_string()),
            auto_fixable: true,
        });
    }

    let mut heading_levels: Vec<u8> = Vec::new();
    for (ch_idx, chapter) in doc.content.iter().enumerate() {
        for node in &chapter.content {
            check_content_accessibility(node, &mut issues, &mut heading_levels, Some(ch_idx));
        }
    }

    issues
}

fn check_content_accessibility(
    node: &ContentNode,
    issues: &mut Vec<ValidationIssue>,
    heading_levels: &mut Vec<u8>,
    chapter_idx: Option<usize>,
) {
    match node {
        ContentNode::Heading { level, children } => {
            let level = *level;
            while heading_levels.last().copied().unwrap_or(0) >= level {
                heading_levels.pop();
            }
            if level > 0 && level != heading_levels.last().copied().unwrap_or(0) + 1 && !heading_levels.is_empty() {
                issues.push(ValidationIssue {
                    severity: crate::validate::Severity::Warning,
                    code: "wcag-heading-order".to_string(),
                    message: format!("Heading level {} skips levels; use sequential headings", level),
                    location: chapter_idx.map(|i| format!("chapter[{}]", i)),
                    auto_fixable: false,
                });
            }
            heading_levels.push(level);
            for n in children {
                check_inline_accessible(n, issues, chapter_idx);
            }
        }
        ContentNode::Image { alt_text, resource_id, .. } => {
            if alt_text.as_deref().unwrap_or("").trim().is_empty() {
                issues.push(ValidationIssue {
                    severity: crate::validate::Severity::Warning,
                    code: "wcag-image-alt".to_string(),
                    message: format!("Image '{}' is missing alt text", resource_id),
                    location: chapter_idx.map(|i| format!("chapter[{}]", i)),
                    auto_fixable: true,
                });
            }
        }
        ContentNode::List { items, .. } => {
            for row in items {
                for n in row {
                    check_content_accessibility(n, issues, heading_levels, chapter_idx);
                }
            }
        }
        ContentNode::BlockQuote { children } => {
            for n in children {
                check_content_accessibility(n, issues, heading_levels, chapter_idx);
            }
        }
        ContentNode::Table { headers, rows } => {
            for cell in headers {
                for n in cell {
                    check_inline_accessible(n, issues, chapter_idx);
                }
            }
            for row in rows {
                for cell in row {
                    for n in cell {
                        check_inline_accessible(n, issues, chapter_idx);
                    }
                }
            }
        }
        ContentNode::Paragraph { children } => {
            for n in children {
                check_inline_accessible(n, issues, chapter_idx);
            }
        }
        _ => {}
    }
}

fn check_inline_accessible(
    _node: &InlineNode,
    _issues: &mut Vec<ValidationIssue>,
    _chapter_idx: Option<usize>,
) {
    // Links could be checked for descriptive text; leave as future extension
}
