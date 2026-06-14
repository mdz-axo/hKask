//! Kata execution engine — scientific capability development for agents.
//!
//! Implements Mike Rother's Toyota Kata methodology:
//! - Improvement Kata: 4-step PDCA cycle (Understand → Grasp → Target → Experiment)
//! - Coaching Kata: 5-question dialogue (Coach guides Learner through IK thinking)
//! - Starter Kata: Practice routines for building scientific thinking habits
//!
//! Manifests are loaded from `registry/manifests/*.yaml`. Templates are rendered
//! via the hKask template registry (Jinja2). Inference uses the centralized router.

use hkask_templates::SqliteRegistry;
use hkask_types::LLMParameters;
use hkask_types::ports::InferencePort;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

// ── Manifest types ─────────────────────────────────────────────────────────

/// Top-level kata manifest deserialized from YAML.
#[derive(Debug, Clone, Deserialize)]
pub struct KataManifest {
    pub manifest: ManifestMeta,
    pub gas: GasConfig,
    #[serde(default)]
    pub steps: Vec<KataStep>,
    #[serde(default)]
    pub questions: Vec<CoachQuestion>,
    #[serde(default)]
    pub practices: Vec<PracticeRoutine>,
    #[serde(default)]
    pub error_handling: ErrorHandling,
    pub cns: CnsConfig,
    #[serde(default)]
    pub outcomes: Vec<Outcome>,
    #[serde(default)]
    pub metrics: Vec<MetricDef>,
    #[serde(default)]
    pub starter_outcomes: Vec<StarterOutcome>,
    #[serde(default)]
    pub audit: AuditConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestMeta {
    pub id: String,
    pub name: String,
    pub kata_type: String,
    pub description: String,
    #[serde(default)]
    pub editor: String,
    #[serde(default)]
    pub visibility: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GasConfig {
    pub cap: u64,
    #[serde(default = "default_cost_per_token")]
    pub cost_per_token: f64,
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f64,
    #[serde(default = "default_hard_limit")]
    pub hard_limit: bool,
}

fn default_cost_per_token() -> f64 {
    0.25
}
fn default_alert_threshold() -> f64 {
    0.7
}
fn default_hard_limit() -> bool {
    true
}

/// A single step in an Improvement Kata cycle.
#[derive(Debug, Clone, Deserialize)]
pub struct KataStep {
    pub ordinal: u32,
    pub action: String,
    pub description: String,
    #[serde(default)]
    pub renderer: Option<String>,
    #[serde(default)]
    pub template_ref: Option<String>,
    #[serde(default)]
    pub model_tier: Option<String>,
    #[serde(default)]
    pub gas_cap: Option<u64>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub mcp: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
}

/// A single question in a Coaching Kata dialogue.
#[derive(Debug, Clone, Deserialize)]
pub struct CoachQuestion {
    pub number: u32,
    pub question: String,
    pub description: String,
    #[serde(default)]
    pub cns_span: Option<String>,
    #[serde(default)]
    pub expected_output: Option<String>,
}

/// A practice routine in a Starter Kata.
#[derive(Debug, Clone, Deserialize)]
pub struct PracticeRoutine {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub frequency: Option<String>,
    #[serde(default)]
    pub duration_minutes: Option<u32>,
    #[serde(default)]
    pub cns_spans: Vec<String>,
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default)]
    pub success_criteria: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorHandling {
    #[serde(default)]
    pub on_gas_exceeded: Option<String>,
    #[serde(default)]
    pub on_timeout: Option<String>,
    #[serde(default)]
    pub max_retries: Option<u32>,
    #[serde(default)]
    pub retry_backoff_seconds: Option<u64>,
}

impl Default for ErrorHandling {
    fn default() -> Self {
        Self {
            on_gas_exceeded: Some("abort".into()),
            on_timeout: Some("retry".into()),
            max_retries: Some(2),
            retry_backoff_seconds: Some(1),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CnsConfig {
    #[serde(default = "default_true")]
    pub emit_spans: bool,
    pub span_namespace: String,
    #[serde(default = "default_true")]
    pub variety_monitoring: bool,
    #[serde(default)]
    pub algedonic_threshold: Option<u64>,
    #[serde(default)]
    pub escalation_target: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct Outcome {
    pub name: String,
    pub condition: String,
    pub action: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub span: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StarterOutcome {
    pub name: String,
    pub condition: String,
    pub action: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub log_level: Option<String>,
    #[serde(default = "default_true")]
    pub include_input: bool,
    #[serde(default = "default_true")]
    pub include_output: bool,
    #[serde(default = "default_true")]
    pub include_gas_cost: bool,
    #[serde(default = "default_true")]
    pub include_cns_events: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: Some("info".into()),
            include_input: true,
            include_output: true,
            include_gas_cost: true,
            include_cns_events: true,
        }
    }
}

// ── Execution types ────────────────────────────────────────────────────────

/// Accumulated state during kata execution.
#[derive(Debug, Clone, Default, Serialize)]
pub struct KataState {
    /// Step outputs keyed by ordinal ("1", "2", ...) or question number.
    pub step_outputs: HashMap<String, serde_json::Value>,
    /// The learner bot's identity.
    pub learner_bot: String,
    /// Free-form context passed between steps.
    pub context: HashMap<String, String>,
    /// Total gas consumed so far.
    pub gas_consumed: u64,
    /// Current step index.
    pub current_step: usize,
}

/// Result of executing a full kata cycle.
#[derive(Debug, Clone, Serialize)]
pub struct KataResult {
    pub manifest_id: String,
    pub kata_type: String,
    pub steps_completed: usize,
    pub total_steps: usize,
    pub gas_consumed: u64,
    pub gas_cap: u64,
    pub state: KataState,
    pub outcome: Option<String>,
}

// ── Engine ─────────────────────────────────────────────────────────────────

/// Execution engine for kata manifests.
///
/// Loads a manifest, walks its steps/questions/practices, renders templates,
/// calls inference, and accumulates state.
pub struct KataEngine {
    inference: Arc<dyn InferencePort>,
    registry: SqliteRegistry,
}

impl KataEngine {
    pub fn new(inference: Arc<dyn InferencePort>, registry: SqliteRegistry) -> Self {
        Self {
            inference,
            registry,
        }
    }

    /// Load a kata manifest from a YAML file.
    pub fn load_manifest(path: &Path) -> Result<KataManifest, KataError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            KataError::LoadFailed(format!("Failed to read {}: {}", path.display(), e))
        })?;
        let manifest: KataManifest = serde_yaml::from_str(&content)
            .map_err(|e| KataError::ParseFailed(format!("Failed to parse manifest: {}", e)))?;
        Ok(manifest)
    }

