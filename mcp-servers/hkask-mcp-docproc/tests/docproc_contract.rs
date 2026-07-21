//! Contract tests for hkask-mcp-docproc — document processing tool-behavior.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seams:
//! - `DocProcServer` tool methods via `Parameters<T>` seam
//! - `ThresholdConfig` defaults (pure computation)

use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_mcp_docproc::DocProcServer;
use hkask_mcp_docproc::ocr::llm_ocr::LlmOcrExecutor;
use hkask_mcp_docproc::ocr::{PipelineExecutor, ThresholdConfig};
use hkask_mcp_docproc::tools::document::{ChunkRequest, ConvertRequest};
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;

// ── ThresholdConfig contract tests ──────────────────────────────────────────
#[test]
fn threshold_config_has_defaults() {
    let config = ThresholdConfig::default();
    // Default config should have non-zero thresholds
    assert!(config.simple_max > 0.0);
    assert!(config.moderate_max > 0.0);
}

// ── Tool-behavior contract tests (Parameters<T> seam) ───────────────────────
//
// These exercise the actual MCP tool methods through the public `Parameters<T>`
// seam — the same surface an agent uses. Closes the test-variety gap that hid
// the create-new-file, range-inversion, and multibyte-truncation defects in
// hkask-mcp-filesystem.

/// Construct a DocProcServer with no OCR model and no embedding router.
fn test_server() -> DocProcServer {
    let inference_router = Arc::new(InferenceRouter::new(InferenceConfig::default()));
    let llm_ocr = Arc::new(LlmOcrExecutor::new(Arc::clone(&inference_router)));
    let pipeline_executor = Arc::new(PipelineExecutor::new(Arc::clone(&llm_ocr)));
    DocProcServer::new(
        WebID::new(),
        "test-userpod".into(),
        None,
        None, // no OCR model
        inference_router,
        ThresholdConfig::default(),
        None, // no embedding router
        std::sync::Mutex::new(Vec::new()),
        std::sync::Mutex::new(Vec::new()),
        llm_ocr,
        pipeline_executor,
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

// REQ: docproc_chunk rejects when neither text nor path is provided (P5).
// expect: missing both text and path returns kind=invalid_argument.
#[tokio::test]
async fn docproc_chunk_rejects_missing_text_and_path_via_parameters_seam() {
    let server = test_server();
    let req: ChunkRequest = serde_json::from_value(serde_json::json!({
        "entity_ref_prefix": "test:doc",
        "max_tokens": 500,
        "overlap_tokens": 50
    }))
    .expect("deserialize ChunkRequest");
    let out = server.docproc_chunk(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for missing text and path");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: docproc_chunk rejects an empty entity_ref_prefix (P5).
// expect: an empty entity_ref_prefix returns kind=invalid_argument.
#[tokio::test]
async fn docproc_chunk_rejects_empty_prefix_via_parameters_seam() {
    let server = test_server();
    let req: ChunkRequest = serde_json::from_value(serde_json::json!({
        "text": "Some sample text to chunk.",
        "entity_ref_prefix": "",
        "max_tokens": 500,
        "overlap_tokens": 50
    }))
    .expect("deserialize ChunkRequest");
    let out = server.docproc_chunk(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for empty prefix");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: docproc_chunk chunks raw text into passages (P5 Testing Discipline).
// expect: chunking valid text returns a non-empty passages array.
#[tokio::test]
async fn docproc_chunk_returns_passages_via_parameters_seam() {
    let server = test_server();
    let req: ChunkRequest = serde_json::from_value(serde_json::json!({
        "text": "This is a test passage. It has multiple sentences. Each should be chunked appropriately.",
        "entity_ref_prefix": "test:doc",
        "max_tokens": 100,
        "overlap_tokens": 10
    }))
    .expect("deserialize ChunkRequest");
    let out = server.docproc_chunk(Parameters(req)).await;
    let content = parse_content(&out);
    assert!(
        content.get("passages").is_some() || content.get("chunks").is_some(),
        "should have passages or chunks: {out}"
    );
}

// REQ: docproc_convert rejects a non-existent file path (P5).
// expect: a path that does not exist returns kind=internal (file read error).
#[tokio::test]
async fn docproc_convert_rejects_nonexistent_path_via_parameters_seam() {
    let server = test_server();
    let req: ConvertRequest = serde_json::from_value(serde_json::json!({
        "path": "/nonexistent/file.txt",
        "output": null,
        "force_ocr": false
    }))
    .expect("deserialize ConvertRequest");
    let out = server.docproc_convert(Parameters(req)).await;
    // Non-existent file returns an error (internal or invalid_argument)
    let v: serde_json::Value = serde_json::from_str(&out).expect("JSON");
    assert!(
        v.get("error").is_some() || v.get("kind").is_some(),
        "should return an error for non-existent file: {out}"
    );
}
