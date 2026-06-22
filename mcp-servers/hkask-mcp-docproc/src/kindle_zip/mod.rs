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
pub use extract::extract_kindle_book;
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
    _metadata_path: &Path,
    formats: &[String],
    output_dir: &Path,
    asin: &str,
    title: &str,
    author: &str,
    toc: &[TocItem],
) -> Result<ExportResult, String> {
    let book_dir = output_dir.join(asin);
    std::fs::create_dir_all(&book_dir).map_err(|e| format!("mkdir: {}", e))?;

    let mut exports: Vec<ExportEntry> = Vec::with_capacity(formats.len());
    let mut total_bytes: u64 = 0;

    for format in formats {
        let format_lower = format.to_lowercase();
        match format_lower.as_str() {
            "pdf" => {
                let path = book_dir.join("book.pdf");
                let bytes = export_pdf::export_pdf(assembled_text, title)?;
                std::fs::write(&path, &bytes).map_err(|e| format!("Write PDF: {}", e))?;
                let size = bytes.len() as u64;
                exports.push(ExportEntry {
                    format: "pdf".into(),
                    path: path.clone(),
                    size_bytes: size,
                });
                total_bytes += size;
            }
            "epub" => {
                let path = book_dir.join("book.epub");
                let bytes = export_epub::export_epub(assembled_text, title, author, toc)?;
                std::fs::write(&path, &bytes).map_err(|e| format!("Write EPUB: {}", e))?;
                let size = bytes.len() as u64;
                exports.push(ExportEntry {
                    format: "epub".into(),
                    path: path.clone(),
                    size_bytes: size,
                });
                total_bytes += size;
            }
            "markdown" | "md" => {
                let path = book_dir.join("book.md");
                let content = export_markdown::export_markdown(assembled_text, title, author, toc);
                std::fs::write(&path, &content).map_err(|e| format!("Write MD: {}", e))?;
                let size = content.len() as u64;
                exports.push(ExportEntry {
                    format: "markdown".into(),
                    path: path.clone(),
                    size_bytes: size,
                });
                total_bytes += size;
            }
            "txt" | "text" => {
                let path = book_dir.join("book.txt");
                std::fs::write(&path, assembled_text).map_err(|e| format!("Write TXT: {}", e))?;
                let size = assembled_text.len() as u64;
                exports.push(ExportEntry {
                    format: "txt".into(),
                    path: path.clone(),
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

    tracing::info!(target: "cns.pipeline.kindle-zip.export",
        asin = %asin,
        formats = ?exports.iter().map(|e| &e.format).collect::<Vec<_>>(),
        total_bytes, "Export complete");

    Ok(ExportResult {
        exports,
        total_bytes,
    })
}
