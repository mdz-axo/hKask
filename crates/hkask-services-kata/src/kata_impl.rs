//! Kata execution engine — cybernetic learning system for agent self-improvement.
//!
//! Implements Mike Rother's Toyota Kata methodology as composable recursive
//! self-improvement tools:
//! - Improvement Kata: 4-step PDCA cycle with closed cybernetic loop
//!   (Understand → Grasp → Target → Experiment → Check → Act)
//! - Coaching Kata: 5-question dialogue grounded in active IK state
//! - Starter Kata: Practice routines with habit tracking and automaticity scoring
//!
//! Every step feeds into the agent's episodic memory stream via structured
//! experience events. Before/after metric capture computes improvement signals.
//! Kata history tracks practice frequency, streaks, and graduation criteria.
//!
//! Manifests are loaded from `registry/manifests/*.yaml`. Templates are rendered
//! via the hKask template registry (Jinja2). Inference uses the centralized router.


use hkask_cns::CnsRuntime;
use hkask_services_core::HkaskSettings;
use hkask_storage::KataHistoryStore;
use hkask_templates::SqliteRegistry;
use hkask_types::ports::InferencePort;
use hkask_types::template::LLMParameters;
use hkask_types::time::now_rfc3339;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) mod coaching;
pub(crate) mod error;
pub(crate) mod execution;
pub(crate) mod history;
pub(crate) mod improvement;
pub(crate) mod manifest;
pub(crate) mod metrics;
pub(crate) mod starter;
pub(crate) mod state;

#[allow(unused_imports)]
pub use error::KataError;
pub use history::{
    ImprovementDirection, ImprovementSignal, KataHistory, PracticeEntry, StepExperience,
};
#[allow(unused_imports)]
pub use manifest::{
    AuditConfig, CnsConfig, CoachQuestion, ErrorHandling, GasConfig, KataManifest, KataStep,
    ManifestMeta, MetricDef, Outcome, PracticeRoutine, StarterOutcome,
};
pub use state::{KataResult, KataState};

// ── Manifest types ─────────────────────────────────────────────────────────

// Moved to src/manifest.rs.

// ── Cybernetic feedback types ───────────────────────────────────────────────

// Moved to src/history.rs.

// ── Execution types ────────────────────────────────────────────────────────

// Moved to src/state.rs.

// ── Engine ─────────────────────────────────────────────────────────────────

/// Consent-checking callback.
pub type ConsentCheckFn = Arc<dyn Fn(&str, &str) -> Result<(), KataError> + Send + Sync>;
/// CNS observer callback.
pub type CnsObserverFn = Arc<dyn Fn(&str, u32, &str) + Send + Sync>;
/// Metric collection callback.
pub type MetricCollectorFn =
    Arc<dyn Fn(&str, &str) -> Result<serde_json::Value, KataError> + Send + Sync>;

/// Execution engine for kata manifests.
///
/// Loads a manifest, walks its steps/questions/practices, renders templates,
/// calls inference, collects before/after metrics, computes improvement signals,
/// tracks habit formation, and accumulates state for memory recording.
pub struct KataEngine {
    inference: Arc<dyn InferencePort>,
    registry: SqliteRegistry,
    /// Optional consent checker — called before kata execution.
    /// Receives (kata_type, learner_bot) and returns Ok(()) if consented.
    consent_check: Option<ConsentCheckFn>,
    /// Optional CNS observer — called after each step with (namespace, step_ordinal, action).
    cns_observer: Option<CnsObserverFn>,
    /// Kata practice history for habit tracking and automaticity scoring.
    history: Option<KataHistory>,
    /// Optional SQLite-backed kata history store for concurrent, queryable persistence.
    history_store: Option<Arc<KataHistoryStore>>,
    /// Optional metric collector — called to capture CNS metrics before/after cycles.
    /// Receives (agent_name, metric_name) and returns the current metric value.
    metric_collector: Option<MetricCollectorFn>,
    /// Optional CNS runtime for variety counter increments and algedonic alert checks.
    /// When present, replaces tracing-only spans with actual CNS state mutations.
    cns_runtime: Option<Arc<RwLock<CnsRuntime>>>,
}

impl KataEngine {
    pub fn new(inference: Arc<dyn InferencePort>, registry: SqliteRegistry) -> Self {
        Self {
            inference,
            registry,
            consent_check: None,
            cns_observer: None,
            history: None,
            history_store: None,
            metric_collector: None,
            cns_runtime: None,
        }
    }

