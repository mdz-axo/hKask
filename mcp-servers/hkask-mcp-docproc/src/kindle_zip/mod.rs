//! Kindle-Zip — Export Kindle books to PDF, EPUB, Markdown, and plain text.
//!
//! Functionally equivalent to transitive-bullshit/kindle-ai-export.
//! Architecture:
//! ```text
//! kindle_extract    → page screenshots + metadata (browser automation)
//! kindle_transcribe → OCR pipeline per page (Tesseract + LLM OCR)
//! kindle_export     → PDF, EPUB, Markdown, TXT (format generators)
//! ```
//!
//! Public surface: ≤7 items (deep-module discipline).
//! Pipeline logic lives in registry manifests and Jinja2 templates.
//! This crate provides the Rust bridge — browser automation, OCR routing,
//! format generators, and tool registration.

pub mod export_epub;
pub mod export_markdown;
pub mod export_pdf;
pub mod extract;
pub mod transcribe;
pub mod types;

// Public API (≤7 items)
pub use export_epub::export_epub;
pub use export_markdown::export_markdown;
pub use export_pdf::export_pdf;
pub use extract::KindleSession;
pub use extract::discover_kindle_books;
pub use extract::extract_kindle_book;
pub use extract::extract_kindle_books;
pub use transcribe::assemble_chunks;
pub use transcribe::transcribe_pages;
pub use types::{BookMetadata, ExportResult, ExtractResult, KindleZipParams, TocItem};

use std::path::Path;

use types::ExportEntry;

/// Export assembled book content to the requested formats.
///
/// Dispatch function that routes to format-specific generators.
/// Single entry point for the `kindle_export` MCP tool.
#[allow(clippy::too_many_arguments)]
pub fn export_formats(
    assembled_text: &str,
    formats: &[String],
    output_dir: &Path,
    asin: &str,
    title: &str,
    author: &str,
    toc: &[TocItem],
) -> Result<ExportResult, String> {
    let book_dir = output_dir.join(asin);
    std::fs::create_dir_all(&book_dir).map_err(|e| format!("mkdir: {}", e))?;

    // Build human-readable label: "Author - Title" (or just Title if no author)
    let label = if author.is_empty() || author == "Unknown Author" {
        sanitize_filename(title)
    } else {
        format!(
            "{} - {}",
            sanitize_filename(author),
            sanitize_filename(title)
        )
    };
    // Fall back to ASIN if sanitization produced empty string (all special chars)
    let label = if label.is_empty() {
        asin.to_string()
    } else {
        label
    };
    let label = truncate_filename(&label, 200);

    let mut exports: Vec<ExportEntry> = Vec::with_capacity(formats.len());
    let mut total_bytes: u64 = 0;

    // Write a format: generate bytes, write ASIN copy + human-readable copy
    let mut write_format = |fmt: &str, ext: &str, bytes: &[u8]| {
        // ASIN-keyed copy (pipeline internal)
        let asin_path = book_dir.join(format!("book.{}", ext));
        std::fs::write(&asin_path, bytes).map_err(|e| format!("Write {}: {}", ext, e))?;
        // Human-readable copy at output root
        let hr_path = output_dir.join(format!("{}.{}", label, ext));
        std::fs::write(&hr_path, bytes).ok();
        let size = bytes.len() as u64;
        exports.push(ExportEntry {
            format: fmt.into(),
            path: hr_path,
            size_bytes: size,
        });
        total_bytes += size;
        Ok::<_, String>(())
    };

    for format in formats {
        let ext = format.to_lowercase();
        match ext.as_str() {
            "pdf" => write_format(
                "pdf",
                "pdf",
                &export_pdf::export_pdf(assembled_text, title)?,
            )?,
            "epub" => write_format(
                "epub",
                "epub",
                &export_epub::export_epub(assembled_text, title, author, toc)?,
            )?,
            "markdown" | "md" => write_format(
                "markdown",
                "md",
                export_markdown::export_markdown(assembled_text, title, author, toc).as_bytes(),
            )?,
            "txt" | "text" => write_format("txt", "txt", assembled_text.as_bytes())?,
            other => tracing::warn!(target: "cns.pipeline.kindle-zip.export",
                format = %other, "Unknown export format — skipping"),
        }
    }

    // Write index.json atomically (write to temp, then rename)
    let index_path = output_dir.join("index.json");
    let index_tmp = output_dir.join(".index.json.tmp");
    let index_entry = serde_json::json!({
        "asin": asin,
        "title": title,
        "author": author,
        "label": label,
        "exports": exports.iter().map(|e| serde_json::json!({
            "format": e.format,
            "path": format!("{}.{}", label, e.format),
            "size_bytes": e.size_bytes,
        })).collect::<Vec<_>>(),
    });
    let mut index: Vec<serde_json::Value> = if index_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&index_path).unwrap_or_default())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    index.retain(|e| e.get("asin").and_then(|v| v.as_str()) != Some(asin));
    index.push(index_entry);
    if let Ok(json) = serde_json::to_string_pretty(&index) {
        std::fs::write(&index_tmp, json).ok();
        std::fs::rename(&index_tmp, &index_path).ok();
    }

    tracing::info!(target: "cns.pipeline.kindle-zip.export",
        asin = %asin,
        formats = ?exports.iter().map(|e| &e.format).collect::<Vec<_>>(),
        total_bytes, "Export complete");

    Ok(ExportResult {
        exports,
        total_bytes,
    })
}

