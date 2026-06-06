//! Request validation and health error sanitization.

use crate::types::{
    BrowseRequest, ExtractRequest, MAX_INSTRUCTION_LENGTH, MAX_JSON_PROMPT_LENGTH,
    MAX_JSON_SCHEMA_BYTES, MAX_QUERY_LENGTH, MAX_URL_LENGTH, SearchRequest, WebError,
};

// --- Task 6: Compound provider timeout (shorter than client timeout) ---
pub const COMPOUND_PROVIDER_TIMEOUT_SECS: u64 = 10;

/// Sanitize a provider error to prevent credential leakage.
///
/// Replaces detailed error messages with generic categories and strips
/// any substrings that look like API keys (matching common prefix patterns).
/// Used in both `health_check_all()` and `search_compound()` to ensure
/// no credentials leak through CNS tracing or compound result metadata.
pub fn sanitize_health_error(error: &str) -> String {
    /// Lazily compiled API key regex pattern for sanitization.
    /// Avoids re-compiling the regex on every call to `sanitize_health_error`.
    static API_KEY_REGEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?:sk-|pk-|fc-|ts-|br-|xai-|ghp_)[a-zA-Z0-9]{8,}").unwrap()
    });

    let sanitized = API_KEY_REGEX.replace_all(error, "[REDACTED]").to_string();

    let lower = sanitized.to_lowercase();
    if lower.contains("401") || lower.contains("403") || lower.contains("auth") {
        "authentication failed".to_string()
    } else if lower.contains("429") || lower.contains("rate") {
        "rate limited".to_string()
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "timeout".to_string()
    } else if lower.contains("unreachable") || lower.contains("connection") || lower.contains("dns")
    {
        "unreachable".to_string()
    } else if lower.contains("no provider") {
        "no provider available".to_string()
    } else {
        "unhealthy".to_string()
    }
}

/// Validate a `SearchRequest` at the port boundary.
///
/// This is the authoritative enforcement point per the Cockburn principle:
/// the port defines the contract, not the adapter entry point.
pub fn validate_search_request(req: &SearchRequest) -> Result<(), WebError> {
    if req.query.is_empty() {
        return Err(WebError::BadArgs("query must not be empty".into()));
    }
    if req.query.len() > MAX_QUERY_LENGTH {
        return Err(WebError::BadArgs(format!(
            "query exceeds maximum length of {} characters",
            MAX_QUERY_LENGTH
        )));
    }
    Ok(())
}

/// Validate an `ExtractRequest` at the port boundary.
pub fn validate_extract_request(req: &ExtractRequest) -> Result<(), WebError> {
    if req.url.len() > MAX_URL_LENGTH {
        return Err(WebError::BadArgs(format!(
            "url exceeds maximum length of {} characters",
            MAX_URL_LENGTH
        )));
    }
    if let Some(ref prompt) = req.json_prompt
        && prompt.len() > MAX_JSON_PROMPT_LENGTH
    {
        return Err(WebError::BadArgs(format!(
            "json_prompt exceeds maximum length of {} characters",
            MAX_JSON_PROMPT_LENGTH
        )));
    }
    if let Some(ref schema) = req.json_schema
        && let Ok(bytes) = serde_json::to_string(schema)
        && bytes.len() > MAX_JSON_SCHEMA_BYTES
    {
        return Err(WebError::BadArgs(format!(
            "json_schema exceeds maximum size of {} bytes",
            MAX_JSON_SCHEMA_BYTES
        )));
    }
    Ok(())
}

