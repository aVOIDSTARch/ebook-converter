//! Extract cover image from a document.

use crate::document::Document;

/// Get cover image bytes if the document has a cover set and the resource exists.
pub fn extract_cover(doc: &Document) -> Option<(Vec<u8>, String)> {
    let cover_id = doc.metadata.cover_image_id.as_ref()?;
    let res = doc.resources.get(cover_id)?;
    Some((res.data.clone(), res.media_type.clone()))
}