    /// Create a KataEngine with inference configured from environment.
    ///
    /// `InferenceRouter::new()` so CLI and API surfaces don't construct
    /// inference directly (P7 — Evolutionary Architecture).
    ///
    pub fn from_env(registry: SqliteRegistry) -> Self {
        let inf_cfg = hkask_inference::InferenceConfig::from_env();
        let inference = hkask_inference::InferenceRouter::new(inf_cfg);
        Self::new(Arc::new(inference), registry)
    }

    /// Set a consent checker that gates kata execution.
    ///
    pub fn with_consent<F>(mut self, check: F) -> Self
    where
        F: Fn(&str, &str) -> Result<(), KataError> + Send + Sync + 'static,
    {
        self.consent_check = Some(Arc::new(check));
        self
    }

    /// Set a CNS observer called after each step completes.
    ///
    pub fn with_cns<F>(mut self, observer: F) -> Self
    where
        F: Fn(&str, u32, &str) + Send + Sync + 'static,
    {
        self.cns_observer = Some(Arc::new(observer));
        self
    }

    /// Set a kata practice history for habit tracking and automaticity scoring.
    ///
    pub fn with_history(mut self, history: KataHistory) -> Self {
        self.history = Some(history);
        self
    }

    /// Set a SQLite-backed kata history store for concurrent, queryable persistence.
    ///
    /// When present, practice entries are persisted to SQLite in addition to
    /// (or instead of) the JSON file. This enables CNS queries against practice
    /// data and cross-session persistence through the daemon's memory pipeline.
    ///
    pub fn with_history_store(mut self, store: Arc<KataHistoryStore>) -> Self {
        self.history_store = Some(store);
        self
    }

    /// Set a metric collector for before/after measurement.
    ///
    pub fn with_metrics<F>(mut self, collector: F) -> Self
    where
        F: Fn(&str, &str) -> Result<serde_json::Value, KataError> + Send + Sync + 'static,
    {
        self.metric_collector = Some(Arc::new(collector));
        self
    }

    /// Set a CNS runtime for variety counter increments and algedonic alert checks.
    ///
    /// When present, kata execution increments CNS variety counters for each
    /// practice and checks algedonic thresholds after cycle completion.
    ///
    pub fn with_cns_runtime(mut self, cns: Arc<RwLock<CnsRuntime>>) -> Self {
        self.cns_runtime = Some(cns);
        self
    }

