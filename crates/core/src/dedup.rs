//! Duplicate detection: hash, ISBN, fuzzy metadata, content fingerprint.

use std::path::Path;

use crate::error::DedupError;

#[derive(Debug, Clone)]
pub enum DuplicateStrategy {
    Hash,
    Isbn,
    Fuzzy,
    ContentFingerprint,
}

#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    pub paths: Vec<std::path::PathBuf>,
    pub strategy: DuplicateStrategy,
    pub similarity: f64,
}

pub fn find_duplicates(
    paths: &[&Path],
    strategy: DuplicateStrategy,
    threshold: f64,
) -> Result<Vec<DuplicateGroup>, DedupError> {
    let _ = (paths, strategy, threshold);
    todo!()
}
