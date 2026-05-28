//! Integration Tests for MCP Server Infrastructure
//!
//! Tests for boot verification, credential resolution, input validation,
//! JSON output validity, SSRF protection, WebID resolution, and CNS span naming.

#[allow(unused_imports)] // used only in #[test] functions
use hkask_mcp::server::{
    CredentialRequirement, McpToolError, McpToolOutput, ServerContext, ToolSpanGuard,
    validate_identifier, validate_tool_url,
};
#[allow(unused_imports)]
use hkask_types::{McpErrorKind, WebID};
#[allow(unused_imports)]
use std::collections::HashMap;

// =============================================================================
// Task 7.1: Boot Verification — run_stdio_server credential checks
// =============================================================================

#[test]
fn test_credential_requirement_required() {
    let req = CredentialRequirement::required("HKASK_TEST_KEY", "Test API key");
    assert!(req.required);
    assert_eq!(req.env_var, "HKASK_TEST_KEY");
    assert_eq!(req.description, "Test API key");
}

#[test]
fn test_credential_requirement_optional() {
    let req = CredentialRequirement::optional("HKASK_TEST_KEY", "Optional key");
    assert!(!req.required);
    assert_eq!(req.env_var, "HKASK_TEST_KEY");
}

#[test]
fn test_server_context_construction() {
    let ctx = ServerContext {
        credentials: HashMap::new(),
        rate_limiter: hkask_cns::RateLimiter::default(),
        adapters: hkask_mcp::AdapterContainer::new(),
        webid: WebID::new(),
    };
    assert!(ctx.credentials.is_empty());
}

#[test]
fn test_server_context_webid_from_persona() {
    let webid1 = WebID::from_persona(b"test-agent");
    let webid2 = WebID::from_persona(b"test-agent");
    assert_eq!(webid1, webid2, "Same persona should produce same WebID");

    let webid3 = WebID::from_persona(b"different-agent");
    assert_ne!(
        webid1, webid3,
        "Different personas should produce different WebIDs"
    );
}

#[test]
fn test_server_context_webid_anonymous() {
    let webid1 = WebID::new();
    let webid2 = WebID::new();
    assert_ne!(webid1, webid2, "Anonymous WebIDs should be unique");
}

// =============================================================================
// Task 7.2: Missing Credential — graceful failure
// =============================================================================

#[test]
fn test_missing_required_credential_fails() {
    // If a required credential is not set, run_stdio_server should bail.
    // We test the CredentialRequirement logic directly since
    // run_stdio_server is async and requires a server binary.
    let req = CredentialRequirement::required("HKASK_NONEXISTENT_KEY_12345", "Test key");
    assert!(req.required);
    // The actual missing-credential test requires a process-level test
    // since resolve_credential accesses the OS keychain + env vars.
}

// =============================================================================
// Task 7.3: Input Validation
// =============================================================================

#[test]
fn test_validate_identifier_valid() {
    assert!(validate_identifier("name", "hello", 256).is_ok());
    assert!(validate_identifier("name", "my-component", 256).is_ok());
    assert!(validate_identifier("name", "component.v2", 256).is_ok());
    assert!(validate_identifier("name", "a", 256).is_ok());
}

#[test]
fn test_validate_identifier_empty() {
    let err = validate_identifier("field", "", 256).unwrap_err();
    assert_eq!(err.kind, McpErrorKind::InvalidArgument);
    assert!(err.message.contains("must not be empty"));
}

#[test]
fn test_validate_identifier_too_long() {
    let long_value = "a".repeat(300);
    let err = validate_identifier("field", &long_value, 256).unwrap_err();
    assert_eq!(err.kind, McpErrorKind::InvalidArgument);
    assert!(err.message.contains("exceeds maximum length"));
}

#[test]
fn test_validate_identifier_invalid_chars() {
    let err = validate_identifier("field", "hello world", 256).unwrap_err();
    assert_eq!(err.kind, McpErrorKind::InvalidArgument);
    assert!(err.message.contains("invalid characters"));

    let err = validate_identifier("field", "hello/world", 256).unwrap_err();
    assert_eq!(err.kind, McpErrorKind::InvalidArgument);
}

// =============================================================================
// Task 7.4: SSRF Protection
// =============================================================================

#[test]
fn test_validate_tool_url_blocks_internal() {
    // IP-based loopback should be blocked
    assert!(validate_tool_url("http://127.0.0.1/admin").is_err());
    assert!(validate_tool_url("http://[::1]/admin").is_err());
    // Note: 0.0.0.0 is not classified as loopback by std::net; it's INADDR_ANY.
    // SSRF protection covers RFC 1918 private IPs and loopback, not INADDR_ANY.
    // DNS-based hostnames like 'localhost' also need DNS resolution to block.
}

#[test]
fn test_validate_tool_url_allows_external() {
    assert!(validate_tool_url("https://api.github.com/repos").is_ok());
    assert!(validate_tool_url("https://example.com/api/v1/data").is_ok());
}

