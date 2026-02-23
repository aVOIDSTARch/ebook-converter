//! Plain text reader: UTF-8 text with optional BOM handling.

use crate::document::*;
use crate::detect::{DetectResult, Format};
use crate::error::ReadError;
use crate::readers::{FormatReader, ReadOptions};
use crate::progress::ProgressHandler;

pub struct TxtReader;

impl FormatReader for TxtReader {
    fn detect(header: &[u8]) -> DetectResult {
        let format = Format::PlainText;
        let confidence = if std::str::from_utf8(header).is_ok() {
            0.6
        } else {
            0.3
        };
        DetectResult {
            format,
            confidence,
            mime_type: format.mime_type(),
        }
    }

    fn read<R: std::io::Read + std::io::Seek>(
        mut input: R,
        opts: &ReadOptions,
        progress: Option<&dyn ProgressHandler>,
    ) -> Result<Document, ReadError> {
        let _ = opts;
        let _ = progress;

        let mut raw = Vec::new();
        input.read_to_end(&mut raw).map_err(ReadError::from)?;

        // Strip BOM if present
        let content = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
            &raw[3..]
        } else {
            &raw[..]
        };

        let text = std::str::from_utf8(content).map_err(|e| ReadError::MalformedFile {
            format: "TXT".into(),
            detail: format!("Invalid UTF-8: {e}"),
        })?;

        // Split into paragraphs: blank lines separate paragraphs; single newlines become spaces within paragraph (or we keep newlines as line breaks)
        let mut content_nodes = Vec::new();
        let paragraphs: Vec<&str> = text
            .split("\n\n")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for para in paragraphs {
            let line = para.replace("\n", " ");
            if line.is_empty() {
                continue;
            }
            content_nodes.push(ContentNode::Paragraph {
                children: vec![InlineNode::Text(line)],
            });
        }

        // If no paragraphs (e.g. empty file or single line without double newline), treat whole text as one paragraph
        if content_nodes.is_empty() && !text.trim().is_empty() {
            let line = text.trim().replace("\n", " ");
            content_nodes.push(ContentNode::Paragraph {
                children: vec![InlineNode::Text(line)],
            });
        }

        let title = text.lines().next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

        let doc = Document {
            metadata: Metadata {
                title: title.clone(),
                ..Default::default()
            },
            toc: vec![],
            content: vec![Chapter {
                id: "chapter-1".to_string(),
                title,
                content: content_nodes,
                text_direction: None,
            }],
            resources: ResourceMap::new(),
            text_direction: TextDirection::default(),
            epub_version: None,
        };

        Ok(doc)
    }
}
