//! Merge multiple documents into one.

use sha2::{Sha256, Digest};

use crate::document::{Document, ResourceMap};
use crate::error::MergeError;

#[derive(Debug, Clone, Default)]
pub struct MergeOptions {
    pub deduplicate_resources: bool,
}

pub fn merge(docs: &[Document], opts: &MergeOptions) -> Result<Document, MergeError> {
    if docs.is_empty() {
        return Err(MergeError::Failed("No documents to merge".to_string()));
    }

    let mut metadata = docs[0].metadata.clone();
    let mut toc = Vec::new();
    let mut content = Vec::new();
    let mut resources = ResourceMap::new();

    for (doc_idx, doc) in docs.iter().enumerate() {
        for entry in &doc.toc {
            toc.push(entry.clone());
        }
        for (ch_idx, chapter) in doc.content.iter().enumerate() {
            let mut ch = chapter.clone();
            ch.id = format!("doc{}-ch{}", doc_idx, ch_idx);
            content.push(ch);
        }

        for (id, res) in doc.resources.iter() {
            if opts.deduplicate_resources {
                let mut hasher = Sha256::new();
                hasher.update(&res.data);
                let key: [u8; 32] = hasher.finalize().into();
                let unique_id = format!("{:02x}{:02x}{:02x}{:02x}", key[0], key[1], key[2], key[3]);
                if resources.iter().any(|(_, r)| r.data == res.data) {
                    continue;
                }
                let mut r = res.clone();
                r.id = unique_id.clone();
                resources.insert(unique_id, r);
            } else {
                let unique_id = format!("doc{}-{}", doc_idx, id);
                let mut r = res.clone();
                r.id = unique_id.clone();
                resources.insert(unique_id, r);
            }
        }
    }

    Ok(Document {
        metadata,
        toc,
        content,
        resources,
        text_direction: docs[0].text_direction,
        epub_version: docs[0].epub_version,
    })
}
