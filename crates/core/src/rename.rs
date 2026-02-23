//! Title formatter / filename templating engine.
//! Format: `{placeholder|modifier}` — e.g., `{author} - {title|kebab}.{ext}`

use crate::document::Metadata;
use crate::error::FormatError;

pub fn format_title(
    filename: &str,
    template: &str,
    metadata: Option<&Metadata>,
) -> Result<String, FormatError> {
    let stem = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let title = metadata
        .and_then(|m| m.title.as_deref())
        .unwrap_or(&stem)
        .to_string();
    let author = metadata
        .and_then(|m| m.authors.first().map(|s| s.as_str()))
        .unwrap_or("Unknown")
        .to_string();

    let mut out = template.to_string();
    out = out.replace("{title}", &title);
    out = out.replace("{author}", &author);
    out = out.replace("{ext}", &ext);
    out = out.replace("{stem}", &stem);

    if out.contains("{title|kebab}") {
        let kebab = title.replace(' ', "-").replace(|c: char| !c.is_alphanumeric() && c != '-', "");
        out = out.replace("{title|kebab}", &kebab);
    }
    if out.contains("{author|kebab}") {
        let kebab = author.replace(' ', "-").replace(|c: char| !c.is_alphanumeric() && c != '-', "");
        out = out.replace("{author|kebab}", &kebab);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Metadata;

    #[test]
    fn format_title_basic() {
        let out = format_title("mybook.epub", "{author} - {title}.{ext}", None).unwrap();
        assert_eq!(out, "Unknown - mybook.epub");
    }

    #[test]
    fn format_title_with_metadata() {
        let meta = Metadata {
            title: Some("The Book".to_string()),
            authors: vec!["Alice".to_string()],
            ..Default::default()
        };
        let out = format_title("x.epub", "{author} - {title}.{ext}", Some(&meta)).unwrap();
        assert_eq!(out, "Alice - The Book.epub");
    }

    #[test]
    fn format_title_kebab() {
        let meta = Metadata {
            title: Some("Hello World".to_string()),
            authors: vec!["Jane Doe".to_string()],
            ..Default::default()
        };
        let out = format_title("x.txt", "{author|kebab}-{title|kebab}.{ext}", Some(&meta)).unwrap();
        assert!(out.contains("Jane-Doe"));
        assert!(out.contains("Hello-World"));
        assert!(out.ends_with(".txt"));
    }
}
