//! Reading statistics: word count, reading time, readability score.

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

use crate::document::{ContentNode, Document, InlineNode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStats {
    pub word_count: u64,
    pub character_count: u64,
    pub sentence_count: u64,
    pub chapter_count: u32,
    pub image_count: u32,
    pub resource_size_bytes: u64,
    pub estimated_reading_time_minutes: f32,
    pub flesch_kincaid_grade: Option<f32>,
}

impl Document {
    pub fn stats(&self) -> DocumentStats {
        let mut word_count: u64 = 0;
        let mut character_count: u64 = 0;
        let mut sentence_count: u64 = 0;
        let mut image_count: u32 = 0;

        for chapter in &self.content {
            for node in &chapter.content {
                let (words, chars, sents, imgs) = content_node_stats(node);
                word_count += words;
                character_count += chars;
                sentence_count += sents;
                image_count += imgs;
            }
        }

        let resource_size_bytes: u64 = self.resources.iter().map(|(_, r)| r.data.len() as u64).sum();

        let estimated_reading_time_minutes = if word_count > 0 {
            (word_count as f32 / 200.0) / 60.0
        } else {
            0.0
        };

        let flesch_kincaid_grade = if sentence_count > 0 && word_count > 0 {
            let words_per_sent = word_count as f32 / sentence_count as f32;
            let syllables = word_count * 2; // rough approximation
            let syll_per_word = syllables as f32 / word_count as f32;
            Some(
                0.39 * words_per_sent + 11.8 * syll_per_word - 15.59
            )
        } else {
            None
        };

        DocumentStats {
            word_count,
            character_count,
            sentence_count,
            chapter_count: self.content.len() as u32,
            image_count,
            resource_size_bytes,
            estimated_reading_time_minutes,
            flesch_kincaid_grade,
        }
    }
}

fn content_node_stats(node: &ContentNode) -> (u64, u64, u64, u32) {
    match node {
        ContentNode::Paragraph { children } => inline_stats(children, 0, 0, 0),
        ContentNode::Heading { children, .. } => inline_stats(children, 0, 0, 0),
        ContentNode::List { items, .. } => {
            let mut w = 0u64;
            let mut c = 0u64;
            let mut s = 0u64;
            for row in items {
                for n in row {
                    let (tw, tc, ts, _) = content_node_stats(n);
                    w += tw;
                    c += tc;
                    s += ts;
                }
            }
            (w, c, s, 0)
        }
        ContentNode::Table { headers, rows } => {
            let mut w = 0u64;
            let mut ch = 0u64;
            let mut sent = 0u64;
            for cell in headers {
                let (tw, tc, ts, _) = inline_stats(cell, 0, 0, 0);
                w += tw;
                ch += tc;
                sent += ts;
            }
            for row in rows {
                for cell in row {
                    let (tw, tc, ts, _) = inline_stats(cell, 0, 0, 0);
                    w += tw;
                    ch += tc;
                    sent += ts;
                }
            }
            (w, ch, sent, 0)
        }
        ContentNode::BlockQuote { children } => {
            let mut w = 0u64;
            let mut c = 0u64;
            let mut s = 0u64;
            for n in children {
                let (tw, tc, ts, _) = content_node_stats(n);
                w += tw;
                c += tc;
                s += ts;
            }
            (w, c, s, 0)
        }
        ContentNode::CodeBlock { code, .. } => {
            let words = code.split_whitespace().count() as u64;
            let chars = code.chars().count() as u64;
            let sents = code.matches(|c| c == '.' || c == '!' || c == '?').count() as u64;
            (words, chars, sents, 0)
        }
        ContentNode::Image { .. } => (0, 0, 0, 1),
        ContentNode::HorizontalRule | ContentNode::RawHtml(_) => (0, 0, 0, 0),
    }
}

fn inline_stats(
    nodes: &[InlineNode],
    mut words: u64,
    mut chars: u64,
    mut sents: u64,
) -> (u64, u64, u64, u32) {
    for n in nodes {
        match n {
            InlineNode::Text(t) => {
                let w: u64 = t.split_word_bounds().filter(|s| !s.chars().all(|c| c.is_whitespace())).count() as u64;
                words += w;
                chars += t.chars().count() as u64;
                sents += t.matches(|c: char| c == '.' || c == '!' || c == '?').count() as u64;
            }
            InlineNode::Emphasis(children) | InlineNode::Strong(children) | InlineNode::Link { children, .. } | InlineNode::Superscript(children) | InlineNode::Subscript(children) => {
                let (tw, tc, ts, _) = inline_stats(children, 0, 0, 0);
                words += tw;
                chars += tc;
                sents += ts;
            }
            InlineNode::Code(c) => {
                words += c.split_whitespace().count() as u64;
                chars += c.chars().count() as u64;
            }
            InlineNode::Ruby { base, .. } => {
                words += base.split_whitespace().count() as u64;
                chars += base.chars().count() as u64;
            }
            InlineNode::LineBreak => {}
        }
    }
    (words, chars, sents, 0)
}
