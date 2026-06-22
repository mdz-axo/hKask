//! Browser-based extraction of Kindle book pages.
//!
//! Navigates Kindle Cloud Reader: login → settings → TOC extraction →
//! page capture with retry → position restore. Idempotent resume support.

use std::path::Path;

use crate::kindle_zip::types::{
    BookMetadata, ExtractResult, PageEntry, PageNav, TocItem, has_page_files, zeropad,
};

pub async fn extract_kindle_book(
    asin: &str,
    amazon_email: &str,
    amazon_password: &str,
    output_dir: &Path,
    chrome_profile: Option<&Path>,
) -> Result<ExtractResult, String> {
    let book_dir = output_dir.join(asin);
    let pages_dir = book_dir.join("pages");
    let metadata_path = book_dir.join("metadata.json");

    std::fs::create_dir_all(&book_dir).map_err(|e| format!("mkdir book dir: {}", e))?;
    std::fs::create_dir_all(&pages_dir).map_err(|e| format!("mkdir pages dir: {}", e))?;

    if metadata_path.exists() && has_page_files(&pages_dir) {
        tracing::info!(target: "cns.pipeline.kindle-zip.extract", asin = %asin, "Resuming from existing extraction");
        return load_existing(&metadata_path, &pages_dir);
    }

    tracing::info!(target: "cns.pipeline.kindle-zip.extract", asin = %asin, "Starting browser-based extraction");
    let meta = extract_via_browser(
        asin,
        amazon_email,
        amazon_password,
        &book_dir,
        &pages_dir,
        chrome_profile,
    )
    .await?;

    tracing::info!(target: "cns.pipeline.kindle-zip.extract",
        asin = %asin, total_pages = meta.total_pages, content_pages = meta.content_pages,
        title = %meta.title, toc_entries = meta.toc.len(), "Extraction complete");
    Ok(meta)
}

async fn extract_via_browser(
    asin: &str,
    amazon_email: &str,
    amazon_password: &str,
    book_dir: &Path,
    pages_dir: &Path,
    chrome_profile: Option<&Path>,
) -> Result<ExtractResult, String> {
    use headless_chrome::{Browser, LaunchOptionsBuilder};

    let _using_profile = chrome_profile.is_some();
    let mut builder = LaunchOptionsBuilder::default();
    builder.headless(true);
    builder.window_size(Some((1280, 720)));
    builder.sandbox(false);
    // Anti-detection: prevent Amazon from identifying headless Chrome
    builder.args(vec![
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--disable-features=IsolateOrigins,site-per-process"),
    ]);
    if let Some(profile_dir) = chrome_profile {
        tracing::info!(target: "cns.pipeline.kindle-zip.extract",
            profile = %profile_dir.display(), "Using Chrome profile for cookie-based auth");
        builder.user_data_dir(Some(profile_dir.to_path_buf()));
    }

    let launch_opts = builder
        .build()
        .map_err(|e| format!("Browser options: {}", e))?;

    let browser = Browser::new(launch_opts)
        .map_err(|e| format!("Launch browser: {}. Chrome installed?", e))?;
    let tab = browser.new_tab().map_err(|e| format!("Tab: {}", e))?;
    let book_url = format!("https://read.amazon.com/?asin={}", asin);

    tab.navigate_to(&book_url)
        .map_err(|e| format!("Nav: {}", e))?;
    tab.wait_until_navigated()
        .map_err(|e| format!("Wait nav: {}", e))?;

    let url = tab.get_url();
    if url.contains("/ap/signin") {
        kindle_login(&tab, amazon_email, amazon_password)?;
        tab.navigate_to(&book_url)
            .map_err(|e| format!("Post-login nav: {}", e))?;
        tab.wait_until_navigated()
            .map_err(|e| format!("Post-login wait: {}", e))?;
    }

    std::thread::sleep(std::time::Duration::from_secs(5));
    verify_selectors(&tab)?;

    // Gap 2: Apply reader settings (single-column + sans-serif) before capture
    apply_reader_settings(&tab);

    // Gap 3: Record initial position so we can restore it after extraction
    let initial_page = read_current_page(&tab);
    tracing::info!(target: "cns.pipeline.kindle-zip.extract", initial_page, "Position recorded");

    let (title, author) = scrape_title_author(&tab)?;

    // Gap 1: Extract TOC from page context (JS evaluation of Kindle reader state)
    let toc = extract_toc(&tab)?;

    let pages = capture_pages(&tab, pages_dir)?;
    let total_pages = pages.len();

    // Gap 3: Restore reading position
    if let Some(start_page) = initial_page {
        navigate_to_page(&tab, start_page);
        tracing::info!(target: "cns.pipeline.kindle-zip.extract", start_page, "Position restored");
    }

    let title_c = title.clone();
    let author_c = author.clone();

    let full = BookMetadata {
        asin: asin.to_string(),
        title: title_c.clone(),
        author: author_c.clone(),
        authors: vec![author_c.clone()],
        description: None,
        cover_url: None,
        pages,
        toc: toc.clone(),
        nav: PageNav {
            start_content_page: 1,
            end_content_page: total_pages as i64,
            total_pages: total_pages as i64,
            total_content_pages: total_pages as i64,
        },
        raw_meta: serde_json::json!({}),
    };
    let json = serde_json::to_string_pretty(&full).map_err(|e| format!("JSON: {}", e))?;
    std::fs::write(book_dir.join("metadata.json"), json).map_err(|e| format!("Write: {}", e))?;

    Ok(ExtractResult {
        metadata_path: book_dir.join("metadata.json"),
        pages_dir: pages_dir.to_path_buf(),
        total_pages,
        content_pages: total_pages,
        title: title_c,
        author: author_c,
        toc,
        cns_span_id: None,
    })
}

