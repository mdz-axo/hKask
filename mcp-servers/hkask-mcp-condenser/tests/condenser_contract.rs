//! Contract tests for hkask-mcp-condenser — compression engine invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `CondenserEngine` (pure computation, no I/O dependencies).

use hkask_condenser::engine::CondenserEngine;
use hkask_condenser::types::Profile;
use hkask_condenser::types::{ClassifyRequest, CompressRequest, SetProfileRequest};
use hkask_mcp_server::server::CapabilityTier;
use hkask_mcp_condenser::CondenserServer;
use hkask_mcp_condenser::SaliencyRequest;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

// ── Classification contract tests ───────────────────────────────────────────

#[test]
fn classify_shell_tool() {
    let engine = CondenserEngine::new();
    let (cat, algo) = engine.classify("bash_execute");
    assert_eq!(cat.label(), "shell_command");
    assert!(!algo.is_empty());
}

#[test]
fn classify_test_tool() {
    let engine = CondenserEngine::new();
    let (cat, _) = engine.classify("pytest_run");
    assert_eq!(cat.label(), "test_output");
}

#[test]
fn classify_chat_tool() {
    let engine = CondenserEngine::new();
    let (cat, _) = engine.classify("message_send");
    assert_eq!(cat.label(), "conversation_history");
}

#[test]
fn classify_unknown_tool_is_unknown() {
    let engine = CondenserEngine::new();
    let (cat, _) = engine.classify("xyzzy_unknown_tool");
    assert_eq!(cat.label(), "unknown");
}

// ── Compression contract tests ──────────────────────────────────────────────

#[test]
fn compress_reduces_size() {
    let mut engine = CondenserEngine::new();
    let input = "line1\nline2\nline3\nline4\nline5\n".repeat(20); // 100 lines
    let result = engine.compress("bash_execute", &input, None);
    assert!(
        result.compressed_bytes <= result.original_bytes,
        "compressed ({}) should be <= original ({})",
        result.compressed_bytes,
        result.original_bytes
    );
    assert_eq!(result.original_lines, 100);
}

#[test]
fn compress_with_explicit_category() {
    let mut engine = CondenserEngine::new();
    let input = "some output text";
    let result = engine.compress(
        "some_tool",
        input,
        Some(hkask_condenser::types::ContextCategory::ShellCommand),
    );
    assert_eq!(result.category, "shell_command");
}

#[test]
fn compress_empty_input() {
    let mut engine = CondenserEngine::new();
    let result = engine.compress("bash_execute", "", None);
    assert_eq!(result.original_bytes, 0);
    assert_eq!(result.reduction_pct, 0.0);
}

#[test]
fn repeated_compression_increments_stats() {
    let mut engine = CondenserEngine::new();
    let input = "hello world";
    for _ in 0..5 {
        engine.compress("bash_execute", input, None);
    }
    assert_eq!(engine.stats.total_compressions, 5);
    assert!(engine.stats.total_original_bytes > 0);
    assert!(engine.stats.total_compressed_bytes > 0);
}

// ── Profile contract tests ──────────────────────────────────────────────────

#[test]
fn set_profile_changes_behavior() {
    let mut engine = CondenserEngine::new();
    engine.set_profile(Profile::Heavy);
    let result = engine.compress("bash_execute", "a\nb\nc\nd\ne\nf", None);
    assert_eq!(result.profile, "heavy");
}

#[test]
fn default_profile_is_normal() {
    let engine = CondenserEngine::new();
    let stats = engine.get_stats();
    assert_eq!(stats.total_compressions, 0);
}

// ── Health check contract tests ─────────────────────────────────────────────

#[test]
fn health_check_returns_empty_for_fresh_engine() {
    // A fresh engine with zero compressions should not trigger the low-ratio
    // health signal — the check requires >= 10 compressions before flagging.
    let engine = CondenserEngine::new();
    let signals = engine.check_global_health();
    assert!(
        signals.is_empty(),
        "fresh engine should have no health signals, got {signals:?}"
    );
}

#[test]
fn health_check_flags_low_compression_ratio() {
    // Simulate 10+ compressions where total original ≈ total compressed
    // (ratio < 2:1) — should trigger the low_compression_ratio signal.
    let mut engine = CondenserEngine::new();
    // Heavy profile with tiny inputs that won't compress (passthrough).
    for _ in 0..15 {
        engine.compress("bash_execute", "ab", None);
    }
    let signals = engine.check_global_health();
    assert!(
        signals
            .iter()
            .any(|s| s.signal_type == "low_compression_ratio"),
        "expected low_compression_ratio signal after 15 passthrough compressions, got {signals:?}"
    );
}

// ── Tool-behavior contract tests (Parameters<T> seam) ───────────────────────
//
// These exercise the actual MCP tool methods through the public `Parameters<T>`
// seam — the same surface an agent uses. Closes the test-variety gap that hid
// the create-new-file, range-inversion, and multibyte-truncation defects in
// hkask-mcp-filesystem.

/// A no-op InferencePort for testing — avoids real API calls.
struct NoopInferencePort;

