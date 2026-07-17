//! Integration tests for per-manifest fusion config wiring.
//!
//! Verifies that the ManifestExecutor correctly propagates the manifest-level
//! FusionConfig to LLMParameters.fusion_config, and that per-step fusion
//! overrides work as expected.

use hkask_ports::{ChatToolDefinition, InferenceError, InferencePort, InferenceResult};
use hkask_templates::bundle::cascade::CascadePhase;
use hkask_templates::bundle::config::{
    BundleAuditConfig, BundleCnsConfig, BundleGasConfig, ConvergenceConfig, ErrorHandlingConfig,
    OcapConfig, RjouleConfig,
};
use hkask_templates::bundle::manifest::{BundleManifest, BundleManifestStep};
use hkask_templates::executor::ManifestExecutor;
use hkask_templates::ports::NoopMcpPort;
use hkask_types::fusion::{FusionConfig, FusionMode, NonEmptyVec};
use hkask_types::template::LLMParameters;
use hkask_types::visibility::Visibility;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Mock inference port that captures the LLMParameters it receives.
struct CapturingInferencePort {
    captured_params: Arc<Mutex<Vec<LLMParameters>>>,
}

impl CapturingInferencePort {
    fn new() -> (Self, Arc<Mutex<Vec<LLMParameters>>>) {
        let params = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                captured_params: params.clone(),
            },
            params,
        )
    }
}

fn canned_result() -> InferenceResult {
    InferenceResult {
        text: r#"{"convergence_metric": 0.0, "rationale": "test", "blockers": []}"#.to_string(),
        model: "test-model".to_string(),
        usage: hkask_ports::InferenceUsage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        },
        finish_reason: "stop".to_string(),
        token_probabilities: None,
        tool_calls: vec![],
    }
}

impl InferencePort for CapturingInferencePort {
    fn generate(
        &self,
        _prompt: &str,
        params: &LLMParameters,
        _tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.captured_params.lock().unwrap().push(params.clone());
        let result = canned_result();
        Box::pin(async move { Ok(result) })
    }

    fn generate_with_model(
        &self,
        _prompt: &str,
        params: &LLMParameters,
        _model_override: Option<&str>,
        _tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.captured_params.lock().unwrap().push(params.clone());
        let result = canned_result();
        Box::pin(async move { Ok(result) })
    }
}

fn build_manifest(fusion: Option<FusionConfig>, step_fusion: Option<bool>) -> BundleManifest {
    BundleManifest {
        id: "test-fusion".to_string(),
        name: "Test Fusion".to_string(),
        description: String::new(),
        version: "0.31.0".to_string(),
        editor: "test".to_string(),
        visibility: Visibility::Public,
        skills: Vec::new(),
        conflicts: Vec::new(),
        complementarities: Vec::new(),
        steps: vec![
            BundleManifestStep {
                ordinal: 1,
                action: "select".to_string(),
                description: "Test step".to_string(),
                renderer: Some("minijinja".to_string()),
                template_ref: Some("test/canned.j2".to_string()),
                mcp: None,
                compute_ref: None,
                gas_cap: 1000,
                timeout_seconds: 10,
                input_mapping: None,
                output_schema: None,
                phase: CascadePhase::Core,
                condition: None,
                fusion: step_fusion,
            },
            BundleManifestStep {
                ordinal: 2,
                action: "abort".to_string(),
                description: "Converged".to_string(),
                renderer: None,
                template_ref: None,
                mcp: None,
                compute_ref: None,
                gas_cap: 0,
                timeout_seconds: 0,
                input_mapping: None,
                output_schema: None,
                phase: CascadePhase::Core,
                condition: None,
                fusion: None,
            },
        ],
        convergence: ConvergenceConfig {
            threshold: 0.15,
            improvement_ratio: 0.0,
            max_iterations: 1,
            min_iterations: 1,
            convergence_field: "step_1_result.convergence_metric".to_string(),
            on_not_reached: "escalate".to_string(),
            ..Default::default()
        },
        gas: BundleGasConfig {
            cap: 10000,
            cost_per_iteration: 100,
            alert_threshold: 0.8,
            hard_limit: true,
        },
        rjoule: RjouleConfig {
            cap: 0,
            alert_threshold: 0.8,
            hard_limit: true,
        },
        error_handling: ErrorHandlingConfig {
            on_gas_exceeded: "abort".to_string(),
            on_timeout: "retry".to_string(),
            max_retries: 1,
            retry_backoff_seconds: 1,
            on_validation_failure: "abort".to_string(),
        },
        ocap: OcapConfig::default(),
        cns: BundleCnsConfig::default(),
        audit: BundleAuditConfig::default(),
        functional_role: Some("flowdef".to_string()),
        category: Some("skill".to_string()),
        inputs: None,
        principles: None,
        fusion,
    }
}

