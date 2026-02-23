//! EPUB reader: parse ZIP → OPF → content documents → IR.
//! Supports both EPUB2 (NCX navigation) and EPUB3 (NAV document).

use std::collections::HashMap;
use std::io::{Read, Seek};

use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;

use crate::detect::{DetectResult, Format};
use crate::document::*;
use crate::error::ReadError;
use crate::progress::{emit_progress, ProgressHandler};
use crate::readers::{FormatReader, ReadOptions};
use crate::security;

pub struct EpubReader;

impl EpubReader {
    /// Read an EPUB file from a byte source into the Document IR.
    pub fn read<R: Read + Seek>(
        input: R,
        opts: &ReadOptions,
        progress: Option<&dyn ProgressHandler>,
    ) -> Result<Document, ReadError> {
        read_epub_impl(input, opts, progress)
    }
}

fn read_epub_impl<R: Read + Seek>(
    input: R,
    opts: &ReadOptions,
    progress: Option<&dyn ProgressHandler>,
) -> Result<Document, ReadError> {
    let mut archive = zip::ZipArchive::new(input)
        .map_err(|e| ReadError::MalformedFile {
            format: "EPUB".into(),
            detail: format!("Invalid ZIP archive: {e}"),
        })?;

        // Security checks
        security::check_file_count(archive.len() as u64, &opts.security)?;

        // Check for DRM
        if let Ok(mut enc_file) = archive.by_name("META-INF/encryption.xml") {
            let mut enc_xml = String::new();
            enc_file.read_to_string(&mut enc_xml).ok();
            security::check_epub_drm(&enc_xml)?;
        }

        emit_progress(progress, "Reading EPUB", 0, Some(5), Some("Parsing container"));

        // 1. Find the OPF file path from container.xml
        let opf_path = find_opf_path(&mut archive)?;

        emit_progress(progress, "Reading EPUB", 1, Some(5), Some("Parsing OPF package"));

        // 2. Parse the OPF file
        let opf_dir = opf_path
            .rfind('/')
            .map(|i| &opf_path[..i + 1])
            .unwrap_or("");
        let opf_dir = opf_dir.to_string();

        let opf_content = read_archive_entry(&mut archive, &opf_path, &opts.security)?;
        let opf = parse_opf(&opf_content, &opf_dir)?;

        emit_progress(progress, "Reading EPUB", 2, Some(5), Some("Parsing content"));

        // 3. Read content documents (spine items) → chapters
        let mut chapters = Vec::new();
        let total_spine = opf.spine_items.len();
        for (i, spine_item) in opf.spine_items.iter().enumerate() {
            if let Some(manifest_item) = opf.manifest.get(spine_item) {
                let full_path = format!("{}{}", opf_dir, manifest_item.href);
                match read_archive_entry(&mut archive, &full_path, &opts.security) {
                    Ok(content) => {
                        let chapter = parse_xhtml_to_chapter(
                            &content,
                            spine_item,
                            &opts.security,
                        );
                        chapters.push(chapter);
                    }
                    Err(e) => {
                        tracing::warn!("Skipping spine item '{}': {}", spine_item, e);
                    }
                }
                emit_progress(
                    progress,
                    "Reading EPUB",
                    2,
                    Some(5),
                    Some(&format!("Chapter {}/{}", i + 1, total_spine)),
                );
            }
        }

        emit_progress(progress, "Reading EPUB", 3, Some(5), Some("Loading resources"));

        // 4. Load resources (images, fonts, stylesheets)
        let mut resources = ResourceMap::new();
        for (id, item) in &opf.manifest {
            if is_resource_media_type(&item.media_type) {
                let full_path = format!("{}{}", opf_dir, item.href);
                if let Ok(data) = read_archive_entry_bytes(&mut archive, &full_path, &opts.security)
                {
                    let resource = Resource {
                        id: id.clone(),
                        media_type: item.media_type.clone(),
                        data,
                        filename: Some(item.href.clone()),
                    };
                    resources.insert(id.clone(), resource);
                }
            }
        }

        emit_progress(progress, "Reading EPUB", 4, Some(5), Some("Parsing navigation"));

        // 5. Parse TOC
        let toc = if opts.parse_toc {
            parse_toc(&mut archive, &opf, &opf_dir, &opts.security)
        } else {
            Vec::new()
        };

        emit_progress(progress, "Reading EPUB", 5, Some(5), Some("Done"));

        Ok(Document {
            metadata: opf.metadata,
            toc,
            content: chapters,
            resources,
            text_direction: opf.text_direction,
            epub_version: opf.epub_version,
        })
}

