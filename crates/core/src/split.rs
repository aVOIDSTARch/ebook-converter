//! Split a document by chapter, heading, or page count.

use crate::document::Document;
use crate::error::SplitError;

#[derive(Debug, Clone)]
pub enum SplitStrategy {
    Chapter,
    Heading(u8),
    PageCount(u32),
}

pub fn split(doc: &Document, strategy: SplitStrategy) -> Result<Vec<Document>, SplitError> {
    let _ = (doc, strategy);
    todo!()
}
