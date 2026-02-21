//! Standalone metadata editing: get, set, strip, copy.

use crate::document::{Document, Metadata};
use crate::error::MetaError;

pub fn meta_get(doc: &Document, field: &str) -> Option<String> {
    let _ = (doc, field);
    todo!()
}

pub fn meta_set(doc: &mut Document, field: &str, value: &str) -> Result<(), MetaError> {
    let _ = (doc, field, value);
    todo!()
}

pub fn meta_strip(doc: &mut Document, fields: Option<&[&str]>) {
    let _ = (doc, fields);
    todo!()
}

pub fn meta_copy(source: &Metadata, target: &mut Metadata, fields: Option<&[&str]>) {
    let _ = (source, target, fields);
    todo!()
}