// ── Login ───────────────────────────────────────────────────────────────────

fn kindle_login(tab: &headless_chrome::Tab, email: &str, password: &str) -> Result<(), String> {
    tab.wait_for_element("input[type=\"email\"]")
        .and_then(|el| el.click().map(|_| ()))
        .map_err(|e| format!("Email field: {}", e))?;
    tab.type_str(email)
        .map_err(|e| format!("Type email: {}", e))?;
    tab.wait_for_element("input[type=\"submit\"]")
        .and_then(|el| el.click().map(|_| ()))
        .map_err(|e| format!("Submit email: {}", e))?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    tab.wait_for_element("input[type=\"password\"]")
        .and_then(|el| el.click().map(|_| ()))
        .map_err(|e| format!("Password field: {}", e))?;
    tab.type_str(password)
        .map_err(|e| format!("Type password: {}", e))?;
    tab.wait_for_element("input[type=\"submit\"]")
        .and_then(|el| el.click().map(|_| ()))
        .map_err(|e| format!("Submit password: {}", e))?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let url = tab.get_url();
    if url.contains("/ap/mfa") || url.contains("/ap/cvf") {
        tracing::warn!(target: "cns.pipeline.kindle-zip.extract", "2FA — waiting 30s for manual entry");
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
    Ok(())
}

// ── Reader Settings (Gap 2) ─────────────────────────────────────────────────

/// Apply optimal reader settings for OCR: single-column layout, sans-serif font.
/// Mirrors the reference project's `updateSettings()` function.
fn apply_reader_settings(tab: &headless_chrome::Tab) {
    // Click settings button
    let settings_btn = tab.wait_for_element(
        "ion-button[aria-label=\"Reader settings\"], button[aria-label=\"Reader settings\"]",
    );
    if let Ok(el) = settings_btn {
        let _ = el.click();
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Select Amazon Ember font (sans-serif, better OCR accuracy)
        if let Ok(ember) = tab.find_element("#AmazonEmber") {
            let _ = ember.click();
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        // Switch to single-column layout
        let single_col = tab.find_element("[role=\"radiogroup\"][aria-label$=\" columns\"]");
        if let Ok(col) = single_col {
            let _ = col.click();
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        // Close settings
        let _ = el.click();
        std::thread::sleep(std::time::Duration::from_millis(500));
        tracing::info!(target: "cns.pipeline.kindle-zip.extract", "Reader settings applied: single-column, Amazon Ember");
    }
}

// ── Position Tracking (Gap 3) ───────────────────────────────────────────────

fn read_current_page(tab: &headless_chrome::Tab) -> Option<usize> {
    tab.evaluate(
        "document.querySelector('ion-footer ion-title')?.textContent || ''",
        false,
    )
    .ok()
    .and_then(|v| v.value)
    .and_then(|v| v.as_str().map(String::from))
    .and_then(|text| {
        // Extract current page number from "Page X of Y"
        text.split(|c: char| !c.is_ascii_digit())
            .find_map(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
    })
}

fn navigate_to_page(tab: &headless_chrome::Tab, page: usize) {
    // Open reader menu → Go to Page → enter number → confirm
    if let Ok(menu) = tab.find_element("ion-button[aria-label=\"Reader menu\"]") {
        let _ = menu.click();
        std::thread::sleep(std::time::Duration::from_millis(500));

        if let Ok(go_to) = tab.find_element("ion-item[role=\"listitem\"]") {
            let _ = go_to.click();
            std::thread::sleep(std::time::Duration::from_millis(300));

            let _ = tab.type_str(&page.to_string());
            std::thread::sleep(std::time::Duration::from_millis(200));

            if let Ok(go_btn) =
                tab.find_element("ion-modal ion-button[item-i-d=\"go-to-modal-go-button\"]")
            {
                let _ = go_btn.click();
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }
}

// ── TOC Extraction (Gap 1) ──────────────────────────────────────────────────

/// Extract table of contents from Kindle reader page state.
///
/// Attempts to read TOC data from the Kindle reader's JavaScript context.
/// The Kindle web reader stores TOC in internal state objects.
/// Falls back gracefully if extraction fails.
fn extract_toc(tab: &headless_chrome::Tab) -> Result<Vec<TocItem>, String> {
    // Try to extract TOC from Kindle's internal reader state via JS evaluation.
    // The Kindle Cloud Reader stores book metadata (including TOC) in
    // window-scoped JavaScript objects loaded from render TAR responses.
    let js = r#"
        (function() {
            // Attempt to find TOC data in common Kindle reader state locations
            try {
                // Method 1: Check for global reader state
                if (window.kr && window.kr.reader && window.kr.reader.toc) {
                    return JSON.stringify(window.kr.reader.toc);
                }
            } catch(e) {}
            try {
                // Method 2: Check for Angular/React component state (Ionic framework)
                var tocEls = document.querySelectorAll('[ng-reflect-toc], [data-toc]');
                if (tocEls.length > 0) {
                    var raw = tocEls[0].getAttribute('ng-reflect-toc') ||
                               tocEls[0].getAttribute('data-toc');
                    if (raw) return raw;
                }
            } catch(e) {}
            try {
                // Method 3: Check for toc data in window.__INITIAL_STATE__ or similar
                var state = window.__INITIAL_STATE__ || window.__NEXT_DATA__ ||
                            window.__NUXT__ || window.__GATSBY__;
                if (state && state.toc) return JSON.stringify(state.toc);
            } catch(e) {}
            // Method 4: Extract TOC from DOM (sidebar/navigation elements)
            try {
                var items = [];
                var tocLinks = document.querySelectorAll(
                    'ion-list[aria-label*="Contents"] ion-item, ' +
                    '[role="navigation"] a, ' +
                    '.toc-item, .chapter-link'
                );
                tocLinks.forEach(function(el) {
                    var label = (el.textContent || '').trim();
                    if (label && label.length > 1) {
                        items.push({label: label, depth: 0});
                    }
                });
                if (items.length > 0) return JSON.stringify(items);
            } catch(e) {}
            return '[]';
        })()
    "#;

    let raw = tab
        .evaluate(js, false)
        .ok()
        .and_then(|v| v.value)
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    // Parse TOC — could be Amazon render TOC format or simple label list
    let toc: Vec<TocItem> = serde_json::from_str(&raw).unwrap_or_default();

    if toc.is_empty() {
        tracing::info!(target: "cns.pipeline.kindle-zip.extract",
            "No TOC extracted from page state — chapter structure will be inferred from text");
    } else {
        tracing::info!(target: "cns.pipeline.kindle-zip.extract",
            toc_entries = toc.len(), "TOC extracted from page state");
    }

    Ok(toc)
}

// ── Metadata ────────────────────────────────────────────────────────────────

fn scrape_title_author(tab: &headless_chrome::Tab) -> Result<(String, String), String> {
    let title = tab
        .get_title()
        .unwrap_or_else(|_| "Unknown Title".to_string());
    let author = tab
        .evaluate(
            "document.querySelector('[data-author]')?.getAttribute('data-author') || 'Unknown Author'",
            false,
        )
        .ok()
        .and_then(|v| v.value)
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "Unknown Author".to_string());
    Ok((title, author))
}

/// Recover title/author via LLM when deterministic extraction returns "Unknown".
///
/// Returns the current values unchanged, plus an optional prompt for LLM recovery.
/// The caller wires this prompt through the inference pipeline and parses the JSON
/// response ({"title": "...", "author": "..."}) to update metadata.
#[allow(dead_code)] // awaiting inference port wiring in MCP tool handler
pub(crate) async fn recover_metadata_via_llm(
    assembled_text: &str,
    current_title: &str,
    current_author: &str,
) -> ((String, String), Option<String>) {
    let title_needs = current_title == "Unknown Title" || current_title.is_empty();
    let author_needs = current_author == "Unknown Author" || current_author.is_empty();
    if !title_needs && !author_needs {
        return (
            (current_title.to_string(), current_author.to_string()),
            None,
        );
    }

    let sample = if assembled_text.len() > 8000 {
        &assembled_text[..8000]
    } else {
        assembled_text
    };

    let prompt = format!(
        "You are a metadata extraction assistant. Extract title and author from this text.\n\
         Return ONLY JSON: {{\"title\": \"...\", \"author\": \"...\"}}\n\
         Content:\n{sample}"
    );

    tracing::info!(
        target: "cns.pipeline.kindle-zip.metadata_recovery",
        title_needs_recovery = title_needs,
        author_needs_recovery = author_needs,
        sample_len = sample.len(),
        "Metadata recovery prompt prepared for LLM extraction"
    );

    (
        (current_title.to_string(), current_author.to_string()),
        Some(prompt),
    )
}

// ── Selector Verification (Gap 17: CNS alert on failure) ────────────────────

fn verify_selectors(tab: &headless_chrome::Tab) -> Result<(), String> {
    const REQUIRED: &[(&str, &str)] = &[
        ("reader_content", "#kr-renderer .kg-full-page-img img"),
        ("next_page", ".kr-chevron-container-right"),
        ("page_footer", "ion-footer ion-title"),
    ];

    let mut failures: Vec<&str> = Vec::new();
    for &(name, selector) in REQUIRED {
        match tab.find_element(selector) {
            Ok(_) => {
                tracing::debug!(target: "cns.pipeline.kindle-zip.selector", selector = name, status = "ok")
            }
            Err(_) => {
                failures.push(name);
                tracing::warn!(target: "cns.pipeline.kindle-zip.selector",
                    selector = name, css = selector, "SELECTOR DRIFT — CNS ALERT");
            }
        }
    }

    if failures.is_empty() {
        tracing::info!(target: "cns.pipeline.kindle-zip.selector", "All selectors valid");
        return Ok(());
    }

    Err(format!(
        "Kindle UI selector drift detected: {}. Amazon may have changed their reader DOM. \
         Run with --recalibrate to auto-discover updated selectors, or update \
         the selectors section in kindle-zip.yaml.",
        failures.join(", ")
    ))
}

// ── Page Capture (Gap 8: per-page CNS spans, Gap 12: retry, Gap 11: blank detection) ─

fn capture_pages(tab: &headless_chrome::Tab, pages_dir: &Path) -> Result<Vec<PageEntry>, String> {
    tab.wait_for_element("#kr-renderer .kg-full-page-img img")
        .map_err(|e| format!("Reader content: {}", e))?;

    let footer_text = tab
        .evaluate(
            "document.querySelector('ion-footer ion-title')?.textContent || ''",
            false,
        )
        .ok()
        .and_then(|v| v.value)
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    let total = parse_page_count(&footer_text).unwrap_or(50);
    let padding = format!("{}", total * 2).len();
    let mut pages: Vec<PageEntry> = Vec::with_capacity(total);
    let mut blank_count = 0usize;
    let mut last_src: Option<String> = None;
    let mut consecutive_failures = 0u32;
    const MAX_CONSECUTIVE_FAILURES: u32 = 10;

    for i in 0..total {
        let page_num = i + 1;
        let t_start = std::time::Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let data = tab
            .capture_screenshot(
                headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
                Some(90),
                None,
                true,
            )
            .map_err(|e| format!("Screenshot page {}: {}", page_num, e))?;

        // Gap 11: Detect blank/WebGL-failed images
        if is_blank_image(&data) {
            blank_count += 1;
            tracing::warn!(target: "cns.pipeline.kindle-zip.capture",
                page = page_num, "Blank/WebGL-failed image detected — page may not have rendered");
            if blank_count >= 3 {
                return Err(format!(
                    "{} consecutive blank pages detected. Kindle reader may require WebGL/GPU. \
                     Try running with a real display or GPU-enabled environment.",
                    blank_count
                ));
            }
        } else {
            blank_count = 0;
        }

        let filename = format!("{}-{}.png", zeropad(i, padding), zeropad(page_num, padding));
        let path = pages_dir.join(&filename);
        std::fs::write(&path, &data).map_err(|e| format!("Write {}: {}", filename, e))?;

        let duration_ms = t_start.elapsed().as_millis() as u64;

        // Gap 8: Per-page CNS span
        tracing::debug!(target: "cns.pipeline.kindle-zip.capture.page",
            page = page_num, bytes = data.len(), duration_ms, blank = blank_count > 0,
            "Page captured");

        pages.push(PageEntry {
            index: i,
            page: page_num,
            screenshot: path,
        });

        // Gap 12: Retry logic for page navigation
        if i < total - 1 {
            let navigated = navigate_next_page(tab, &mut last_src);
            if navigated {
                consecutive_failures = 0;
            } else {
                consecutive_failures += 1;
                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    tracing::error!(target: "cns.pipeline.kindle-zip.capture",
                        consecutive_failures, page = page_num,
                        "Page navigation failed repeatedly — breaking capture loop");
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }

    tracing::info!(target: "cns.pipeline.kindle-zip.capture",
        captured = pages.len(), total, blank_pages = blank_count, "Page capture complete");
    Ok(pages)
}

/// Navigate to next page. Returns true if navigation likely succeeded.
fn navigate_next_page(tab: &headless_chrome::Tab, last_src: &mut Option<String>) -> bool {
    // Read current image src before clicking
    let before = tab
        .evaluate(
            "document.querySelector('#kr-renderer .kg-full-page-img img')?.getAttribute('src') || ''",
            false,
        )
        .ok()
        .and_then(|v| v.value)
        .and_then(|v| v.as_str().map(String::from));

    // Click next page button
    let _ = tab
        .find_element(".kr-chevron-container-right")
        .and_then(|el| el.click().map(|_| ()));

    // Wait briefly and check if src changed
    std::thread::sleep(std::time::Duration::from_millis(200));
    let after = tab
        .evaluate(
            "document.querySelector('#kr-renderer .kg-full-page-img img')?.getAttribute('src') || ''",
            false,
        )
        .ok()
        .and_then(|v| v.value)
        .and_then(|v| v.as_str().map(String::from));

    // Navigation succeeded if src changed from what we had
    let changed = before.is_some() && after.is_some() && before != after;
    if changed {
        *last_src = after;
    } else if before.is_none() {
        // Can't read src — assume navigation worked
        return true;
    }
    changed
}

/// Gap 11: Detect images that are blank (WebGL failure) by checking entropy.
/// A blank white/black image has very low byte variance.
fn is_blank_image(data: &[u8]) -> bool {
    if data.len() < 1024 {
        return true; // too small to be a real page
    }
    // Sample bytes to check variance — blank PNGs are highly compressible
    let sample = &data[data.len() / 4..data.len() * 3 / 4];
    let mut unique: u8 = sample.first().copied().unwrap_or(0);
    let mut switches = 0u32;
    for &b in sample.iter().skip(1).take(256) {
        if b != unique {
            switches += 1;
            unique = b;
        }
    }
    // Very low byte variance suggests a blank/empty render
    switches < 5
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn parse_page_count(footer: &str) -> Option<usize> {
    let numbers: Vec<usize> = footer
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse::<usize>().ok())
        .filter(|&n| n > 1 && n < 100_000)
        .collect();
    if numbers.len() >= 2 {
        numbers.last().copied()
    } else {
        None
    }
}

fn load_existing(metadata_path: &Path, pages_dir: &Path) -> Result<ExtractResult, String> {
    let json = std::fs::read_to_string(metadata_path).map_err(|e| format!("Read: {}", e))?;
    let metadata: BookMetadata =
        serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))?;
    Ok(ExtractResult {
        metadata_path: metadata_path.to_path_buf(),
        pages_dir: pages_dir.to_path_buf(),
        total_pages: metadata.pages.len(),
        content_pages: metadata.nav.total_content_pages.max(1) as usize,
        title: metadata.title,
        author: metadata.author,
        toc: metadata.toc,
        cns_span_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_page_count_standard() {
        assert_eq!(parse_page_count("Page 5 of 342"), Some(342));
    }
    #[test]
    fn parse_page_count_slash() {
        assert_eq!(parse_page_count("5 / 342"), Some(342));
    }
    #[test]
    fn parse_page_count_single_ignored() {
        assert_eq!(parse_page_count("42"), None);
    }
    #[test]
    fn parse_page_count_empty() {
        assert_eq!(parse_page_count(""), None);
    }
    #[test]
    fn parse_page_count_no_numbers() {
        assert_eq!(parse_page_count("Hello World"), None);
    }

    #[test]
    fn blank_detection_real_png() {
        // A real PNG header followed by varied bytes — should NOT be blank
        let data: Vec<u8> = (0..2048).map(|i| (i % 256) as u8).collect();
        assert!(!is_blank_image(&data));
    }

    #[test]
    fn blank_detection_uniform_bytes() {
        // All same byte value — should be blank
        let data = vec![128u8; 2048];
        assert!(is_blank_image(&data));
    }

    #[test]
    fn blank_detection_too_small() {
        assert!(is_blank_image(&[0u8; 512]));
    }
}
