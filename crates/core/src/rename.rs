//! Title formatter / filename templating engine.
//! Parses filenames into semantic parts and reassembles using a format string.
//! Format: `{placeholder|modifier}` — e.g., `{author_last} - {title|kebab}.{ext}`

use crate::document::Metadata;
use crate::error::FormatError;

pub fn format_title(
    filename: &str,
    template: &str,
    metadata: Option<&Metadata>,
) -> Result<String, FormatError> {
    let _ = (filename, template, metadata);
    todo!()
}
