//! Kata execution engine — scientific capability development for agents.
//!
//! Implements Mike Rother's Toyota Kata methodology:
//! - Improvement Kata: 4-step PDCA cycle (Understand → Grasp → Target → Experiment)
//! - Coaching Kata: 5-question dialogue (Coach guides Learner through IK thinking)
//! - Starter Kata: Practice routines for building scientific thinking habits
//!
//! Manifests are loaded from `registry/manifests/*.yaml`. Templates are rendered
//! via the hKask template registry (Jinja2). Inference uses the centralized router.

use crate::settings::HkaskSettings;
use hkask_templates::SqliteRegistry;
use hkask_types::LLMParameters;
use hkask_types::ports::InferencePort;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing;

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
    /// When true, uses the system classifier model (Gemma 4 26B) instead of the generation model.
    #[serde(default)]
    pub classifier: bool,
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// Manifest ID this state belongs to.
    #[serde(default)]
    pub manifest_id: String,
}

impl KataState {
    /// Save state to a JSON file for later resumption.
    pub fn save(&self, path: &Path) -> Result<(), KataError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| KataError::LoadFailed(format!("Failed to serialize state: {}", e)))?;
        std::fs::write(path, &json).map_err(|e| {
            KataError::LoadFailed(format!(
                "Failed to write state to {}: {}",
                path.display(),
                e
            ))
        })?;
        Ok(())
    }

    /// Load state from a previously saved JSON file.
    pub fn load(path: &Path) -> Result<Self, KataError> {
        let json = std::fs::read_to_string(path).map_err(|e| {
            KataError::LoadFailed(format!(
                "Failed to read state from {}: {}",
                path.display(),
                e
            ))
        })?;
        serde_json::from_str(&json)
            .map_err(|e| KataError::ParseFailed(format!("Failed to parse state: {}", e)))
    }
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
    /// Optional consent checker — called before kata execution.
    /// Receives (kata_type, learner_bot) and returns Ok(()) if consented.
    consent_check: Option<Arc<dyn Fn(&str, &str) -> Result<(), KataError> + Send + Sync>>,
    /// Optional CNS observer — called after each step with (namespace, step_ordinal, action).
    cns_observer: Option<Arc<dyn Fn(&str, u32, &str) + Send + Sync>>,
}

impl KataEngine {
    pub fn new(inference: Arc<dyn InferencePort>, registry: SqliteRegistry) -> Self {
        Self {
            inference,
            registry,
            consent_check: None,
            cns_observer: None,
        }
    }

    /// Set a consent checker that gates kata execution.
    pub fn with_consent<F>(mut self, check: F) -> Self
    where
        F: Fn(&str, &str) -> Result<(), KataError> + Send + Sync + 'static,
    {
        self.consent_check = Some(Arc::new(check));
        self
    }

