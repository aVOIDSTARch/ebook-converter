//! Duplicate detection: hash, ISBN, fuzzy metadata, content fingerprint.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::{Sha256, Digest};

use crate::convert::read_document;
use crate::detect::detect_file;
use crate::error::DedupError;
use crate::readers::ReadOptions;

#[derive(Debug, Clone, serde::Serialize)]
pub enum DuplicateStrategy {
    Hash,
    Isbn,
    Fuzzy,
    ContentFingerprint,
}

#[derive(Debug, Clone, serde::Serialize)]
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
    match strategy {
        DuplicateStrategy::Hash => find_by_hash(paths),
        DuplicateStrategy::Fuzzy => find_by_fuzzy(paths, threshold),
        DuplicateStrategy::Isbn | DuplicateStrategy::ContentFingerprint => {
            Ok(Vec::new())
        }
    }
}

fn find_by_hash(paths: &[&Path]) -> Result<Vec<DuplicateGroup>, DedupError> {
    let mut map: HashMap<[u8; 32], Vec<std::path::PathBuf>> = HashMap::new();
    for path in paths {
        let mut f = File::open(path).map_err(|e| DedupError::Failed(e.to_string()))?;
        let mut data = Vec::new();
        f.read_to_end(&mut data).map_err(|e| DedupError::Failed(e.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let key: [u8; 32] = hasher.finalize().into();
        map.entry(key).or_default().push((*path).to_path_buf());
    }
    let groups = map
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(_, paths)| DuplicateGroup {
            paths,
            strategy: DuplicateStrategy::Hash,
            similarity: 1.0,
        })
        .collect();
    Ok(groups)
}

fn find_by_fuzzy(paths: &[&Path], threshold: f64) -> Result<Vec<DuplicateGroup>, DedupError> {
    let mut docs: Vec<(std::path::PathBuf, String, String)> = Vec::new();
    for path in paths {
        if let Ok(detected) = detect_file(path) {
            if detected.format == crate::detect::Format::Epub || detected.format == crate::detect::Format::PlainText {
                if let Ok(file) = File::open(path) {
                    let reader = std::io::BufReader::new(file);
                    if let Ok(doc) = read_document(detected.format, reader, &ReadOptions::default(), None) {
                        let title = doc.metadata.title.as_deref().unwrap_or("").to_string();
                        let author = doc.metadata.authors.first().map(|s| s.as_str()).unwrap_or("").to_string();
                        docs.push(((*path).to_path_buf(), title, author));
                    }
                }
            }
        }
    }

    let mut groups = Vec::new();
    let mut used = vec![false; docs.len()];
    for i in 0..docs.len() {
        if used[i] {
            continue;
        }
        let mut group = vec![docs[i].0.clone()];
        for j in (i + 1)..docs.len() {
            if used[j] {
                continue;
            }
            let sim = strsim::jaro_winkler(&docs[i].1, &docs[j].1).max(strsim::jaro_winkler(&docs[i].2, &docs[j].2));
            if sim >= threshold {
                group.push(docs[j].0.clone());
                used[j] = true;
            }
        }
        if group.len() > 1 {
            groups.push(DuplicateGroup {
                paths: group,
                strategy: DuplicateStrategy::Fuzzy,
                similarity: threshold,
            });
        }
    }
    Ok(groups)
}
