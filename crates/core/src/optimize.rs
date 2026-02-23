//! Size optimization: image recompression, font subsetting, CSS/HTML minification.

use std::collections::HashMap;

use sha2::{Sha256, Digest};

use crate::document::{Document, ResourceMap};

#[derive(Debug, Clone)]
pub struct OptimizeOptions {
    pub image_quality: u8,
    pub subset_fonts: bool,
    pub strip_css: bool,
    pub minify_html: bool,
    pub dedup_resources: bool,
    pub strip_metadata: bool,
}

impl Default for OptimizeOptions {
    fn default() -> Self {
        Self {
            image_quality: 80,
            subset_fonts: true,
            strip_css: true,
            minify_html: false,
            dedup_resources: true,
            strip_metadata: false,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OptimizeReport {
    pub original_size_bytes: u64,
    pub optimized_size_bytes: u64,
    pub actions: Vec<String>,
}

pub fn optimize(doc: &mut Document, opts: &OptimizeOptions) -> OptimizeReport {
    let original_size: u64 = doc.resources.iter().map(|(_, r)| r.data.len() as u64).sum();
    let mut actions = Vec::new();

    if opts.dedup_resources {
        let mut seen: HashMap<[u8; 32], String> = HashMap::new();
        let mut new_resources = ResourceMap::new();
        for (id, res) in doc.resources.iter() {
            let mut hasher = Sha256::new();
            hasher.update(&res.data);
            let key: [u8; 32] = hasher.finalize().into();
            if let Some(first_id) = seen.get(&key) {
                actions.push(format!("Dedup resource {} -> {}", id, first_id));
                continue;
            }
            seen.insert(key, id.clone());
            new_resources.insert(id.clone(), res.clone());
        }
        if new_resources.len() != doc.resources.len() {
            doc.resources = new_resources;
        }
    }

    let optimized_size: u64 = doc.resources.iter().map(|(_, r)| r.data.len() as u64).sum();
    OptimizeReport {
        original_size_bytes: original_size,
        optimized_size_bytes: optimized_size,
        actions,
    }
}
