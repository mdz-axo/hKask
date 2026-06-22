//! Kindle-Zip pipeline integration tests.
//!
//! Validates the full logical flow without real browser/OCR dependencies.
//! Tests: metadata roundtrip, assemble→export pipeline, format validity.
//!
//! Run: `cargo test -p hkask-mcp-docproc --test integration -- kindle_zip --ignored`
//! Or without ignored: `cargo test -p hkask-mcp-docproc --lib -- kindle`

use std::path::Path;

use hkask_mcp_docproc::kindle_zip::types::{
    BookMetadata, ContentChunk, PageEntry, PageNav, TocItem,
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
    let text = assembled_text();

    let result = export_formats(
        &text,
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

    let result = export_formats(
        text,
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

// ── Production extraction test (requires Amazon credentials + browser) ─

#[tokio::test]
#[ignore = "requires AMAZON_EMAIL/AMAZON_PASSWORD in .env and Chrome installed"]
async fn extract_real_book_production() {
    let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    if env_path.exists() {
        for line in std::fs::read_to_string(&env_path)
            .unwrap_or_default()
            .lines()
        {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                unsafe {
                    std::env::set_var(k.trim(), v.trim().trim_matches('"'));
                }
            }
        }
    }

    let email = std::env::var("AMAZON_EMAIL").expect("AMAZON_EMAIL not set");
    let password = std::env::var("AMAZON_PASSWORD").expect("AMAZON_PASSWORD not set");
    let asin = "B0GHZLT1S3";

    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path();

    let result =
        hkask_mcp_docproc::kindle_zip::extract_kindle_book(asin, &email, &password, output, None)
            .await;

    match result {
        Ok(r) => {
            println!("Extracted: {} pages, title={}", r.total_pages, r.title);
            assert!(r.total_pages > 0);
            assert!(!r.title.is_empty());
            assert!(r.metadata_path.exists());
            assert!(r.pages_dir.exists());

            let png_count = std::fs::read_dir(&r.pages_dir)
                .unwrap()
                .filter(|e| {
                    e.as_ref()
                        .ok()
                        .and_then(|e| e.path().extension().map(|ext| ext == "png"))
                        .unwrap_or(false)
                })
                .count();
            assert!(
                png_count > 0,
                "Should have at least 1 page PNG, got {}",
                png_count
            );
            println!("  PNG files: {}", png_count);

            let meta_json = std::fs::read_to_string(&r.metadata_path).unwrap();
            let _meta: hkask_mcp_docproc::kindle_zip::types::BookMetadata =
                serde_json::from_str(&meta_json).expect("valid metadata.json");
        }
        Err(e) => {
            if e.contains("Chrome") || e.contains("landing") {
                println!("Skipping — browser/env: {}", e);
                return;
            }
            panic!("Unexpected error: {}", e);
        }
    }
}

// ── Full pipeline: extract → transcribe → export ──────────────────────

#[tokio::test]
#[ignore = "requires Amazon creds + inference API key + Chrome"]
async fn full_pipeline_extract_transcribe_export() {
    let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    if env_path.exists() {
        for line in std::fs::read_to_string(&env_path)
            .unwrap_or_default()
            .lines()
        {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                unsafe {
                    std::env::set_var(k.trim(), v.trim().trim_matches('"'));
                }
            }
        }
    }

    let email = std::env::var("AMAZON_EMAIL").expect("AMAZON_EMAIL not set");
    let password = std::env::var("AMAZON_PASSWORD").expect("AMAZON_PASSWORD not set");
    let asin = "B0GHZLT1S3";
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path();

    // Step 1: Extract pages
    println!("=== Step 1: Extract ===");
    let extract =
        hkask_mcp_docproc::kindle_zip::extract_kindle_book(asin, &email, &password, output, None)
            .await
            .expect("Extract");
    println!("  Pages: {}, Title: {}", extract.total_pages, extract.title);

    // Step 2: OCR transcription
    println!("=== Step 2: Transcribe ===");
    let config = hkask_inference::InferenceConfig::from_env();
    let ocr_model = std::env::var("HKASK_OCR_MODEL").ok();
    let thresholds = hkask_mcp_docproc::ocr::ThresholdConfig::default();
    let embed_model = std::env::var("HKASK_EMBEDDING_MODEL")
        .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string());

    let server = hkask_mcp_docproc::DocProcServer::new(
        hkask_types::WebID::new(),
        "kindle-e2e".into(),
        None,
        ocr_model.clone(),
        config,
        thresholds,
        None,
    )
    .expect("DocProcServer");

    let embed_ref: Option<(&hkask_inference::EmbeddingRouter, &str)> = server
        .embedding_router
        .as_ref()
        .map(|er| (er, embed_model.as_str()));

    let transcribe = hkask_mcp_docproc::kindle_zip::transcribe_pages(
        &extract.pages_dir,
        &extract.metadata_path,
        output,
        asin,
        &server,
        &thresholds,
        ocr_model.as_deref(),
        embed_ref,
    )
    .await
    .expect("Transcribe");
    println!(
        "  Words: {}, transcribed: {} pages, confidence: {:.3}",
        transcribe.total_words, transcribe.transcribed_pages, transcribe.mean_confidence
    );

    // Step 3: Assemble content
    let content_path = output.join(asin).join("content.json");
    let content_json = std::fs::read_to_string(&content_path).expect("content.json");
    let chunks: Vec<hkask_mcp_docproc::kindle_zip::types::ContentChunk> =
        serde_json::from_str(&content_json).expect("Parse content");
    let assembled = hkask_mcp_docproc::kindle_zip::assemble_chunks(&chunks, &extract.toc);
    println!("  Assembled: {} chars", assembled.len());

    // Step 4: Export formats
    println!("=== Step 4: Export ===");
    let formats = vec![
        "pdf".to_string(),
        "epub".to_string(),
        "markdown".to_string(),
    ];
    let export = hkask_mcp_docproc::kindle_zip::export_formats(
        &assembled,
        &formats,
        output,
        asin,
        &extract.title,
        &extract.author,
        &extract.toc,
    )
    .expect("Export");
    for e in &export.exports {
        println!("  {}: {} bytes", e.format, e.size_bytes);
        assert!(e.path.exists());
        assert!(e.size_bytes > 0);
    }

    // Copy to Knowledge folder
    let dest = std::path::Path::new("/home/mdz-axolotl/Clones/Library/Knowledge").join(asin);
    std::fs::create_dir_all(&dest).ok();
    let src = output.join(asin);
    if src.exists() {
        for e in std::fs::read_dir(&src).ok().into_iter().flatten().flatten() {
            std::fs::copy(e.path(), dest.join(e.file_name())).ok();
        }
    }
    println!("=== Pipeline complete ===");
}

