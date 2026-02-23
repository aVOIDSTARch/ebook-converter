//! Plain text writer: flatten document content to UTF-8 text.

use std::io::Write;

use crate::document::*;
use crate::error::WriteError;
use crate::writers::{FormatWriter, WriteOptions};
use crate::progress::ProgressHandler;

pub struct TxtWriter;

impl FormatWriter for TxtWriter {
    fn write<W: std::io::Write>(
        doc: &Document,
        mut output: W,
        _opts: &WriteOptions,
        _progress: Option<&dyn ProgressHandler>,
    ) -> Result<(), WriteError> {
        if let Some(ref title) = doc.metadata.title {
            writeln!(output, "{}", title)?;
            writeln!(output)?;
        }

        for chapter in &doc.content {
            if let Some(ref title) = chapter.title {
                writeln!(output, "{}", title)?;
                writeln!(output)?;
            }
            for node in &chapter.content {
                write_content_node(node, &mut output)?;
            }
        }

        Ok(())
    }
}

fn write_content_node<W: Write>(node: &ContentNode, output: &mut W) -> Result<(), WriteError> {
    match node {
        ContentNode::Paragraph { children } => {
            let text = flatten_inlines(children);
            writeln!(output, "{}", text)?;
        }
        ContentNode::Heading { level: _, children } => {
            let text = flatten_inlines(children);
            writeln!(output, "{}", text)?;
            writeln!(output)?;
        }
        ContentNode::List { ordered, items } => {
            for (i, item) in items.iter().enumerate() {
                let prefix = if *ordered {
                    format!("{}. ", i + 1)
                } else {
                    "- ".to_string()
                };
                for sub in item {
                    let line = content_node_to_line(sub);
                    writeln!(output, "{}{}", prefix, line)?;
                }
            }
        }
        ContentNode::Table { headers, rows } => {
            let header_line: Vec<String> = headers.iter().map(|c| flatten_inlines(c)).collect();
            writeln!(output, "{}", header_line.join("\t"))?;
            for row in rows {
                let row_line: Vec<String> = row.iter().map(|c| flatten_inlines(c)).collect();
                writeln!(output, "{}", row_line.join("\t"))?;
            }
        }
        ContentNode::BlockQuote { children } => {
            for c in children {
                write_content_node(c, output)?;
            }
        }
        ContentNode::CodeBlock { code, .. } => {
            writeln!(output, "{}", code)?;
        }
        ContentNode::Image {
            resource_id,
            alt_text,
            ..
        } => {
            let placeholder = alt_text
                .as_deref()
                .unwrap_or(resource_id)
                .trim();
            if placeholder.is_empty() {
                writeln!(output, "[image: {}]", resource_id)?;
            } else {
                writeln!(output, "[image: {}]", placeholder)?;
            }
        }
        ContentNode::HorizontalRule => {
            writeln!(output, "---")?;
        }
        ContentNode::RawHtml(s) => {
            writeln!(output, "{}", s)?;
        }
    }
    Ok(())
}

fn content_node_to_line(node: &ContentNode) -> String {
    match node {
        ContentNode::Paragraph { children } => flatten_inlines(children),
        ContentNode::Heading { children, .. } => flatten_inlines(children),
        ContentNode::List { items, .. } => items
            .iter()
            .flat_map(|row| row.iter().map(content_node_to_line))
            .collect::<Vec<_>>()
            .join(" "),
        ContentNode::Table { .. } => "[table]".to_string(),
        ContentNode::BlockQuote { children } => children
            .iter()
            .map(content_node_to_line)
            .collect::<Vec<_>>()
            .join(" "),
        ContentNode::CodeBlock { code, .. } => code.clone(),
        ContentNode::Image { resource_id, .. } => format!("[image: {}]", resource_id),
        ContentNode::HorizontalRule => "---".to_string(),
        ContentNode::RawHtml(s) => s.clone(),
    }
}

fn flatten_inlines(nodes: &[InlineNode]) -> String {
    let mut s = String::new();
    for node in nodes {
        flatten_inline(node, &mut s);
    }
    s
}

fn flatten_inline(node: &InlineNode, out: &mut String) {
    match node {
        InlineNode::Text(t) => out.push_str(t),
        InlineNode::Emphasis(children) | InlineNode::Strong(children) => {
            for c in children {
                flatten_inline(c, out);
            }
        }
        InlineNode::Code(c) => out.push_str(c),
        InlineNode::Link { children, .. } => {
            for c in children {
                flatten_inline(c, out);
            }
        }
        InlineNode::Superscript(children) | InlineNode::Subscript(children) => {
            for c in children {
                flatten_inline(c, out);
            }
        }
        InlineNode::Ruby { base, .. } => out.push_str(base),
        InlineNode::LineBreak => out.push(' '),
    }
}
