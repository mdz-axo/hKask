//! Tests for executor fix A (step.condition: Jinja render + == comparisons) and
//! fix B (loop action binds its input_mapping into context for carry-forward).

use hkask_ports::{ChatToolDefinition, InferenceError, InferencePort, InferenceResult};
use hkask_templates::bundle::cascade::CascadePhase;
use hkask_templates::bundle::config::{
    BundleAuditConfig, BundleCnsConfig, BundleGasConfig, ConvergenceConfig, ErrorHandlingConfig,
    OcapConfig, RjouleConfig,
};
use hkask_templates::bundle::manifest::{BundleManifest, BundleManifestStep};
use hkask_templates::executor::ManifestExecutor;
mod common;
use common::NoopToolPort;
use hkask_types::template::LLMParameters;
use hkask_types::visibility::Visibility;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

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

/// Mock that returns a scripted canned response per inference call (by index)
/// and captures every rendered prompt.
struct ScriptedPort {
    prompts: Arc<Mutex<Vec<String>>>,
    responses: Vec<String>,
    call: AtomicUsize,
}

impl ScriptedPort {
    fn new(responses: Vec<&str>) -> (Self, Arc<Mutex<Vec<String>>>) {
        let prompts = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                prompts: prompts.clone(),
                responses: responses.into_iter().map(String::from).collect(),
                call: AtomicUsize::new(0),
            },
            prompts,
        )
    }
}

