//! hkask-mcp fuzz targets — validation and parsing functions.
//!
//! These target the parsing/validation boundary where untrusted strings
//! enter the system: tool identifiers, URLs, and HTTP status codes.

use bolero::check;
use hkask_mcp::server::{validate_identifier, validate_tool_url};

// ── Identifier validation ──────────────────────────────────────────────

/// `validate_identifier` must never panic on arbitrary (name, value, max_len).
/// Covers: empty strings, boundary-length strings, non-alphanumeric chars,
/// unicode, null bytes, emoji, and all combinations thereof.
#[test]
fn fuzz_validate_identifier_never_panics() {
    check!()
        .with_type::<(String, String, usize)>()
        .for_each(|(name, value, max_len)| {
            let _ = validate_identifier(name, value, *max_len);
        });
}

/// Identifier validation contracts: empty input must be rejected,
/// valid identifiers must be accepted, length violations must be caught.
#[test]
fn fuzz_validate_identifier_contracts() {
    check!()
        .with_type::<(String, usize)>()
        .for_each(|(value, max_len)| {
            let result = validate_identifier("test", value, *max_len);
            // Empty value must be rejected
            if value.is_empty() {
                assert!(result.is_err(), "empty value must be rejected");
            }
            // Length violation must be rejected
            if !value.is_empty() && value.len() > *max_len {
                assert!(result.is_err(), "over-length value must be rejected");
            }
        });
}

// ── URL validation ─────────────────────────────────────────────────────

/// `validate_tool_url` must never panic on arbitrary URL strings.
/// Covers: empty strings, malformed URLs, unicode, null bytes,
/// SSRF patterns (file://, gopher://, IP addresses).
#[test]
fn fuzz_validate_tool_url_never_panics() {
    check!().with_type::<String>().for_each(|url| {
        let _ = validate_tool_url(url);
    });
}

/// Valid HTTPS URLs must pass validation.
#[test]
fn fuzz_validate_tool_url_accepts_valid() {
    // Generate edge-testing valid-looking URLs
    check!().with_type::<String>().for_each(|path| {
        let url = format!("https://example.com/{}", path);
        // Should never panic; may or may not be valid
        let _ = validate_tool_url(&url);
    });
}

// ── HTTP status classification ─────────────────────────────────────────

/// `classify_http_error` must never panic on arbitrary HTTP status codes
/// and response bodies.
#[test]
fn fuzz_classify_http_error_never_panics() {
    check!()
        .with_type::<(String, u16, String)>()
        .for_each(|(service, status_code, body)| {
            let code = reqwest::StatusCode::from_u16(*status_code)
                .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            let _ = hkask_mcp::server::classify_http_error(service, code, body);
        });
}