impl FormatReader for EpubReader {
    fn detect(header: &[u8]) -> DetectResult {
        if header.starts_with(b"PK\x03\x04") {
            DetectResult {
                format: Format::Epub,
                confidence: 0.7,
                mime_type: Format::Epub.mime_type(),
            }
        } else {
            DetectResult {
                format: Format::Epub,
                confidence: 0.0,
                mime_type: Format::Epub.mime_type(),
            }
        }
    }

    fn read<R: Read + Seek>(
        input: R,
        opts: &ReadOptions,
        progress: Option<&dyn ProgressHandler>,
    ) -> Result<Document, ReadError> {
        Self::read(input, opts, progress)
    }
}

// --- OPF Parsing ---

#[derive(Debug)]
struct OpfData {
    metadata: Metadata,
    manifest: HashMap<String, ManifestItem>,
    spine_items: Vec<String>,
    epub_version: Option<EpubVersion>,
    text_direction: TextDirection,
    toc_id: Option<String>, // NCX id for EPUB2
    nav_href: Option<String>, // NAV doc href for EPUB3
}

#[derive(Debug, Clone)]
struct ManifestItem {
    href: String,
    media_type: String,
    properties: Option<String>,
}

fn find_opf_path<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Result<String, ReadError> {
    let container = read_archive_entry_string(archive, "META-INF/container.xml")?;

    let mut reader = XmlReader::from_str(&container);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e))
                if e.local_name().as_ref() == b"rootfile" =>
            {
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"full-path" {
                        return Ok(String::from_utf8_lossy(&attr.value).to_string());
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ReadError::MalformedFile {
                    format: "EPUB".into(),
                    detail: format!("Failed to parse container.xml: {e}"),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    Err(ReadError::MissingContent(
        "No rootfile found in container.xml".into(),
    ))
}

fn parse_opf(content: &str, _opf_dir: &str) -> Result<OpfData, ReadError> {
    let mut reader = XmlReader::from_str(content);
    let mut buf = Vec::new();

    let mut metadata = Metadata::default();
    let mut manifest = HashMap::new();
    let mut spine_items = Vec::new();
    let mut epub_version = None;
    let mut text_direction = TextDirection::Ltr;
    let mut toc_id = None;
    let mut nav_href = None;

    // Track parsing state
    let mut in_metadata = false;
    let mut current_element: Option<String> = None;
    let mut current_text = String::new();

    // Parse package attributes for version and direction
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                let name = String::from_utf8_lossy(local.as_ref()).to_string();

                match name.as_str() {
                    "package" => {
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.local_name().as_ref())
                                .to_string();
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key.as_str() {
                                "version" => {
                                    epub_version = match val.as_str() {
                                        v if v.starts_with('2') => Some(EpubVersion::V2),
                                        v if v.starts_with('3') => Some(EpubVersion::V3),
                                        _ => None,
                                    };
                                }
                                "dir" => {
                                    text_direction = match val.to_lowercase().as_str() {
                                        "rtl" => TextDirection::Rtl,
                                        "ltr" => TextDirection::Ltr,
                                        _ => TextDirection::Auto,
                                    };
                                }
                                _ => {}
                            }
                        }
                    }
                    "metadata" => {
                        in_metadata = true;
                    }
                    "title" | "creator" | "language" | "publisher" | "date" | "description"
                    | "subject" | "identifier" | "rights"
                        if in_metadata =>
                    {
                        current_element = Some(name.clone());
                        current_text.clear();
                    }
                    "meta" if in_metadata => {
                        // EPUB3 meta elements with property attribute
                        let mut property = None;
                        let mut meta_content = None;
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.local_name().as_ref())
                                .to_string();
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key.as_str() {
                                "property" => property = Some(val),
                                "content" => meta_content = Some(val),
                                "name" if val == "cover" => {
                                    // EPUB2 cover meta
                                    property = Some("cover".to_string());
                                }
                                _ => {}
                            }
                        }
                        // EPUB2 style: <meta name="cover" content="cover-image-id"/>
                        if property.as_deref() == Some("cover") {
                            if let Some(cover_id) = meta_content {
                                metadata.cover_image_id = Some(cover_id);
                            }
                        }
                        if matches!(e, quick_xml::events::BytesStart { .. }) {
                            // Element has content between tags
                            current_element = property.map(|p| format!("meta:{p}"));
                            current_text.clear();
                        }
                    }
                    "item" => {
                        let mut id = String::new();
                        let mut href = String::new();
                        let mut media_type = String::new();
                        let mut properties = None;

                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.local_name().as_ref())
                                .to_string();
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key.as_str() {
                                "id" => id = val,
                                "href" => href = val,
                                "media-type" => media_type = val,
                                "properties" => properties = Some(val),
                                _ => {}
                            }
                        }

                        // Check for EPUB3 NAV document
                        if properties.as_deref().map_or(false, |p| p.contains("nav")) {
                            nav_href = Some(href.clone());
                        }

                        manifest.insert(
                            id,
                            ManifestItem {
                                href,
                                media_type,
                                properties,
                            },
                        );
                    }
                    "spine" => {
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.local_name().as_ref())
                                .to_string();
                            if key == "toc" {
                                toc_id = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                        }
                    }
                    "itemref" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"idref" {
                                spine_items.push(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if current_element.is_some() {
                    current_text
                        .push_str(&e.unescape().unwrap_or_default());
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();

                if name == "metadata" {
                    in_metadata = false;
                }

                if let Some(ref elem) = current_element {
                    let text = current_text.trim().to_string();
                    if !text.is_empty() {
                        match elem.as_str() {
                            "title" => metadata.title = Some(text),
                            "creator" => metadata.authors.push(text),
                            "language" => metadata.language = Some(text),
                            "publisher" => metadata.publisher = Some(text),
                            "date" => metadata.publish_date = Some(text),
                            "description" => metadata.description = Some(text),
                            "subject" => metadata.subjects.push(text),
                            "rights" => metadata.rights = Some(text),
                            "identifier" => {
                                // Try to detect ISBN
                                let cleaned: String = text.chars()
                                    .filter(|c| c.is_ascii_digit() || *c == '-' || *c == 'X' || *c == 'x')
                                    .collect();
                                let digits: String = cleaned.chars()
                                    .filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x')
                                    .collect();
                                if digits.len() == 13 {
                                    metadata.isbn_13 = Some(digits);
                                } else if digits.len() == 10 {
                                    metadata.isbn_10 = Some(digits);
                                }
                            }
                            s if s.starts_with("meta:") => {
                                let prop = &s[5..];
                                match prop {
                                    "belongs-to-collection" => {
                                        if metadata.series.is_none() {
                                            metadata.series = Some(SeriesInfo {
                                                name: text,
                                                position: None,
                                            });
                                        }
                                    }
                                    "group-position" => {
                                        if let Ok(pos) = text.parse::<f32>() {
                                            if let Some(ref mut series) = metadata.series {
                                                series.position = Some(pos);
                                            }
                                        }
                                    }
                                    _ => {
                                        metadata.custom.insert(prop.to_string(), text);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    current_element = None;
                    current_text.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ReadError::MalformedFile {
                    format: "EPUB".into(),
                    detail: format!("Failed to parse OPF: {e}"),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(OpfData {
        metadata,
        manifest,
        spine_items,
        epub_version,
        text_direction,
        toc_id,
        nav_href,
    })
}

// --- XHTML Content Parsing ---

fn parse_xhtml_to_chapter(
    content: &str,
    id: &str,
    limits: &crate::security::SecurityLimits,
) -> Chapter {
    let mut nodes = Vec::new();
    let mut reader = XmlReader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut in_body = false;
    let mut depth: u32 = 0;
    let mut inline_stack: Vec<Vec<InlineNode>> = Vec::new();
    let mut chapter_title: Option<String> = None;
    let mut current_heading_level: Option<u8> = None;
    let mut link_href_stack: Vec<String> = Vec::new();

    // Simple state machine for parsing XHTML into content nodes
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                if depth > limits.max_nesting_depth {
                    tracing::warn!(
                        "Nesting depth {} exceeds limit {}, truncating",
                        depth,
                        limits.max_nesting_depth
                    );
                    break;
                }

                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();

                match name.as_str() {
                    "body" => {
                        in_body = true;
                    }
                    "p" if in_body => {
                        inline_stack.push(Vec::new());
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" if in_body => {
                        let level = name.as_bytes()[1] - b'0';
                        current_heading_level = Some(level);
                        inline_stack.push(Vec::new());
                    }
                    "em" | "i" if in_body && !inline_stack.is_empty() => {
                        inline_stack.push(Vec::new());
                    }
                    "strong" | "b" if in_body && !inline_stack.is_empty() => {
                        inline_stack.push(Vec::new());
                    }
                    "a" if in_body && !inline_stack.is_empty() => {
                        let mut href = String::new();
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.local_name().as_ref())
                                .to_string();
                            if key == "href" {
                                href = String::from_utf8_lossy(&attr.value).to_string();
                                break;
                            }
                        }
                        link_href_stack.push(href);
                        inline_stack.push(Vec::new());
                    }
                    "code" if in_body && !inline_stack.is_empty() => {
                        inline_stack.push(Vec::new());
                    }
                    "blockquote" if in_body => {
                        // We'll collect block-level content inside
                    }
                    "ul" | "ol" if in_body => {
                        // List handling
                    }
                    "pre" if in_body => {
                        inline_stack.push(Vec::new());
                    }
                    "img" if in_body => {
                        let mut src = String::new();
                        let mut alt = None;
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.local_name().as_ref())
                                .to_string();
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key.as_str() {
                                "src" => src = val,
                                "alt" => alt = Some(val),
                                _ => {}
                            }
                        }
                        if !src.is_empty() {
                            nodes.push(ContentNode::Image {
                                resource_id: src,
                                alt_text: alt,
                                caption: None,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if name == "br" && in_body {
                    if let Some(inlines) = inline_stack.last_mut() {
                        inlines.push(InlineNode::LineBreak);
                    }
                } else if name == "hr" && in_body {
                    nodes.push(ContentNode::HorizontalRule);
                } else if name == "img" && in_body {
                    let mut src = String::new();
                    let mut alt = None;
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.local_name().as_ref())
                            .to_string();
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        match key.as_str() {
                            "src" => src = val,
                            "alt" => alt = Some(val),
                            _ => {}
                        }
                    }
                    if !src.is_empty() {
                        nodes.push(ContentNode::Image {
                            resource_id: src,
                            alt_text: alt,
                            caption: None,
                        });
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_body {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if !text.is_empty() {
                        if let Some(inlines) = inline_stack.last_mut() {
                            inlines.push(InlineNode::Text(text));
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                if depth > 0 {
                    depth -= 1;
                }
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();

                match name.as_str() {
                    "body" => {
                        in_body = false;
                    }
                    "p" if in_body => {
                        if let Some(children) = inline_stack.pop() {
                            if !children.is_empty() {
                                nodes.push(ContentNode::Paragraph { children });
                            }
                        }
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" if in_body => {
                        if let Some(children) = inline_stack.pop() {
                            let level = current_heading_level.unwrap_or(1);
                            // Capture first heading as chapter title
                            if chapter_title.is_none() {
                                chapter_title = extract_text_from_inlines(&children);
                            }
                            nodes.push(ContentNode::Heading { level, children });
                            current_heading_level = None;
                        }
                    }
                    "em" | "i" if in_body => {
                        if let Some(children) = inline_stack.pop() {
                            if let Some(parent) = inline_stack.last_mut() {
                                parent.push(InlineNode::Emphasis(children));
                            }
                        }
                    }
                    "strong" | "b" if in_body => {
                        if let Some(children) = inline_stack.pop() {
                            if let Some(parent) = inline_stack.last_mut() {
                                parent.push(InlineNode::Strong(children));
                            }
                        }
                    }
                    "a" if in_body => {
                        if let Some(children) = inline_stack.pop() {
                            if let Some(parent) = inline_stack.last_mut() {
                                let href = link_href_stack.pop().unwrap_or_default();
                                parent.push(InlineNode::Link {
                                    href,
                                    children,
                                });
                            }
                        }
                    }
                    "code" if in_body => {
                        if let Some(children) = inline_stack.pop() {
                            if let Some(parent) = inline_stack.last_mut() {
                                let text = extract_text_from_inlines(&children)
                                    .unwrap_or_default();
                                parent.push(InlineNode::Code(text));
                            }
                        }
                    }
                    "pre" if in_body => {
                        if let Some(children) = inline_stack.pop() {
                            let code = extract_text_from_inlines(&children)
                                .unwrap_or_default();
                            nodes.push(ContentNode::CodeBlock {
                                language: None,
                                code,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Chapter {
        id: id.to_string(),
        title: chapter_title,
        content: nodes,
        text_direction: None,
    }
}

fn extract_text_from_inlines(inlines: &[InlineNode]) -> Option<String> {
    let mut text = String::new();
    for node in inlines {
        match node {
            InlineNode::Text(t) => text.push_str(t),
            InlineNode::Emphasis(children) | InlineNode::Strong(children) => {
                if let Some(t) = extract_text_from_inlines(children) {
                    text.push_str(&t);
                }
            }
            InlineNode::Code(c) => text.push_str(c),
            InlineNode::Link { children, .. } => {
                if let Some(t) = extract_text_from_inlines(children) {
                    text.push_str(&t);
                }
            }
            InlineNode::LineBreak => text.push(' '),
            _ => {}
        }
    }
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

// --- TOC Parsing ---

fn parse_toc<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    opf: &OpfData,
    opf_dir: &str,
    limits: &crate::security::SecurityLimits,
) -> Vec<TocEntry> {
    // Try EPUB3 NAV document first
    if let Some(ref nav_href) = opf.nav_href {
        let full_path = format!("{}{}", opf_dir, nav_href);
        if let Ok(content) = read_archive_entry(archive, &full_path, limits) {
            if let Some(entries) = parse_nav_document(&content) {
                return entries;
            }
        }
    }

    // Fall back to EPUB2 NCX
    if let Some(ref toc_id) = opf.toc_id {
        if let Some(item) = opf.manifest.get(toc_id) {
            let full_path = format!("{}{}", opf_dir, item.href);
            if let Ok(content) = read_archive_entry(archive, &full_path, limits) {
                return parse_ncx(&content);
            }
        }
    }

    Vec::new()
}

/// Parse EPUB3 NAV document (HTML with <nav epub:type="toc">).
fn parse_nav_document(content: &str) -> Option<Vec<TocEntry>> {
    // Simple scraper-based approach for HTML NAV
    let document = scraper::Html::parse_document(content);
    let nav_selector = scraper::Selector::parse("nav[epub\\:type='toc'], nav[role='doc-toc'], nav").ok()?;

    let nav = document.select(&nav_selector).next()?;

    let ol_selector = scraper::Selector::parse("ol").ok()?;
    let ol = nav.select(&ol_selector).next()?;

    Some(parse_nav_ol(&ol))
}

fn parse_nav_ol(ol: &scraper::ElementRef) -> Vec<TocEntry> {
    let li_selector = scraper::Selector::parse(":scope > li").unwrap_or_else(|_| {
        scraper::Selector::parse("li").unwrap()
    });
    let a_selector = scraper::Selector::parse("a").unwrap();
    let ol_selector = scraper::Selector::parse("ol").unwrap();

    let mut entries = Vec::new();
    for li in ol.select(&li_selector) {
        if let Some(a) = li.select(&a_selector).next() {
            let title = a.text().collect::<String>().trim().to_string();
            let href = a.value().attr("href").unwrap_or("").to_string();

            let children = li
                .select(&ol_selector)
                .next()
                .map(|nested_ol| parse_nav_ol(&nested_ol))
                .unwrap_or_default();

            if !title.is_empty() {
                entries.push(TocEntry {
                    title,
                    href,
                    children,
                });
            }
        }
    }
    entries
}

/// Parse EPUB2 NCX (XML).
fn parse_ncx(content: &str) -> Vec<TocEntry> {
    let mut reader = XmlReader::from_str(content);
    let mut buf = Vec::new();
    let mut entries = Vec::new();
    let mut stack: Vec<Vec<TocEntry>> = vec![Vec::new()];
    let mut current_title = String::new();
    let mut current_href = String::new();
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match name.as_str() {
                    "navPoint" => {
                        stack.push(Vec::new());
                        current_title.clear();
                        current_href.clear();
                    }
                    "text" => {
                        in_text = true;
                    }
                    "content" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"src" {
                                current_href =
                                    String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if name == "content" {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"src" {
                            current_href = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text {
                    current_title
                        .push_str(&e.unescape().unwrap_or_default());
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match name.as_str() {
                    "text" => {
                        in_text = false;
                    }
                    "navPoint" => {
                        let children = stack.pop().unwrap_or_default();
                        let entry = TocEntry {
                            title: current_title.trim().to_string(),
                            href: current_href.clone(),
                            children,
                        };
                        if let Some(parent) = stack.last_mut() {
                            parent.push(entry);
                        }
                        current_title.clear();
                        current_href.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if let Some(top) = stack.pop() {
        entries = top;
    }

    entries
}

// --- Archive Helpers ---

fn read_archive_entry<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    path: &str,
    limits: &crate::security::SecurityLimits,
) -> Result<String, ReadError> {
    let bytes = read_archive_entry_bytes(archive, path, limits)?;
    String::from_utf8(bytes).map_err(|e| ReadError::MalformedFile {
        format: "EPUB".into(),
        detail: format!("Invalid UTF-8 in {}: {}", path, e),
    })
}

fn read_archive_entry_string<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    path: &str,
) -> Result<String, ReadError> {
    let mut file = archive.by_name(path).map_err(|_| {
        ReadError::MissingContent(format!("Missing required file: {path}"))
    })?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| ReadError::MalformedFile {
            format: "EPUB".into(),
            detail: format!("Failed to read {}: {}", path, e),
        })?;
    Ok(content)
}

fn read_archive_entry_bytes<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    path: &str,
    limits: &crate::security::SecurityLimits,
) -> Result<Vec<u8>, ReadError> {
    let mut file = archive.by_name(path).map_err(|_| {
        ReadError::MissingContent(format!("Missing file: {path}"))
    })?;

    // Security: check path traversal
    security::check_path_traversal(path)?;

    // Security: check individual resource size
    security::check_resource_size(path, file.size(), limits)?;

    let mut buf = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buf)
        .map_err(|e| ReadError::MalformedFile {
            format: "EPUB".into(),
            detail: format!("Failed to read {}: {}", path, e),
        })?;
    Ok(buf)
}

fn is_resource_media_type(media_type: &str) -> bool {
    matches!(
        media_type,
        "image/jpeg"
            | "image/png"
            | "image/gif"
            | "image/svg+xml"
            | "font/otf"
            | "font/ttf"
            | "font/woff"
            | "font/woff2"
            | "application/font-sfnt"
            | "application/x-font-ttf"
            | "application/x-font-opentype"
            | "text/css"
    )
}
