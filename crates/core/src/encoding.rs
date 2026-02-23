//! Unicode normalization, encoding detection, smart quotes, ligatures.

use crate::document::Document;

#[derive(Debug, Clone, Copy)]
pub enum UnicodeForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

impl UnicodeForm {
    /// Parse from config string (e.g. "NFC", "NFD").
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "NFD" => UnicodeForm::Nfd,
            "NFKC" => UnicodeForm::Nfkc,
            "NFKD" => UnicodeForm::Nfkd,
            _ => UnicodeForm::Nfc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncodingOptions {
    pub unicode_form: UnicodeForm,
    pub smart_quotes: bool,
    pub normalize_ligatures: bool,
    pub normalize_dashes: bool,
    pub normalize_whitespace: bool,
    pub fix_macos_nfd: bool,
}

impl Default for EncodingOptions {
    fn default() -> Self {
        Self {
            unicode_form: UnicodeForm::Nfc,
            smart_quotes: false,
            normalize_ligatures: false,
            normalize_dashes: false,
            normalize_whitespace: true,
            fix_macos_nfd: true,
        }
    }
}

pub fn normalize_encoding(doc: &mut Document, _opts: &EncodingOptions) {
    use unicode_normalization::UnicodeNormalization;

    fn normalize_str(s: &str) -> String {
        s.nfc().collect()
    }

    if let Some(ref t) = doc.metadata.title {
        doc.metadata.title = Some(normalize_str(t));
    }
    for a in &mut doc.metadata.authors {
        *a = normalize_str(a);
    }
    if let Some(ref l) = doc.metadata.language {
        doc.metadata.language = Some(normalize_str(l));
    }

    for chapter in &mut doc.content {
        if let Some(ref t) = chapter.title {
            chapter.title = Some(normalize_str(t));
        }
        for node in &mut chapter.content {
            normalize_content_node(node);
        }
    }
}

fn normalize_content_node(node: &mut crate::document::ContentNode) {
    use unicode_normalization::UnicodeNormalization;

    match node {
        crate::document::ContentNode::Paragraph { children } |
        crate::document::ContentNode::Heading { children, .. } => {
            for n in children {
                normalize_inline(n);
            }
        }
        crate::document::ContentNode::List { items, .. } => {
            for row in items {
                for n in row {
                    normalize_content_node(n);
                }
            }
        }
        crate::document::ContentNode::Table { headers, rows } => {
            for cell in headers {
                for n in cell {
                    normalize_inline(n);
                }
            }
            for row in rows {
                for cell in row {
                    for n in cell {
                        normalize_inline(n);
                    }
                }
            }
        }
        crate::document::ContentNode::BlockQuote { children } => {
            for n in children {
                normalize_content_node(n);
            }
        }
        crate::document::ContentNode::CodeBlock { code, .. } => {
            *code = code.nfc().collect();
        }
        crate::document::ContentNode::Image { alt_text, caption, .. } => {
            if let Some(ref t) = alt_text {
                *alt_text = Some(t.nfc().collect());
            }
            if let Some(ref t) = caption {
                *caption = Some(t.nfc().collect());
            }
        }
        _ => {}
    }
}

fn normalize_inline(node: &mut crate::document::InlineNode) {
    use unicode_normalization::UnicodeNormalization;

    match node {
        crate::document::InlineNode::Text(s) => *s = s.nfc().collect(),
        crate::document::InlineNode::Emphasis(children) |
        crate::document::InlineNode::Strong(children) |
        crate::document::InlineNode::Link { children, .. } |
        crate::document::InlineNode::Superscript(children) |
        crate::document::InlineNode::Subscript(children) => {
            for n in children {
                normalize_inline(n);
            }
        }
        crate::document::InlineNode::Code(s) => *s = s.nfc().collect(),
        crate::document::InlineNode::Ruby { base, annotation } => {
            *base = base.nfc().collect();
            *annotation = annotation.nfc().collect();
        }
        crate::document::InlineNode::LineBreak => {}
    }
}
