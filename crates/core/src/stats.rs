//! Reading statistics: word count, reading time, readability score.

use serde::{Deserialize, Serialize};

use crate::document::Document;

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
        let _ = self;
        todo!()
    }
}
