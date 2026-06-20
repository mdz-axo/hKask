//! Book discovery — find a book's ASIN by title in the Kindle Cloud Reader library.
//!
//! Uses CDP `Runtime.evaluate` to walk the library DOM looking for book links
//! whose text matches the given title, then extracts the ASIN from the link's href.

use crate::ChromeCdpClient;

/// Find a book's ASIN by searching the Kindle library DOM for matching title text.
///
/// The Chrome tab must already be on `https://read.amazon.com/kindle-library`.
/// Returns `None` if no matching book is found.
pub async fn find_asin_by_title(
    chrome: &mut ChromeCdpClient,
    title: &str,
) -> Result<Option<String>, String> {
    let escaped = title
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "\\\"");

    let js = format!(
        "(function() {{\
            var t = '{escaped}'.toLowerCase();\
            var links = document.querySelectorAll('a[href*=\"asin=\"]');\
            for (var i = 0; i < links.length; i++) {{\
                if ((links[i].textContent || '').toLowerCase().indexOf(t) >= 0) {{\
                    var m = links[i].href.match(/asin=([A-Z0-9]+)/);\
                    if (m) return m[1];\
                }}\
            }}\
            var imgs = document.querySelectorAll('img[alt]');\
            for (var i = 0; i < imgs.length; i++) {{\
                if ((imgs[i].alt || '').toLowerCase().indexOf(t) >= 0) {{\
                    var a = imgs[i].closest('a[href*=\"asin=\"]');\
                    if (a) {{\
                        var m = a.href.match(/asin=([A-Z0-9]+)/);\
                        if (m) return m[1];\
                    }}\
                }}\
            }}\
            return '';\
        }})()"
    );

    let result = chrome.evaluate(&js).await?;
    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}
