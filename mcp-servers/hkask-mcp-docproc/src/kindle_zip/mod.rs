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

    for format in formats {
        let format_lower = format.to_lowercase();
        match format_lower.as_str() {
            "pdf" => {
                let path = book_dir.join("book.pdf");
                let bytes = export_pdf::export_pdf(assembled_text, title)?;
                std::fs::write(&path, &bytes).map_err(|e| format!("Write PDF: {}", e))?;
                // Human-readable copy at output root
                let hr_path = output_dir.join(format!("{}.pdf", label));
                std::fs::write(&hr_path, &bytes).ok();
                let size = bytes.len() as u64;
                exports.push(ExportEntry {
                    format: "pdf".into(),
                    path: hr_path,
                    size_bytes: size,
                });
                total_bytes += size;
            }
            "epub" => {
                let path = book_dir.join("book.epub");
                let bytes = export_epub::export_epub(assembled_text, title, author, toc)?;
                std::fs::write(&path, &bytes).map_err(|e| format!("Write EPUB: {}", e))?;
                let hr_path = output_dir.join(format!("{}.epub", label));
                std::fs::write(&hr_path, &bytes).ok();
                let size = bytes.len() as u64;
                exports.push(ExportEntry {
                    format: "epub".into(),
                    path: hr_path,
                    size_bytes: size,
                });
                total_bytes += size;
            }
            "markdown" | "md" => {
                let path = book_dir.join("book.md");
                let content = export_markdown::export_markdown(assembled_text, title, author, toc);
                std::fs::write(&path, &content).map_err(|e| format!("Write MD: {}", e))?;
                let hr_path = output_dir.join(format!("{}.md", label));
                std::fs::write(&hr_path, &content).ok();
                let size = content.len() as u64;
                exports.push(ExportEntry {
                    format: "markdown".into(),
                    path: hr_path,
                    size_bytes: size,
                });
                total_bytes += size;
            }
            "txt" | "text" => {
                let path = book_dir.join("book.txt");
                std::fs::write(&path, assembled_text).map_err(|e| format!("Write TXT: {}", e))?;
                let hr_path = output_dir.join(format!("{}.txt", label));
                std::fs::write(&hr_path, assembled_text).ok();
                let size = assembled_text.len() as u64;
                exports.push(ExportEntry {
                    format: "txt".into(),
                    path: hr_path,
                    size_bytes: size,
                });
                total_bytes += size;
            }
            other => {
                tracing::warn!(target: "cns.pipeline.kindle-zip.export",
                    format = %other, "Unknown export format — skipping");
            }
        }
    }

    // Write index.json for agent discoverability
    let index_path = output_dir.join("index.json");
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
    // Append to index.json (create or update the array)
    let mut index: Vec<serde_json::Value> = if index_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&index_path).unwrap_or_default())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    // Replace existing entry for same ASIN, otherwise append
    index.retain(|e| e.get("asin").and_then(|v| v.as_str()) != Some(asin));
    index.push(index_entry);
    if let Ok(json) = serde_json::to_string_pretty(&index) {
        std::fs::write(&index_path, json).ok();
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
pub fn sanitize_filename(s: &str) -> String {
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