    /// Execute a full kata cycle.
    ///
    /// Dispatches to the appropriate runner based on `manifest.manifest.kata_type`:
    /// - "improvement" → run improvement steps
    /// - "coaching" → run coaching questions
    /// - "starter" → run practice routines
    pub async fn execute(
        &self,
        manifest: &KataManifest,
        learner_bot: &str,
        initial_context: HashMap<String, String>,
    ) -> Result<KataResult, KataError> {
        let mut state = KataState {
            learner_bot: learner_bot.to_string(),
            context: initial_context,
            ..Default::default()
        };

        match manifest.manifest.kata_type.as_str() {
            "improvement" => self.run_improvement(manifest, &mut state).await,
            "coaching" => self.run_coaching(manifest, &mut state).await,
            "starter" => self.run_starter(manifest, &mut state).await,
            other => Err(KataError::UnknownType(other.to_string())),
        }
    }

    /// Run an Improvement Kata: walk 4 steps, render templates, call LLM.
    async fn run_improvement(
        &self,
        manifest: &KataManifest,
        state: &mut KataState,
    ) -> Result<KataResult, KataError> {
        let total_steps = manifest.steps.len();
        if total_steps == 0 {
            return Err(KataError::NoSteps(manifest.manifest.id.clone()));
        }

        for step in &manifest.steps {
            // Gas gate
            let step_gas = step.gas_cap.unwrap_or(2000);
            if state.gas_consumed + step_gas > manifest.gas.cap {
                return Err(KataError::GasExceeded {
                    consumed: state.gas_consumed,
                    cap: manifest.gas.cap,
                });
            }

            let output = self.execute_step(manifest, step, state).await?;
            state.step_outputs.insert(step.ordinal.to_string(), output);
            state.gas_consumed += step_gas;
            state.current_step = step.ordinal as usize;
        }

        Ok(KataResult {
            manifest_id: manifest.manifest.id.clone(),
            kata_type: "improvement".into(),
            steps_completed: total_steps,
            total_steps,
            gas_consumed: state.gas_consumed,
            gas_cap: manifest.gas.cap,
            state: state.clone(),
            outcome: None,
        })
    }