    /// Record a practice entry to the SQLite-backed history store, if available.
    ///
    /// This enables concurrent, queryable persistence through the daemon's
    /// memory pipeline. When the store is not set, this is a no-op — the
    /// caller should fall back to JSON-based persistence.
    ///
    pub fn record_history_entry(
        &self,
        agent_name: &str,
        date: &str,
        kata_type: &str,
        practice_name: &str,
        steps_completed: usize,
        gas_consumed: u64,
    ) -> Result<Option<i64>, KataError> {
        if let Some(ref store) = self.history_store {
            let id = store
                .record(
                    agent_name,
                    date,
                    kata_type,
                    practice_name,
                    steps_completed,
                    gas_consumed,
                )
                .map_err(|e| KataError::LoadFailed(format!("History store: {}", e)))?;
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }

    /// Load a kata manifest from a YAML file.
    ///
    pub fn load_manifest(path: &Path) -> Result<KataManifest, KataError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            KataError::LoadFailed(format!("Failed to read {}: {}", path.display(), e))
        })?;
        let manifest: KataManifest = serde_yaml_neo::from_str(&content)
            .map_err(|e| KataError::ParseFailed(format!("Failed to parse manifest: {}", e)))?;
        Ok(manifest)
    }

    /// Run a bundle manifest that orchestrates kata selection and execution.
    ///
    /// Bundle manifests (like kata-pattern.yaml) don't have a fixed kata_type.
    /// Instead, they use a selector template to route to the appropriate kata
    /// based on the agent's history, automaticity, and context.
    ///
    pub async fn run_bundle(
        &self,
        manifest: &KataManifest,
        learner_bot: &str,
        initial_context: HashMap<String, String>,
    ) -> Result<KataResult, KataError> {
        let manifests_dir = std::path::PathBuf::from("registry/manifests");

        // Step 1: Select the appropriate kata type
        let selector_output = if let Some(step) = manifest.steps.first() {
            let state = KataState {
                learner_bot: learner_bot.to_string(),
                context: initial_context.clone(),
                ..Default::default()
            };

            if manifest.cns.emit_spans {
                // P9: CNS span
                tracing::info!(
                    target: "cns.kata",
                    namespace = %manifest.cns.span_namespace,
                    kata_type = "bundle",
                    bot = %learner_bot,
                    "CNS"
                );
            }

            self.execute_step(manifest, step, &state).await?
        } else {
            return Err(KataError::NoSteps(manifest.manifest.id.clone()));
        };

        // Step 2: Route to the selected kata
        let selected = selector_output
            .get("selected_kata")
            .and_then(|v| v.as_str())
            .unwrap_or("starter");

        let kata_manifest_name = match selected {
            "improvement" => "kata-improvement.yaml",
            "coaching" => "kata-coaching.yaml",
            _ => "kata-starter.yaml",
        };

        // P9: CNS span
        tracing::info!(
            target: "cns.kata",
            namespace = %manifest.cns.span_namespace,
            selected = %selected,
            manifest = %kata_manifest_name,
            bot = %learner_bot,
            "CNS"
        );

        // Load and execute the selected kata manifest
        let kata_path = manifests_dir.join(kata_manifest_name);
        let kata_manifest = Self::load_manifest(&kata_path)?;
        self.execute(&kata_manifest, learner_bot, initial_context)
            .await
    }

    /// Execute a full kata cycle.
    ///
    /// Dispatches to the appropriate runner based on `manifest.manifest.kata_type`:
    /// - "improvement" → run improvement steps with before/after metrics
    /// - "coaching" → run coaching questions (requires optional IK state reference)
    /// - "starter" → run practice routines with habit tracking
    ///
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
                // Capture before metrics
                self.capture_before_metrics(manifest, learner_bot, &mut state);

                // P9: CNS span
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "cns.kata",
                        namespace = %manifest.cns.span_namespace,
                        kata_type = "improvement",
                        bot = %learner_bot,
                        "CNS"
                    );
                }
                let mut result = self.run_improvement(manifest, &mut state).await?;

                // Capture after metrics and compute improvement signal
                self.capture_after_metrics(manifest, learner_bot, &mut state);
                let signal = self.compute_improvement_signal(&state);
                result.improvement_signal = signal;
                result.step_experiences = state.step_experiences.clone();

                // CNS algedonic check: is variety deficit exceeding threshold?
                self.check_cns_alerts(manifest, "improvement").await;

                // P9: CNS span
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "cns.kata",
                        namespace = %manifest.cns.span_namespace,
                        steps = result.steps_completed,
                        gas = result.gas_consumed,
                        has_signal = result.improvement_signal.is_some(),
                        "CNS"
                    );
                }
                Ok(result)
            }
            "coaching" => {
                // Learner consent required for Coaching Kata
                if let Some(ref check) = self.consent_check {
                    check("coaching", learner_bot)?;
                }
                // P9: CNS span
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "cns.kata",
                        namespace = %manifest.cns.span_namespace,
                        kata_type = "coaching",
                        bot = %learner_bot,
                        "CNS"
                    );
                }
                let mut result = self.run_coaching(manifest, &mut state).await?;
                result.step_experiences = state.step_experiences.clone();

                // CNS algedonic check: is coaching variety deficit exceeding threshold?
                self.check_cns_alerts(manifest, "coaching").await;

                // P9: CNS span
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "cns.kata",
                        namespace = %manifest.cns.span_namespace,
                        questions = result.steps_completed,
                        gas = result.gas_consumed,
                        "CNS"
                    );
                }
                Ok(result)
            }
            "starter" => {
                // Track automaticity before starting
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let auto_before = self
                    .history
                    .as_ref()
                    .map(|h| h.compute_automaticity(learner_bot, &today))
                    .unwrap_or(0.0);

                // P9: CNS span
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "cns.kata",
                        namespace = %manifest.cns.span_namespace,
                        kata_type = "starter",
                        bot = %learner_bot,
                        automaticity_before = auto_before,
                        "CNS"
                    );
                }
                let mut result = self.run_starter(manifest, &mut state).await?;
                result.step_experiences = state.step_experiences.clone();

                // Compute automaticity delta (history mutation happens in CLI layer)
                let auto_after = self
                    .history
                    .as_ref()
                    .map(|h| h.compute_automaticity(learner_bot, &today))
                    .unwrap_or(0.0);
                result.automaticity_delta = Some(auto_after - auto_before);

                // CNS automaticity measurement: track habit formation progress
                if auto_after > 0.0 {
                    self.increment_cns_variety(
                        &manifest.cns.span_namespace,
                        "kata.automaticity.score",
                    )
                    .await;
                    if auto_after > 0.5 {
                        self.increment_cns_variety(
                            &manifest.cns.span_namespace,
                            "kata.habit.formation",
                        )
                        .await;
                    }
                }

                // CNS algedonic check: is starter practice variety deficit exceeding threshold?
                self.check_cns_alerts(manifest, "starter").await;

                // P9: CNS span
                if manifest.cns.emit_spans {
                    tracing::info!(
                        target: "cns.kata",
                        namespace = %manifest.cns.span_namespace,
                        practices = result.steps_completed,
                        automaticity_after = auto_after,
                        automaticity_delta = result.automaticity_delta,
                        "CNS"
                    );
                }
                Ok(result)
            }
            other => Err(KataError::UnknownType(other.to_string())),
        }
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
        disable_thinking: false,
        adapter: None,
    }
}