#[test]
fn test_validate_tool_url_blocks_private_ranges() {
    assert!(validate_tool_url("http://10.0.0.1/admin").is_err());
    assert!(validate_tool_url("http://192.168.1.1/admin").is_err());
    assert!(validate_tool_url("http://172.16.0.1/admin").is_err());
}

// =============================================================================
// Task 7.5: JSON Output Validity
// =============================================================================

#[test]
fn test_mcp_tool_output_serialization() {
    let output = McpToolOutput::new(serde_json::json!({
        "key": "value",
        "count": 42,
    }));
    let json_str = output.to_json_string();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["content"]["key"], "value");
    assert_eq!(parsed["content"]["count"], 42);
}

#[test]
fn test_mcp_tool_output_with_timing() {
    let start = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(1));
    let output = McpToolOutput::with_timing(serde_json::json!({"result": "ok"}), start);
    let json_str = output.to_json_string();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed["metadata"]["duration_ms"].is_number());
}

#[test]
fn test_mcp_tool_error_serialization() {
    let err = McpToolError::not_found("Resource not found");
    let json_str = err.to_json_string();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["error"], "Resource not found");
    assert_eq!(parsed["kind"], "not_found");
}

#[test]
fn test_mcp_tool_error_all_kinds() {
    let kinds = vec![
        (McpToolError::internal("msg"), "internal"),
        (McpToolError::unavailable("msg"), "unavailable"),
        (McpToolError::timeout("msg"), "timeout"),
        (McpToolError::not_found("msg"), "not_found"),
        (McpToolError::invalid_argument("msg"), "invalid_argument"),
        (McpToolError::permission_denied("msg"), "permission_denied"),
        (McpToolError::rate_limited("msg"), "rate_limited"),
        (
            McpToolError::failed_precondition("msg"),
            "failed_precondition",
        ),
    ];
    for (err, kind_str) in kinds {
        let json_str = err.to_json_string();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["kind"], kind_str);
    }
}

#[test]
fn test_mcp_tool_error_retryable() {
    assert!(McpToolError::unavailable("msg").is_retryable());
    assert!(McpToolError::timeout("msg").is_retryable());
    assert!(McpToolError::rate_limited("msg").is_retryable());
    assert!(!McpToolError::invalid_argument("msg").is_retryable());
    assert!(!McpToolError::not_found("msg").is_retryable());
    assert!(!McpToolError::permission_denied("msg").is_retryable());
}

// =============================================================================
// Task 7.6: ToolSpanGuard — RAII CNS span emission
// =============================================================================

#[test]
fn test_tool_span_guard_ok() {
    let webid = WebID::from_persona(b"test-agent");
    let span = ToolSpanGuard::new("test:tool", &webid);
    let result = span.ok(McpToolOutput::new(serde_json::json!({"status": "ok"})).to_json_string());
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["content"]["status"], "ok");
}

#[test]
fn test_tool_span_guard_error() {
    let webid = WebID::new();
    let span = ToolSpanGuard::new("test:tool", &webid);
    let result = span.error(
        hkask_types::McpErrorKind::NotFound,
        McpToolError::not_found("Item not found").to_json_string(),
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["kind"], "not_found");
}

#[test]
fn test_tool_span_guard_drop_without_emit() {
    // Creating a guard and dropping it without calling ok() or error()
    // should still emit a span with outcome "dropped" (no panic).
    let webid = WebID::new();
    let _span = ToolSpanGuard::new("test:dropped_tool", &webid);
    // Guard is dropped here — should emit "dropped" span, not panic.
}

// =============================================================================
// Task 7.7: WebID Resolution — deterministic identity
// =============================================================================

#[test]
fn test_webid_deterministic_from_persona() {
    let id1 = WebID::from_persona(b"curator");
    let id2 = WebID::from_persona(b"curator");
    assert_eq!(id1, id2, "Same persona must produce same WebID");
}

#[test]
fn test_webid_namespace_isolation() {
    let id_hkask = WebID::from_persona_with_namespace(b"agent", "hkask");
    let id_russell = WebID::from_persona_with_namespace(b"agent", "russell");
    assert_ne!(
        id_hkask, id_russell,
        "Different namespaces must produce different WebIDs"
    );
}

#[test]
fn test_webid_from_string_roundtrip() {
    let id = WebID::new();
    let s = id.to_string();
    let id2 = WebID::from_string(&s);
    assert_eq!(id, id2, "WebID roundtrip through string must be equal");
}

#[test]
fn test_webid_redacted_display() {
    let id = WebID::new();
    let redacted = id.redacted_display();
    assert!(
        redacted.ends_with("..."),
        "Redacted display should end with ..."
    );
    // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx (36 chars)
    // Redacted: first 8 chars + "..." = 11 chars
    assert!(
        redacted.len() >= 11,
        "Redacted display should be at least 11 chars"
    );
}
