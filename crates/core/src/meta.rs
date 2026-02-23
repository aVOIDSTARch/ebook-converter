//! Standalone metadata editing: get, set, strip, copy.

use crate::document::{Document, Metadata};
use crate::error::MetaError;

pub fn meta_get(doc: &Document, field: &str) -> Option<String> {
    let m = &doc.metadata;
    match field.to_lowercase().as_str() {
        "title" => m.title.clone(),
        "subtitle" => m.subtitle.clone(),
        "author" | "authors" => if m.authors.is_empty() { None } else { Some(m.authors.join(", ")) },
        "language" => m.language.clone(),
        "publisher" => m.publisher.clone(),
        "publish_date" => m.publish_date.clone(),
        "isbn" | "isbn_10" => m.isbn_10.clone(),
        "isbn_13" => m.isbn_13.clone(),
        "description" => m.description.clone(),
        "rights" => m.rights.clone(),
        _ => m.custom.get(field).cloned(),
    }
}

pub fn meta_set(doc: &mut Document, field: &str, value: &str) -> Result<(), MetaError> {
    let m = &mut doc.metadata;
    match field.to_lowercase().as_str() {
        "title" => { m.title = Some(value.to_string()); }
        "subtitle" => { m.subtitle = Some(value.to_string()); }
        "author" | "authors" => { m.authors = value.split(',').map(|s| s.trim().to_string()).collect(); }
        "language" => { m.language = Some(value.to_string()); }
        "publisher" => { m.publisher = Some(value.to_string()); }
        "publish_date" => { m.publish_date = Some(value.to_string()); }
        "isbn_10" => { m.isbn_10 = Some(value.to_string()); }
        "isbn_13" => { m.isbn_13 = Some(value.to_string()); }
        "description" => { m.description = Some(value.to_string()); }
        "rights" => { m.rights = Some(value.to_string()); }
        _ => { m.custom.insert(field.to_string(), value.to_string()); }
    }
    Ok(())
}

pub fn meta_strip(doc: &mut Document, fields: Option<&[&str]>) {
    let m = &mut doc.metadata;
    let default_strip: &[&str] = &["subtitle", "publisher", "publish_date", "isbn_10", "isbn_13", "description", "rights"];
    let to_strip: &[&str] = fields.unwrap_or(default_strip);
    for f in to_strip {
        match f.to_lowercase().as_str() {
            "title" => m.title = None,
            "subtitle" => m.subtitle = None,
            "authors" | "author" => m.authors.clear(),
            "language" => m.language = None,
            "publisher" => m.publisher = None,
            "publish_date" => m.publish_date = None,
            "isbn_10" => m.isbn_10 = None,
            "isbn_13" => m.isbn_13 = None,
            "description" => m.description = None,
            "rights" => m.rights = None,
            _ => { m.custom.remove(&f.to_string()); }
        }
    }
}

pub fn meta_copy(source: &Metadata, target: &mut Metadata, fields: Option<&[&str]>) {
    let default_fields: &[&str] = &["title", "authors", "language", "publisher", "publish_date", "isbn_10", "isbn_13", "description"];
    let fields: &[&str] = fields.unwrap_or(default_fields);
    for f in fields {
        match f.to_lowercase().as_str() {
            "title" => target.title = source.title.clone(),
            "subtitle" => target.subtitle = source.subtitle.clone(),
            "authors" | "author" => target.authors = source.authors.clone(),
            "language" => target.language = source.language.clone(),
            "publisher" => target.publisher = source.publisher.clone(),
            "publish_date" => target.publish_date = source.publish_date.clone(),
            "isbn_10" => target.isbn_10 = source.isbn_10.clone(),
            "isbn_13" => target.isbn_13 = source.isbn_13.clone(),
            "description" => target.description = source.description.clone(),
            "rights" => target.rights = source.rights.clone(),
            _ => { if let Some(v) = source.custom.get(&f.to_string()) { target.custom.insert(f.to_string(), v.clone()); } }
        }
    }
}
