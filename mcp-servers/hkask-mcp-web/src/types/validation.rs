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

    #[test]
    fn sanitize_health_error_strips_api_keys() {
        // When the input contains an API key pattern, it gets replaced with [REDACTED]
        // before categorization. The function first strips keys, then categorizes.
        // Input with key but no auth/rate/timeout keywords → categorized as "unhealthy"
        let result = sanitize_health_error("Provider failed with sk-abc123def456");
        // The key is stripped; the result is a generic category string
        assert!(!result.contains("sk-abc123def456"));
        // Input that triggers auth categorization also strips the key first
        let result2 = sanitize_health_error("Auth failed: sk-abc123def456");
        assert!(!result2.contains("sk-abc123def456"));
        assert_eq!(result2, "authentication failed");
    }

    #[test]
    fn sanitize_health_error_categorizes_auth() {
        assert_eq!(
            sanitize_health_error("Got 401 unauthorized"),
            "authentication failed"
        );
        assert_eq!(
            sanitize_health_error("Got 403 forbidden"),
            "authentication failed"
        );
        assert_eq!(
            sanitize_health_error("Auth error occurred"),
            "authentication failed"
        );
    }

    #[test]
    fn sanitize_health_error_categorizes_rate_limit() {
        assert_eq!(sanitize_health_error("Got 429 rate limit"), "rate limited");
        assert_eq!(
            sanitize_health_error("Rate limited by provider"),
            "rate limited"
        );
    }

    #[test]
    fn sanitize_health_error_categorizes_timeout() {
        assert_eq!(sanitize_health_error("Connection timed out"), "timeout");
        assert_eq!(
            sanitize_health_error("Request timeout after 30s"),
            "timeout"
        );
    }

    #[test]
    fn sanitize_health_error_categorizes_unreachable() {
        assert_eq!(sanitize_health_error("Host unreachable"), "unreachable");
        assert_eq!(
            sanitize_health_error("DNS connection refused"),
            "unreachable"
        );
    }

    #[test]
    fn sanitize_health_error_defaults_to_unhealthy() {
        assert_eq!(sanitize_health_error("Some random error"), "unhealthy");
    }

    #[test]
    fn compound_provider_timeout_constant() {
        assert_eq!(COMPOUND_PROVIDER_TIMEOUT_SECS, 10);
    }

    #[test]
    fn validate_search_request_rejects_empty() {
        let req = SearchRequest {
            query: "".into(),
            num_results: Some(10),
            include_domains: None,
            exclude_domains: None,
            freshness: None,
            strategy: None,
        };
        assert!(validate_search_request(&req).is_err());
    }

    #[test]
    fn validate_search_request_accepts_valid() {
        let req = SearchRequest {
            query: "test query".into(),
            num_results: Some(10),
            include_domains: None,
            exclude_domains: None,
            freshness: None,
            strategy: None,
        };
        assert!(validate_search_request(&req).is_ok());
    }

    #[test]
    fn validate_search_request_rejects_too_long() {
        let req = SearchRequest {
            query: "a".repeat(MAX_QUERY_LENGTH + 1),
            num_results: Some(10),
            include_domains: None,
            exclude_domains: None,
            freshness: None,
            strategy: None,
        };
        assert!(validate_search_request(&req).is_err());
    }

    #[test]
    fn validate_extract_request_rejects_long_url() {
        let req = ExtractRequest {
            url: "a".repeat(MAX_URL_LENGTH + 1),
            format: Some("markdown".into()),
            json_prompt: None,
            json_schema: None,
            main_content_only: None,
            wait_for_ms: None,
        };
        assert!(validate_extract_request(&req).is_err());
    }

    #[test]
    fn validate_browse_request_rejects_long_url() {
        let req = BrowseRequest {
            url: "a".repeat(MAX_URL_LENGTH + 1),
            instruction: None,
            timeout_secs: None,
        };
        assert!(validate_browse_request(&req).is_err());
    }
}
