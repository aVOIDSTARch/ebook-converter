//! EPUB writer: IR → content documents → OPF → ZIP.
//! Defaults to EPUB3; supports EPUB2 via `--epub-version 2`.

use std::io::{Cursor, Write};

use crate::document::*;
use crate::error::WriteError;
use crate::writers::{FormatWriter, WriteOptions};
use crate::progress::ProgressHandler;

fn zip_err(e: zip::result::ZipError) -> WriteError {
    WriteError::WriteFailed {
        format: "EPUB".into(),
        detail: e.to_string(),
    }
}

pub struct EpubWriter;

const OPF_DIR: &str = "OEBPS/";
const OPF_PATH: &str = "OEBPS/content.opf";

impl FormatWriter for EpubWriter {
    fn write<W: std::io::Write>(
        doc: &Document,
        output: W,
        opts: &WriteOptions,
        progress: Option<&dyn ProgressHandler>,
    ) -> Result<(), WriteError> {
        let _ = progress;

        let epub3 = opts
            .epub_version
            .map(|v| v == EpubVersion::V3)
            .unwrap_or(true);

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);

        let opts_store: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let opts_deflate: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // 1. mimetype first, uncompressed (EPUB requirement)
        zip.start_file("mimetype", opts_store).map_err(zip_err)?;
        zip.write_all(b"application/epub+zip")?;

        // 2. META-INF/container.xml
        zip.start_file("META-INF/container.xml", opts_deflate).map_err(zip_err)?;
        write!(
            zip,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="{}" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
            OPF_PATH
        )?;

        // 3. OPF
        zip.start_file(OPF_PATH, opts_deflate).map_err(zip_err)?;
        write_opf(doc, epub3, &mut zip)?;

        // 4. Content XHTML files
        for (i, chapter) in doc.content.iter().enumerate() {
            let href = format!("chapter{}.xhtml", i + 1);
            zip.start_file(
                format!("{}{}", OPF_DIR, href),
                opts_deflate,
            ).map_err(zip_err)?;
            write_chapter_xhtml(chapter, &mut zip)?;
        }

        // 5. Resources (images, etc.)
        for (id, res) in doc.resources.iter() {
            let default_name = format!("{}.bin", id);
            let name = res.filename.as_deref().unwrap_or(&default_name);
            let path = format!("{}resources/{}", OPF_DIR, name);
            zip.start_file(path.clone(), opts_deflate).map_err(zip_err)?;
            zip.write_all(&res.data)?;
        }

        let cursor = zip.finish().map_err(|e| WriteError::WriteFailed {
            format: "EPUB".into(),
            detail: format!("Zip finish: {}", e),
        })?;
        let buffer = cursor.into_inner();

        let mut out = output;
        out.write_all(&buffer)?;

        Ok(())
    }
}

