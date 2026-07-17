//! Decisive test: does a `select` step's `input_mapping` resolve `{{ }}` string
//! values from prior step results into the rendered template context?
//!
//! Step 1 produces `derived_value`; step 2's template references `{{ derived_value }}`
//! with `input_mapping: { derived_value: "{{ step_1_result.derived_value }}" }`.
//! If input_mapping is applied, the step-2 rendered prompt contains "FROM_STEP_ONE".
//! If not, minijinja (lenient) renders the missing variable to empty.

use hkask_ports::{ChatToolDefinition, InferenceError, InferencePort, InferenceResult};
use hkask_templates::bundle::cascade::CascadePhase;
use hkask_templates::bundle::config::{
    BundleAuditConfig, BundleCnsConfig, BundleGasConfig, ConvergenceConfig, ErrorHandlingConfig,
    OcapConfig, RjouleConfig,
};
use hkask_templates::bundle::manifest::{BundleManifest, BundleManifestStep};
use hkask_templates::executor::ManifestExecutor;
use hkask_templates::ports::NoopMcpPort;
use hkask_types::template::LLMParameters;
use hkask_types::visibility::Visibility;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct PromptCapturingPort {
    prompts: Arc<Mutex<Vec<String>>>,
    call: Arc<AtomicUsize>,
}

impl PromptCapturingPort {
    fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
        let prompts = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                prompts: prompts.clone(),
                call: Arc::new(AtomicUsize::new(0)),
            },
            prompts,
        )
    }
}

fn result(text: &str) -> InferenceResult {
    InferenceResult {
        text: text.to_string(),
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

impl InferencePort for PromptCapturingPort {
    fn generate(
        &self,
        prompt: &str,
        _params: &LLMParameters,
        _tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        let n = self.call.fetch_add(1, Ordering::SeqCst);
        self.prompts.lock().unwrap().push(prompt.to_string());
        // Step 1 (n==0) emits derived_value; later steps just converge.
        let text = if n == 0 {
            r#"{"derived_value": "FROM_STEP_ONE", "convergence_metric": 0.0}"#
        } else {
            r#"{"convergence_metric": 0.0, "rationale": "ok", "blockers": []}"#
        };
        let r = result(text);
        Box::pin(async move { Ok(r) })
    }

    fn generate_with_model(
        &self,
        prompt: &str,
        params: &LLMParameters,
        model_override: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        let _ = model_override;
        self.generate(prompt, params, tools)
    }
}

fn setup_templates() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("hkask-select-input-mapping-test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(
        dir.join("step1.j2"),
        r#"Produce derived_value.
{"derived_value": "FROM_STEP_ONE", "convergence_metric": 0.0}"#,
    )
    .expect("write step1");
    // step2 references the MAPPED name {{ derived_value }}.
    std::fs::write(
        dir.join("step2.j2"),
        r#"Echo section: {{ derived_value }}
{"convergence_metric": 0.0, "rationale": "ok", "blockers": []}"#,
    )
    .expect("write step2");
    dir
}

#[tokio::test]
async fn select_input_mapping_resolves_cross_step_variable() {
    let dir = setup_templates();
    let (mock, prompts) = PromptCapturingPort::new();
    let executor = ManifestExecutor::new(
        Arc::new(mock),
        Arc::new(NoopMcpPort),
        LLMParameters::default(),
        b"test-secret".to_vec(),
    )
    .with_template_base_path(dir);

    let manifest = BundleManifest {
        id: "select-input-mapping".to_string(),
        name: "Select Input Mapping".to_string(),
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
                description: "produce derived_value".to_string(),
                renderer: Some("minijinja".to_string()),
                template_ref: Some("step1.j2".to_string()),
                mcp: None,
                compute_ref: None,
                gas_cap: 1000,
                timeout_seconds: 10,
                input_mapping: None,
                output_schema: None,
                phase: CascadePhase::Core,
                condition: None,
                fusion: None,
            },
            BundleManifestStep {
                ordinal: 2,
                action: "select".to_string(),
                description: "consume derived_value via input_mapping".to_string(),
                renderer: Some("minijinja".to_string()),
                template_ref: Some("step2.j2".to_string()),
                mcp: None,
                compute_ref: None,
                gas_cap: 1000,
                timeout_seconds: 10,
                // The mapping under test: a {{ }} string resolved from step_1_result.
                input_mapping: Some(serde_json::json!({
                    "derived_value": "{{ step_1_result.derived_value }}"
                })),
                output_schema: None,
                phase: CascadePhase::Core,
                condition: None,
                fusion: None,
            },
            BundleManifestStep {
                ordinal: 3,
                action: "abort".to_string(),
                description: "converged".to_string(),
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
        fusion: None,
    };

    let context = HashMap::new();
    let _ = executor.execute_manifest(&manifest, context).await;

    let captured = prompts.lock().unwrap();
    assert!(
        captured.len() >= 2,
        "step 1 and step 2 should both invoke inference; got {} calls",
        captured.len()
    );
    let step2_prompt = &captured[1];
    assert!(
        step2_prompt.contains("FROM_STEP_ONE"),
        "select input_mapping should resolve {{ step_1_result.derived_value }} into the \
         step-2 template. Rendered step-2 prompt did NOT contain 'FROM_STEP_ONE' — \
         input_mapping for select steps is not being applied.\n--- step2 prompt ---\n{}",
        step2_prompt
    );
}
