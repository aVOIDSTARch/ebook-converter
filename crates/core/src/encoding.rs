//! Unicode normalization, encoding detection, smart quotes, ligatures.

use crate::document::Document;

#[derive(Debug, Clone, Copy)]
pub enum UnicodeForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
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

pub fn normalize_encoding(doc: &mut Document, opts: &EncodingOptions) {
    let _ = (doc, opts);
    todo!()
}
