//! Browser-based extraction of Kindle book pages.
//!
//! Navigates Kindle Cloud Reader, logs in, captures page screenshots,
//! and records metadata. Supports resume from existing extraction.

use std::path::Path;

use crate::kindle_zip::types::{
    BookMetadata, ExtractResult, PageEntry, PageNav, has_page_files, zeropad,
};

/// Extract book pages from Kindle Cloud Reader via headless browser.
///
/// Idempotent — resumes from existing extraction if pages exist on disk.
pub async fn extract_kindle_book(
    asin: &str,
    amazon_email: &str,
    amazon_password: &str,
    output_dir: &Path,
) -> Result<ExtractResult, String> {
    let book_dir = output_dir.join(asin);
    let pages_dir = book_dir.join("pages");
    let metadata_path = book_dir.join("metadata.json");

    std::fs::create_dir_all(&book_dir).map_err(|e| format!("mkdir book dir: {}", e))?;
    std::fs::create_dir_all(&pages_dir).map_err(|e| format!("mkdir pages dir: {}", e))?;

    if metadata_path.exists() && has_page_files(&pages_dir) {
        tracing::info!(
            target: "cns.pipeline.kindle-zip.extract",
            asin = %asin, "Resuming from existing extraction"
        );
        return load_existing(&metadata_path, &pages_dir);
    }

    tracing::info!(
        target: "cns.pipeline.kindle-zip.extract",
        asin = %asin, "Starting browser-based extraction"
    );

    let meta =
        extract_via_browser(asin, amazon_email, amazon_password, &book_dir, &pages_dir).await?;

    tracing::info!(
        target: "cns.pipeline.kindle-zip.extract",
        asin = %asin,
        total_pages = meta.total_pages,
        content_pages = meta.content_pages,
        title = %meta.title,
        "Extraction complete"
    );

    Ok(meta)
}

async fn extract_via_browser(
    asin: &str,
    amazon_email: &str,
    amazon_password: &str,
    book_dir: &Path,
    pages_dir: &Path,
) -> Result<ExtractResult, String> {
    use headless_chrome::{Browser, LaunchOptionsBuilder};

    let launch_opts = LaunchOptionsBuilder::default()
        .headless(true)
        .window_size(Some((1280, 720)))
        .sandbox(false)
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

    // Verify Kindle UI selectors before extraction (Guardrail — aborts on failure)
    verify_selectors(&tab)?;

    let (title, author) = scrape_title_author(&tab)?;
    let pages = capture_pages(&tab, pages_dir)?;

    let total_pages = pages.len();
    let title_c = title.clone();
    let author_c = author.clone();
    let toc: Vec<crate::kindle_zip::types::TocItem> = vec![]; // TOC populated from network responses in production

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
        tracing::warn!(
            target: "cns.pipeline.kindle-zip.extract",
            "2FA detected — waiting 30s for manual entry"
        );
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
    Ok(())
}

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

/// Verify that Kindle UI selectors resolve in the current DOM.
///
/// Guardrail: aborts extraction if any critical selector fails.
/// Suggests `--recalibrate` mode for auto-discovery of updated selectors.
fn verify_selectors(tab: &headless_chrome::Tab) -> Result<(), String> {
    // Selectors from manifest (mirrors kindle-zip.yaml selectors: section)
    const REQUIRED: &[(&str, &str)] = &[
        ("reader_content", "#kr-renderer .kg-full-page-img img"),
        ("next_page", ".kr-chevron-container-right"),
        ("page_footer", "ion-footer ion-title"),
    ];

    let mut failures: Vec<&str> = Vec::new();
    for &(name, selector) in REQUIRED {
        match tab.find_element(selector) {
            Ok(_) => {
                tracing::debug!(
                    target: "cns.pipeline.kindle-zip.selector",
                    selector = name, status = "ok"
                );
            }
            Err(_) => {
                failures.push(name);
                tracing::warn!(
                    target: "cns.pipeline.kindle-zip.selector",
                    selector = name,
                    css = selector,
                    "SELECTOR DRIFT DETECTED"
                );
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

fn capture_pages(tab: &headless_chrome::Tab, pages_dir: &Path) -> Result<Vec<PageEntry>, String> {
    // Navigate to first page and detect total page count
    tab.wait_for_element("#kr-renderer .kg-full-page-img img")
        .map_err(|e| format!("Reader content: {}", e))?;

    // Try to read page count from footer
    let footer_text = tab
        .evaluate(
            "document.querySelector('ion-footer ion-title')?.textContent || ''",
            false,
        )
        .ok()
        .and_then(|v| v.value)
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    // Parse "Page X of Y" or similar patterns
    let total = parse_page_count(&footer_text).unwrap_or(50); // fallback: 50 pages
    let padding = format!("{}", total * 2).len();
    let mut pages: Vec<PageEntry> = Vec::with_capacity(total);

    for i in 0..total {
        let page_num = i + 1;
        std::thread::sleep(std::time::Duration::from_millis(500));

        let data = tab
            .capture_screenshot(
                headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
                Some(90),
                None,
                true,
            )
            .map_err(|e| format!("Screenshot page {}: {}", page_num, e))?;

        let filename = format!("{}-{}.png", zeropad(i, padding), zeropad(page_num, padding));
        let path = pages_dir.join(&filename);
        std::fs::write(&path, &data).map_err(|e| format!("Write {}: {}", filename, e))?;

        pages.push(PageEntry {
            index: i,
            page: page_num,
            screenshot: path,
        });

        if i < total - 1 {
            let _ = tab
                .find_element(".kr-chevron-container-right")
                .and_then(|el| el.click().map(|_| ()));
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }
    Ok(pages)
}

/// Parse page count from Kindle footer text like "Page 5 of 342" or "5 / 342".
/// Requires at least 2 numbers to distinguish page-from-total patterns.
fn parse_page_count(footer: &str) -> Option<usize> {
    let numbers: Vec<usize> = footer
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse::<usize>().ok())
        .filter(|&n| n > 1 && n < 100_000)
        .collect();
    // Need at least 2 numbers: [current_page, total_pages]
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
    fn parse_page_count_single_number_ignored() {
        assert_eq!(parse_page_count("42"), None); // ambiguous: could be page or total
    }

    #[test]
    fn parse_page_count_empty() {
        assert_eq!(parse_page_count(""), None);
    }

    #[test]
    fn parse_page_count_no_numbers() {
        assert_eq!(parse_page_count("Hello World"), None);
    }
}