    /// Set a CNS observer called after each step completes.
    pub fn with_cns<F>(mut self, observer: F) -> Self
    where
        F: Fn(&str, u32, &str) + Send + Sync + 'static,
    {
        self.cns_observer = Some(Arc::new(observer));
        self
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
            "improvement" => {
                // Curator consent required for Improvement Kata
                if let Some(ref check) = self.consent_check {
                    check("improvement", learner_bot)?;
                }
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "hkask.kata",
                        namespace = %manifest.cns.span_namespace,
                        kata_type = "improvement",
                        bot = %learner_bot,
                        "kata.cycle.start"
                    );
                }
                let result = self.run_improvement(manifest, &mut state).await?;
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "hkask.kata",
                        namespace = %manifest.cns.span_namespace,
                        steps = result.steps_completed,
                        gas = result.gas_consumed,
                        "kata.cycle.complete"
                    );
                }
                Ok(result)
            }
            "coaching" => {
                // Learner consent required for Coaching Kata
                if let Some(ref check) = self.consent_check {
                    check("coaching", learner_bot)?;
                }
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "hkask.kata",
                        namespace = %manifest.cns.span_namespace,
                        kata_type = "coaching",
                        bot = %learner_bot,
                        "kata.cycle.start"
                    );
                }
                let result = self.run_coaching(manifest, &mut state).await?;
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "hkask.kata",
                        namespace = %manifest.cns.span_namespace,
                        questions = result.steps_completed,
                        gas = result.gas_consumed,
                        "kata.cycle.complete"
                    );
                }
                Ok(result)
            }
            "starter" => {
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "hkask.kata",
                        namespace = %manifest.cns.span_namespace,
                        kata_type = "starter",
                        bot = %learner_bot,
                        "kata.cycle.start"
                    );
                }
                let result = self.run_starter(manifest, &mut state).await?;
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "hkask.kata",
                        namespace = %manifest.cns.span_namespace,
                        practices = result.steps_completed,
                        "kata.cycle.complete"
                    );
                }
                Ok(result)
            }
            other => Err(KataError::UnknownType(other.to_string())),
        }
    }

    /// Run an Improvement Kata: walk 4 steps, render templates, call LLM.
    async fn run_improvement(
        &self,
        manifest: &KataManifest,
        state: &mut KataState,
    ) -> Result<KataResult, KataError> {
        self.run_improvement_from(manifest, state).await
    }

    /// Resume an Improvement Kata from saved state, skipping completed steps.
    pub async fn run_improvement_from(
        &self,
        manifest: &KataManifest,
        state: &mut KataState,
    ) -> Result<KataResult, KataError> {
        let total_steps = manifest.steps.len();
        if total_steps == 0 {
            return Err(KataError::NoSteps(manifest.manifest.id.clone()));
        }

        for step in &manifest.steps {
            // Skip already-completed steps when resuming
            if (step.ordinal as usize) <= state.current_step && !state.step_outputs.is_empty() {
                continue;
            }

            // CNS span: step start
            if manifest.cns.emit_spans {
                tracing::info!(
                    target: "hkask.kata",
                    namespace = %manifest.cns.span_namespace,
                    step = step.ordinal,
                    action = %step.action,
                    bot = %state.learner_bot,
                    "kata.step.start"
                );
            }

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

            // CNS span: step complete
            if manifest.cns.emit_spans {
                tracing::info!(
                    target: "hkask.kata",
                    namespace = %manifest.cns.span_namespace,
                    step = step.ordinal,
                    gas = state.gas_consumed,
                    "kata.step.complete"
                );
            }

            // CNS observer callback
            if let Some(ref obs) = self.cns_observer {
                obs(&manifest.cns.span_namespace, step.ordinal, &step.action);
            }
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
        self.run_coaching_from(manifest, state).await
    }

    /// Resume a Coaching Kata from saved state, skipping completed questions.
    pub async fn run_coaching_from(
        &self,
        manifest: &KataManifest,
        state: &mut KataState,
    ) -> Result<KataResult, KataError> {
        let total = manifest.questions.len();
        if total == 0 {
            return Err(KataError::NoSteps(manifest.manifest.id.clone()));
        }

        for q in &manifest.questions {
            // Skip already-completed questions when resuming
            if (q.number as usize) <= state.current_step && !state.step_outputs.is_empty() {
                continue;
            }

            // CNS span
            if manifest.cns.emit_spans {
                tracing::info!(
                    target: "hkask.kata",
                    namespace = %manifest.cns.span_namespace,
                    question = q.number,
                    bot = %state.learner_bot,
                    "kata.coaching.question"
                );
            }
            let step_gas = 2000; // coaching questions use default gas
            if state.gas_consumed + step_gas > manifest.gas.cap {
                return Err(KataError::GasExceeded {
                    consumed: state.gas_consumed,
                    cap: manifest.gas.cap,
                });
            }

            // Build coaching prompt from question + accumulated context
            let prev_context = state
                .step_outputs
                .iter()
                .map(|(k, v)| {
                    let text = v.get("response").and_then(|r| r.as_str()).unwrap_or("");
                    format!("Q{}: {}", k.trim_start_matches('q'), text)
                })
                .collect::<Vec<_>>()
                .join("\n");

            let prompt = format!(
                "You are a Toyota Kata coach conducting a 5-question coaching cycle.\n\
                 Your role: ask questions that reveal the learner's thinking pattern.\n\
                 Never give solutions. Never say 'you should'. Only ask.\n\n\
                 Previous answers from the learner:\n\
                 {prev}\n\n\
                 Now ask Question {n}: {q}\n\
                 Context: {desc}\n\n\
                 Ask the question in a way that makes the learner think.\n\
                 Then, as the learner, respond with specific data and observations\n\
                 from your Improvement Kata storyboard.",
                prev = if prev_context.is_empty() {
                    "(first question — no prior answers)"
                } else {
                    &prev_context
                },
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

            // CNS observer callback
            if let Some(ref obs) = self.cns_observer {
                obs(&manifest.cns.span_namespace, q.number, "coaching_question");
            }
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
    pub async fn run_starter(
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

        // Call inference — use classifier model for classification steps
        let result = if step.classifier {
            let cls_model = HkaskSettings::load().classifier_model();
            // Route classifier to DeepInfra (model name lacks provider prefix)
            let routed = format!("DI/{}", cls_model);
            self.inference
                .generate_with_model(&prompt, &default_llm_params(), Some(&routed))
                .await
                .map_err(|e| KataError::InferenceFailed(format!("Step {}: {}", step.ordinal, e)))?
        } else {
            self.inference
                .generate(&prompt, &default_llm_params())
                .await
                .map_err(|e| KataError::InferenceFailed(format!("Step {}: {}", step.ordinal, e)))?
        };

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