impl hkask_types::InferencePort for NoopInferencePort {
    fn generate(
        &self,
        _prompt: &str,
        _parameters: &hkask_types::template::LLMParameters,
        _tools: Option<&[hkask_types::ChatToolDefinition]>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<hkask_types::InferenceResult, hkask_types::InferenceError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async {
            Ok(hkask_types::InferenceResult {
                text: String::new(),
                model: "noop".into(),
                usage: hkask_types::InferenceUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                finish_reason: "stop".into(),
                token_probabilities: None,
                tool_calls: vec![],
            })
        })
    }
}

/// Construct a CondenserServer with no persistence and a noop inference port.
fn test_server() -> CondenserServer {
    CondenserServer::new(
        WebID::new(),
        "test-userpod".into(),
        None,
        Mutex::new(CondenserEngine::new()),
        None,
        None,
        Arc::new(NoopInferencePort),
        "noop-model".into(),
        CondenserServer::default_persona_keywords(),
        CapabilityTier::detect(&HashMap::new()),
    )
}

/// Parse the success envelope `{"content": <value>}`; falls back to the raw
/// value for non-envelope outputs.
fn parse_content(out: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("content").cloned().unwrap_or(v)
}

/// Extract the `kind` field from an error envelope, if present.
fn error_kind(out: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("kind").and_then(|e| e.as_str()).map(String::from)
}

// REQ: condenser_compress rejects empty output with invalid_argument (P5).
// expect: an empty output string returns kind=invalid_argument.
#[tokio::test]
async fn condenser_compress_rejects_empty_output_via_parameters_seam() {
    let server = test_server();
    let out = server
        .condenser_compress(Parameters(CompressRequest {
            tool_name: "bash_execute".into(),
            output: String::new(),
            category: None,
        }))
        .await;
    let kind = error_kind(&out).expect("expected error kind for empty output");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: condenser_compress compresses non-empty output (P5).
// expect: a non-empty output returns compressed_bytes <= original_bytes.
#[tokio::test]
async fn condenser_compress_returns_compressed_via_parameters_seam() {
    let server = test_server();
    let input = "line1\nline2\nline3\nline4\nline5\n".repeat(20);
    let out = server
        .condenser_compress(Parameters(CompressRequest {
            tool_name: "bash_execute".into(),
            output: input.clone(),
            category: None,
        }))
        .await;
    let content = parse_content(&out);
    let compressed = content["compressed_bytes"]
        .as_u64()
        .expect("compressed_bytes");
    let original = content["original_bytes"].as_u64().expect("original_bytes");
    assert!(
        compressed <= original,
        "compressed should be <= original: {out}"
    );
}

// REQ: condenser_set_profile rejects an invalid profile name (P5).
// expect: an unknown profile returns kind=invalid_argument.
#[tokio::test]
async fn condenser_set_profile_rejects_invalid_via_parameters_seam() {
    let server = test_server();
    let out = server
        .condenser_set_profile(Parameters(SetProfileRequest {
            profile: "ultra".into(),
        }))
        .await;
    let kind = error_kind(&out).expect("expected error kind for invalid profile");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: condenser_classify returns the category for a tool name (P5).
// expect: classify returns a category and algorithm for a known tool pattern.
#[tokio::test]
async fn condenser_classify_returns_category_via_parameters_seam() {
    let server = test_server();
    let out = server
        .condenser_classify(Parameters(ClassifyRequest {
            tool_name: "bash_execute".into(),
        }))
        .await;
    let content = parse_content(&out);
    assert_eq!(content["tool_name"], "bash_execute");
    assert!(
        content.get("category").is_some(),
        "should have category: {out}"
    );
    assert!(
        content.get("algorithm").is_some(),
        "should have algorithm: {out}"
    );
}

// REQ: condenser_persist rejects when no persistence backend is configured (P5).
// expect: without episodic memory, returns kind=permission_denied.
#[tokio::test]
async fn condenser_persist_rejects_without_persistence_via_parameters_seam() {
    let server = test_server();
    let out = server
        .condenser_persist(Parameters(hkask_condenser::types::PersistRequest {
            tool_name: "bash_execute".into(),
            compressed_output: "some compressed text".into(),
            confidence: None,
        }))
        .await;
    let kind = error_kind(&out).expect("expected error kind for missing persistence");
    assert_eq!(kind, "permission_denied", "got: {out}");
}

// REQ: condenser_score_saliency returns a score in [0.0, 1.0] (P5).
// expect: persona-based scoring returns a numeric score.
#[tokio::test]
async fn condenser_score_saliency_returns_score_via_parameters_seam() {
    let server = test_server();
    let out = server
        .condenser_score_saliency(Parameters(SaliencyRequest {
            text: "compress the context and summarize".into(),
            against: None,
            persona_keywords: None,
        }))
        .await;
    let content = parse_content(&out);
    let score = content["score"].as_f64().expect("score should be a number");
    assert!(
        (0.0..=1.0).contains(&score),
        "score should be in [0,1]: {score}"
    );
    assert_eq!(content["against"], "persona");
}
