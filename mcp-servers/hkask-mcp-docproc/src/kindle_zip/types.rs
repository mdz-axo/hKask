//! Kindle-Zip shared types — Book metadata, page entries, TOC, and result structs.
//!
//! All types are serializable for MDS provenance and content.json interchange.
//! Public surface: ≤4 types (BookMetadata, ContentChunk, ExtractResult, ExportResult).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Book metadata extracted from Kindle Cloud Reader or loaded from disk.
///
/// Mirrors the reference project's `BookMetadata` type with hKask conventions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMetadata {
    pub asin: String,
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    pub pages: Vec<PageEntry>,
    pub toc: Vec<TocItem>,
    pub nav: PageNav,
    #[serde(default)]
    pub raw_meta: serde_json::Value,
}

/// A single page entry — index, page number, and screenshot path on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageEntry {
    pub index: usize,
    pub page: usize,
    pub screenshot: PathBuf,
}

/// Table of contents item with depth tracking for chapter hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocItem {
    pub label: String,
    pub depth: usize,
    #[serde(default)]
    pub page: Option<usize>,
    #[serde(default)]
    pub position_id: Option<u64>,
}

/// Page navigation state — tracks content boundaries within the full page range.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageNav {
    #[serde(default)]
    pub start_content_page: i64,
    #[serde(default)]
    pub end_content_page: i64,
    #[serde(default)]
    pub total_pages: i64,
    #[serde(default)]
    pub total_content_pages: i64,
}

/// A single transcribed page chunk with provenance metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentChunk {
    pub index: usize,
    pub page: usize,
    pub text: String,
    #[serde(default)]
    pub screenshot: Option<PathBuf>,
    #[serde(default)]
    pub confidence: Option<f32>,
    /// MDS provenance: which step/engine produced this chunk.
    #[serde(default)]
    pub provenance: Option<ProvenanceRecord>,
}

/// MDS provenance triple — records what generated an artifact and with what parameters.
///
/// Enables audit trail: every artifact knows its origin step, engine, model,
/// and parameter hash for reproducibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub step_id: String,
    pub engine: String,
    pub model: Option<String>,
    #[serde(default)]
    pub parameter_hash: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

/// Result of the extraction step — page screenshots + metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub asin: String,
    pub metadata_path: PathBuf,
    pub pages_dir: PathBuf,
    pub total_pages: usize,
    pub content_pages: usize,
    pub title: String,
    pub author: String,
    pub toc: Vec<TocItem>,
    /// CNS span ID for observability linking (reserved for future CNS wiring).
    #[serde(default, skip_serializing)]
    pub cns_span_id: Option<String>,
}

/// Result of the transcription step — OCR output + quality metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeResult {
    pub content_path: PathBuf,
    pub total_words: usize,
    pub transcribed_pages: usize,
    pub failed_pages: usize,
    pub mean_confidence: f32,
    #[serde(default)]
    pub cns_span_id: Option<String>,
}

/// A single exported file entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEntry {
    pub format: String,
    pub path: PathBuf,
    pub size_bytes: u64,
}

/// Result of the export step — all generated format files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub exports: Vec<ExportEntry>,
    pub total_bytes: u64,
}

/// Consolidated MCP parameter type for kindle-zip tools.
///
/// Used by all three tools (extract, transcribe, export) with
/// optional fields defaulted per-tool.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct KindleZipParams {
    // ── Extract params ──
    pub asin: String,
    #[serde(default)]
    pub amazon_email: Option<String>,
    #[serde(default)]
    pub amazon_password: Option<String>,
    /// Path to Chrome/Chromium user data directory for cookie-based auth.
    /// When set, reuses existing browser session (no login required).
    /// Typical: ~/.config/google-chrome/Default or ~/.config/chromium/Default
    #[serde(default)]
    pub chrome_profile: Option<String>,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,

    // ── Transcribe params ──
    #[serde(default)]
    pub pages_dir: Option<String>,
    #[serde(default)]
    pub metadata_path: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    // ── Export params ──
    #[serde(default)]
    pub assembled_text: Option<String>,
    #[serde(default = "default_export_formats")]
    pub formats: Vec<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
}

