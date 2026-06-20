//! Page capture — opens the Kindle reader, intercepts rendered page images,
//! and navigates through the book.
//!
//! Based on the blob interception technique from transitive-bullshit/kindle-ai-export:
//! We override `URL.createObjectURL` in the page context to capture the high-resolution
//! PNG images that Kindle's renderer produces, rather than taking viewport screenshots.

use crate::ChromeCdpClient;
use crate::kindle::selectors;
use std::collections::HashMap;

/// Open the Kindle reader for a given ASIN and prepare it for capture.
///
/// Navigates to `https://read.amazon.com/?asin={asin}`, waits for the reader
/// to load, and optionally sets single-column layout.
pub async fn open_reader(chrome: &mut ChromeCdpClient, asin: &str) -> Result<(), String> {
    chrome
        .evaluate(&format!(
            "window.location.href = 'https://read.amazon.com/?asin={asin}'"
        ))
        .await?;
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    Ok(())
}

/// Inject the blob interception script into the page.
///
/// Overrides `URL.createObjectURL` to capture PNG blobs as they're created by
/// Kindle's renderer. Captured blobs are sent back via CDP binding.
pub async fn inject_blob_intercept(chrome: &mut ChromeCdpClient) -> Result<(), String> {
    // Register a CDP binding that the page JS will call with captured blob data.
    // The binding name must match what the injected script calls.
    chrome
        .send_command(
            "Runtime.addBinding",
            serde_json::json!({ "name": "captureKindleBlob" }),
        )
        .await?;

    // Inject the override script to run on every page load.
    let init_script = r#"
        (function() {
            var _orig = URL.createObjectURL.bind(URL);
            URL.createObjectURL = function(blob) {
                var url = _orig(blob);
                if (blob.type === 'image/png' || blob.type === 'image/webp') {
                    (async function() {
                        var buf = await blob.arrayBuffer();
                        var bytes = new Uint8Array(buf);
                        var binary = '';
                        for (var i = 0; i < bytes.length; i++) {
                            binary += String.fromCharCode(bytes[i]);
                        }
                        var b64 = btoa(binary);
                        captureKindleBlob(JSON.stringify({url: url, type: blob.type, base64: b64}));
                    })();
                }
                return url;
            };
        })()
    "#;

    chrome
        .send_command(
            "Page.addScriptToEvaluateOnNewDocument",
            serde_json::json!({
                "source": init_script,
                "worldName": "kindle-capture",
            }),
        )
        .await?;

    Ok(())
}

/// Blob capture state — collects base64-encoded blobs as they arrive
/// from the page via CDP binding events.
pub struct BlobCapture {
    /// Blobs collected so far, keyed by their blob URL.
    pub blobs: HashMap<String, CapturedBlob>,
}

pub struct CapturedBlob {
    pub mime_type: String,
    pub base64: String,
}

impl BlobCapture {
    pub fn new() -> Self {
        Self {
            blobs: HashMap::new(),
        }
    }
}

impl Default for BlobCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl BlobCapture {
    /// Process a CDP `Runtime.bindingCalled` event payload.
    /// Extracts the captured blob data if it matches the expected binding name.
    pub fn handle_binding(&mut self, name: &str, payload: &str) {
        if name != "captureKindleBlob" {
            return;
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(payload) {
            let url = parsed.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let mime = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let b64 = parsed.get("base64").and_then(|v| v.as_str()).unwrap_or("");
            self.blobs.insert(
                url.to_string(),
                CapturedBlob {
                    mime_type: mime.to_string(),
                    base64: b64.to_string(),
                },
            );
        }
    }

    /// Decode a captured blob by its URL into raw PNG bytes.
    pub fn decode_blob(&self, url: &str) -> Option<Vec<u8>> {
        let blob = self.blobs.get(url)?;
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &blob.base64).ok()
    }
}

/// Click the next-page chevron and wait for the rendered image to change.
///
/// Returns `true` if navigation succeeded (image src changed), `false` if
/// the image didn't change after retries (likely end of book).
pub async fn next_page(chrome: &mut ChromeCdpClient) -> Result<bool, String> {
    // Get current image src before clicking
    let get_src = format!(
        "document.querySelector('{}')?.getAttribute('src') || ''",
        selectors::RENDERED_IMAGE
    );
    let before = chrome.evaluate(&get_src).await?;

    // Click the next-page chevron
    chrome
        .evaluate(&format!(
            "var el = document.querySelector('{}'); if (el) {{ el.click(); return 'clicked'; }} return 'no-chevron';",
            selectors::NEXT_PAGE_CHEVRON
        ))
        .await?;

    // Wait for the image src to change (or timeout)
    for _ in 0..50 {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let after = chrome.evaluate(&get_src).await?;
        if after != before && !after.is_empty() {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Capture the current page as PNG bytes.
///
/// Tries blob interception first (for full resolution), falls back to
/// CDP screenshot if no blob is available.
pub async fn capture_page(
    chrome: &mut ChromeCdpClient,
    capture: &BlobCapture,
) -> Result<Vec<u8>, String> {
    // Get the current rendered image's blob URL
    let src = chrome
        .evaluate(&format!(
            "document.querySelector('{}')?.getAttribute('src') || ''",
            selectors::RENDERED_IMAGE
        ))
        .await?;

    // Try blob interception first
    if let Some(png) = capture.decode_blob(&src) {
        return Ok(png);
    }

    // Fall back to CDP screenshot
    chrome.capture_screenshot().await
}
