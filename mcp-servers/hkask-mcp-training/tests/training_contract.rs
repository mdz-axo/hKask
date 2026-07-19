//! Contract tests for hkask-mcp-training — MLSchema concepts and adapter types.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: MLSchema ontology constants, adapter metadata types, and request deserialization.

// ── MLSchema concept tests ─────────────────────────────────────────────────

#[test]
fn mls_concepts_are_non_empty() {
    assert!(!hkask_mcp_training::mlschema::MODEL.is_empty());
    assert!(!hkask_mcp_training::mlschema::RUN.is_empty());
    assert!(!hkask_mcp_training::mlschema::DATA.is_empty());
    assert!(!hkask_mcp_training::mlschema::HYPER_PARAMETER.is_empty());
    assert!(!hkask_mcp_training::mlschema::EVALUATION.is_empty());
}

#[test]
fn mls_concepts_have_correct_prefix() {
    assert!(hkask_mcp_training::mlschema::MODEL.starts_with("mls:"));
    assert!(hkask_mcp_training::mlschema::RUN.starts_with("mls:"));
    assert!(hkask_mcp_training::mlschema::DATA.starts_with("mls:"));
}

#[test]
fn mls_derivation_concept_exists() {
    assert_eq!(
        hkask_mcp_training::mlschema::WAS_DERIVED_FROM,
        "mls:wasDerivedFrom"
    );
}

// ── Adapter type tests ─────────────────────────────────────────────────────

#[test]
fn trained_lora_adapter_type_exists() {
    let _type_name = std::any::type_name::<hkask_adapter::TrainedLoRAAdapter>();
    assert!(_type_name.contains("hkask_adapter"));
}

#[test]
fn adapter_metrics_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::adapters::AdapterMetrics>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

// ── Request type tests ─────────────────────────────────────────────────────

#[test]
fn ingest_qa_request_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::types::IngestQaRequest>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

#[test]
fn training_submit_request_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::types::TrainSubmitRequest>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

#[test]
fn training_host_id_enum_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::providers::TrainingHostId>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

#[test]
fn training_job_status_enum_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::providers::TrainingJobStatus>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

// ── Dataset pipeline tests ─────────────────────────────────────────────────

#[test]
fn dataset_pipeline_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::dataset::DatasetPipeline>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

// ── Schema generation tests ────────────────────────────────────────────────

#[test]
fn request_types_have_schemas() {
    let schema = schemars::schema_for!(hkask_mcp_training::types::IngestQaRequest);
    let schema_json = serde_json::to_value(&schema).expect("schema should serialize");
    assert!(schema_json.is_object());

    let schema = schemars::schema_for!(hkask_mcp_training::types::TrainSubmitRequest);
    let schema_json = serde_json::to_value(&schema).expect("schema should serialize");
    assert!(schema_json.is_object());
}

// ── Tool-behavior contract tests (Parameters<T> seam) ───────────────────────
//
// These exercise the actual MCP tool methods through the public `Parameters<T>`
// seam — the same surface an agent uses. Closes the test-variety gap that hid
// the create-new-file, range-inversion, and multibyte-truncation defects in
// hkask-mcp-filesystem.

use hkask_adapter::AdapterStore;
use hkask_database::sqlite::SqliteDriver;
use hkask_inference::InferenceConfig;
use hkask_mcp_training::TrainingServer;
use hkask_mcp_training::dataset::DatasetPipeline;
use hkask_mcp_training::providers::{
    CostEstimate, ProviderError, TrainingHarnessId, TrainingHost, TrainingHostId, TrainingJob,
    TrainingJobStatus,
};
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A mock TrainingHost that returns empty results — no external API calls.
struct MockTrainingHost;

#[async_trait::async_trait]
impl TrainingHost for MockTrainingHost {
    async fn submit(&self, _job: &TrainingJob) -> Result<String, ProviderError> {
        Err(ProviderError::Unavailable("mock host".into()))
    }
    async fn status(&self, _job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        Ok(TrainingJobStatus::Failed)
    }
    async fn cancel(&self, _job_id: &str) -> Result<(), ProviderError> {
        Ok(())
    }
    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        Ok(vec![])
    }
    async fn delete_adapter(&self, _adapter_id: &str) -> Result<(), ProviderError> {
        Ok(())
    }
    async fn estimate_cost(&self, _job: &TrainingJob) -> CostEstimate {
        CostEstimate::default()
    }
}

/// Construct a TrainingServer with a mock host and in-memory adapter store.
fn test_server() -> TrainingServer {
    let pool = SqliteDriver::in_memory_pool().expect("in-memory pool");
    let driver: Arc<dyn hkask_database::driver::DatabaseDriver> = Arc::new(SqliteDriver::new(pool));
    let adapter_store = Arc::new(AdapterStore::from_driver(driver));
    TrainingServer::new(
        WebID::new(),
        "test-replicant".into(),
        None,
        None, // no semantic memory
        Box::new(MockTrainingHost),
        TrainingHostId::Together,
        TrainingHarnessId::Axolotl,
        Mutex::new(DatasetPipeline::new(PathBuf::from("/tmp/hkask-test-cache"))),
        adapter_store,
        None, // no job store
        None, // no adapter router
        InferenceConfig::default(),
        Mutex::new(std::collections::HashMap::new()),
    )
}

