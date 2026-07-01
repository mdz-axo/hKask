//! Input validation — shared sanitization for MCP tool parameters.

use super::error::McpToolError;

/// Validate a string identifier.
/// Validate an identifier (tool name, server name, etc.).
///
/// pre:  name and value are non-empty, max_len > 0
/// post: returns Ok(()) if valid (non-empty, ≤max_len, alphanumeric+hyphen+underscore)
/// post: returns Err if invalid
#[must_use = "result must be used"]
pub fn validate_identifier(name: &str, value: &str, max_len: usize) -> Result<(), McpToolError> {
    if value.is_empty() {
        return Err(McpToolError::invalid_argument(format!(
            "{name} must not be empty"
        )));
    }
    if value.len() > max_len {
        return Err(McpToolError::invalid_argument(format!(
            "{name} exceeds maximum length of {max_len} (got {})",
            value.len()
        )));
    }
    if !value
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
    {
        return Err(McpToolError::invalid_argument(format!(
            "{name} contains invalid characters (allowed: alphanumeric, _, ., -)"
        )));
    }
    Ok(())
}

/// Validate a URL parameter against SSRF protection rules.
///
/// Delegates to `hkask_mcp::validate_url()` with the default (strict) config.
/// Use this for any tool that accepts a user-provided URL.
/// Validate a tool URL (http/https only, no path traversal).
///
/// pre:  url is non-empty
/// post: returns Ok(()) if valid http/https URL
/// post: returns Err if invalid scheme or format
#[must_use = "result must be used"]
pub fn validate_tool_url(url: &str) -> Result<(), McpToolError> {
    crate::security::validate_url(url, &crate::security::UrlValidationConfig::default())
        .map_err(|e| McpToolError::invalid_argument(format!("URL validation failed: {e}")))
}
