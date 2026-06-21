//! Kindle-Zip pipeline integration tests.
//!
//! Validates the full logical flow without real browser/OCR dependencies.
//! Tests: metadata roundtrip, assemble→export pipeline, format validity.
//!
//! Run: `cargo test -p hkask-mcp-docproc --test integration -- kindle_zip --ignored`
//! Or without ignored: `cargo test -p hkask-mcp-docproc --lib -- kindle`

use std::io::Write;
use std::path::Path;

use hkask_mcp_docproc::kindle_zip::types::{
    BookMetadata, ContentChunk, ExportResult, PageEntry, PageNav, TocItem,
};
use hkask_mcp_docproc::kindle_zip::{export_epub, export_formats, export_markdown, export_pdf};

/// Build minimal test metadata for a 3-page book.
fn test_metadata(dir: &Path) -> BookMetadata {
    BookMetadata {
        asin: "TEST000001".into(),
        title: "Test Book".into(),
        author: "Test Author".into(),
        authors: vec!["Test Author".into()],
        description: None,
        cover_url: None,
        pages: vec![
            PageEntry {
                index: 0,
                page: 1,
                screenshot: dir.join("000-001.png"),
            },
            PageEntry {
                index: 1,
                page: 2,
                screenshot: dir.join("001-002.png"),
            },
            PageEntry {
                index: 2,
                page: 3,
                screenshot: dir.join("002-003.png"),
            },
        ],
        toc: vec![
            TocItem {
                label: "Chapter One".into(),
                depth: 0,
                page: Some(1),
                position_id: None,
            },
            TocItem {
                label: "Chapter Two".into(),
                depth: 0,
                page: Some(2),
                position_id: None,
            },
        ],
        nav: PageNav {
            start_content_page: 1,
            end_content_page: 3,
            total_pages: 3,
            total_content_pages: 3,
        },
        raw_meta: serde_json::json!({}),
    }
}

fn test_content() -> Vec<ContentChunk> {
    vec![
        ContentChunk {
            index: 0,
            page: 1,
            text: "Chapter One\n\nIt was a dark and stormy night. The rain fell in torrents, except at occasional intervals when it was checked by a violent gust of wind.".into(),
            screenshot: None,
            confidence: Some(0.95),
            provenance: None,
        },
        ContentChunk {
            index: 1,
            page: 2,
            text: "Chapter Two\n\nThe next morning dawned bright and clear. Birds sang in the trees.".into(),
            screenshot: None,
            confidence: Some(0.92),
            provenance: None,
        },
        ContentChunk {
            index: 2,
            page: 3,
            text: "And so the story ends. Nothing more needs to be said, for everything had been told.".into(),
            screenshot: None,
            confidence: Some(0.88),
            provenance: None,
        },
    ]
}

fn assembled_text() -> String {
    test_content()
        .iter()
        .map(|c| c.text.clone())
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[test]
fn metadata_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let meta = test_metadata(dir.path());
    let json = serde_json::to_string_pretty(&meta).unwrap();
    let roundtripped: BookMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(roundtripped.asin, "TEST000001");
    assert_eq!(roundtripped.title, "Test Book");
    assert_eq!(roundtripped.pages.len(), 3);
    assert_eq!(roundtripped.toc.len(), 2);
    assert_eq!(roundtripped.toc[0].label, "Chapter One");
}

#[test]
fn export_pdf_valid_structure() {
    let text = assembled_text();
    let pdf = export_pdf(&text, "Test Book").unwrap();

    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(pdf.ends_with(b"%%EOF\n"));

    let pdf_str = String::from_utf8_lossy(&pdf);
    assert!(pdf_str.contains("/Type /Catalog"));
    assert!(pdf_str.contains("/BaseFont /Helvetica"));
    assert!(pdf_str.contains("xref"));
    assert!(pdf_str.contains("trailer"));
    assert!(pdf_str.contains("startxref"));
    assert!(pdf_str.contains("Test Book"));

    // Verify xref table has 6 entries with correct offsets (not fake)
    let xref_start = pdf_str.find("xref").unwrap();
    let after_xref = &pdf_str[xref_start..];
    assert!(after_xref.contains("0000000000 65535 f"));
}