/// Parse the success envelope `{"content": <value>}`; falls back to the raw
/// value for non-envelope outputs.
fn parse_content(out: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("content").cloned().unwrap_or(v)
}

/// Extract the `kind` field from an error envelope, if present.
#[allow(dead_code)]
fn error_kind(out: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("kind").and_then(|e| e.as_str()).map(String::from)
}

// REQ: training_list_adapters returns an empty list for a fresh server (P5).
// expect: list_adapters returns an empty adapters array.
#[tokio::test]
async fn training_list_adapters_returns_empty_via_parameters_seam() {
    let server = test_server();
    let out = server.training_list_adapters().await;
    let content = parse_content(&out);
    assert!(
        content["adapters"].is_array(),
        "adapters should be an array: {out}"
    );
    assert_eq!(
        content["adapters"].as_array().unwrap().len(),
        0,
        "got: {out}"
    );
}

// REQ: training_preflight_check fails on a missing adapter_config.json (P5).
// expect: preflight returns all_pass=false, failed_at=load.
#[tokio::test]
async fn training_preflight_check_fails_on_missing_config_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_training::types::TrainPreflightCheckRequest =
        serde_json::from_value(serde_json::json!({
            "adapter_path": "/nonexistent/path",
            "model": null,
            "test_prompt": null,
            "min_response_chars": null
        }))
        .expect("deserialize TrainPreflightCheckRequest");
    let out = server.training_preflight_check(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(
        content["all_pass"], false,
        "should fail on missing config: {out}"
    );
    assert_eq!(
        content["failed_at"], "load",
        "should fail at load check: {out}"
    );
}

