//! Adapter unit tests migrated from inline tests
//!
//! Tests for: cns_emitter, git_cas, keystore_port, mcp_runtime, memory_storage
//! Note: ACP runtime tests moved to crates/hkask-agents/src/acp.rs

use hkask_agents::adapters::{
    cns_emitter::{self, CnsEmitterAdapter},
    git_cas::{GitCasAdapter, MockGitCas},
    mcp_runtime::McpRuntimeAdapter,
    memory_storage::MemoryStorageAdapter,
};
use hkask_agents::pod::{
    CNSSpanPort, GitCASPort, MCPRuntimePort, MemoryStoragePort,
};
use hkask_types::{CapabilityAction, CapabilityResource, CapabilityToken, WebID};
use serde_json::json;
use std::fs;
use std::path::Path;

// ============================================================================
// CNS Emitter Adapter Tests
// ============================================================================

#[test]
fn test_cns_emitter_adapter_new() {
    let webid = WebID::new();
    let _adapter = CnsEmitterAdapter::new(webid);
    assert!(true);
}

#[test]
fn test_cns_emitter_emit_event() {
    let webid = WebID::new();
    let adapter = CnsEmitterAdapter::new(webid);

    let observation = json!({"test": "event"});
    adapter.emit_event("cns.agent_pod.test", "observe", &observation, 1.0);

    // CNS event emitted (no return value to check)
    assert!(true);
}

// ============================================================================
// Git CAS Adapter Tests
// ============================================================================

#[test]
fn test_git_cas_adapter_new() {
    let temp_dir = std::env::temp_dir().join("hkask_git_test");
    fs::create_dir_all(&temp_dir).unwrap();

    let adapter = GitCasAdapter::new(&temp_dir);
    assert!(adapter.is_ok());

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_git_cas_adapter_nonexistent_path() {
    let nonexistent = std::path::PathBuf::from("/nonexistent/path/that/does/not/exist");
    let result = GitCasAdapter::new(&nonexistent);
    assert!(result.is_err());
}

#[test]
fn test_validate_path_traversal() {
    let temp_dir = std::env::temp_dir().join("hkask_git_test_validate");
    fs::create_dir_all(&temp_dir).unwrap();

    let adapter = GitCasAdapter::new(&temp_dir).unwrap();

    // Test parent directory traversal
    let result = adapter.validate_path(Path::new("../etc/passwd"));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Parent directory traversal"));

    // Test absolute path
    let result = adapter.validate_path(Path::new("/etc/passwd"));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Absolute paths"));

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_validate_valid_path() {
    let temp_dir = std::env::temp_dir().join("hkask_git_test_valid");
    fs::create_dir_all(&temp_dir).unwrap();

    let adapter = GitCasAdapter::new(&temp_dir).unwrap();

    // Test valid relative path
    let result = adapter.validate_path(Path::new("my-crate"));
    assert!(result.is_ok());

    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_mock_git_cas() {
    let mock = MockGitCas::new();
    let result = mock.load_template_crate("test-crate");
    assert!(result.is_ok());

    let template_crate = result.unwrap();
    assert_eq!(template_crate.name, "mock");
    assert_eq!(
        template_crate.git_sha,
        "0000000000000000000000000000000000000000"
    );
}

// ============================================================================
// MCP Runtime Adapter Tests
// ============================================================================

#[test]
fn test_mcp_runtime_adapter_new() {
    let _adapter = McpRuntimeAdapter::new();
    assert!(true);
}

#[test]
fn test_mcp_grant_tool_access() {
    let adapter = McpRuntimeAdapter::new();
    let token = CapabilityToken::new(
        CapabilityResource::Tool,
        "*".to_string(),
        CapabilityAction::Execute,
        WebID::new(),
        WebID::new(),
        b"test-secret",
    );

    let result = adapter.grant_tool_access(token);
    assert!(result.is_ok());
}

#[test]
fn test_mcp_invoke_tool() {
    let adapter = McpRuntimeAdapter::new();
    let token = CapabilityToken::new(
        CapabilityResource::Tool,
        "*".to_string(),
        CapabilityAction::Execute,
        WebID::new(),
        WebID::new(),
        b"test-secret",
    );

    let input = json!({"param": "value"});
    let result = adapter.invoke_tool("test_tool", input, &token);

    assert!(result.is_ok());
}

// ============================================================================
// Memory Storage Adapter Tests
// ============================================================================

#[test]
fn test_memory_storage_in_memory() {
    let _adapter = MemoryStorageAdapter::in_memory().unwrap();
    assert!(true);
}

#[test]
fn test_store_semantic_triple() {
    let adapter = MemoryStorageAdapter::in_memory().unwrap();
    let producer_webid = WebID::new();
    let content = json!({
        "entity": "test-entity",
        "attribute": "test-attribute",
        "value": "test-value"
    });

    let token = CapabilityToken::new(
        CapabilityResource::Tool,
        "test".to_string(),
        CapabilityAction::Execute,
        WebID::new(),
        producer_webid,
        b"test-secret",
    );

    let result =
        adapter.store_artifact(producer_webid, "semantic_triple", content, "public", &token);

    assert!(result.is_ok());
}

#[test]
fn test_store_episodic_triple() {
    let adapter = MemoryStorageAdapter::in_memory().unwrap();
    let producer_webid = WebID::new();
    let content = json!({
        "entity": "test-entity",
        "attribute": "test-attribute",
        "value": "test-value"
    });

    let token = CapabilityToken::new(
        CapabilityResource::Tool,
        "test".to_string(),
        CapabilityAction::Execute,
        WebID::new(),
        producer_webid,
        b"test-secret",
    );

    let result = adapter.store_artifact(
        producer_webid,
        "episodic_triple",
        content,
        "private",
        &token,
    );

    assert!(result.is_ok());
}

#[test]
fn test_store_embedding() {
    let _adapter = MemoryStorageAdapter::in_memory().unwrap();
    let producer_webid = WebID::new();
    let content = json!({
        "vector": [0.1, 0.2, 0.3, 0.4, 0.5],
        "model": "test-model"
    });

    let token = CapabilityToken::new(
        CapabilityResource::Tool,
        "test".to_string(),
        CapabilityAction::Execute,
        WebID::new(),
        producer_webid,
        b"test-secret",
    );

    let result = _adapter.store_artifact(producer_webid, "embedding", content, "public", &token);

    assert!(result.is_ok());
}