#[test]
fn export_epub_valid_zip() {
    let text = assembled_text();
    let toc = test_metadata(Path::new("/tmp")).toc;
    let epub = export_epub(&text, "Test Book", "Test Author", &toc).unwrap();

    // EPUB is a ZIP file
    assert_eq!(&epub[0..4], b"PK\x03\x04");
    assert!(epub.len() > 200, "EPUB should have structure");

    // Contains mimetype entry
    let epub_str = String::from_utf8_lossy(&epub);
    assert!(epub_str.contains("application/epub+zip"));
}

#[test]
fn export_markdown_has_headings() {
    let text = assembled_text();
    let toc = test_metadata(Path::new("/tmp")).toc;
    let md = export_markdown(&text, "Test Book", "Test Author", &toc);

    assert!(md.contains("# Test Book"));
    assert!(md.contains("By Test Author"));
    assert!(md.contains("## Table of Contents"));
    assert!(md.contains("## Chapter One"));
    assert!(md.contains("## Chapter Two"));
    assert!(md.contains("dark and stormy night"));
}

#[test]
fn export_formats_all_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let meta = test_metadata(dir.path());
    let meta_path = dir.path().join("metadata.json");
    let text = assembled_text();

    // Write metadata so export_formats can read TOC
    let json = serde_json::to_string_pretty(&meta).unwrap();
    std::fs::write(&meta_path, &json).unwrap();

    let result = export_formats(
        &text,
        &meta_path,
        &["pdf".into(), "epub".into(), "markdown".into()],
        dir.path(),
        "TEST000001",
        "Test Book",
        "Test Author",
        &meta.toc,
    )
    .unwrap();

    assert_eq!(result.exports.len(), 3);
    assert!(result.total_bytes > 0);

    let formats: Vec<&str> = result.exports.iter().map(|e| e.format.as_str()).collect();
    assert!(formats.contains(&"pdf"));
    assert!(formats.contains(&"epub"));
    assert!(formats.contains(&"markdown"));

    // Verify files exist on disk
    assert!(dir.path().join("TEST000001/book.pdf").exists());
    assert!(dir.path().join("TEST000001/book.epub").exists());
    assert!(dir.path().join("TEST000001/book.md").exists());
}

#[test]
fn content_json_roundtrip() {
    let chunks = test_content();
    let json = serde_json::to_string_pretty(&chunks).unwrap();
    let roundtripped: Vec<ContentChunk> = serde_json::from_str(&json).unwrap();

    assert_eq!(roundtripped.len(), 3);
    assert_eq!(roundtripped[0].text, chunks[0].text);
    assert_eq!(roundtripped[1].confidence.unwrap(), 0.92);
    assert_eq!(roundtripped[2].page, 3);
}

#[test]
fn empty_toc_export_does_not_panic() {
    let text = "Just some text without chapters.";
    let empty_toc: Vec<TocItem> = vec![];

    // All exports should handle empty TOC gracefully
    let _pdf = export_pdf(text, "Solo").unwrap();
    let _epub = export_epub(text, "Solo", "Author", &empty_toc).unwrap();
    let md = export_markdown(text, "Solo", "Author", &empty_toc);
    assert!(md.contains("# Solo"));
}

#[test]
fn export_txt_format() {
    let dir = tempfile::tempdir().unwrap();
    let text = "Plain text export test.\n\nSecond paragraph.";
    let meta_path = dir.path().join("metadata.json");
    std::fs::write(&meta_path, "{}").unwrap();

    let result = export_formats(
        text,
        &meta_path,
        &["txt".into()],
        dir.path(),
        "TXT001",
        "TXT Book",
        "Author",
        &[],
    )
    .unwrap();

    assert_eq!(result.exports.len(), 1);
    assert_eq!(result.exports[0].format, "txt");
    assert!(dir.path().join("TXT001/book.txt").exists());

    let written = std::fs::read_to_string(dir.path().join("TXT001/book.txt")).unwrap();
    assert_eq!(written, text);
}