/// Validate a `BrowseRequest` at the port boundary.
pub fn validate_browse_request(req: &BrowseRequest) -> Result<(), WebError> {
    if req.url.len() > MAX_URL_LENGTH {
        return Err(WebError::BadArgs(format!(
            "url exceeds maximum length of {} characters",
            MAX_URL_LENGTH
        )));
    }
    if let Some(ref instr) = req.instruction
        && instr.len() > MAX_INSTRUCTION_LENGTH
    {
        return Err(WebError::BadArgs(format!(
            "instruction exceeds maximum length of {} characters",
            MAX_INSTRUCTION_LENGTH
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_search_request ──────────────────────────────────────────

    // P8 invariant: empty query is rejected at the port boundary
    #[test]
    fn search_request_rejects_empty_query() {
        let req = SearchRequest {
            query: String::new(),
            num_results: None,
            include_domains: None,
            exclude_domains: None,
            freshness: None,
            strategy: None,
        };
        let err = validate_search_request(&req).unwrap_err();
        assert!(
            err.to_string().contains("empty"),
            "empty query must be rejected with 'empty' message, got: {err}"
        );
    }

    // P8 invariant: query exceeding MAX_QUERY_LENGTH is rejected
    #[test]
    fn search_request_rejects_oversized_query() {
        let req = SearchRequest {
            query: "x".repeat(MAX_QUERY_LENGTH + 1),
            num_results: None,
            include_domains: None,
            exclude_domains: None,
            freshness: None,
            strategy: None,
        };
        let err = validate_search_request(&req).unwrap_err();
        assert!(
            err.to_string().contains(&MAX_QUERY_LENGTH.to_string()),
            "oversized query must mention the limit, got: {err}"
        );
    }

    // P8 invariant: valid query passes validation
    #[test]
    fn search_request_accepts_valid_query() {
        let req = SearchRequest {
            query: "test query".to_string(),
            num_results: None,
            include_domains: None,
            exclude_domains: None,
            freshness: None,
            strategy: None,
        };
        assert!(
            validate_search_request(&req).is_ok(),
            "valid query must pass validation"
        );
    }

    // ── validate_extract_request ──────────────────────────────────────────

    // P8 invariant: URL exceeding MAX_URL_LENGTH is rejected
    #[test]
    fn extract_request_rejects_oversized_url() {
        let req = ExtractRequest {
            url: "x".repeat(MAX_URL_LENGTH + 1),
            format: None,
            json_prompt: None,
            json_schema: None,
            main_content_only: None,
            wait_for_ms: None,
        };
        let err = validate_extract_request(&req).unwrap_err();
        assert!(
            err.to_string().contains(&MAX_URL_LENGTH.to_string()),
            "oversized URL must mention the limit, got: {err}"
        );
    }

    // P8 invariant: json_prompt exceeding MAX_JSON_PROMPT_LENGTH is rejected
    #[test]
    fn extract_request_rejects_oversized_json_prompt() {
        let req = ExtractRequest {
            url: "https://example.com".to_string(),
            format: None,
            json_prompt: Some("x".repeat(MAX_JSON_PROMPT_LENGTH + 1)),
            json_schema: None,
            main_content_only: None,
            wait_for_ms: None,
        };
        let err = validate_extract_request(&req).unwrap_err();
        assert!(
            err.to_string()
                .contains(&MAX_JSON_PROMPT_LENGTH.to_string()),
            "oversized json_prompt must mention the limit, got: {err}"
        );
    }

    // P8 invariant: valid extract request passes
    #[test]
    fn extract_request_accepts_valid_request() {
        let req = ExtractRequest {
            url: "https://example.com".to_string(),
            format: None,
            json_prompt: None,
            json_schema: None,
            main_content_only: None,
            wait_for_ms: None,
        };
        assert!(
            validate_extract_request(&req).is_ok(),
            "valid extract request must pass validation"
        );
    }

    // ── validate_browse_request ───────────────────────────────────────────

    // P8 invariant: URL exceeding MAX_URL_LENGTH is rejected
    #[test]
    fn browse_request_rejects_oversized_url() {
        let req = BrowseRequest {
            url: "x".repeat(MAX_URL_LENGTH + 1),
            instruction: None,
            timeout_secs: None,
        };
        let err = validate_browse_request(&req).unwrap_err();
        assert!(
            err.to_string().contains(&MAX_URL_LENGTH.to_string()),
            "oversized URL must mention the limit, got: {err}"
        );
    }

    // P8 invariant: instruction exceeding MAX_INSTRUCTION_LENGTH is rejected
    #[test]
    fn browse_request_rejects_oversized_instruction() {
        let req = BrowseRequest {
            url: "https://example.com".to_string(),
            instruction: Some("x".repeat(MAX_INSTRUCTION_LENGTH + 1)),
            timeout_secs: None,
        };
        let err = validate_browse_request(&req).unwrap_err();
        assert!(
            err.to_string()
                .contains(&MAX_INSTRUCTION_LENGTH.to_string()),
            "oversized instruction must mention the limit, got: {err}"
        );
    }

    // P8 invariant: valid browse request passes
    #[test]
    fn browse_request_accepts_valid_request() {
        let req = BrowseRequest {
            url: "https://example.com".to_string(),
            instruction: None,
            timeout_secs: None,
        };
        assert!(
            validate_browse_request(&req).is_ok(),
            "valid browse request must pass validation"
        );
    }

    // ── sanitize_health_error ─────────────────────────────────────────────

    // P8 invariant: API keys with common prefixes are stripped
    #[test]
    fn sanitize_strips_sk_prefix_keys() {
        let input = "Authorization failed: sk-abc123def456ghi789key";
        let output = sanitize_health_error(input);
        assert!(
            !output.contains("sk-abc123def456ghi789key"),
            "sk- prefixed keys must be redacted"
        );
        assert!(
            output.contains("[REDACTED]"),
            "redacted key must show [REDACTED]"
        );
    }

    // P8 invariant: pk-, fc-, ts-, br-, xai-, ghp_ prefixes are also stripped
    #[test]
    fn sanitize_strips_all_key_prefixes() {
        for prefix in ["pk-", "fc-", "ts-", "br-", "xai-", "ghp_"] {
            let key = format!("{prefix}abcdefgh1234");
            let input = format!("Error: {key}");
            let output = sanitize_health_error(&input);
            assert!(
                !output.contains(&key),
                "key with {prefix} prefix must be redacted"
            );
        }
    }

    // P8 invariant: 401/403/auth → "authentication failed"
    #[test]
    fn sanitize_maps_401_to_auth_failed() {
        assert_eq!(
            sanitize_health_error("Request returned 401"),
            "authentication failed"
        );
        assert_eq!(
            sanitize_health_error("Request returned 403 Forbidden"),
            "authentication failed"
        );
        assert_eq!(
            sanitize_health_error("auth token expired"),
            "authentication failed"
        );
    }

    // P8 invariant: 429/rate → "rate limited"
    #[test]
    fn sanitize_maps_429_to_rate_limited() {
        assert_eq!(
            sanitize_health_error("Request returned 429"),
            "rate limited"
        );
        assert_eq!(sanitize_health_error("rate limit exceeded"), "rate limited");
    }

    // P8 invariant: timeout → "timeout"
    #[test]
    fn sanitize_maps_timeout() {
        assert_eq!(sanitize_health_error("connection timed out"), "timeout");
        assert_eq!(
            sanitize_health_error("request timeout after 30s"),
            "timeout"
        );
    }

    // P8 invariant: unreachable/connection/dns → "unreachable"
    #[test]
    fn sanitize_maps_unreachable() {
        assert_eq!(sanitize_health_error("host unreachable"), "unreachable");
        assert_eq!(sanitize_health_error("connection refused"), "unreachable");
        assert_eq!(
            sanitize_health_error("DNS resolution failed"),
            "unreachable"
        );
    }

    // P8 invariant: "no provider" → "no provider available"
    #[test]
    fn sanitize_maps_no_provider() {
        assert_eq!(
            sanitize_health_error("no provider configured"),
            "no provider available"
        );
    }

    // P8 invariant: everything else → "unhealthy"
    #[test]
    fn sanitize_maps_unknown_to_unhealthy() {
        assert_eq!(sanitize_health_error("something went wrong"), "unhealthy");
    }
}
