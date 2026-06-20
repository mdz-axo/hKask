//! Kindle Cloud Reader automation — discovers books, captures pages, assembles PDFs.
//!
//! Uses Chrome DevTools Protocol to drive the user's local Chrome browser.
//! Based on patterns from:
//!   - transitive-bullshit/kindle-ai-export (Playwright, blob interception, selectors)
//!   - Xetera/kindle-api (ASIN URL pattern, cookie auth)

pub mod assembly;
pub mod capture;
pub mod discovery;

/// A captured page from the Kindle reader.
pub struct CapturedPage {
    /// 1-based page number.
    pub number: usize,
    /// PNG image bytes.
    pub png_bytes: Vec<u8>,
}

/// Selectors validated against Kindle Cloud Reader as of 2026.
pub mod selectors {
    /// The rendered page image inside the Kindle reader viewport.
    pub const RENDERED_IMAGE: &str = "#kr-renderer .kg-full-page-img img";
    /// Right-side chevron button to advance to the next page.
    pub const NEXT_PAGE_CHEVRON: &str = ".kr-chevron-container-right";
    /// Reader settings button.
    pub const SETTINGS_BUTTON: &str = "ion-button[aria-label=\"Reader settings\"]";
    /// Footer element showing page number.
    pub const PAGE_FOOTER: &str = "ion-footer ion-title";
}
