//! Open Library metadata provider.

use crate::error::LookupError;
use crate::lookup::{MetadataProvider, MetadataQuery, MetadataResult};

pub struct OpenLibraryProvider;

impl OpenLibraryProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenLibraryProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataProvider for OpenLibraryProvider {
    fn name(&self) -> &str {
        "openlibrary"
    }

    fn search(&self, query: &MetadataQuery) -> Result<Vec<MetadataResult>, LookupError> {
        let q = query.title.as_deref().unwrap_or("").to_string();
        if q.is_empty() && query.author.is_none() && query.isbn.is_none() {
            return Ok(Vec::new());
        }
        let url = if let Some(isbn) = &query.isbn {
            format!("https://openlibrary.org/isbn/{}.json", isbn.replace('-', ""))
        } else {
            let search_q = format!(
                "{} {}",
                query.title.as_deref().unwrap_or(""),
                query.author.as_deref().unwrap_or("")
            ).trim().replace(' ', "+");
            if search_q.is_empty() {
                return Ok(Vec::new());
            }
            format!("https://openlibrary.org/search.json?q={}", search_q)
        };

        let body = reqwest::blocking::get(&url).map_err(|e| LookupError::Network(e.to_string()))?.text().map_err(|e| LookupError::Network(e.to_string()))?;
        parse_openlibrary_response(&body).map_err(|e| LookupError::ProviderError { provider: "openlibrary".to_string(), message: e })
    }

    fn lookup_isbn(&self, isbn: &str) -> Result<MetadataResult, LookupError> {
        let results = self.search(&MetadataQuery { title: None, author: None, isbn: Some(isbn.to_string()) })?;
        results.into_iter().next().ok_or(LookupError::NotFound)
    }

    fn fetch_cover(&self, _result: &MetadataResult) -> Result<Option<Vec<u8>>, LookupError> {
        Ok(None)
    }
}

fn parse_openlibrary_response(body: &str) -> Result<Vec<MetadataResult>, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    if let Some(docs) = v.get("docs").and_then(|d| d.as_array()) {
        let mut results = Vec::new();
        for doc in docs {
            let title = doc.get("title").and_then(|t| t.as_str()).map(String::from);
            let authors: Vec<String> = doc.get("author_name").and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|v| v.as_str()).map(String::from).collect()).unwrap_or_default();
            let isbn = doc.get("isbn").and_then(|i| i.as_array()).and_then(|a| a.first()).and_then(|v| v.as_str()).map(String::from);
            let description = doc.get("first_sentence").and_then(|s| s.as_str()).map(String::from);
            results.push(MetadataResult {
                title,
                authors,
                isbn_10: None,
                isbn_13: isbn,
                description,
                cover_url: None,
                publisher: None,
                publish_date: doc.get("first_publish_year").and_then(|y| y.as_u64()).map(|y| y.to_string()),
                subjects: Vec::new(),
                series: None,
                series_number: None,
                page_count: None,
                language: None,
            });
        }
        return Ok(results);
    }
    if v.get("title").is_some() {
        let title = v.get("title").and_then(|t| t.as_str()).map(String::from);
        let authors: Vec<String> = v.get("authors").and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|o| o.get("key").and_then(|k| k.as_str()).map(|s| s.replace("/authors/", ""))).collect()).unwrap_or_default();
        return Ok(vec![MetadataResult {
            title,
            authors,
            isbn_10: None,
            isbn_13: v.get("isbn_10").or(v.get("identifiers")).and_then(|x| x.as_str().or_else(|| x.get("isbn_10").and_then(|a| a.as_array()).and_then(|a| a.first()).and_then(|v| v.as_str()))).map(String::from),
            description: None,
            cover_url: None,
            publisher: None,
            publish_date: None,
            subjects: Vec::new(),
            series: None,
            series_number: None,
            page_count: None,
            language: None,
        }]);
    }
    Ok(Vec::new())
}
