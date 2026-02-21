//! Metadata lookup via pluggable providers (Open Library, Google Books, custom).

use crate::error::LookupError;

pub trait MetadataProvider: Send + Sync {
    fn name(&self) -> &str;
    fn search(&self, query: &MetadataQuery) -> Result<Vec<MetadataResult>, LookupError>;
    fn lookup_isbn(&self, isbn: &str) -> Result<MetadataResult, LookupError>;
    fn fetch_cover(&self, result: &MetadataResult) -> Result<Option<Vec<u8>>, LookupError>;
}

#[derive(Debug, Clone)]
pub struct MetadataQuery {
    pub title: Option<String>,
    pub author: Option<String>,
    pub isbn: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MetadataResult {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub isbn_10: Option<String>,
    pub isbn_13: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub publisher: Option<String>,
    pub publish_date: Option<String>,
    pub subjects: Vec<String>,
    pub series: Option<String>,
    pub series_number: Option<f32>,
    pub page_count: Option<u32>,
    pub language: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LookupOptions {
    pub max_results: usize,
    pub use_cache: bool,
}

impl Default for LookupOptions {
    fn default() -> Self {
        Self {
            max_results: 5,
            use_cache: true,
        }
    }
}