/// Create a temp directory with a minimal Jinja2 template that outputs JSON.
fn setup_test_templates() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("hkask-fusion-test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::create_dir_all(dir.join("test")).expect("create test subdir");
    std::fs::write(
        dir.join("test/canned.j2"),
        r#"{"convergence_metric": 0.0, "rationale": "test", "blockers": []}"#,
    )
    .expect("write test template");
    dir
}

#[tokio::test]
async fn manifest_fusion_config_propagates_to_params() {
    let template_dir = setup_test_templates();
    let (mock, captured) = CapturingInferencePort::new();
    let executor = ManifestExecutor::new(
        Arc::new(mock),
        Arc::new(NoopMcpPort),
        LLMParameters::default(),
        b"test-secret".to_vec(),
    )
    .with_template_base_path(template_dir);

    let fusion = FusionConfig {
        judge: "test-judge".to_string(),
        panel: NonEmptyVec::from_vec(vec!["model-a".to_string(), "model-b".to_string()]).unwrap(),
        mode: FusionMode::Synthesis,
        skills: Vec::new(),
        max_rounds: 3,
    };

    let manifest = build_manifest(Some(fusion.clone()), None);
    let context = HashMap::new();
    let _ = executor.execute_manifest(&manifest, context).await;

    let params = captured.lock().unwrap();
    assert!(
        !params.is_empty(),
        "at least one inference call should have been made"
    );
    let first = &params[0];
    assert!(
        first.fusion_config.is_some(),
        "manifest fusion config should propagate to params.fusion_config"
    );
    let fc = first.fusion_config.as_ref().unwrap();
    assert_eq!(fc.judge, "test-judge");
    assert_eq!(fc.panel.len(), 2);
    assert!(!first.bypass_fusion, "bypass_fusion should be false");
}

#[tokio::test]
async fn step_fusion_false_bypasses_manifest_fusion() {
    let template_dir = setup_test_templates();
    let (mock, captured) = CapturingInferencePort::new();
    let executor = ManifestExecutor::new(
        Arc::new(mock),
        Arc::new(NoopMcpPort),
        LLMParameters::default(),
        b"test-secret".to_vec(),
    )
    .with_template_base_path(template_dir);

    let fusion = FusionConfig {
        judge: "test-judge".to_string(),
        panel: NonEmptyVec::one("model-a".to_string()),
        mode: FusionMode::Synthesis,
        skills: Vec::new(),
        max_rounds: 3,
    };

    let manifest = build_manifest(Some(fusion), Some(false));
    let context = HashMap::new();
    let _ = executor.execute_manifest(&manifest, context).await;

    let params = captured.lock().unwrap();
    assert!(
        !params.is_empty(),
        "at least one inference call should have been made"
    );
    let first = &params[0];
    assert!(
        first.bypass_fusion,
        "step.fusion=Some(false) should set bypass_fusion=true"
    );
    assert!(
        first.fusion_config.is_none(),
        "step.fusion=Some(false) should not set fusion_config"
    );
}

#[tokio::test]
async fn no_manifest_fusion_uses_global_default() {
    let template_dir = setup_test_templates();
    let (mock, captured) = CapturingInferencePort::new();
    let executor = ManifestExecutor::new(
        Arc::new(mock),
        Arc::new(NoopMcpPort),
        LLMParameters::default(),
        b"test-secret".to_vec(),
    )
    .with_template_base_path(template_dir);

    let manifest = build_manifest(None, None);
    let context = HashMap::new();
    let _ = executor.execute_manifest(&manifest, context).await;

    let params = captured.lock().unwrap();
    assert!(
        !params.is_empty(),
        "at least one inference call should have been made"
    );
    let first = &params[0];
    assert!(
        first.fusion_config.is_none(),
        "no manifest fusion → params.fusion_config should be None (global default applies)"
    );
    assert!(
        !first.bypass_fusion,
        "no manifest fusion → bypass_fusion should be false (global default)"
    );
}