fn write_opf<W: Write>(doc: &Document, epub3: bool, w: &mut W) -> Result<(), WriteError> {
    let version = if epub3 { "3.0" } else { "2.0" };
    let uid = doc
        .metadata
        .isbn_13
        .as_deref()
        .or(doc.metadata.isbn_10.as_deref())
        .unwrap_or("urn:uuid:default");

    writeln!(
        w,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="{}" unique-identifier="uid">"#,
        version
    )?;
    writeln!(w, "  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">")?;

    if let Some(ref t) = doc.metadata.title {
        writeln!(w, "    <dc:title>{}</dc:title>", escape_xml(t))?;
    }
    for a in &doc.metadata.authors {
        writeln!(w, "    <dc:creator>{}</dc:creator>", escape_xml(a))?;
    }
    if let Some(ref lang) = doc.metadata.language {
        writeln!(w, "    <dc:language>{}</dc:language>", escape_xml(lang))?;
    }
    writeln!(w, "    <dc:identifier id=\"uid\">{}</dc:identifier>", escape_xml(uid))?;
    if let Some(ref p) = doc.metadata.publisher {
        writeln!(w, "    <dc:publisher>{}</dc:publisher>", escape_xml(p))?;
    }
    if let Some(ref d) = doc.metadata.description {
        writeln!(w, "    <dc:description>{}</dc:description>", escape_xml(d))?;
    }

    writeln!(w, "  </metadata>")?;
    writeln!(w, "  <manifest>")?;

    for (i, _) in doc.content.iter().enumerate() {
        let id = format!("chapter{}", i + 1);
        let href = format!("chapter{}.xhtml", i + 1);
        writeln!(
            w,
            "    <item id=\"{}\" href=\"{}\" media-type=\"application/xhtml+xml\"/>",
            id, href
        )?;
    }

    for (id, res) in doc.resources.iter() {
        let default_name = format!("{}.bin", id);
        let name = res.filename.as_deref().unwrap_or(&default_name);
        let href = format!("resources/{}", name);
        writeln!(
            w,
            "    <item id=\"{}\" href=\"{}\" media-type=\"{}\"/>",
            id,
            href,
            escape_xml(&res.media_type)
        )?;
    }

    writeln!(w, "  </manifest>")?;
    writeln!(w, "  <spine>")?;

    for (i, _) in doc.content.iter().enumerate() {
        let id = format!("chapter{}", i + 1);
        writeln!(w, "    <itemref idref=\"{}\"/>", id)?;
    }

    writeln!(w, "  </spine>")?;
    writeln!(w, "</package>")?;

    Ok(())
}

fn write_chapter_xhtml<W: Write>(chapter: &Chapter, w: &mut W) -> Result<(), WriteError> {
    writeln!(
        w,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
  <meta charset="UTF-8"/>
  <title>{}</title>
</head>
<body>"#,
        chapter
            .title
            .as_deref()
            .unwrap_or("Chapter")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('&', "&amp;")
    )?;

    for node in &chapter.content {
        write_content_node_xhtml(node, w)?;
    }

    writeln!(w, "</body>")?;
    writeln!(w, "</html>")?;

    Ok(())
}

fn write_content_node_xhtml<W: Write>(node: &ContentNode, w: &mut W) -> Result<(), WriteError> {
    match node {
        ContentNode::Paragraph { children } => {
            write!(w, "<p>")?;
            for c in children {
                write_inline_xhtml(c, w)?;
            }
            writeln!(w, "</p>")?;
        }
        ContentNode::Heading { level, children } => {
            write!(w, "<h{}>", level)?;
            for c in children {
                write_inline_xhtml(c, w)?;
            }
            writeln!(w, "</h{}>", level)?;
        }
        ContentNode::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            writeln!(w, "<{}>", tag)?;
            for item in items {
                write!(w, "<li>")?;
                for sub in item {
                    write_content_node_xhtml(sub, w)?;
                }
                writeln!(w, "</li>")?;
            }
            writeln!(w, "</{}>", tag)?;
        }
        ContentNode::Table { headers, rows } => {
            writeln!(w, "<table>")?;
            writeln!(w, "<thead><tr>")?;
            for cell in headers {
                write!(w, "<th>")?;
                for c in cell {
                    write_inline_xhtml(c, w)?;
                }
                write!(w, "</th>")?;
            }
            writeln!(w, "</tr></thead><tbody>")?;
            for row in rows {
                write!(w, "<tr>")?;
                for cell in row {
                    write!(w, "<td>")?;
                    for c in cell {
                        write_inline_xhtml(c, w)?;
                    }
                    write!(w, "</td>")?;
                }
                writeln!(w, "</tr>")?;
            }
            writeln!(w, "</tbody></table>")?;
        }
        ContentNode::BlockQuote { children } => {
            writeln!(w, "<blockquote>")?;
            for c in children {
                write_content_node_xhtml(c, w)?;
            }
            writeln!(w, "</blockquote>")?;
        }
        ContentNode::CodeBlock { code, .. } => {
            writeln!(w, "<pre><code>{}</code></pre>", escape_xml(code))?;
        }
        ContentNode::Image {
            resource_id,
            alt_text,
            ..
        } => {
            let alt = alt_text
                .as_deref()
                .unwrap_or("")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('&', "&amp;")
                .replace('"', "&quot;");
            writeln!(
                w,
                "<img src=\"resources/{}\" alt=\"{}\"/>",
                resource_id.replace('"', "%22"),
                alt
            )?;
        }
        ContentNode::HorizontalRule => {
            writeln!(w, "<hr/>")?;
        }
        ContentNode::RawHtml(s) => {
            write!(w, "{}", s)?;
        }
    }
    Ok(())
}

fn write_inline_xhtml<W: Write>(node: &InlineNode, w: &mut W) -> Result<(), WriteError> {
    match node {
        InlineNode::Text(s) => write!(w, "{}", escape_xml(s))?,
        InlineNode::Emphasis(children) => {
            write!(w, "<em>")?;
            for c in children {
                write_inline_xhtml(c, w)?;
            }
            write!(w, "</em>")?;
        }
        InlineNode::Strong(children) => {
            write!(w, "<strong>")?;
            for c in children {
                write_inline_xhtml(c, w)?;
            }
            write!(w, "</strong>")?;
        }
        InlineNode::Code(s) => write!(w, "<code>{}</code>", escape_xml(s))?,
        InlineNode::Link { href, children } => {
            let href_esc = href
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");
            write!(w, "<a href=\"{}\">", href_esc)?;
            for c in children {
                write_inline_xhtml(c, w)?;
            }
            write!(w, "</a>")?;
        }
        InlineNode::Superscript(children) => {
            write!(w, "<sup>")?;
            for c in children {
                write_inline_xhtml(c, w)?;
            }
            write!(w, "</sup>")?;
        }
        InlineNode::Subscript(children) => {
            write!(w, "<sub>")?;
            for c in children {
                write_inline_xhtml(c, w)?;
            }
            write!(w, "</sub>")?;
        }
        InlineNode::Ruby { base, annotation } => {
            write!(
                w,
                "<ruby>{}<rt>{}</rt></ruby>",
                escape_xml(base),
                escape_xml(annotation)
            )?;
        }
        InlineNode::LineBreak => write!(w, "<br/>")?,
    }
    Ok(())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