// ── Metrics + CNS ──────────────────────────────────────────────────────────
// Moved to metrics.rs — impl blocks loaded via `mod metrics;`.
//
// ── Improvement Kata runner ─────────────────────────────────────────────────
// Moved to improvement.rs.
//
// ── Coaching Kata runner ────────────────────────────────────────────────────
// Moved to coaching.rs.
//
// ── Starter Kata runner ─────────────────────────────────────────────────────
// Moved to starter.rs.
//
// ── Step execution + templates ──────────────────────────────────────────────
// Moved to execution.rs.

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    //
    // All 23 kata templates must render without errors when given
    // a typical KataState with learner_bot, context, and previous_steps.
    #[test]
    fn all_improvement_templates_render_with_context() {
        let state = KataState {
            learner_bot: "TestBot".into(),
            context: [("capability".into(), "span_emission".into())]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        let templates = [
            "kata-improvement/improvement-step1-direction",
            "kata-improvement/improvement-step2-current",
            "kata-improvement/improvement-step3-target",
            "kata-improvement/improvement-step4-experiment",
        ];

        for template_ref in &templates {
            // Path relative to project root (cargo test runs from crate dir)
            let disk_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("registry/templates")
                .join(template_ref)
                .with_extension("j2");
            assert!(
                disk_path.exists(),
                "Template file must exist: {}",
                disk_path.display()
            );

            let content = std::fs::read_to_string(&disk_path).unwrap();
            let env = minijinja::Environment::new();
            let ctx = minijinja::context! {
                learner_bot => &state.learner_bot,
                previous_steps => serde_json::to_value(&state.step_outputs).unwrap(),
                context => serde_json::to_value(&state.context).unwrap(),
                metric_before => serde_json::Value::Null,
                metric_after => serde_json::Value::Null,
                ik_state_ref => serde_json::Value::Null,
            };
            let rendered = env
                .render_str(&content, ctx)
                .unwrap_or_else(|e| panic!("Template {} failed to render: {}", template_ref, e));
            assert!(
                !rendered.is_empty(),
                "Template {} rendered empty output",
                template_ref
            );
        }
    }

    //
    // Every kata template must reference the learner's identity so the
    // LLM knows who it's acting as. Missing {{ learner_bot }} means the
    // template is a static form, not a kata practice prompt.
    #[test]
    fn all_templates_reference_learner_bot() {
        let template_dirs = [
            "registry/templates/kata-improvement",
            "registry/templates/kata-coaching",
            "registry/templates/kata-starter",
            "registry/templates/kata",
        ];

        let mut checked = 0;
        for dir in &template_dirs {
            let dir_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(dir);
            if !dir_path.exists() {
                continue;
            }
            for entry in std::fs::read_dir(dir_path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "j2") {
                    let content = std::fs::read_to_string(&path).unwrap();
                    assert!(
                        content.contains("{{ learner_bot }}"),
                        "Template {} must contain {{{{ learner_bot }}}}",
                        path.display()
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(
            checked, 23,
            "All 23 kata templates must contain learner_bot"
        );
    }
}

// ── Errors ─────────────────────────────────────────────────────────────────

// Moved to src/error.rs.
