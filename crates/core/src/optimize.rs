//! Size optimization: image recompression, font subsetting, CSS/HTML minification.

use crate::document::Document;

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

#[derive(Debug, Clone)]
pub struct OptimizeReport {
    pub original_size_bytes: u64,
    pub optimized_size_bytes: u64,
    pub actions: Vec<String>,
}

pub fn optimize(doc: &mut Document, opts: &OptimizeOptions) -> OptimizeReport {
    let _ = (doc, opts);
    todo!()
}