impl InferencePort for ScriptedPort {
    fn generate(
        &self,
        prompt: &str,
        _params: &LLMParameters,
        _tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        let n = self.call.fetch_add(1, Ordering::SeqCst);
        self.prompts.lock().unwrap().push(prompt.to_string());
        let text = self
            .responses
            .get(n)
            .cloned()
            .unwrap_or_else(|| r#"{"convergence_metric": 0.0}"#.to_string());
        let r = result(&text);
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

fn step(
    ordinal: u32,
    action: &str,
    template_ref: Option<&str>,
    condition: Option<&str>,
) -> BundleManifestStep {
    BundleManifestStep {
        ordinal,
        action: action.to_string(),
        description: String::new(),
        renderer: Some("minijinja".to_string()),
        template_ref: template_ref.map(str::to_string),
        mcp: None,
        compute_ref: None,
        gas_cap: 1000,
        timeout_seconds: 10,
        input_mapping: None,
        output_schema: None,
        phase: CascadePhase::Core,
        condition: condition.map(str::to_string),
        fusion: None,
    }
}

fn abort_step(ordinal: u32) -> BundleManifestStep {
    BundleManifestStep {
        ordinal,
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
    }
}

fn manifest(steps: Vec<BundleManifestStep>, conv_field: &str) -> BundleManifest {
    BundleManifest {
        id: "test".to_string(),
        name: "Test".to_string(),
        description: String::new(),
        version: "0.31.0".to_string(),
        editor: "test".to_string(),
        visibility: Visibility::Public,
        skills: Vec::new(),
        conflicts: Vec::new(),
        complementarities: Vec::new(),
        steps,
        convergence: ConvergenceConfig {
            threshold: 0.15,
            improvement_ratio: 0.0,
            max_iterations: 3,
            min_iterations: 0,
            convergence_field: conv_field.to_string(),
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
    }
}

fn write_templates(dir: &std::path::Path, files: &[(&str, &str)]) {
    for (name, content) in files {
        std::fs::write(dir.join(name), content).expect("write template");
    }
}

// ── Fix A: `==` comparison conditions genuinely gate steps ───────────────────

#[tokio::test]
async fn condition_eq_runs_step_when_matching() {
    let dir = std::env::temp_dir().join("hkask-cond-match");
    std::fs::create_dir_all(&dir).expect("mkdir");
    write_templates(
        &dir,
        &[
            (
                "s1.j2",
                r#"mode step
{"mode": "plussing"}"#,
            ),
            (
                "s2.j2",
                r#"echo mode={{ mode }}
{"out": "ran"}"#,
            ),
        ],
    );
    let s2 = {
        let mut s = step(
            2,
            "select",
            Some("s2.j2"),
            Some("step_1_result.mode == 'plussing'"),
        );
        s.input_mapping = Some(serde_json::json!({ "mode": "{{ step_1_result.mode }}" }));
        s
    };
    let (mock, prompts) = ScriptedPort::new(vec![r#"{"mode": "plussing"}"#, r#"{"out": "ran"}"#]);
    let executor = ManifestExecutor::new(
        Arc::new(mock),
        Arc::new(NoopToolPort),
        LLMParameters::default(),
        b"test-secret".to_vec(),
    )
    .with_template_base_path(dir);
    let _ = executor
        .execute_manifest(
            &manifest(
                vec![step(1, "select", Some("s1.j2"), None), s2, abort_step(3)],
                "step_1_result.convergence_metric",
            ),
            HashMap::new(),
        )
        .await;
    let captured = prompts.lock().unwrap();
    assert!(
        captured.len() >= 2,
        "step 2 should have run (condition matched)"
    );
    assert!(
        captured[1].contains("plussing"),
        "step 2 should have run and resolved mode; got:\n{}",
        captured[1]
    );
}

#[tokio::test]
async fn condition_eq_skips_step_when_not_matching() {
    let dir = std::env::temp_dir().join("hkask-cond-skip");
    std::fs::create_dir_all(&dir).expect("mkdir");
    write_templates(
        &dir,
        &[
            (
                "s1.j2",
                r#"mode step
{"mode": "plussing"}"#,
            ),
            (
                "s2.j2",
                r#"echo mode={{ mode }}
{"out": "ran"}"#,
            ),
        ],
    );
    let s2 = {
        let mut s = step(
            2,
            "select",
            Some("s2.j2"),
            Some("step_1_result.mode == 'other'"),
        );
        s.input_mapping = Some(serde_json::json!({ "mode": "{{ step_1_result.mode }}" }));
        s
    };
    let (mock, prompts) = ScriptedPort::new(vec![r#"{"mode": "plussing"}"#]);
    let executor = ManifestExecutor::new(
        Arc::new(mock),
        Arc::new(NoopToolPort),
        LLMParameters::default(),
        b"test-secret".to_vec(),
    )
    .with_template_base_path(dir);
    let _ = executor
        .execute_manifest(
            &manifest(
                vec![step(1, "select", Some("s1.j2"), None), s2, abort_step(3)],
                "step_1_result.convergence_metric",
            ),
            HashMap::new(),
        )
        .await;
    let captured = prompts.lock().unwrap();
    assert_eq!(
        captured.len(),
        1,
        "step 2 should be SKIPPED (condition did not match); got {} inference calls",
        captured.len()
    );
}

// ── Fix C (deeper): {{ }} mapping must resolve arrays/objects to real JSON values,
// not stringified reprs that double-encode under | tojson in the template body. ────

#[tokio::test]
async fn select_input_mapping_resolves_arrays_and_objects() {
    let dir = std::env::temp_dir().join("hkask-select-mapping-struct");
    std::fs::create_dir_all(&dir).expect("mkdir");
    write_templates(
        &dir,
        &[
            (
                "s1.j2",
                r#"produce structured data
{"items": [{"id": 1}, {"id": 2}], "config": {"x": 10}, "convergence_metric": 0.0}"#,
            ),
            (
                "s2.j2",
                r#"items={{ items | tojson }} cfg={{ config.x }}
{"convergence_metric": 0.0}"#,
            ),
        ],
    );
    let mut s2 = step(2, "select", Some("s2.j2"), None);
    s2.input_mapping = Some(serde_json::json!({
        "items": "{{ step_1_result.items }}",
        "config": "{{ step_1_result.config }}"
    }));
    let (mock, prompts) = ScriptedPort::new(vec![
        r#"{"items": [{"id": 1}, {"id": 2}], "config": {"x": 10}, "convergence_metric": 0.0}"#,
        r#"{"convergence_metric": 0.0}"#,
    ]);
    let executor = ManifestExecutor::new(
        Arc::new(mock),
        Arc::new(NoopMcpPort),
        LLMParameters::default(),
        b"test-secret".to_vec(),
    )
    .with_template_base_path(dir);
    let _ = executor
        .execute_manifest(
            &manifest(
                vec![step(1, "select", Some("s1.j2"), None), s2, abort_step(3)],
                "step_1_result.convergence_metric",
            ),
            HashMap::new(),
        )
        .await;
    let captured = prompts.lock().unwrap();
    let s2_prompt = &captured[1];
    // If `items` resolved to a real array, | tojson yields [{"id":1}, ...] (unescaped braces,
    // compact — no space after colon). If it were a stringified repr, | tojson would
    // double-encode to "[{\"id\":1}, ...]".
    assert!(
        s2_prompt.contains("{\"id\":1}"),
        "items should resolve to a real array (not a double-encoded string). Prompt:\n{}",
        s2_prompt
    );
    assert!(
        s2_prompt.contains("cfg=10"),
        "config should resolve to a real object so config.x == 10. Prompt:\n{}",
        s2_prompt
    );
}

// ── Fix B: loop input_mapping carries state into the next iteration ──────────

#[tokio::test]
async fn loop_input_mapping_carries_prior_state() {
    let dir = std::env::temp_dir().join("hkask-loop-carry");
    std::fs::create_dir_all(&dir).expect("mkdir");
    write_templates(
        &dir,
        &[
            (
                "s1.j2",
                r#"produce prob
{"prob": 0.5}"#,
            ),
            (
                "s2.j2",
                r#"convergence sees prior={{ prior }}
{"convergence_metric": 1.0}"#,
            ),
        ],
    );
    let mut s2 = step(2, "select", Some("s2.j2"), None);
    s2.input_mapping = Some(serde_json::json!({ "prior": "{{ prior | default(null) }}" }));
    let mut s3 = step(3, "loop", None, None);
    s3.input_mapping =
        Some(serde_json::json!({ "loop_target": 1, "prior": "{{ step_1_result.prob }}" }));
    // Scripted responses per call index:
    //  0: s1 iter1 -> prob 0.5 ; 1: s2 iter1 -> conv 1.0 (not converged)
    //  2: s1 iter2 -> prob 0.9 ; 3: s2 iter2 -> conv 0.0 (converged)
    let (mock, prompts) = ScriptedPort::new(vec![
        r#"{"prob": 0.5}"#,
        r#"{"convergence_metric": 1.0}"#,
        r#"{"prob": 0.9}"#,
        r#"{"convergence_metric": 0.0}"#,
    ]);
    let executor = ManifestExecutor::new(
        Arc::new(mock),
        Arc::new(NoopMcpPort),
        LLMParameters::default(),
        b"test-secret".to_vec(),
    )
    .with_template_base_path(dir);
    let _ = executor
        .execute_manifest(
            &manifest(
                vec![step(1, "select", Some("s1.j2"), None), s2, s3],
                "step_2_result.convergence_metric",
            ),
            HashMap::new(),
        )
        .await;
    let captured = prompts.lock().unwrap();
    // s2 is called twice: iter1 (prior=null) and iter2 (prior=0.5 carried by loop).
    let s2_iter2 = captured
        .iter()
        .filter(|p| p.contains("convergence sees prior"))
        .nth(1);
    assert!(s2_iter2.is_some(), "step 2 should have run on iteration 2");
    assert!(
        s2_iter2.unwrap().contains("prior=0.5"),
        "loop input_mapping should carry step_1_result.prob (0.5) into iteration 2's \
         step 2. Iter-2 step-2 prompt:\n{}",
        s2_iter2.unwrap()
    );
}
