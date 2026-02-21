//! Merge multiple documents into one.

use crate::document::Document;
use crate::error::MergeError;

#[derive(Debug, Clone, Default)]
pub struct MergeOptions {
    pub deduplicate_resources: bool,
}

pub fn merge(docs: &[Document], opts: &MergeOptions) -> Result<Document, MergeError> {
    let _ = (docs, opts);
    todo!()
}
