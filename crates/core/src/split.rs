//! Split a document by chapter, heading, or page count.

use crate::document::Document;
use crate::error::SplitError;

#[derive(Debug, Clone)]
pub enum SplitStrategy {
    Chapter,
    Heading(u8),
    PageCount(u32),
}

pub fn split(doc: &Document, strategy: SplitStrategy) -> Result<Vec<Document>, SplitError> {
    match strategy {
        SplitStrategy::Chapter => {
            let mut out = Vec::new();
            for chapter in &doc.content {
                let new_doc = Document {
                    metadata: doc.metadata.clone(),
                    toc: vec![],
                    content: vec![chapter.clone()],
                    resources: doc.resources.clone(),
                    text_direction: doc.text_direction,
                    epub_version: doc.epub_version,
                };
                out.push(new_doc);
            }
            Ok(out)
        }
        SplitStrategy::Heading(_level) => {
            let mut out = Vec::new();
            for chapter in &doc.content {
                let new_doc = Document {
                    metadata: doc.metadata.clone(),
                    toc: vec![],
                    content: vec![chapter.clone()],
                    resources: doc.resources.clone(),
                    text_direction: doc.text_direction,
                    epub_version: doc.epub_version,
                };
                out.push(new_doc);
            }
            Ok(out)
        }
        SplitStrategy::PageCount(chars_per_page) => {
            if chars_per_page == 0 {
                return Err(SplitError::Failed("Page count must be > 0".to_string()));
            }
            let mut all_content = Vec::new();
            for ch in &doc.content {
                all_content.push(ch.clone());
            }
            let total_chars: usize = doc.content.iter().flat_map(|c| c.content.iter()).map(|n| content_node_char_count(n)).sum();
            let num_parts = (total_chars / chars_per_page as usize).max(1);
            let mut out = Vec::new();
            let mut offset = 0;
            for _ in 0..num_parts {
                let mut content = Vec::new();
                let mut count = 0;
                while offset < doc.content.len() && count < chars_per_page as usize {
                    content.push(doc.content[offset].clone());
                    count += doc.content[offset].content.iter().map(content_node_char_count).sum::<usize>();
                    offset += 1;
                }
                if content.is_empty() {
                    break;
                }
                out.push(Document {
                    metadata: doc.metadata.clone(),
                    toc: vec![],
                    content,
                    resources: doc.resources.clone(),
                    text_direction: doc.text_direction,
                    epub_version: doc.epub_version,
                });
            }
            Ok(out)
        }
    }
}

fn content_node_char_count(n: &crate::document::ContentNode) -> usize {
    match n {
        crate::document::ContentNode::Paragraph { children } => inline_char_count(children),
        crate::document::ContentNode::Heading { children, .. } => inline_char_count(children),
        crate::document::ContentNode::CodeBlock { code, .. } => code.len(),
        crate::document::ContentNode::BlockQuote { children } => children.iter().map(content_node_char_count).sum(),
        crate::document::ContentNode::List { items, .. } => items.iter().flat_map(|r| r.iter()).map(content_node_char_count).sum(),
        crate::document::ContentNode::Table { headers, rows } => {
            headers.iter().flat_map(|c| c.iter()).map(|c| inline_char_count(std::slice::from_ref(c))).sum::<usize>()
                + rows.iter().flat_map(|r| r.iter().flat_map(|c| c.iter())).map(|c| inline_char_count(std::slice::from_ref(c))).sum::<usize>()
        }
        _ => 0,
    }
}

fn inline_char_count(nodes: &[crate::document::InlineNode]) -> usize {
    nodes.iter().map(|n| match n {
        crate::document::InlineNode::Text(s) => s.len(),
        crate::document::InlineNode::Emphasis(c) | crate::document::InlineNode::Strong(c) | crate::document::InlineNode::Link { children: c, .. } => inline_char_count(c),
        crate::document::InlineNode::Code(s) => s.len(),
        crate::document::InlineNode::Ruby { base, annotation } => base.len() + annotation.len(),
        _ => 0,
    }).sum()
}