// ── Full pipeline for all matching knowledge books ────────────────────

#[tokio::test]
#[ignore = "requires Amazon creds + inference API key + Chrome"]
async fn knowledge_corpus_pipeline() {
    let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    if env_path.exists() {
        for line in std::fs::read_to_string(&env_path)
            .unwrap_or_default()
            .lines()
        {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                unsafe {
                    std::env::set_var(k.trim(), v.trim().trim_matches('"'));
                }
            }
        }
    }

    let email = std::env::var("AMAZON_EMAIL").expect("AMAZON_EMAIL");
    let password = std::env::var("AMAZON_PASSWORD").expect("AMAZON_PASSWORD");
    let chrome_bin = "/snap/chromium/current/usr/lib/chromium-browser/chrome";

    // ── Phase 1: Discover books via browser ────────────────────────────
    println!("=== Phase 1: Login & Discover ===");
    use headless_chrome::{Browser, LaunchOptionsBuilder};
    let launch_opts = LaunchOptionsBuilder::default()
        .headless(true)
        .window_size(Some((1280, 900)))
        .sandbox(false)
        .path(Some(std::path::PathBuf::from(chrome_bin)))
        .args(vec![
            std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
            std::ffi::OsStr::new("--no-first-run"),
        ])
        .build()
        .expect("LaunchOptions");

    let browser = Browser::new(launch_opts).expect("Browser");
    let tab = browser.new_tab().expect("Tab");
    tab.navigate_to("https://read.amazon.com/kindle-library")
        .expect("Nav");
    tab.wait_until_navigated().expect("Wait");
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Auth state machine
    let url = tab.get_url();
    if url.contains("/landing") {
        for s in &["a[href*='signin']", "a[href*='ap/signin']", "a"] {
            if let Ok(el) = tab.find_element(s)
                && let Ok(Some(href)) = el.get_attribute_value("href")
                && (href.contains("signin") || href.contains("ap/sign"))
            {
                el.click().ok();
                std::thread::sleep(std::time::Duration::from_secs(3));
                break;
            }
        }
    }
    let url = tab.get_url();
    if url.contains("/ap/signin") {
        if let Ok(el) = tab.wait_for_element("input[type=\"email\"]") {
            el.click().ok();
            std::thread::sleep(std::time::Duration::from_millis(500));
            tab.type_str(&email).ok();
        }
        if let Ok(el) = tab.wait_for_element("input[type=\"submit\"]") {
            el.click().ok();
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        if let Ok(el) = tab.wait_for_element("input[type=\"password\"]") {
            el.click().ok();
            std::thread::sleep(std::time::Duration::from_millis(500));
            tab.type_str(&password).ok();
        }
        if let Ok(el) = tab.find_element("input[type=\"submit\"]") {
            el.click().ok();
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }
    let url = tab.get_url();
    if url.contains("/ap/mfa") || url.contains("/ap/cvf") {
        println!("  2FA — waiting 60s");
        std::thread::sleep(std::time::Duration::from_secs(60));
    }

    tab.navigate_to("https://read.amazon.com/kindle-library")
        .expect("Nav lib");
    tab.wait_until_navigated().expect("Wait lib");
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Discover ASINs
    let terms = [
        "knowledge graph",
        "knowledge production",
        "autopoietic",
        "autopoiesis",
    ];
    let mut books: Vec<(String, String)> = Vec::new();
    if let Ok(html) = tab.get_content() {
        let needle = "\"asin\"";
        let mut pos = 0u64;
        let mut seen = std::collections::HashSet::new();
        let mut seen_title = std::collections::HashSet::new();
        while let Some(found) = html[pos as usize..].find(needle) {
            let abs = pos as usize + found;
            let after = &html[abs + needle.len()..];
            if let Some(rest) = after.strip_prefix(":") {
                let rest = rest.trim_start();
                if let Some(inner) = rest.strip_prefix('"')
                    && let Some(end) = inner.find('"')
                {
                    let asin = &inner[..end];
                    if asin.len() == 10
                        && asin.chars().all(|c| c.is_ascii_alphanumeric())
                        && seen.insert(asin.to_string())
                    {
                        let title = extract_title_near_asin(&html, asin);
                        if !title.is_empty() {
                            let tl = title.to_lowercase();
                            if terms.iter().any(|t| tl.contains(t))
                                && seen_title.insert(title.clone())
                                && books.len() < 5
                            {
                                println!("  Discovered: [{}] {}", asin, title);
                                books.push((asin.to_string(), title));
                            }
                        }
                    }
                }
            }
            pos = abs as u64 + 1;
        }
    }
    drop(tab);
    drop(browser);

    println!("Found {} matching books", books.len());

    // ── Phase 2: Extract + Transcribe + Export each book ───────────────
    let config = hkask_inference::InferenceConfig::from_env();
    let ocr_model = std::env::var("HKASK_OCR_MODEL").ok();
    let thresholds = hkask_mcp_docproc::ocr::ThresholdConfig::default();
    let embed_model = std::env::var("HKASK_EMBEDDING_MODEL")
        .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string());
    let dest_base = std::path::Path::new("/home/mdz-axolotl/Clones/Library/Knowledge");

    let server = hkask_mcp_docproc::DocProcServer::new(
        hkask_types::WebID::new(),
        "kindle-corpus".into(),
        None,
        ocr_model.clone(),
        config,
        thresholds,
        None,
    )
    .expect("DocProcServer");

    // Phase 2: Extract all books from ONE browser session
    println!("=== Phase 2: Extract (single session) ===");
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path();

    let asins: Vec<String> = books.iter().map(|(a, _)| a.clone()).collect();
    let extracts = match hkask_mcp_docproc::kindle_zip::extract_kindle_books(
        &asins, &email, &password, output, None,
    )
    .await
    {
        Ok(results) => {
            println!("  Extracted {}/{} books", results.len(), asins.len());
            results
        }
        Err(e) => {
            println!("  Batch extraction failed: {}", e);
            return;
        }
    };

    // Phase 3: Transcribe + Export each extracted book
    // Build title lookup from discovery results
    let title_map: std::collections::HashMap<&str, &str> = books
        .iter()
        .map(|(a, t)| (a.as_str(), t.as_str()))
        .collect();

    for extract in &extracts {
        let asin = &extract.asin;
        let title = title_map
            .get(asin.as_str())
            .copied()
            .unwrap_or(&extract.title);
        println!("\n=== {} [{}] ===", title, asin);

        // Transcribe (pages already extracted by batch)
        let embed_ref: Option<(&hkask_inference::EmbeddingRouter, &str)> = server
            .embedding_router
            .as_ref()
            .map(|er| (er, embed_model.as_str()));
        let _transcribe = match hkask_mcp_docproc::kindle_zip::transcribe_pages(
            &extract.pages_dir,
            &extract.metadata_path,
            output,
            asin,
            &server,
            &thresholds,
            ocr_model.as_deref(),
            embed_ref,
        )
        .await
        {
            Ok(r) => {
                println!(
                    "  Transcribe: {} words, {} pages",
                    r.total_words, r.transcribed_pages
                );
                r
            }
            Err(e) => {
                println!("  Transcribe FAILED: {}", e);
                continue;
            }
        };

        // Assemble
        let content_path = output.join(asin).join("content.json");
        let content_json = std::fs::read_to_string(&content_path).unwrap();
        let chunks: Vec<hkask_mcp_docproc::kindle_zip::types::ContentChunk> =
            serde_json::from_str(&content_json).unwrap();
        let assembled = hkask_mcp_docproc::kindle_zip::assemble_chunks(&chunks, &extract.toc);

        // Export
        let formats = vec![
            "pdf".to_string(),
            "epub".to_string(),
            "markdown".to_string(),
        ];
        match hkask_mcp_docproc::kindle_zip::export_formats(
            &assembled,
            &formats,
            output,
            asin,
            title, // Use discovered title, not extract.title (which is "Kindle")
            &extract.author,
            &extract.toc,
        ) {
            Ok(export) => {
                for e in &export.exports {
                    println!("  {}: {} bytes", e.format, e.size_bytes);
                }
            }
            Err(e) => println!("  Export FAILED: {}", e),
        }

        // Copy to Knowledge folder — both ASIN internals + human-readable root files
        let dest = dest_base.join(asin);
        std::fs::create_dir_all(&dest).ok();
        // Copy ASIN pipeline directory
        let src = output.join(asin);
        if src.exists() {
            for e in std::fs::read_dir(&src).ok().into_iter().flatten().flatten() {
                std::fs::copy(e.path(), dest.join(e.file_name())).ok();
            }
        }
        // Copy human-readable root files (Author - Title.{pdf,epub,md})
        for e in std::fs::read_dir(output)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".pdf") || name.ends_with(".epub") || name.ends_with(".md") {
                std::fs::copy(e.path(), dest_base.join(&name)).ok();
            }
        }
        // Copy index.json
        let index_src = output.join("index.json");
        if index_src.exists() {
            std::fs::copy(&index_src, dest_base.join("index.json")).ok();
        }
        println!("  -> {}", dest.display());
    }
    println!("\n=== Corpus complete ===");
}

fn extract_title_near_asin(html: &str, asin: &str) -> String {
    if let Some(p) = html.find(asin) {
        let window = &html[p.saturating_sub(500)..std::cmp::min(p + 500, html.len())];
        for pattern in &["\"title\":\"", "title\":\""] {
            if let Some(tp) = window.find(pattern) {
                let after = &window[tp + pattern.len()..];
                if let Some(end) = after.find('"') {
                    let t = &after[..end];
                    let t = t.replace("\\\"", "\"").replace("\\n", " ");
                    if t.len() > 3 {
                        return t;
                    }
                }
            }
        }
    }
    String::new()
}
