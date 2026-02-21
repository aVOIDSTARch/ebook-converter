use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The unified intermediate representation for all ebook formats.
/// Every format is parsed into this struct, and every writer emits from it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub metadata: Metadata,
    pub toc: Vec<TocEntry>,
    pub content: Vec<Chapter>,
    pub resources: ResourceMap,
    pub text_direction: TextDirection,
    pub epub_version: Option<EpubVersion>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub language: Option<String>,
    pub publisher: Option<String>,
    pub publish_date: Option<String>,
    pub isbn_10: Option<String>,
    pub isbn_13: Option<String>,
    pub description: Option<String>,
    pub subjects: Vec<String>,
    pub series: Option<SeriesInfo>,
    pub cover_image_id: Option<String>,
    pub page_count: Option<u32>,
    pub rights: Option<String>,
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesInfo {
    pub name: String,
    pub position: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub title: String,
    pub href: String,
    pub children: Vec<TocEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub title: Option<String>,
    pub content: Vec<ContentNode>,
    pub text_direction: Option<TextDirection>,
}

/// Content nodes — the core building blocks of document content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentNode {
    Paragraph {
        children: Vec<InlineNode>,
    },
    Heading {
        level: u8,
        children: Vec<InlineNode>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<ContentNode>>,
    },
    Table {
        headers: Vec<Vec<InlineNode>>,
        rows: Vec<Vec<Vec<InlineNode>>>,
    },
    BlockQuote {
        children: Vec<ContentNode>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Image {
        resource_id: String,
        alt_text: Option<String>,
        caption: Option<String>,
    },
    HorizontalRule,
    RawHtml(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InlineNode {
    Text(String),
    Emphasis(Vec<InlineNode>),
    Strong(Vec<InlineNode>),
    Code(String),
    Link {
        href: String,
        children: Vec<InlineNode>,
    },
    Superscript(Vec<InlineNode>),
    Subscript(Vec<InlineNode>),
    Ruby {
        base: String,
        annotation: String,
    },
    LineBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextDirection {
    Ltr,
    Rtl,
    Auto,
}

impl Default for TextDirection {
    fn default() -> Self {
        Self::Ltr
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpubVersion {
    V2,
    V3,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceMap {
    resources: HashMap<String, Resource>,
}

impl ResourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: String, resource: Resource) {
        self.resources.insert(id, resource);
    }

    pub fn get(&self, id: &str) -> Option<&Resource> {
        self.resources.get(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<Resource> {
        self.resources.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Resource)> {
        self.resources.iter()
    }

    pub fn len(&self) -> usize {
        self.resources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub media_type: String,
    pub data: Vec<u8>,
    pub filename: Option<String>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            metadata: Metadata::default(),
            toc: Vec::new(),
            content: Vec::new(),
            resources: ResourceMap::default(),
            text_direction: TextDirection::default(),
            epub_version: None,
        }
    }
}
