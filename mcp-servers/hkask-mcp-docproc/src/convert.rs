//! Format detection, text extraction, and HTML/markdown preprocessing for docproc server.

/// Detect document format from file path/extension.
///
/// Returns `(format_name, supported, note)` where `supported` indicates whether
/// `docproc_convert` can extract text from this format.
///
/// Supported formats (text extraction works): pdf, markdown, html, plain
/// Recognized but unsupported (needs additional Rust crates): docx, pptx, xlsx, csv, rtf
pub fn detect_format(path: &str) -> (&'static str, bool, Option<&'static str>) {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "pdf" => ("pdf", true, None),
        "md" | "markdown" => ("markdown", true, None),
        "html" | "htm" => ("html", true, None),
        "txt" => ("plain", true, None),
        "docx" | "doc" => (
            "docx",
            false,
            Some("DOCX support requires additional Rust crate"),
        ),
        "pptx" | "ppt" => (
            "pptx",
            false,
            Some("PPTX support requires additional Rust crate"),
        ),
        "xlsx" | "xls" => (
            "xlsx",
            false,
            Some("XLSX support requires additional Rust crate"),
        ),
        "csv" => (
            "csv",
            false,
            Some("CSV support requires additional Rust crate"),
        ),
        "rtf" => (
            "rtf",
            false,
            Some("RTF support requires additional Rust crate"),
        ),
        _ => ("unknown", false, None),
    }
}

/// Whether a format is supported for text extraction by `docproc_convert`.
pub fn is_format_supported(format: &str) -> bool {
    matches!(format, "pdf" | "markdown" | "html" | "plain")
}

/// Strip YAML frontmatter (delimited by `---`) from content.
pub fn strip_frontmatter(content: &str) -> String {
    if content.starts_with("---") {
        content
            .splitn(3, "---")
            .nth(2)
            .unwrap_or(content)
            .trim()
            .to_string()
    } else {
        content.to_string()
    }
}

/// Strip HTML tags and extract visible text content.
///
/// Removes script/style elements entirely, then strips all remaining
/// HTML tags to produce clean plain text. Collapses consecutive whitespace.
pub fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_strip_tag = false;
    let chars: Vec<char> = html.chars().collect();
    let len = chars.len();

    /// Block-level tags that should insert a space when stripped.
    const BLOCK_TAGS: &[&str] = &[
        "p",
        "div",
        "br",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "li",
        "tr",
        "table",
        "blockquote",
        "pre",
        "section",
        "article",
        "header",
        "footer",
        "main",
        "aside",
        "nav",
        "figure",
    ];

    let mut i = 0;
    while i < len {
        let ch = chars[i];

        if ch == '<' {
            let remaining: String = chars[i..].iter().collect();
            let lower_remaining = remaining.to_lowercase();

            // Check for closing script/style tags
            if lower_remaining.starts_with("</script") || lower_remaining.starts_with("</style") {
                // Insert space boundary when exiting a strip tag
                if in_strip_tag
                    && !result.is_empty()
                    && !result.chars().last().is_none_or(|c| c.is_whitespace())
                {
                    result.push(' ');
                }
                in_strip_tag = false;
                while i < len && chars[i] != '>' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                continue;
            }

            // Check for opening script/style tags
            if lower_remaining.starts_with("<script") || lower_remaining.starts_with("<style") {
                // Insert space boundary when entering a strip tag
                if !result.is_empty() && !result.chars().last().is_none_or(|c| c.is_whitespace()) {
                    result.push(' ');
                }
                in_strip_tag = true;
                while i < len && chars[i] != '>' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                continue;
            }

            // For regular tags, check if it's a block-level tag
            // and insert a space if needed (for word boundaries)
            let tag_name = remaining
                .trim_start_matches('<')
                .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .next()
                .unwrap_or("")
                .to_lowercase();
            let is_block = BLOCK_TAGS.contains(&tag_name.as_str());

            if is_block
                && !result.is_empty()
                && !result.chars().last().is_none_or(|c| c.is_whitespace())
            {
                result.push(' ');
            }

            in_tag = true;
            i += 1;
            continue;
        }

        if ch == '>' {
            in_tag = false;
            i += 1;
            continue;
        }

        if !in_tag && !in_strip_tag {
            result.push(ch);
        }

        i += 1;
    }

    // Collapse whitespace
    let collapsed: String = result.split_whitespace().collect::<Vec<&str>>().join(" ");

    collapsed
}
