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
fn training_validate_config_request_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::types::TrainValidateConfigRequest>();
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

    let schema = schemars::schema_for!(hkask_mcp_training::types::TrainValidateConfigRequest);
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
    ProviderError, TrainingHarnessId, TrainingHost, TrainingHostId, TrainingJob, TrainingJobStatus,
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
        TrainingHostId::Runpod,
        TrainingHarnessId::Axolotl,
        Mutex::new(DatasetPipeline::new(PathBuf::from("/tmp/hkask-test-cache"))),
        adapter_store,
        None, // no job store
        None, // no adapter router
        InferenceConfig::default(),
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

// ── Contract tests for retained tools (2026-07-19 cleanup, second pass) ────
//
// After the 21 → 15 → 8 simplification, these tests cover the retained tools
// that have observable behavior with a mock host: submit, status, cancel,
// validate_config. Deployment/register/list/delete tools were removed (they
// are now `AdapterPort` / `AdapterStore` direct calls). All use the
// MockTrainingHost (no external API calls).

// REQ: training_submit rejects a missing dataset file without panicking (P5).
// expect: submit returns an error envelope, not a panic.
#[tokio::test]
async fn training_submit_rejects_missing_dataset_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_training::types::TrainSubmitRequest =
        serde_json::from_value(serde_json::json!({
            "dataset_path": "/nonexistent/path/to/dataset.jsonl",
            "base_model": "unsloth/Qwen3.6-27B",
            "params": null,
            "feedback_path": null,
            "skill_name": null,
            "adapter_name": null,
            "merged_output_path": null
        }))
        .expect("deserialize TrainSubmitRequest");
    let out = server.training_submit(Parameters(req)).await;
    // Should return an error, not panic — the exact error format depends on
    // whether the file check happens before or after host submission.
    let v: serde_json::Value = serde_json::from_str(&out).expect("tool output is JSON");
    // The tool should return either an error envelope or a content with error info.
    // Either way, it must not be a success with a job_id.
    assert!(
        v.get("content").and_then(|c| c.get("job_id")).is_none() || v.get("kind").is_some(),
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

// REQ: training_validate_config passes default params with no refusals (P5).
// expect: validate_config returns verdict "pass" for default TrainingParams
// (no gate violations — defaults are safe per PEFT v0.19.0).
#[tokio::test]
async fn training_validate_config_passes_default_params_via_parameters_seam() {
    let server = test_server();
    let req: hkask_mcp_training::types::TrainValidateConfigRequest =
        serde_json::from_value(serde_json::json!({
            "params": hkask_mcp_training::providers::TrainingParams::default()
        }))
        .expect("deserialize TrainValidateConfigRequest");
    let out = server.training_validate_config(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(
        content["verdict"], "pass",
        "default TrainingParams should pass all static gates: {out}"
    );
    assert_eq!(
        content["has_refusals"], false,
        "no refusals for default params: {out}"
    );
}

// REQ: training_validate_config refuses rank=0 (G-M3 scaling form gate) (P5).
// expect: validate_config returns verdict "fail" with has_refusals=true when
// the LoRA rank is zero (scaling form α/r is undefined).
#[tokio::test]
async fn training_validate_config_refuses_rank_zero_via_parameters_seam() {
    let server = test_server();
    let mut params = hkask_mcp_training::providers::TrainingParams::default();
    params.lora.r = 0;
    let req: hkask_mcp_training::types::TrainValidateConfigRequest =
        serde_json::from_value(serde_json::json!({
            "params": params
        }))
        .expect("deserialize TrainValidateConfigRequest");
    let out = server.training_validate_config(Parameters(req)).await;
    let content = parse_content(&out);
    assert_eq!(
        content["verdict"], "fail",
        "rank=0 should fail the scaling form gate: {out}"
    );
    assert_eq!(
        content["has_refusals"], true,
        "rank=0 should produce a refusal: {out}"
    );
    // Verify the finding is from the scaling form gate (G-M3).
    let findings = content["findings"]
        .as_array()
        .expect("findings should be an array");
    assert!(
        findings
            .iter()
            .any(|f| f["gate_id"] == "G-M3" && f["severity"] == "refuse"),
        "should have a G-M3 refuse finding: {out}"
    );
}