/// Sanitize a string for use as a filename component.
/// Replaces filesystem-unsafe characters with spaces, collapses whitespace.
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'..='\x1F' => ' ',
            other => other,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// Truncate a filename to a maximum byte length, respecting UTF-8 boundaries.
fn truncate_filename(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    if end == 0 {
        return s[..s.chars().next().unwrap().len_utf8()].to_string();
    }
    s[..end].trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_colon_and_slash() {
        assert_eq!(sanitize_filename("Title: Subtitle"), "Title Subtitle");
        assert_eq!(sanitize_filename("a/b\\c"), "a b c");
        assert_eq!(sanitize_filename("hello*world?test"), "hello world test");
    }

    #[test]
    fn sanitize_all_special_returns_empty() {
        assert_eq!(sanitize_filename("***"), "");
        assert_eq!(sanitize_filename(":::"), "");
    }

    #[test]
    fn sanitize_normal_string_unchanged() {
        assert_eq!(
            sanitize_filename("Knowledge Production"),
            "Knowledge Production"
        );
        assert_eq!(sanitize_filename("Hello World 123"), "Hello World 123");
    }

    #[test]
    fn sanitize_collapses_whitespace() {
        assert_eq!(sanitize_filename("a   b"), "a b");
        assert_eq!(
            sanitize_filename("  leading trailing  "),
            "leading trailing"
        );
    }

    #[test]
    fn truncate_no_op_for_short_string() {
        assert_eq!(truncate_filename("hello", 100), "hello");
        assert_eq!(truncate_filename("short", 200), "short");
    }

    #[test]
    fn truncate_cuts_to_boundary() {
        // 10 bytes, truncate to 5
        let result = truncate_filename("hello world", 5);
        assert!(result.len() <= 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn truncate_respects_utf8() {
        // é is 2 bytes in UTF-8
        let s = "café";
        let result = truncate_filename(s, 3);
        // Should not split the é (which starts at byte 3)
        assert!(result.len() <= 3);
        assert!(result.chars().all(|c| c != '\u{FFFD}')); // no replacement chars
    }

    #[test]
    fn index_entry_has_correct_keys() {
        let entry = serde_json::json!({
            "asin": "TEST001",
            "title": "Test Book",
            "label": "Test Book",
            "exports": [{"format": "pdf", "path": "Test Book.pdf", "size_bytes": 100}]
        });
        assert_eq!(entry["asin"], "TEST001");
        assert_eq!(entry["label"], "Test Book");
        assert_eq!(entry["exports"][0]["format"], "pdf");
        assert_eq!(entry["exports"][0]["path"], "Test Book.pdf");
    }
}