// REQ: training_preflight_check fails on a missing adapter_model.safetensors (P5).
// expect: preflight returns all_pass=false, failed_at=weights when config exists but weights don't.
#[tokio::test]
async fn training_preflight_check_fails_on_missing_weights_via_parameters_seam() {
    let server = test_server();
    // Create a temp dir with a valid adapter_config.json but no safetensors
    let temp_dir = std::env::temp_dir().join("hkask_preflight_test_no_weights");
    std::fs::create_dir_all(&temp_dir).ok();
    std::fs::write(
        temp_dir.join("adapter_config.json"),
        r#"{"init_lora_weights": true, "r": 32, "lora_alpha": 64, "base_model_name_or_path": "test"}"#,
    )
    .ok();

    let req: hkask_mcp_training::types::TrainPreflightCheckRequest =
        serde_json::from_value(serde_json::json!({
            "adapter_path": temp_dir.to_string_lossy(),
            "model": null,
            "test_prompt": null,
            "min_response_chars": null
        }))
        .expect("deserialize TrainPreflightCheckRequest");
    let out = server.training_preflight_check(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(
        content["all_pass"], false,
        "should fail on missing weights: {out}"
    );
    assert_eq!(
        content["failed_at"], "weights",
        "should fail at weights check: {out}"
    );
    std::fs::remove_dir_all(&temp_dir).ok();
}

// REQ: training_preflight_check passes when config and weights exist (P5).
// expect: preflight returns all_pass=true with load and weights both pass (sanity skipped).
#[tokio::test]
async fn training_preflight_check_passes_with_valid_adapter_via_parameters_seam() {
    let server = test_server();
    // Create a temp dir with a valid config and a dummy safetensors file
    let temp_dir = std::env::temp_dir().join("hkask_preflight_test_valid");
    std::fs::create_dir_all(&temp_dir).ok();
    std::fs::write(
        temp_dir.join("adapter_config.json"),
        r#"{"init_lora_weights": true, "r": 32, "lora_alpha": 64, "base_model_name_or_path": "unsloth/Qwen3.6-27B"}"#,
    )
    .ok();
    // Write a dummy safetensors file > 1KB
    std::fs::write(temp_dir.join("adapter_model.safetensors"), vec![0u8; 2048]).ok();

    let req: hkask_mcp_training::types::TrainPreflightCheckRequest =
        serde_json::from_value(serde_json::json!({
            "adapter_path": temp_dir.to_string_lossy(),
            "model": null,
            "test_prompt": null,
            "min_response_chars": null
        }))
        .expect("deserialize TrainPreflightCheckRequest");
    let out = server.training_preflight_check(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(
        content["all_pass"], true,
        "should pass with valid adapter (no inference): {out}"
    );
    let checks = content["checks"].as_array().expect("checks is array");
    assert!(
        checks.len() >= 2,
        "should have load and weights checks: {out}"
    );
    assert_eq!(checks[0]["check"], "load");
    assert_eq!(checks[0]["status"], "pass");
    assert_eq!(checks[1]["check"], "weights");
    assert_eq!(checks[1]["status"], "pass");
    std::fs::remove_dir_all(&temp_dir).ok();
}

// ── Contract tests for previously-untested tools (2026-07-19 cleanup) ──────
//
// These tests close the coverage gap identified in the post-mortem: 18 of 21
// tools had no contract tests. After the 21→14 simplification, these 5 tests
// cover the most critical untested tools: submit, status, cancel, deploy, teardown.
// All use the MockTrainingHost (no external API calls).

// REQ: training_submit rejects a missing dataset file without panicking (P5).
// expect: submit returns an error envelope, not a panic.
#[tokio::test]
async fn training_submit_rejects_missing_dataset_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_training::types::TrainSubmitRequest =
        serde_json::from_value(serde_json::json!({
            "dataset_path": "/nonexistent/path/to/dataset.jsonl",
            "base_model": "unsloth/Qwen3.6-27B",
            "params": null
        }))
        .expect("deserialize TrainSubmitRequest");
    let out = server.training_submit(Parameters(req)).await;
    // Should return an error, not panic — the exact error format depends on
    // whether the file check happens before or after host submission.
    let v: serde_json::Value = serde_json::from_str(&out).expect("tool output is JSON");
    // The tool should return either an error envelope or a content with error info.
    // Either way, it must not be a success with a job_id.
    assert!(
        v.get("content").and_then(|c| c.get("job_id")).is_none()
            || v.get("kind").is_some(),
        "submit should not return a job_id for a missing dataset: {out}"
    );
}

// REQ: training_status returns the host's status for a given job_id (P5).
// expect: status returns "failed" for the mock host (which always returns Failed).
#[tokio::test]
async fn training_status_returns_failed_for_mock_host_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_training::types::TrainStatusRequest =
        serde_json::from_value(serde_json::json!({
            "job_id": "test-job-123"
        }))
        .expect("deserialize TrainStatusRequest");
    let out = server.training_status(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(
        content["job_id"], "test-job-123",
        "status should echo the job_id: {out}"
    );
    // MockTrainingHost returns TrainingJobStatus::Failed
    assert_eq!(
        content["status"], "failed",
        "mock host should return failed status: {out}"
    );
}

// REQ: training_cancel succeeds when the host cancel succeeds (P5).
// expect: cancel returns status "cancelled" for the mock host (which returns Ok).
#[tokio::test]
async fn training_cancel_succeeds_with_mock_host_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_training::types::TrainCancelRequest =
        serde_json::from_value(serde_json::json!({
            "job_id": "test-job-456"
        }))
        .expect("deserialize TrainCancelRequest");
    let out = server.training_cancel(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(
        content["job_id"], "test-job-456",
        "cancel should echo the job_id: {out}"
    );
    assert_eq!(
        content["status"], "cancelled",
        "mock host cancel should succeed: {out}"
    );
}

// REQ: training_deploy rejects an unknown adapter name without panicking (P5).
// expect: deploy returns an error for an adapter that doesn't exist in the store.
#[tokio::test]
async fn training_deploy_rejects_unknown_adapter_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_training::types::TrainDeployRequest =
        serde_json::from_value(serde_json::json!({
            "adapter_name": "nonexistent-adapter-xyz",
            "provider": "together",
            "base_model": null,
            "gpu_type": null
        }))
        .expect("deserialize TrainDeployRequest");
    let out = server.training_deploy(Parameters(req)).await;
    // Should return an error, not a panic — the adapter doesn't exist in the store.
    let v: serde_json::Value = serde_json::from_str(&out).expect("tool output is JSON");
    assert!(
        v.get("content").and_then(|c| c.get("endpoint_url")).is_none()
            || v.get("kind").is_some(),
        "deploy should not return an endpoint for an unknown adapter: {out}"
    );
}

// REQ: training_teardown handles an unknown deployment ID without panicking (P5).
// expect: teardown returns gracefully (error or no-op) for a nonexistent deployment.
#[tokio::test]
async fn training_teardown_handles_unknown_deployment_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_training::types::TrainTeardownRequest =
        serde_json::from_value(serde_json::json!({
            "deployment_id": "nonexistent-deployment-xyz"
        }))
        .expect("deserialize TrainTeardownRequest");
    let out = server.training_teardown(Parameters(req)).await;
    // Should not panic — the deployment doesn't exist.
    let v: serde_json::Value = serde_json::from_str(&out).expect("tool output is JSON");
    // The tool should return either a success (no-op) or an error envelope.
    // Either way, it must be valid JSON and not a panic.
    assert!(
        v.is_object(),
        "teardown should return valid JSON for unknown deployment: {out}"
    );
}
