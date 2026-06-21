//! Markdown export — formats assembled content with chapter headings and TOC.
//!
//! Pure function of (text, title, author, toc) → markdown string.
//! Public surface: 1 function (`export_markdown`).

use crate::kindle_zip::types::{TocItem, split_into_chapters};

/// Generate Markdown from assembled book content.
///
/// Produces: `# Title` → byline → TOC with anchors → chapter sections.
pub fn export_markdown(text: &str, title: &str, author: &str, toc: &[TocItem]) -> String {
    let mut md = String::new();

    // Title page
    md.push_str(&format!("# {}\n", title));
    md.push_str(&format!("> By {}\n\n---\n\n", author));

    // Table of Contents
    md.push_str("## Table of Contents\n\n");
    for item in toc {
        if item.depth > 1 {
            continue;
        }
        let anchor = item
            .label
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
            .replace(' ', "-");
        md.push_str(&format!(
            "{}- [{}](#{})\n",
            "  ".repeat(item.depth.saturating_sub(0)),
            item.label,
            anchor
        ));
    }
    md.push_str("\n---\n\n");

    // Content by chapter
    let chapters = split_into_chapters(text, toc);
    for (ch_title, ch_text) in &chapters {
        md.push_str(&format!("## {}\n\n", ch_title));
        md.push_str(ch_text);
        md.push_str("\n\n");
    }

    md
}