    /// Run a Coaching Kata: walk 5 questions, each is a prompt→LLM→response cycle.
    async fn run_coaching(
        &self,
        manifest: &KataManifest,
        state: &mut KataState,
    ) -> Result<KataResult, KataError> {
        let total = manifest.questions.len();
        if total == 0 {
            return Err(KataError::NoSteps(manifest.manifest.id.clone()));
        }

        for q in &manifest.questions {
            let step_gas = 2000; // coaching questions use default gas
            if state.gas_consumed + step_gas > manifest.gas.cap {
                return Err(KataError::GasExceeded {
                    consumed: state.gas_consumed,
                    cap: manifest.gas.cap,
                });
            }

            // Build coaching prompt from question + accumulated context
            let prompt = format!(
                "You are coaching a learner through the Improvement Kata.\n\
                 Previous context:\n{prev}\n\n\
                 Question {n}: {q}\n\
                 Description: {desc}\n\n\
                 Respond as the learner would — be specific, data-driven, and honest.",
                prev = state
                    .step_outputs
                    .iter()
                    .map(|(k, v)| format!("  Step {}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n"),
                n = q.number,
                q = q.question,
                desc = q.description,
            );

            let response = self
                .inference
                .generate(&prompt, &default_llm_params())
                .await
                .map_err(|e| {
                    KataError::InferenceFailed(format!("Coaching Q{}: {}", q.number, e))
                })?;

            state.step_outputs.insert(
                format!("q{}", q.number),
                serde_json::json!({"response": response.text, "question": q.question}),
            );
            state.gas_consumed += step_gas;
            state.current_step = q.number as usize;
        }

        Ok(KataResult {
            manifest_id: manifest.manifest.id.clone(),
            kata_type: "coaching".into(),
            steps_completed: total,
            total_steps: total,
            gas_consumed: state.gas_consumed,
            gas_cap: manifest.gas.cap,
            state: state.clone(),
            outcome: None,
        })
    }

    /// Run a Starter Kata: execute practice routines (no LLM calls — habit formation).
    async fn run_starter(
        &self,
        manifest: &KataManifest,
        state: &mut KataState,
    ) -> Result<KataResult, KataError> {
        let total = manifest.practices.len();
        if total == 0 {
            return Err(KataError::NoSteps(manifest.manifest.id.clone()));
        }

        for practice in &manifest.practices {
            // Starter kata practices are habit-forming routines — no LLM calls.
            // Record the practice execution in state.
            state.step_outputs.insert(
                practice.name.clone(),
                serde_json::json!({
                    "practice": practice.name,
                    "steps": practice.steps,
                    "success_criteria": practice.success_criteria,
                    "status": "executed",
                }),
            );
            state.current_step += 1;
        }

        Ok(KataResult {
            manifest_id: manifest.manifest.id.clone(),
            kata_type: "starter".into(),
            steps_completed: total,
            total_steps: total,
            gas_consumed: 0, // starter kata has no LLM gas cost
            gas_cap: manifest.gas.cap,
            state: state.clone(),
            outcome: None,
        })
    }

    /// Execute a single Improvement Kata step: render template → call LLM → validate.
    async fn execute_step(
        &self,
        _manifest: &KataManifest,
        step: &KataStep,
        state: &KataState,
    ) -> Result<serde_json::Value, KataError> {
        let template_ref = step.template_ref.as_deref().unwrap_or("");

        // Build prompt: render template if available, otherwise use description
        let prompt = if !template_ref.is_empty() {
            self.render_template(template_ref, state)?
        } else {
            step.description.clone()
        };

        // Call inference
        let result = self
            .inference
            .generate(&prompt, &default_llm_params())
            .await
            .map_err(|e| KataError::InferenceFailed(format!("Step {}: {}", step.ordinal, e)))?;

        let response = result.text;

        // Try to parse as JSON if output_schema is defined
        if let Some(ref _schema) = step.output_schema {
            match serde_json::from_str::<serde_json::Value>(&response) {
                Ok(json) => return Ok(json),
                Err(_) => {
                    // Not valid JSON — wrap the text response
                    return Ok(serde_json::json!({"response": response}));
                }
            }
        }

        Ok(serde_json::json!({"response": response}))
    }

    /// Render a Jinja2 template with the current kata state as context.
    ///
    /// First tries the SQLite registry, then falls back to reading from
    /// `registry/templates/{template_ref}` on disk.
    fn render_template(&self, template_ref: &str, state: &KataState) -> Result<String, KataError> {
        let ctx = minijinja::context! {
            learner_bot => state.learner_bot.clone(),
            previous_steps => state.step_outputs.clone(),
            context => state.context.clone(),
        };

        // Try registry first, then disk fallback
        let template_content = match self.registry.get_entry(template_ref) {
            Ok(entry) => std::fs::read_to_string(&entry.source_path).map_err(|e| {
                KataError::TemplateNotFound(format!(
                    "Failed to read template '{}' at {}: {}",
                    template_ref, entry.source_path, e
                ))
            })?,
            Err(_) => {
                // Disk fallback: try registry/templates/{template_ref} and {template_ref}.j2
                let disk_path = std::path::PathBuf::from("registry/templates").join(template_ref);
                let with_ext = disk_path.with_extension("j2");
                let read_path = if with_ext.exists() {
                    &with_ext
                } else {
                    &disk_path
                };
                std::fs::read_to_string(read_path).map_err(|_| {
                    KataError::TemplateNotFound(format!(
                        "Template '{}' not found in registry or at {} or {}",
                        template_ref,
                        disk_path.display(),
                        with_ext.display()
                    ))
                })?
            }
        };

        let env = minijinja::Environment::new();
        let rendered = env
            .render_str(&template_content, ctx)
            .map_err(|e| KataError::TemplateNotFound(format!("Render failed: {}", e)))?;

        Ok(rendered)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Default LLM parameters for kata execution.
fn default_llm_params() -> LLMParameters {
    LLMParameters {
        temperature: 0.3,
        top_p: 0.9,
        top_k: 40,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        min_p: 0.0,
        typical_p: 0.0,
        max_tokens: 512,
        seed: None,
    }
}

// ── Errors ─────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum KataError {
    #[error("Failed to load manifest: {0}")]
    LoadFailed(String),
    #[error("Failed to parse manifest: {0}")]
    ParseFailed(String),
    #[error("Unknown kata type: {0}")]
    UnknownType(String),
    #[error("Manifest '{0}' has no steps/questions/practices")]
    NoSteps(String),
    #[error("Gas exceeded: consumed {consumed}, cap {cap}")]
    GasExceeded { consumed: u64, cap: u64 },
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
}