fn default_output_dir() -> String {
    "output".into()
}
fn default_max_tokens() -> u32 {
    8192
}
fn default_concurrency() -> usize {
    4
}
fn default_export_formats() -> Vec<String> {
    vec!["pdf".into(), "epub".into(), "markdown".into()]
}

// ── Utility functions ───────────────────────────────────────────────────────

/// Zero-pad a number to a given width (e.g., zeropad(5, 3) → "005").
pub(crate) fn zeropad(n: usize, width: usize) -> String {
    format!("{:0width$}", n, width = width)
}

/// Check if a directory contains any PNG files (for resume detection).
pub(crate) fn has_page_files(pages_dir: &std::path::Path) -> bool {
    pages_dir
        .read_dir()
        .map(|mut dir| {
            dir.any(|e| {
                e.ok()
                    .and_then(|e| e.path().extension().map(|ext| ext == "png"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Escape a string for XML embedding (five entities).
pub(crate) fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Escape a string for PDF literal strings (parens and backslashes).
pub(crate) fn escape_pdf_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace('\n', " ")
}

/// Simple word-boundary text wrapping at a character width.
pub(crate) fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::with_capacity(width);
    for word in text.split_whitespace() {
        if current.len() + word.len() + 1 > width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Split text into chapters based on TOC heading positions.
pub(crate) fn split_into_chapters(text: &str, toc: &[TocItem]) -> Vec<(String, String)> {
    if toc.is_empty() {
        return vec![("Content".to_string(), text.to_string())];
    }
    let mut chapters: Vec<(String, String)> = Vec::new();
    let mut prev_pos: Option<usize> = None;
    for (i, item) in toc.iter().enumerate() {
        if item.depth > 1 {
            continue;
        }
        let label = &item.label;
        if let Some(pos) = text.find(label.as_str()) {
            if let Some(prev) = prev_pos {
                let chapter_text = text[prev..pos].trim().to_string();
                let prev_label = &toc[i.saturating_sub(1)].label;
                chapters.push((prev_label.clone(), chapter_text));
            }
            prev_pos = Some(pos + label.len());
        }
    }
    if let Some(start) = prev_pos
        && !toc.is_empty()
    {
        let last_label = &toc[toc.len() - 1].label;
        let chapter_text = text[start..].trim().to_string();
        chapters.push((last_label.clone(), chapter_text));
    }
    if chapters.is_empty() {
        chapters.push(("Content".to_string(), text.to_string()));
    }
    chapters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeropad_values() {
        assert_eq!(zeropad(5, 3), "005");
        assert_eq!(zeropad(123, 2), "123");
    }

    #[test]
    fn wrap_text_basic() {
        assert_eq!(wrap_text("hello world", 80), vec!["hello world"]);
        let long = wrap_text("this is a very long line that should wrap", 20);
        assert!(long.len() > 1);
    }

    #[test]
    fn escape_xml_entities() {
        assert_eq!(
            escape_xml("<title>Hello & World</title>"),
            "&lt;title&gt;Hello &amp; World&lt;/title&gt;"
        );
    }

    #[test]
    fn escape_pdf_parens() {
        assert_eq!(escape_pdf_string("hello (world)"), "hello \\(world\\)");
    }

    #[test]
    fn split_chapters_empty_toc() {
        let ch = split_into_chapters("Some text", &[]);
        assert_eq!(ch.len(), 1);
        assert_eq!(ch[0].0, "Content");
    }

    #[test]
    fn default_output_dir_value() {
        assert_eq!(default_output_dir(), "output");
    }

    #[test]
    fn default_export_formats_count() {
        let f = default_export_formats();
        assert_eq!(f.len(), 3);
        assert!(f.contains(&"pdf".to_string()));
    }
}
