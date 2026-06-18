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

pub(crate) mod manifest;
pub(crate) mod error;
pub(crate) mod history;
pub(crate) mod state;
pub(crate) mod metrics;
pub(crate) mod improvement;
pub(crate) mod coaching;
pub(crate) mod starter;
pub(crate) mod execution;

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
    /// REQ: P9-svc-kata-107
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  inference must be a valid InferencePort; registry must be initialized
    /// post: returns KataEngine with inference and registry wired; all optional components (consent, CNS, history, metrics) default to None
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
    /// \[NORMATIVE\] Encapsulates `InferenceConfig::from_env()` and
    /// `InferenceRouter::new()` so CLI and API surfaces don't construct
    /// inference directly (P7 — Evolutionary Architecture).
    ///
    /// REQ: P9-svc-kata-108
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  registry must be initialized; inference env vars must be set or defaults used
    /// post: returns KataEngine with InferenceRouter built from env config
    pub fn from_env(registry: SqliteRegistry) -> Self {
        let inf_cfg = hkask_inference::InferenceConfig::from_env();
        let inference = hkask_inference::InferenceRouter::new(inf_cfg);
        Self::new(Arc::new(inference), registry)
    }

    /// Set a consent checker that gates kata execution.
    ///
    /// REQ: P9-svc-kata-109
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  check must be a valid Fn(&str, &str) -> Result<(), KataError>
    /// post: returns self with consent_check set; kata execution will call check before running
    pub fn with_consent<F>(mut self, check: F) -> Self
    where
        F: Fn(&str, &str) -> Result<(), KataError> + Send + Sync + 'static,
    {
        self.consent_check = Some(Arc::new(check));
        self
    }

    /// Set a CNS observer called after each step completes.
    ///
    /// REQ: P9-svc-kata-110
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  observer must be a valid Fn(&str, u32, &str)
    /// post: returns self with cns_observer set; observer is called after each kata step
    pub fn with_cns<F>(mut self, observer: F) -> Self
    where
        F: Fn(&str, u32, &str) + Send + Sync + 'static,
    {
        self.cns_observer = Some(Arc::new(observer));
        self
    }

    /// Set a kata practice history for habit tracking and automaticity scoring.
    ///
    /// REQ: P9-svc-kata-111
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  history must be a valid KataHistory
    /// post: returns self with history set; starter kata uses it for automaticity computation
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
    /// REQ: P9-svc-kata-112
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  store must be a valid Arc<KataHistoryStore>
    /// post: returns self with history_store set; record_history_entry will persist to SQLite
    pub fn with_history_store(mut self, store: Arc<KataHistoryStore>) -> Self {
        self.history_store = Some(store);
        self
    }

    /// Set a metric collector for before/after measurement.
    ///
    /// REQ: P9-svc-kata-113
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  collector must be a valid Fn(&str, &str) -> Result<Value, KataError>
    /// post: returns self with metric_collector set; improvement kata captures before/after metrics
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
    /// REQ: P9-svc-kata-114
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  cns must be a valid Arc<RwLock<CnsRuntime>>
    /// post: returns self with cns_runtime set; kata cycles will increment variety and check alerts
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
    /// REQ: P9-svc-kata-115
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  agent_name, date, kata_type, practice_name must be non-empty
    /// post: returns Some(row_id) if history_store is set and record succeeds; None if store not configured; Err on store failure
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
    /// REQ: P9-svc-kata-116
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  path must exist and contain valid YAML
    /// post: returns KataManifest deserialized from file; Err(LoadFailed) on I/O error; Err(ParseFailed) on invalid YAML
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
    /// REQ: P9-svc-kata-117
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  manifest must have at least one step for selector; learner_bot must be non-empty
    /// post: returns KataResult from the selected kata execution; Err on selector failure or kata execution error
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
    /// REQ: P9-svc-kata-118
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  manifest.manifest.kata_type must be "improvement", "coaching", or "starter"; learner_bot must be non-empty
    /// post: returns KataResult with steps_completed, gas_consumed, and kata-type-specific outputs; Err(UnknownType) on invalid kata_type
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

    /// Capture metrics declared in the manifest before the kata cycle begins.
    fn capture_before_metrics(&self, manifest: &KataManifest, agent: &str, state: &mut KataState) {
        if manifest.metrics.is_empty() {
            return;
        }
        let Some(collector) = self.metric_collector.as_ref() else {
            return;
        };
        let mut metrics = serde_json::Map::new();
        for m in &manifest.metrics {
            if let Some(ref span) = m.span {
                match collector(agent, span) {
                    Ok(value) => {
                        metrics.insert(m.name.clone(), value);
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "cns.kata",
                            metric = %m.name,
                            error = %e,
                            "Failed to capture before metric"
                        );
                    }
                }
            }
        }
        if !metrics.is_empty() {
            state.metric_before = Some(serde_json::Value::Object(metrics));
        }
    }

    /// Capture metrics after the kata cycle completes.
    fn capture_after_metrics(&self, manifest: &KataManifest, agent: &str, state: &mut KataState) {
        if manifest.metrics.is_empty() {
            return;
        }
        let Some(collector) = self.metric_collector.as_ref() else {
            return;
        };
        let mut metrics = serde_json::Map::new();
        for m in &manifest.metrics {
            if let Some(ref span) = m.span {
                match collector(agent, span) {
                    Ok(value) => {
                        metrics.insert(m.name.clone(), value);
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "cns.kata",
                            metric = %m.name,
                            error = %e,
                            "Failed to capture after metric"
                        );
                    }
                }
            }
        }
        if !metrics.is_empty() {
            state.metric_after = Some(serde_json::Value::Object(metrics));
        }
    }

    /// Compute improvement signal from before/after metrics.
    ///
    /// Produces IS evidence: the measured delta between before and after values.
    /// This is the cybernetic feedback that closes the PDCA loop.
    fn compute_improvement_signal(&self, state: &KataState) -> Option<ImprovementSignal> {
        let before = state.metric_before.as_ref()?;
        let after = state.metric_after.as_ref()?;

        // Compute delta for numeric values
        let delta = match (before, after) {
            (serde_json::Value::Number(b), serde_json::Value::Number(a)) => {
                let bf = b.as_f64()?;
                let af = a.as_f64()?;
                Some(af - bf)
            }
            _ => None,
        };

        let direction = match delta {
            Some(d) if d > 0.0 => ImprovementDirection::Positive,
            Some(d) if d < 0.0 => ImprovementDirection::Negative,
            Some(_) => ImprovementDirection::Stalled,
            None => ImprovementDirection::NotMeasured,
        };

        Some(ImprovementSignal {
            metric_before: Some(before.clone()),
            metric_after: Some(after.clone()),
            delta,
            direction,
        })
    }

    /// Increment a CNS variety counter for a kata practice event.
    ///
    /// When CNS runtime is wired, this replaces tracing-only spans with
    /// actual variety counter state mutations tracked by the Cybernetic
    /// Nervous System.
    async fn increment_cns_variety(&self, domain: &str, state_name: &str) {
        if let Some(ref cns) = self.cns_runtime {
            cns.read().await.increment_variety(domain, state_name).await;
        }
    }

    /// Check CNS algedonic thresholds after a kata cycle and emit alerts if needed.
    async fn check_cns_alerts(&self, manifest: &KataManifest, kata_type: &str) {
        let Some(ref cns) = self.cns_runtime else {
            return;
        };
        let alert = cns
            .read()
            .await
            .check_variety(&manifest.cns.span_namespace)
            .await;
        // P9: CNS span (algedonic alert)
        if let Some(a) = alert {
            tracing::warn!(
                target: "cns.kata",
                namespace = %manifest.cns.span_namespace,
                kata_type = %kata_type,
                severity = ?a.severity,
                deficit = a.deficit,
                threshold = a.threshold,
                "CNS"
            );
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
    ///
    /// REQ: P9-svc-kata-119
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  manifest must have at least one step; state.learner_bot must be non-empty
    /// post: returns KataResult with steps_completed, gas_consumed, and step_experiences; Err(NoSteps) if manifest has no steps; Err(GasExceeded) if gas cap exceeded
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

            // P9: CNS span
            if manifest.cns.emit_spans {
                tracing::info!(
                    target: "cns.kata",
                    namespace = %manifest.cns.span_namespace,
                    step = step.ordinal,
                    action = %step.action,
                    bot = %state.learner_bot,
                    "CNS"
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

            // PDCA Check phase: compare output against declared target/expectations
            let check_result = self.check_step_output(step, &output);
            // P9: CNS span
            if manifest.cns.emit_spans {
                tracing::info!(
                    target: "cns.kata",
                    namespace = %manifest.cns.span_namespace,
                    step = step.ordinal,
                    passed_check = check_result,
                    "CNS"
                );
            }

            state
                .step_outputs
                .insert(step.ordinal.to_string(), output.clone());
            state.gas_consumed += step_gas;
            state.current_step = step.ordinal as usize;

            // Emit structured step experience for memory recording
            let summary = output
                .get("response")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();
            state.step_experiences.push(StepExperience {
                agent: state.learner_bot.clone(),
                kata_type: "improvement".into(),
                step_label: format!("{}", step.ordinal),
                action: step.action.clone(),
                output_summary: summary,
                gas_used: step_gas,
                timestamp: now_rfc3339(),
            });

            // P9: CNS span
            if manifest.cns.emit_spans {
                tracing::info!(
                    target: "cns.kata",
                    namespace = %manifest.cns.span_namespace,
                    step = step.ordinal,
                    gas = state.gas_consumed,
                    "CNS"
                );
            }

            // CNS observer callback
            if let Some(ref obs) = self.cns_observer {
                obs(&manifest.cns.span_namespace, step.ordinal, &step.action);
            }

            // CNS variety counter: record improvement step execution
            self.increment_cns_variety(&manifest.cns.span_namespace, "kata.practices.completed")
                .await;
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
            improvement_signal: None,
            step_experiences: vec![],
            automaticity_delta: None,
        })
    }

    /// PDCA Check phase — compare step output against declared expectations.
    ///
    /// Returns true if the output passes basic validation (contains expected fields),
    /// false if the output appears malformed or empty.
    fn check_step_output(&self, step: &KataStep, output: &serde_json::Value) -> bool {
        // If no output schema is declared, we can't validate — pass by default
        let schema = match &step.output_schema {
            Some(s) => s,
            None => return true,
        };

        // Check for expected properties
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for key in props.keys() {
                if output.get(key).is_none() {
                    // Check if maybe the key is nested under "response"
                    if let Some(resp) = output.get("response") {
                        if resp.get(key).is_none() {
                            tracing::debug!(
                                target: "cns.kata",
                                step = step.ordinal,
                                missing = %key,
                                "Step output missing expected field"
                            );
                            return false;
                        }
                    } else {
                        tracing::debug!(
                                    target: "cns.kata",
                            step = step.ordinal,
                            missing = %key,
                            "Step output missing expected field"
                        );
                        return false;
                    }
                }
            }
        }
        true
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
    ///
    /// If the state contains an `ik_state_ref`, coaching questions are grounded
    /// in the learner's actual Improvement Kata storyboard data.
    ///
    /// REQ: P9-svc-kata-120
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  manifest must have at least one question; state.learner_bot must be non-empty
    /// post: returns KataResult with steps_completed (question count), gas_consumed, and step_experiences; Err(NoSteps) if no questions; Err(GasExceeded) if gas cap exceeded
    pub async fn run_coaching_from(
        &self,
        manifest: &KataManifest,
        state: &mut KataState,
    ) -> Result<KataResult, KataError> {
        let total = manifest.questions.len();
        if total == 0 {
            return Err(KataError::NoSteps(manifest.manifest.id.clone()));
        }

        // Check if we have an IK state reference to ground the coaching
        let ik_context = state.ik_state_ref.clone();

        for q in &manifest.questions {
            // Skip already-completed questions when resuming
            if (q.number as usize) <= state.current_step && !state.step_outputs.is_empty() {
                continue;
            }

            // P9: CNS span
            if manifest.cns.emit_spans {
                tracing::info!(
                    target: "cns.kata",
                    namespace = %manifest.cns.span_namespace,
                    question = q.number,
                    bot = %state.learner_bot,
                    has_ik_state = ik_context.is_some(),
                    "CNS"
                );
            }
            let step_gas = 2000; // coaching questions use default gas
            if state.gas_consumed + step_gas > manifest.gas.cap {
                return Err(KataError::GasExceeded {
                    consumed: state.gas_consumed,
                    cap: manifest.gas.cap,
                });
            }

            // Build coaching prompt from question + accumulated context + IK state data
            let prev_context = state
                .step_outputs
                .iter()
                .map(|(k, v)| {
                    let text = v.get("response").and_then(|r| r.as_str()).unwrap_or("");
                    format!("Q{}: {}", k.trim_start_matches('q'), text)
                })
                .collect::<Vec<_>>()
                .join("\n");

            // Ground coaching in actual IK data when available
            let ik_data_section = match &ik_context {
                Some(ik_ref) => format!(
                    "\nThe learner's current Improvement Kata storyboard:\n{}\n\n",
                    ik_ref
                ),
                None => String::new(),
            };

            let prompt = format!(
                "You are a Toyota Kata coach conducting a 5-question coaching cycle.\n\
                 Your role: ask questions that reveal the learner's thinking pattern.\n\
                 Never give solutions. Never say 'you should'. Only ask.\n\
                 {ik_data}\n\
                 Previous answers from the learner:\n\
                 {prev}\n\n\
                 Now ask Question {n}: {q}\n\
                 Context: {desc}\n\n\
                 Ask the question in a way that makes the learner think.\n\
                 Then, as the learner, respond with specific data and observations\n\
                 from your Improvement Kata storyboard.",
                ik_data = ik_data_section,
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

            // Emit structured step experience for memory recording
            state.step_experiences.push(StepExperience {
                agent: state.learner_bot.clone(),
                kata_type: "coaching".into(),
                step_label: format!("q{}", q.number),
                action: "coaching_question".into(),
                output_summary: response.text.chars().take(200).collect(),
                gas_used: step_gas,
                timestamp: now_rfc3339(),
            });

            // CNS observer callback
            if let Some(ref obs) = self.cns_observer {
                obs(&manifest.cns.span_namespace, q.number, "coaching_question");
            }

            // CNS variety counter: record coaching question asked
            self.increment_cns_variety(&manifest.cns.span_namespace, "kata.practices.completed")
                .await;
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
            improvement_signal: None,
            step_experiences: vec![],
            automaticity_delta: None,
        })
    }

    /// Run a Starter Kata: execute practice routines with habit tracking.
    ///
    /// Each practice is recorded as a structured experience. The engine
    /// tracks practice frequency, computes automaticity, and emits CNS
    /// automaticity signals. No LLM calls — starter kata is pure habit formation.
    ///
    /// REQ: P9-svc-kata-121
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  manifest must have at least one practice; state.learner_bot must be non-empty
    /// post: returns KataResult with steps_completed (practice count), automaticity_delta, and step_experiences; Err(NoSteps) if no practices
    pub async fn run_starter(
        &self,
        manifest: &KataManifest,
        state: &mut KataState,
    ) -> Result<KataResult, KataError> {
        let total = manifest.practices.len();
        if total == 0 {
            return Err(KataError::NoSteps(manifest.manifest.id.clone()));
        }

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Check habit health before starting
        if let Some(ref history) = self.history {
            let auto = history.compute_automaticity(&state.learner_bot, &today);
            let streak = history.current_streak(&state.learner_bot, &today);
            let needs_intervention = history.needs_habit_intervention(&state.learner_bot, &today);

            if manifest.cns.emit_spans {
                // P9: CNS span
                tracing::info!(
                    target: "cns.kata",
                    namespace = %manifest.cns.span_namespace,
                    bot = %state.learner_bot,
                    automaticity = auto,
                    streak_days = streak,
                    needs_intervention = needs_intervention,
                    "CNS"
                );
            }

            // P9: CNS span (algedonic warning)
            if needs_intervention {
                tracing::warn!(
                    target: "cns.kata",
                    namespace = %manifest.cns.span_namespace,
                    bot = %state.learner_bot,
                    days_since_last = history.days_since_last(&state.learner_bot, &today),
                    "CNS"
                );
            }
        }

        for practice in &manifest.practices {
            // Record the practice execution in state with structured metadata
            let output = serde_json::json!({
                "practice": practice.name,
                "description": practice.description,
                "frequency": practice.frequency,
                "duration_minutes": practice.duration_minutes,
                "steps": practice.steps,
                "success_criteria": practice.success_criteria,
                "cns_spans": practice.cns_spans,
                "status": "executed",
                "date": today,
            });
            state
                .step_outputs
                .insert(practice.name.clone(), output.clone());
            state.current_step += 1;

            // Emit structured step experience for memory recording
            state.step_experiences.push(StepExperience {
                agent: state.learner_bot.clone(),
                kata_type: "starter".into(),
                step_label: practice.name.clone(),
                action: "practice_routine".into(),
                output_summary: practice.description.clone(),
                gas_used: 0, // starter kata has no LLM gas cost
                timestamp: now_rfc3339(),
            });

            // P9: CNS span
            if manifest.cns.emit_spans {
                tracing::info!(
                    target: "cns.kata",
                    namespace = %manifest.cns.span_namespace,
                    practice = %practice.name,
                    bot = %state.learner_bot,
                    "CNS"
                );
            }

            // CNS variety counter: record starter practice
            self.increment_cns_variety(&manifest.cns.span_namespace, "kata.practices.completed")
                .await;
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
            improvement_signal: None,
            step_experiences: vec![],
            automaticity_delta: None,
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
        // Convert HashMaps to serde_json Value objects for minijinja iteration
        let context_json = serde_json::to_value(&state.context).unwrap_or(serde_json::Value::Null);
        let steps_json =
            serde_json::to_value(&state.step_outputs).unwrap_or(serde_json::Value::Null);
        let metric_before_json = state
            .metric_before
            .clone()
            .unwrap_or(serde_json::Value::Null);
        let metric_after_json = state
            .metric_after
            .clone()
            .unwrap_or(serde_json::Value::Null);
        let ik_ref_json = serde_json::Value::String(state.ik_state_ref.clone().unwrap_or_default());

        let ctx = minijinja::context! {
            learner_bot => state.learner_bot.clone(),
            previous_steps => steps_json,
            context => context_json,
            metric_before => metric_before_json,
            metric_after => metric_after_json,
            ik_state_ref => ik_ref_json,
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

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: P9-svc-kata-kata-template-001 — templates render with standard context
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

    // REQ: P9-svc-kata-kata-template-002 — templates contain learner_bot reference
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

// ── Errors ─────────────────────────────────────────────────────────────────

// Moved to src/error.rs.
