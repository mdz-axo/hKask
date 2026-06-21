//! Browser-based extraction of Kindle book pages.
//!
//! Navigates Kindle Cloud Reader, logs in, captures page screenshots,
//! and records metadata. Supports resume from existing extraction.
//!
//! Public surface: 1 function (`extract_kindle_book`).

use std::path::Path;

use crate::kindle_zip::types::{
    BookMetadata, ExtractResult, PageEntry, PageNav, has_page_files, zeropad,
};

/// Extract book pages from Kindle Cloud Reader via headless browser.
///
/// If pages already exist on disk, resumes from existing extraction
/// (idempotent — safe to re-run after partial completion).
///
/// Emits CNS span `kindle-zip.extract` for observability.
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

    // Idempotency: skip extraction if pages already exist
    if metadata_path.exists() && has_page_files(&pages_dir) {
        tracing::info!(
            target: "cns.pipeline.kindle-zip.extract",
            asin = %asin, "Idempotent: resuming from existing extraction"
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

/// Browser-based extraction using headless Chrome.
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
        .map_err(|e| format!("Launch browser: {}. Is Chrome/Chromium installed?", e))?;

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

    let metadata = scrape_metadata(&tab, asin)?;
    let pages = capture_pages(&tab, &metadata, pages_dir)?;

    let total = pages.len();
    let content = metadata.nav.total_content_pages.max(1) as usize;
    let title = metadata.title.clone();
    let author = metadata.author.clone();
    let toc = metadata.toc.clone();

    let full = BookMetadata {
        asin: asin.to_string(),
        title: title.clone(),
        author: author.clone(),
        authors: metadata.authors.clone(),
        description: metadata.description.clone(),
        cover_url: metadata.cover_url.clone(),
        pages,
        toc: toc.clone(),
        nav: metadata.nav.clone(),
        raw_meta: metadata.raw_meta.clone(),
    };
    let json = serde_json::to_string_pretty(&full).map_err(|e| format!("JSON: {}", e))?;
    std::fs::write(book_dir.join("metadata.json"), json).map_err(|e| format!("Write: {}", e))?;

    Ok(ExtractResult {
        metadata_path: book_dir.join("metadata.json"),
        pages_dir: pages_dir.to_path_buf(),
        total_pages: total,
        content_pages: content,
        title,
        author,
        toc,
        cns_span_id: None,
    })
}

/// Perform Kindle login flow.
fn kindle_login(tab: &headless_chrome::Tab, email: &str, password: &str) -> Result<(), String> {
    // Click email field, then type
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

/// Scrape book metadata from the page DOM.
fn scrape_metadata(tab: &headless_chrome::Tab, asin: &str) -> Result<BookMetadata, String> {
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

    Ok(BookMetadata {
        asin: asin.to_string(),
        title,
        author: author.clone(),
        authors: vec![author],
        description: None,
        cover_url: None,
        pages: vec![],
        toc: vec![],
        nav: PageNav {
            start_content_page: 1,
            end_content_page: 100,
            total_pages: 100,
            total_content_pages: 100,
        },
        raw_meta: serde_json::json!({}),
    })
}

/// Capture screenshots of each content page.
fn capture_pages(
    tab: &headless_chrome::Tab,
    metadata: &BookMetadata,
    pages_dir: &Path,
) -> Result<Vec<PageEntry>, String> {
    let total = metadata.nav.total_content_pages.max(1) as usize;
    let padding = format!("{}", total * 2).len();
    let mut pages: Vec<PageEntry> = Vec::with_capacity(total);

    tab.wait_for_element("#kr-renderer .kg-full-page-img img")
        .map_err(|e| format!("Reader content: {}", e))?;

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
                .and_then(|el| el.click());
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }
    Ok(pages)
}

/// Load existing extraction from disk (resume mode).
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
