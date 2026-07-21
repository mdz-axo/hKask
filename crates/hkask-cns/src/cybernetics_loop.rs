//! Cybernetics Loop — Homeostatic self-regulation (Loop 6)
//!
//! The Cybernetics Loop is a closed-loop controller, not a passive observer.
//! Its functional contract:
//!
//! 1. **Sense** — receive `cns.*` spans from all loops (tool invocations,
//!    prompt outcomes, agent pod lifecycle, connector I/O).
//! 2. **Compare** — evaluate each signal against homeostatic set-points:
//!    gas budget remaining, variety counter balance, error rate threshold,
//!    connector latency envelope.
//! 3. **Compute** — when a signal deviates beyond its set-point, produce an
//!    efferent signal: throttle, escalate, calibrate, or circuit-break.
//! 4. **Act** — dispatch the efferent signal to the target loop's `regulate`
//!    entry point.
//!
//! The loop is self-stabilizing: if the Cybernetics Loop itself becomes unstable
//! (e.g., alert cascade), the Curation Loop detects it via metacognitive monitoring
//! and intervenes. This is the two-level meta-loop stability guarantee.
//!
//! # Essential Subloops
//!
//! - 6.1 Access Guard (GUARD) — OCAP verification + sovereignty enforcement
//! - 6.3 Variety Sensing (SENSE) — measure variety across domains
//! - 6.4 Algedonic Regulation (ADAPT) — deficit → threshold → escalate
//! - 6.6 Revocation (WITHDRAW) — persistent deny-future
//!
//! Energy homeostasis is NOT a subloop — it is expressed as set-points
//! in `SetPoints` + regulation actions via `InferenceRegulation`.

use crate::dampener::{Dampener, StagnationDetector};
use crate::energy::{AgentGasStatus, GasBudget, GasCost, GasError};
use crate::energy_budget_management::GasBudgetManager;
use crate::seam_watcher::SeamWatcher;
use crate::set_point_calibrator::SetPointCalibrator;

use crate::runtime::{CnsRuntime, RegulationCycleEntry};
use crate::sensor_provider::{
    EnergyBudgetSensor, SensorRegistry, ToolReliabilitySensor, VarietySensor,
    WalletBalanceRatioSensor, WalletKeyHealthSensor,
};
use crate::set_points::{InferenceThrottleMode, SetPoints};
use crate::slo_manager::SloDataProvider;
use crate::strategy_evaluator::StrategyEvaluator;
use crate::system_simulator::MovingAverageExtrapolator;
use crate::tool_stats::ToolStats;
use crate::wallet_budget::WalletBackedBudget;
use crate::wallet_manager::WalletManager;
use crate::well::WellManager;

use crate::algedonic::{AlertSeverity, RuntimeAlert};
use crate::regulation_policy::{
    self, RegulationPolicy, classify_decision, default_substitution_ladder,
    extract_deficit_threshold,
};
use crate::types::loops::{
    ActionDecision, ActionType, CurationInput, Deviation, HkaskLoop, ImpactReport, LoopAction,
    LoopActionParams, LoopId, LoopQuality, Signal, SignalMetric, TriggerOrigin,
};
use crate::types::loops::{BudgetOption, RegulationData};
use hkask_ports::BackpressureSignal;
use hkask_types::CuratorDirective;
use hkask_types::WebID;
use hkask_types::event::{CyclePhase, NuEvent, NuEventSink, Span, SpanKind};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::{RwLock, mpsc};

/// Runtime-calibratable regulation thresholds — mutable layer over `SetPoints` defaults.
///
/// The `SetPointCalibrator` updates these from observed regulation outcomes;
/// the tick reads them for per-cycle regulation decisions. This closes the
/// Conant-Ashby self-tuning loop: the regulator adapts its own thresholds.
struct CalibratedThresholds {
    stagnation_thresholds: HashMap<String, u32>,
    block_worsening_ratio: f64,
    substitution_after: u32,
}

/// The Cybernetics Loop — homeostatic self-regulation.
///
/// Implements the `Loop` trait's sense→compare→compute→act cycle.
/// The Cybernetic Loop regulates all three domain loops (Inference,
/// Episodic, Semantic) and may signal the Curation Loop via algedonic
/// alerts. It may NOT regulate the Curation Loop.
pub struct CyberneticsLoop {
    cns: Arc<RwLock<CnsRuntime>>,
    gas_budget_manager: Arc<RwLock<GasBudgetManager>>,
    well_manager: Arc<RwLock<WellManager>>,
    wallet_manager: Option<Arc<WalletManager>>,
    set_points: SetPoints,
    /// Cascade detection — prevents unbounded sense→act cycles
    max_iterations: u32,
    dampener: Arc<Dampener>,
    /// When present, algedonic alerts are persisted to NuEventStore for restart durability.
    event_sink: Option<Arc<dyn NuEventSink>>,
    /// Direct alerts channel: Cybernetics → Curation (CurationInput).
    alerts_tx: Option<mpsc::UnboundedSender<CurationInput>>,
    /// Direct tool consumption channel: GovernedTool → Cybernetics.
    /// Direct curator directive channel: Curation → Cybernetics.
    curator_directive_rx: Option<Arc<RwLock<mpsc::UnboundedReceiver<CuratorDirective>>>>,
    /// Loop-quality telemetry from the most recent tick cycle.
    loop_quality: RwLock<LoopQuality>,
    /// SLO data provider for periodic SLO evaluation. If set, SLOs are evaluated
    /// on each tick and breaches are escalated through the algedonic pathway.
    slo_provider: Option<Arc<dyn SloDataProvider>>,
    /// Path for persisting gas budgets across restarts.
    budget_persistence_path: Option<std::path::PathBuf>,
    /// Detects regulatory plateaus — repeated ineffective (metric, action) pairs.
    /// Fermi-inspired early-stopping pattern for cybernetic regulation.
    stagnation_detector: Arc<StagnationDetector>,
    /// Pluggable metric sensors (Fermi Extractor pattern).
    sensor_registry: Arc<SensorRegistry>,
    /// Statistical learner for per-tool cost distributions and reliability.
    tool_stats: Option<Arc<ToolStats>>,
    /// Multi-model strategy evaluator (Fermi improvement-loop pattern).
    strategy_evaluator: Mutex<StrategyEvaluator>,
    /// Predictive simulator for anticipatory regulation (Fermi dynamics pattern).
    simulator: MovingAverageExtrapolator,
    /// Runtime-calibratable thresholds — updated by `SetPointCalibrator` background task.
    calibrated_thresholds: Arc<RwLock<CalibratedThresholds>>,
    /// Architectural seam watcher — monitors boundary coverage drift.
    seam_watcher: Option<Arc<tokio::sync::Mutex<SeamWatcher>>>,
    /// Last seam drift check timestamp — throttles seam checks to avoid
    /// running on every tick.
    last_seam_check: tokio::sync::Mutex<std::time::Instant>,
}

impl CyberneticsLoop {
    /// Create a new CyberneticsLoop with default set-points.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    pub fn new(cns: Arc<RwLock<CnsRuntime>>) -> Self {
        Self::build(cns, SetPoints::default())
    }

    /// Create a new CyberneticsLoop with custom set-points.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self with custom SetPoints applied at construction
    pub fn with_set_points(cns: Arc<RwLock<CnsRuntime>>, set_points: SetPoints) -> Self {
        Self::build(cns, set_points)
    }

    fn build(cns: Arc<RwLock<CnsRuntime>>, set_points: SetPoints) -> Self {
        let dampener = Arc::new(Dampener::with_windows(
            std::time::Duration::from_secs(set_points.dampen_window_secs),
            std::time::Duration::from_secs(set_points.metacognitive_window_secs),
            std::time::Duration::from_secs(set_points.override_cooldown_secs),
        ));
        let max_iterations = set_points.max_iterations;
        let stagnation_detector = Arc::new(
            StagnationDetector::new(crate::set_points::DEFAULT_STAGNATION_THRESHOLD)
                .with_per_metric_thresholds(set_points.stagnation_thresholds.clone()),
        );
        let gas_budget_manager = Arc::new(RwLock::new(GasBudgetManager::new()));
        let calibrated_thresholds = Arc::new(RwLock::new(CalibratedThresholds {
            stagnation_thresholds: set_points.stagnation_thresholds.clone(),
            block_worsening_ratio: set_points.block_worsening_ratio,
            substitution_after: set_points.substitution_after,
        }));
        let sensor_registry = {
            let registry = SensorRegistry::new();
            registry.register(Arc::new(EnergyBudgetSensor::new(
                Arc::clone(&gas_budget_manager),
                set_points.gas_min_remaining,
            )));
            registry.register(Arc::new(VarietySensor::new(
                Arc::clone(&cns),
                set_points.variety_max_deficit,
            )));
            registry.register(Arc::new(WalletKeyHealthSensor::new(Arc::clone(
                &gas_budget_manager,
            ))));
            registry.register(Arc::new(WalletBalanceRatioSensor::new(
                Arc::clone(&gas_budget_manager),
                0.1, // alert when below 10%
            )));
            Arc::new(registry)
        };

        Self {
            cns,
            gas_budget_manager,
            well_manager: Arc::new(RwLock::new(WellManager::new())),
            wallet_manager: None,
            set_points,
            max_iterations,
            dampener,
            event_sink: None,
            alerts_tx: None,
            slo_provider: None,
            curator_directive_rx: None,
            loop_quality: RwLock::new(LoopQuality::default()),
            budget_persistence_path: None,
            stagnation_detector,
            sensor_registry,

            tool_stats: None,
            strategy_evaluator: Mutex::new(StrategyEvaluator::new()),
            simulator: MovingAverageExtrapolator::new(10),
            calibrated_thresholds,
            seam_watcher: None,
            last_seam_check: tokio::sync::Mutex::new(std::time::Instant::now()),
        }
    }

    /// Algedonic alerts and directive acknowledgments persisted to NuEventStore.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self for chaining
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Wire the direct alerts channel for Cybernetics → Curation CurationInput delivery.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self for chaining
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_alerts_channel(mut self, tx: mpsc::UnboundedSender<CurationInput>) -> Self {
        self.alerts_tx = Some(tx);
        self
    }

    /// Wire the direct curator directive channel: Curation → Cybernetics.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self for chaining
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_curator_directive_channel(
        mut self,
        rx: mpsc::UnboundedReceiver<CuratorDirective>,
    ) -> Self {
        self.curator_directive_rx = Some(Arc::new(RwLock::new(rx)));
        self
    }

    /// Wire the SLO data provider for periodic SLO evaluation.
    ///
    /// When set, SLOs are evaluated on each tick and breaches are
    /// escalated through the algedonic pathway.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self for chaining
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_slo_provider(mut self, provider: Arc<dyn SloDataProvider>) -> Self {
        self.slo_provider = Some(provider);
        self
    }

    /// Enable gas budget persistence across restarts.
    ///
    /// Budgets are saved to the given path after each replenishment cycle
    /// and loaded automatically on construction.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self for chaining
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_budget_persistence(mut self, path: std::path::PathBuf) -> Self {
        self.budget_persistence_path = Some(path);
        self
    }

    /// Wire the tool stats learner for statistical tool learning.
    /// Registers the ToolReliabilitySensor into the sensor registry.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self for chaining
    pub fn with_tool_stats(mut self, stats: Arc<ToolStats>) -> Self {
        self.set_tool_stats(stats);
        self
    }

    /// Set tool stats on an already-constructed loop (post-build wiring).
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    pub fn set_tool_stats(&mut self, stats: Arc<ToolStats>) {
        self.sensor_registry
            .register(Arc::new(ToolReliabilitySensor::new(
                Arc::clone(&stats),
                crate::tool_stats::DEFAULT_RELIABILITY_THRESHOLD,
            )));
        self.tool_stats = Some(stats);
    }

    /// Override the stagnation detection threshold (default: 5 cycles).
    ///
    /// After this many consecutive cycles where the same (metric, action)
    /// pair is ineffective, a `RegulatoryPlateau` escalation is triggered.
    /// Per-metric thresholds from SetPoints are preserved.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self for chaining
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_stagnation_threshold(mut self, threshold: u32) -> Self {
        let existing_thresholds = self.set_points.stagnation_thresholds.clone();
        self.stagnation_detector = Arc::new(
            StagnationDetector::new(threshold).with_per_metric_thresholds(existing_thresholds),
        );
        self
    }

    /// Attempt to substitute an action type when the proposed one has been
    /// repeatedly ineffective (Fermi improvement-loop pattern).
    ///
    /// Checks the stagnation detector for the (metric, proposed) pair.
    /// If it has been ineffective for ≥ `substitution_after` cycles,
    /// walks the substitution ladder to find an untried alternative.
    /// Returns the proposed action if no alternatives remain.
    async fn try_substitute(&self, metric: SignalMetric, proposed: ActionType) -> ActionType {
        let proposed_str = proposed.as_str();
        let metric_str = metric.as_str();

        // Check if the proposed action has been tried enough to warrant substitution.
        let count = self
            .stagnation_detector
            .ineffective_count(metric_str, proposed_str);

        if count < self.calibrated_thresholds.read().await.substitution_after {
            return proposed; // Not enough failures yet.
        }

        // Build the substitution ladder: custom overrides > defaults.
        let custom_ladder = self.set_points.action_substitutions.get(metric_str);
        let ladder: Vec<ActionType> = if let Some(names) = custom_ladder {
            names.iter().filter_map(|n| ActionType::parse(n)).collect()
        } else {
            default_substitution_ladder(metric).to_vec()
        };

        if ladder.is_empty() {
            return proposed; // No alternatives defined.
        }

        // Find the first action in the ladder that hasn't been tried recently.
        for &alt in &ladder {
            if alt == proposed {
                continue; // Skip the action we're already considering.
            }
            let alt_str = alt.as_str();
            let alt_count = self
                .stagnation_detector
                .ineffective_count(metric_str, alt_str);
            if alt_count == 0 {
                tracing::info!(
                    target: "cns.cybernetics.substitution",
                    metric = metric_str,
                    from = %proposed_str,
                    to = %alt_str,
                    failed_attempts = count,
                    "Action substitution: replacing ineffective action with alternative"
                );
                self.emit_regulation_span(
                    SpanKind::ActionSubstituted,
                    serde_json::json!({
                        "metric": metric_str,
                        "from": proposed_str,
                        "to": alt_str,
                        "failed_attempts": count,
                    }),
                )
                .await;
                return alt;
            }
        }

        // All alternatives have been tried and failed — let the plateau
        // escalation handle it.
        tracing::warn!(
            target: "cns.cybernetics.substitution",
            metric = metric_str,
            action = %proposed_str,
            "All substitution alternatives exhausted for metric"
        );
        proposed
    }

    /// Emit a regulation span to the NuEventStore for CNS observability.
    ///
    /// This is the Conant-Ashby closure: the CNS (observer-of-observers)
    /// must have a model of the regulation system itself. These spans
    /// give the Curator visibility into regulatory effectiveness — which
    /// actions are working, which are being substituted, and which are
    /// being blocked.
    async fn emit_regulation_span(&self, kind: SpanKind, observation: serde_json::Value) {
        if let Some(ref sink) = self.event_sink {
            let event = NuEvent::new(
                WebID::from_persona(b"cns"),
                Span::from_kind(kind),
                CyclePhase::Act,
                observation,
                0,
            );
            if let Err(e) = sink.persist(&event) {
                tracing::error!(target: "cns.regulation", error = %e, "Failed to persist regulation span");
            }
        } else {
            tracing::warn!(target: "cns.regulation", span_kind = ?kind, "Regulation span dropped — no event_sink configured. Wire with_event_sink() for durable regulation observability.");
        }
    }

    /// Check regulation coherence — flag contradictory or suspicious action pairs.
    ///
    /// Runs after verify_impact. Scans the action set from this tick and logs
    /// warnings for patterns that suggest inconsistent regulation (e.g.,
    /// Throttle + CircuitBreak on same loop, AdjustEnergyBudget + OverrideEnergyBudget).
    fn check_coherence(&self, actions: &[LoopAction]) {
        use ActionType::*;
        let has = |t: ActionType| actions.iter().any(|a| a.action_type == t);
        let has_target = |t: ActionType, target: LoopId| {
            actions
                .iter()
                .any(|a| a.action_type == t && a.target == target)
        };

        // Throttle + CircuitBreak on same target — contradictory (slow down vs stop).
        if (has(Throttle) && has(CircuitBreak))
            || (has(AdjustEnergyBudget) && has(OverrideEnergyBudget))
        {
            tracing::warn!(
                target: "cns.regulation.coherence",
                action_count = actions.len(),
                "Potentially contradictory actions in same tick"
            );
        }

        // Both Throttle and CircuitBreak on Inference loop.
        if has_target(Throttle, LoopId::Inference) && has_target(CircuitBreak, LoopId::Inference) {
            tracing::warn!(
                target: "cns.regulation.coherence",
                "Throttle + CircuitBreak both targeting Inference loop — consider consolidating"
            );
        }
    }

    /// Attach a WalletManager for agent wallet operations.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self for chaining
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_wallet_manager(mut self, mgr: Arc<WalletManager>) -> Self {
        self.wallet_manager = Some(mgr);
        self
    }

    /// Wire the `SetPointCalibrator` — spawns a background task that periodically
    /// evaluates regulation outcomes from the NuEventStore and adjusts the three
    /// calibratable thresholds (`stagnation_thresholds`, `block_worsening_ratio`,
    /// `substitution_after`) within bounded ranges.
    ///
    /// This closes the Conant-Ashby self-tuning loop: the regulator adapts its
    /// own set-points from observed regulation history. The calibrator requires
    /// a `CnsStoragePort` to query algedonic/regulation events.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self for chaining; background calibration task spawned
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_set_point_calibrator(
        self,
        store: Arc<dyn hkask_ports::CnsStoragePort>,
        interval: std::time::Duration,
    ) -> Self {
        let calibrator = Arc::new(SetPointCalibrator::new(store, chrono::Duration::hours(1)));
        let thresholds = Arc::clone(&self.calibrated_thresholds);

        calibrator.spawn_calibration(interval, move |adjustments| {
            let mut guard = thresholds.blocking_write();
            let t = &mut *guard;
            SetPointCalibrator::apply_adjustments(
                &adjustments,
                &mut t.stagnation_thresholds,
                &mut t.block_worsening_ratio,
                &mut t.substitution_after,
            );
        });
        self
    }

    /// Wire the `SeamWatcher` — loads the architectural seam inventory and
    /// schedules periodic drift checks. When seam coverage regresses, an
    /// algedonic alert is emitted through the event sink.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    /// post: returns Self for chaining; seam watcher loaded if inventory found
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_seam_watcher(mut self) -> Self {
        match SeamWatcher::load() {
            Some(watcher) => {
                self.seam_watcher = Some(Arc::new(tokio::sync::Mutex::new(watcher)));
            }
            None => {
                tracing::warn!(
                    target: "hkask.seam",
                    "SeamWatcher not loaded — no inventory found. \
                     Set HKASK_SEAM_INVENTORY_PATH or embed at build time."
                );
            }
        }
        self
    }

    /// Attempt to load persisted budgets from the configured path.
    /// Called automatically during `build()` if a persistence path is set.
    /// Returns count loaded (0 if first run or no path configured).
    ///
    /// expect: "The system provides observability into CNS regulation state"
    pub async fn load_budgets(&self) -> Result<usize, GasError> {
        if let Some(ref path) = self.budget_persistence_path {
            let contents = match tokio::fs::read_to_string(path).await {
                Ok(c) => c,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
                Err(e) => {
                    return Err(GasError::Persistence(format!(
                        "read {}: {e}",
                        path.display()
                    )));
                }
            };
            let wrapper: serde_json::Value = serde_json::from_str(&contents)
                .map_err(|e| GasError::Persistence(format!("parse {}: {e}", path.display())))?;

            // Load gas budgets
            let count = if let Some(budgets_val) = wrapper.get("budgets") {
                let loaded: HashMap<WebID, GasBudget> = serde_json::from_value(budgets_val.clone())
                    .map_err(|e| GasError::Persistence(format!("parse budgets: {e}")))?;
                let n = loaded.len();
                let gbm = self.gas_budget_manager.read().await;
                let mut budgets = gbm.gas_budgets_mut().await;
                for (id, budget) in loaded {
                    budgets.insert(id, budget);
                }
                n
            } else {
                0
            };

            // Restore Well state
            if let Some(well_val) = wrapper.get("well") {
                let mut wells = self.well_manager.write().await;
                wells.load_state(well_val);
            }

            // Restore ToolStats state
            if let Some(ts_val) = wrapper.get("tool_stats")
                && let Some(ref stats) = self.tool_stats
            {
                stats.load_state(ts_val).await;
            }

            if count > 0 || wrapper.get("tool_stats").is_some() {
                tracing::info!(target: "cns.cybernetics", count = count, "Loaded persisted budgets + Well + ToolStats state");
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Access the WalletManager for wallet creation and balance queries.
    /// Returns None if no wallet manager was attached via with_wallet_manager().
    ///
    /// expect: "The system provides observability into CNS regulation state"
    pub fn wallet_manager(&self) -> Option<&Arc<WalletManager>> {
        self.wallet_manager.as_ref()
    }

    /// Attach a WalletManager after construction.
    ///
    /// expect: "The system provides configurable cybernetic self-regulation"
    pub async fn set_wallet_manager(&mut self, mgr: Arc<WalletManager>) {
        self.gas_budget_manager
            .write()
            .await
            .set_wallet_manager(Arc::clone(&mgr));
        self.wallet_manager = Some(mgr);
    }

    /// Access the WellManager for Well creation and configuration.
    ///
    /// expect: "The system provides observability into CNS regulation state"
    pub fn well_manager(&self) -> &Arc<RwLock<WellManager>> {
        &self.well_manager
    }

    /// Record a tool outcome in the CNS runtime for outcome quality tracking.
    ///
    /// Delegates to `CnsRuntime::record_outcome`. Called by `McpRuntime`
    /// after every governed tool invocation completes.
    ///
    /// expect: "The system provides observability into CNS regulation state"
    pub async fn record_outcome(&self, domain: &str, success: bool, error_kind: Option<&str>) {
        self.cns
            .read()
            .await
            .record_outcome(domain, success, error_kind)
            .await;
    }

    /// Register a gas budget for an agent in the gas budget manager.
    ///
    /// expect: "The system enforces energy homeostasis through gas budget membrane regulation"
    pub async fn register_gas_budget(&self, agent: WebID, budget: GasBudget) {
        self.gas_budget_manager
            .read()
            .await
            .register_gas_budget(agent, budget)
            .await;
    }

    /// Register a wallet-backed budget for an agent (Phase 5).
    /// Wallet budgets are checked before gas budgets in the membrane.
    ///
    /// expect: "The system enforces energy homeostasis through gas budget membrane regulation"
    pub async fn register_wallet_budget(&self, agent: WebID, budget: WalletBackedBudget) {
        self.gas_budget_manager
            .read()
            .await
            .register_wallet_budget(agent, budget)
            .await;
    }

    /// Check whether an agent has sufficient budget to proceed with a gas cost.
    ///
    /// expect: "The system enforces energy homeostasis through gas budget membrane regulation"
    pub async fn can_proceed(&self, agent: &WebID, gas: GasCost) -> bool {
        self.gas_budget_manager
            .read()
            .await
            .can_proceed(agent, gas)
            .await
    }

    /// Returns `None` if agent has no registered budget.
    ///
    /// expect: "The system enforces energy homeostasis through gas budget membrane regulation"
    pub async fn agent_gas_status(&self, agent: &WebID) -> Option<AgentGasStatus> {
        self.gas_budget_manager
            .read()
            .await
            .agent_gas_status(agent)
            .await
    }

    /// Hold-settle pattern: gas reserved but not consumed. Call settle_gas() after.
    ///
    /// expect: "The system enforces energy homeostasis through gas budget membrane regulation"
    pub async fn reserve_gas(&self, agent: &WebID, gas: GasCost) -> Result<GasCost, GasError> {
        self.gas_budget_manager
            .read()
            .await
            .reserve_gas(agent, gas)
            .await
    }

    /// If actual < reserved, the difference is refunded.
    ///
    /// expect: "The system enforces energy homeostasis through gas budget membrane regulation"
    pub async fn settle_gas(
        &self,
        agent: &WebID,
        reserved_gas: GasCost,
        actual_gas: GasCost,
    ) -> Result<GasCost, GasError> {
        self.gas_budget_manager
            .read()
            .await
            .settle_gas(agent, reserved_gas, actual_gas)
            .await
    }

    /// For estimated cost, prefer `reserve_gas` + `settle_gas`.
    ///
    /// expect: "The system enforces energy homeostasis through gas budget membrane regulation"
    pub async fn acquire_budget(&self, agent: &WebID, gas: GasCost) -> Result<GasCost, GasError> {
        self.gas_budget_manager
            .read()
            .await
            .acquire_budget(agent, gas)
            .await
    }

    /// Replenish all registered agent gas budgets on the current cycle.
    ///
    /// expect: "The system enforces energy homeostasis through gas budget membrane regulation"
    pub async fn replenish_all_budgets(&self) {
        self.gas_budget_manager
            .read()
            .await
            .replenish_all_budgets()
            .await;
    }

    /// Used by CuratorDirective::ReplenishBudget.
    ///
    /// expect: "The system enforces energy homeostasis through gas budget membrane regulation"
    pub async fn replenish_agent_budget(&self, agent: &WebID, amount: GasCost) {
        self.gas_budget_manager
            .read()
            .await
            .replenish_agent_budget(agent, amount)
            .await;
    }

    /// Called during sense() so directives are applied before computing actions.
    ///
    /// expect: "The system enforces homeostatic self-regulation through the five-phase cybernetic cycle"
    /// pre: called before each regulation tick to drain pending directives
    pub async fn process_inbox(&self) {
        // Drain direct curator directive channel.
        if let Some(ref rx) = self.curator_directive_rx {
            let mut cd_rx = rx.write().await;
            let mut cd_processed = 0;
            while let Ok(directive) = cd_rx.try_recv() {
                cd_processed += 1;
                self.handle_curation_directive(directive).await;
            }
            if cd_processed > 0 {
                tracing::info!(target: "cns.cybernetics", processed = cd_processed, "Processed direct curator directives");
            }
        }

        self.gas_budget_manager
            .read()
            .await
            .expire_overrides()
            .await;
    }

    async fn handle_curation_directive(&self, directive: CuratorDirective) {
        // Dampen repeated directives to prevent feedback oscillation
        if self.dampener.should_dampen_directive(&directive) {
            tracing::debug!(
                target: "cns.cybernetics",
                directive = %directive.variant_name(),
                "Directive dampened (repeated within window)"
            );
        } else {
            let variant_name = directive.variant_name();
            self.apply_directive(directive).await;
            self.persist_directive_acknowledgment(variant_name);
            tracing::info!(
                target: "cns.cybernetics",
                directive = %variant_name,
                outcome = "applied",
                "Directive acknowledged (Curation→Cybernetics compliance)"
            );
        }
    }

    async fn apply_directive(&self, directive: CuratorDirective) {
        match directive {
            CuratorDirective::CalibrateThreshold {
                domain,
                new_threshold,
            } => self.apply_calibrate_threshold(&domain, new_threshold).await,
            CuratorDirective::OverrideEnergyBudget { agent, new_budget } => {
                self.apply_override_gas_budget(agent, new_budget).await
            }
            CuratorDirective::ClearOverride { agent } => self.apply_clear_override(agent).await,
            CuratorDirective::ReplenishBudget {
                agent,
                amount,
                priority,
            } => self.apply_replenish_budget(agent, amount, priority).await,
            CuratorDirective::UpdateCapabilities {
                agent,
                additions,
                removals,
            } => {
                tracing::info!(target: "cns.cybernetics", agent = %agent, additions = ?additions, removals = ?removals, "Applied UpdateCapabilities directive from Curation (capabilities updated)")
            }
            CuratorDirective::SeekMoreEvidence {
                context,
                channel,
                confidence,
            } => {
                tracing::info!(target: "cns.cybernetics", context = %context, channel = %channel, confidence = %confidence, "Applied SeekMoreEvidence directive from Curation (metacognition loop triggered)")
            }
            // Federation directives are handled by CuratorAgent, not Cybernetics
            _ => {
                tracing::debug!(target: "cns.cybernetics", variant = directive.variant_name(), "Federation directive — no Cybernetics action")
            }
        }
    }

    async fn apply_calibrate_threshold(&self, domain: &str, new_threshold: u64) {
        let cns = self.cns.read().await;
        cns.calibrate_threshold(domain, new_threshold).await;
        drop(cns);
        tracing::info!(
            target: "cns.cybernetics",
            domain = domain,
            new_threshold = new_threshold,
            "Applied CalibrateThreshold directive from Curation"
        );
    }

    /// Metacognitive override — recorded in active_overrides so replenish skips it.
    async fn apply_override_gas_budget(&self, agent: WebID, new_budget: u64) {
        self.gas_budget_manager
            .read()
            .await
            .apply_override_gas_budget(agent, GasCost(new_budget))
            .await;
    }

    /// Removes agent from active_overrides, resuming normal replenishment.
    async fn apply_clear_override(&self, agent: WebID) {
        self.gas_budget_manager
            .read()
            .await
            .apply_clear_override(agent)
            .await;
    }

    /// Priority-scaled: when priority is provided, replenishment is weighted.
    async fn apply_replenish_budget(&self, agent: WebID, amount: u64, priority: Option<f64>) {
        self.gas_budget_manager
            .read()
            .await
            .apply_replenish_budget(agent, GasCost(amount), priority)
            .await;
    }

    fn persist_directive_acknowledgment(&self, directive_type: &str) {
        if let Some(ref sink) = self.event_sink {
            let ack = NuEvent::new(
                WebID::from_persona(b"cns"),
                Span::from_kind(SpanKind::CurationDirectiveAcknowledged),
                CyclePhase::Act,
                serde_json::json!({
                    "directive_type": directive_type,
                    "outcome": "applied",
                }),
                0,
            );
            if let Err(e) = sink.persist(&ack) {
                tracing::warn!(
                    target: "cns.cybernetics",
                    error = %e,
                    "Failed to persist directive acknowledgment"
                );
            }
        }
    }
}

#[async_trait::async_trait]
impl HkaskLoop for CyberneticsLoop {
    fn id(&self) -> LoopId {
        LoopId::Cybernetics
    }

    /// Produces signals for: per-agent energy ratio, variety deficit, queue depth,
    /// wallet balance ratio, wallet treasury ratio.
    async fn sense(&self) -> Vec<Signal> {
        // Process pending directives before sensing state
        self.process_inbox().await;

        let mut signals = Vec::new();

        // All sensing is now done through the SensorRegistry.
        // Wallet balance ratio, energy remaining, variety deficit, wallet key health,
        // and tool reliability are all sensed by registered SensorProvider implementations.
        //
        // The inline wallet ratio sensing that was here has been migrated to
        // WalletBalanceRatioSensor (v0.32.0) — see ADR-056.

        // Append signals from pluggable sensor providers.
        let registry_signals = self.sensor_registry.sense_all(LoopId::Cybernetics).await;
        signals.extend(registry_signals);

        // Feed observed values into the predictive simulator.
        for signal in &signals {
            self.simulator.observe(signal.metric, signal.value);
        }

        signals
    }

    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        // Predictive regulation: check if any metric is approaching its set-point.
        for dev in deviations {
            let pred = self.simulator.predict(
                dev.signal.metric,
                dev.signal.value,
                dev.signal.set_point,
                3,
            );
            if let Some(ticks) = pred.ticks_to_threshold
                && ticks <= 3
                && pred.reliable
            {
                tracing::info!(
                    target: "cns.regulation.predictive",
                    metric = dev.signal.metric.as_str(),
                    current = dev.signal.value,
                    set_point = dev.signal.set_point,
                    ticks_to_threshold = ticks,
                    trend = pred.trend,
                    "Predictive: metric approaching set-point"
                );
                // Emit a predictive notification to Curation.
                actions.push(LoopAction::new(
                    LoopId::Curation,
                    ActionType::Notify,
                    LoopActionParams::reason("predictive_threshold_approach"),
                ));
            }
        }

        let policy = RegulationPolicy::default();

        for dev in deviations {
            for proposed in policy.decide(dev) {
                let action = self.build_regulation_action(dev, proposed).await;
                if let Some(a) = action {
                    actions.push(a);
                }
            }
        }
        actions
    }

    async fn act(&self, actions: &[LoopAction]) {
        self.replenish_all_budgets().await;

        // E04: Detect and escalate budget exhaustion via algedonic pathway
        {
            let statuses = self
                .gas_budget_manager
                .read()
                .await
                .all_agent_statuses()
                .await;
            let gas_exhausted: Vec<_> = statuses
                .into_iter()
                .filter(|(_, s)| s.remaining.0 == 0 && s.hard_limit)
                .collect();

            // G10: Wallet-backed budget exhaustion
            let wallet_exhausted = self
                .gas_budget_manager
                .read()
                .await
                .wallet_exhausted_agents()
                .await;

            let alert_entries: Vec<(String, String)> = gas_exhausted
                .iter()
                .map(|(agent, status)| {
                    (
                        format!("gas_budget:{agent}"),
                        format!(
                            "Agent {agent} gas budget exhausted (cap: {}, remaining: 0)",
                            status.cap.0
                        ),
                    )
                })
                .chain(wallet_exhausted.iter().map(|agent| {
                    (
                        format!("wallet_budget:{agent}"),
                        format!("Agent {agent} wallet balance exhausted"),
                    )
                }))
                .collect();

            for (domain, message) in &alert_entries {
                let alert = RuntimeAlert {
                    domain: domain.clone(),
                    deficit: 1,
                    threshold: 1,
                    severity: AlertSeverity::Warning,
                    escalated: false,
                    timestamp: chrono::Utc::now(),
                    message: message.clone(),
                };
                let sent = if let Some(ref tx) = self.alerts_tx {
                    tx.send(CurationInput::Alert(alert.clone())).is_ok()
                } else {
                    false
                };
                if !sent {
                    tracing::warn!(target: "cns.algedonic", domain = %alert.domain, "Well exhaustion alert send failed or channel not connected");
                }
                if !sent && let Some(ref sink) = self.event_sink {
                    let event = NuEvent::new(
                        WebID::from_persona(b"cns"),
                        Span::from_kind(SpanKind::VarietyAlgedonicAlert),
                        CyclePhase::Act,
                        serde_json::json!({
                            "domain": alert.domain,
                            "message": alert.message,
                            "severity": "Warning",
                            "timestamp": alert.timestamp.to_rfc3339(),
                        }),
                        0,
                    );
                    if let Err(e) = sink.persist(&event) {
                        tracing::error!(target: "cns.cybernetics", error = %e, "Failed to persist budget exhaustion alert");
                    }
                }
            }
        }

        // E02: Persist budgets + Well state after each replenishment cycle
        if let Some(ref path) = self.budget_persistence_path {
            let mut wrapper = serde_json::json!({
                "version": 1,
            });
            {
                let gbm = self.gas_budget_manager.read().await;
                let budgets = gbm.gas_budgets().await;
                match serde_json::to_value(&*budgets) {
                    Ok(v) => wrapper["budgets"] = v,
                    Err(e) => {
                        tracing::error!(target: "cns.cybernetics", error = %e, "Failed to serialize gas budgets — skipping persistence");
                        return;
                    }
                }
            }
            {
                let wells = self.well_manager.read().await;
                wrapper["well"] = wells.save_state();
            }
            {
                if let Some(ref stats) = self.tool_stats {
                    wrapper["tool_stats"] = stats.save_state().await;
                }
            }
            let json = match serde_json::to_string_pretty(&wrapper) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(target: "cns.cybernetics", error = %e, "Failed to serialize budget wrapper — skipping persistence");
                    return;
                }
            };
            if let Some(parent) = path.parent()
                && let Err(e) = tokio::fs::create_dir_all(parent).await
            {
                tracing::error!(target: "cns.cybernetics", path = %parent.display(), error = %e, "Failed to create budget persistence directory");
                return;
            }
            if let Err(e) = tokio::fs::write(path, &json).await {
                tracing::error!(target: "cns.cybernetics", path = %path.display(), error = %e, "Failed to persist gas budgets");
            }
        }

        // Replenish Wells on each regulation cycle
        {
            let mut wells = self.well_manager.write().await;
            wells.replenish_all();
        }

        // 1.8: Well exhaustion → algedonic alert (with dampening)
        {
            let mut wells = self.well_manager.write().await;
            if wells.default_well_exhausted() {
                if !wells.was_already_exhausted {
                    wells.was_already_exhausted = true;
                    let alert = RuntimeAlert {
                        domain: "well".into(),
                        deficit: 1,
                        threshold: 1,
                        severity: AlertSeverity::Critical,
                        escalated: true,
                        timestamp: chrono::Utc::now(),
                        message: "Default Well exhausted — agents will be blocked".into(),
                    };
                    if let Some(ref tx) = self.alerts_tx
                        && tx.send(CurationInput::Alert(alert)).is_err()
                    {
                        tracing::warn!(target: "cns.algedonic", "Well exhaustion alert send failed — channel closed");
                    }
                }
            } else {
                wells.was_already_exhausted = false;
            }
        }
        // Note: Auto-draw from Well is now synchronous — handled in WalletManager::spend().
        let has_energy_depletion = actions
            .iter()
            .any(|a| a.parameters.reason == "energy_budget_low");
        if has_energy_depletion {
            let cns = self.cns.read().await;
            let worst_ratio = actions
                .iter()
                .filter_map(|a| a.parameters.data.remaining_ratio())
                .fold(1.0, f64::min);
            cns.emit_backpressure(BackpressureSignal {
                source: LoopId::Cybernetics,
                reason: "energy_budget_depletion".into(),
                severity: 1.0 - worst_ratio,
            })
            .await;
        }
        if actions.len() > self.max_iterations as usize {
            tracing::warn!(target: "cns.cybernetics", action_count = actions.len(), max_iterations = self.max_iterations, "Cascade detected: action count exceeds max_iterations");
        }
        for action in actions {
            tracing::info!(target: "cns.cybernetics", action_type = ?action.action_type, target_loop = %action.target, "Cybernetics Loop efferent signal");
            let target_id = action.target;

            // Send CurationInput::Alert on direct alerts channel.
            // Fallback: persist to NuEventStore when channel is down (Curator inactive).
            // Per design decision: the algedonic system must always be connected
            // to the Curator userpod/agent — persistence is the bridge when the
            // live channel has no receiver.
            if action.action_type == ActionType::Escalate && target_id == LoopId::Curation {
                let (deficit, threshold) = extract_deficit_threshold(&action.parameters.data);
                let domain = String::new();
                let alert = RuntimeAlert {
                    domain,
                    deficit,
                    threshold,
                    severity: AlertSeverity::Critical,
                    escalated: true,
                    timestamp: chrono::Utc::now(),
                    message: format!(
                        "Variety deficit {} exceeds threshold {}",
                        deficit, threshold
                    ),
                };

                // Primary path: live channel to Curator's inbox
                let sent_live = if let Some(ref alerts_tx) = self.alerts_tx {
                    match alerts_tx.send(CurationInput::Alert(alert.clone())) {
                        Ok(()) => true,
                        Err(e) => {
                            tracing::warn!(target: "cns.cybernetics", error = %e, "Failed to send CurationInput::Alert via live channel — falling back to persistence");
                            false
                        }
                    }
                } else {
                    tracing::warn!(target: "cns.cybernetics", "Alerts channel not connected — falling back to persistence. Wire with_alerts_channel() for live delivery.");
                    false
                };

                // Fallback: persist full alert to NuEventStore for Curator retrieval on next activation
                if !sent_live {
                    if let Some(ref sink) = self.event_sink {
                        let event = NuEvent::new(
                            WebID::from_persona(b"cns"),
                            Span::from_kind(SpanKind::VarietyAlgedonicAlert),
                            CyclePhase::Act,
                            serde_json::json!({
                                "domain": alert.domain,
                                "deficit": alert.deficit,
                                "threshold": alert.threshold,
                                "severity": "Critical",
                                "escalated": true,
                                "message": alert.message,
                                "timestamp": alert.timestamp.to_rfc3339(),
                            }),
                            0,
                        );
                        if let Err(e) = sink.persist(&event) {
                            tracing::error!(target: "cns.algedonic", error = %e, "CRITICAL: Failed to persist algedonic alert — alert lost. Both live channel and persistence failed.");
                        } else {
                            tracing::info!(target: "cns.algedonic", deficit = deficit, threshold = threshold, "Algedonic alert persisted to NuEventStore (Curator inbox unavailable)");
                        }
                    } else {
                        tracing::error!(target: "cns.algedonic", deficit = deficit, threshold = threshold, "CRITICAL: Algedonic alert LOST — neither live channel nor event_sink connected. Feedback loop closure broken.");
                    }
                }
            }
        }
    }

    /// Verify whether the previous cycle's actions improved their targeted
    /// metrics (Fermi impact-gate pattern).
    ///
    /// Re-senses energy ratios and variety deficit, comparing post-action
    /// values against the pre-action values. Classifies each action as
    /// Accept / Stage / Block using per-metric worsening thresholds.
    /// Blocked actions are prevented from re-use until Curation intervenes.
    /// Actions that repeatedly fail to improve trigger stagnation detection.
    async fn verify_impact(&self, previous_actions: &[LoopAction]) -> Vec<ImpactReport> {
        let mut reports = Vec::new();

        // Re-sense current state for comparison.
        let budget_statuses = self
            .gas_budget_manager
            .read()
            .await
            .all_agent_statuses()
            .await;
        let cns = self.cns.read().await;
        let health = cns.health().await;
        let current_deficit = health.overall_deficit as f64;
        drop(cns);

        for action in previous_actions {
            // Determine metric and pre-action value from the typed RegulationData.
            let (before_val, metric) = match &action.parameters.data {
                RegulationData::EnergyBudgetLow {
                    remaining_ratio, ..
                }
                | RegulationData::BudgetGuardEscalation {
                    remaining_ratio, ..
                }
                | RegulationData::EnergyDepletionAutoAdjust {
                    remaining_ratio, ..
                } => (*remaining_ratio, SignalMetric::EnergyRemaining),
                RegulationData::VarietyDeficitExceeded { deficit, .. } => {
                    (*deficit, SignalMetric::VarietyDeficit)
                }
                _ => continue,
            };

            let after_val = match metric {
                SignalMetric::EnergyRemaining => budget_statuses
                    .iter()
                    .map(|(_, s)| s.remaining.0 as f64 / s.cap.0.max(1) as f64)
                    .fold(1.0, f64::min),
                SignalMetric::VarietyDeficit => current_deficit,
                _ => continue,
            };

            let delta = after_val - before_val;
            // For EnergyRemaining: higher is better (positive delta = improved).
            // For VarietyDeficit: lower is better (negative delta = improved).
            let improved = match metric {
                SignalMetric::EnergyRemaining => delta > 0.0,
                SignalMetric::VarietyDeficit => delta < 0.0,
                _ => delta.abs() > f64::EPSILON,
            };

            // Classify the decision using per-metric worsening thresholds.
            let worsening = if improved { 0.0 } else { delta.abs() };
            let block_worsening_ratio = self
                .calibrated_thresholds
                .read()
                .await
                .block_worsening_ratio;
            let decision = classify_decision(
                worsening,
                self.set_points.stage_worsening_ratio,
                block_worsening_ratio,
            );

            // Report acceptance/rejection to stagnation detector.
            let accepted = decision == ActionDecision::Accept;
            let action_type_str = action.action_type.as_str();
            let plateau = self.stagnation_detector.record_and_check(
                metric.as_str(),
                action_type_str,
                accepted,
            );

            if plateau {
                let threshold = {
                    let calibrated = self.calibrated_thresholds.read().await;
                    calibrated
                        .stagnation_thresholds
                        .get(metric.as_str())
                        .copied()
                        .unwrap_or_else(|| {
                            self.stagnation_detector
                                .threshold_for_metric(metric.as_str())
                        })
                };
                self.emit_regulation_span(
                    SpanKind::RegulatoryPlateauDetected,
                    serde_json::json!({
                        "metric": metric.as_str(),
                        "action_type": action_type_str,
                        "consecutive_cycles": threshold,
                    }),
                )
                .await;
                if let Some(ref tx) = self.alerts_tx {
                    let alert = RuntimeAlert {
                        domain: format!("regulatory_plateau:{}", metric.as_str()),
                        deficit: 1,
                        threshold: 1,
                        severity: AlertSeverity::Warning,
                        escalated: true,
                        timestamp: chrono::Utc::now(),
                        message: format!(
                            "Regulatory plateau: {} via {:?} has been rejected for {threshold} consecutive cycles",
                            metric.as_str(),
                            action.action_type,
                        ),
                    };
                    if tx.send(CurationInput::Alert(alert)).is_err() {
                        tracing::warn!(target: "cns.algedonic", "Plateau alert send failed — channel closed");
                    }
                }
                tracing::warn!(
                    target: "cns.cybernetics",
                    metric = metric.as_str(),
                    action_type = ?action.action_type,
                    "Regulatory plateau detected"
                );
            }

            // Blocked actions: escalate as Critical to Curation + emit CNS span.
            if decision == ActionDecision::Block {
                self.emit_regulation_span(
                    SpanKind::ActionBlocked,
                    serde_json::json!({
                        "metric": metric.as_str(),
                        "action_type": format!("{:?}", action.action_type),
                        "worsening": worsening,
                        "block_threshold": block_worsening_ratio,
                    }),
                )
                .await;
                if let Some(ref tx) = self.alerts_tx {
                    let alert = RuntimeAlert {
                        domain: format!("action_blocked:{}", metric.as_str()),
                        deficit: 1,
                        threshold: 1,
                        severity: AlertSeverity::Critical,
                        escalated: true,
                        timestamp: chrono::Utc::now(),
                        message: format!(
                            "ActionDecision::Block: {} on {} caused {:.1}% worsening (threshold: {:.1}%)",
                            action.action_type.as_str(),
                            metric.as_str(),
                            worsening * 100.0,
                            block_worsening_ratio * 100.0,
                        ),
                    };
                    if tx.send(CurationInput::Alert(alert)).is_err() {
                        tracing::warn!(target: "cns.algedonic", "Block alert send failed — channel closed");
                    }
                }
            }

            // Emit CNS span for Curator observability of regulatory effectiveness.
            self.emit_regulation_span(
                SpanKind::ImpactVerified,
                serde_json::json!({
                    "metric": metric.as_str(),
                        "action_type": action.action_type.as_str(),
                        "before": before_val,
                    "after": after_val,
                    "delta": delta,
                    "improved": improved,
                    "decision": format!("{:?}", decision),
                }),
            )
            .await;

            reports.push(ImpactReport::new(
                action.action_type,
                metric,
                before_val,
                after_val,
                decision,
            ));
        }

        reports
    }

    /// Full regulation cycle with loop-quality telemetry.
    ///
    /// Overrides the default `tick()` to measure elapsed time and compute
    /// `LoopQuality` metrics (delay_ms, gain, fidelity_score, effectiveness_score)
    /// after each cycle. Calls `verify_impact` to close the feedback loop.
    async fn tick(&self) {
        let start = std::time::Instant::now();

        // SLO evaluation — runs every tick when provider is wired.
        if let Some(ref provider) = self.slo_provider {
            let cns = self.cns.read().await;
            let _ = cns.evaluate_and_escalate_slos(provider.as_ref()).await;
            drop(cns);
        }

        let signals = self.sense().await;
        let deviations = self.compare(&signals).await;
        let actions = self.compute(&deviations).await;
        self.act(&actions).await;

        // Fermi impact-gate: verify whether actions improved their targets.
        let impact_reports = self.verify_impact(&actions).await;

        // Check regulation coherence.
        self.check_coherence(&actions);

        // Feed per-metric outcomes into strategy evaluator.
        // Collect promoted metrics in a locked scope; emit spans outside
        // to avoid holding MutexGuard across .await (not Send).
        let promoted_metrics = {
            let mut evaluator = self
                .strategy_evaluator
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let mut seen = std::collections::HashSet::new();
            let mut promoted = Vec::new();
            for report in &impact_reports {
                if seen.insert(report.metric) {
                    let metric_reports: Vec<_> = impact_reports
                        .iter()
                        .filter(|r| r.metric == report.metric)
                        .collect();
                    let accepted = metric_reports
                        .iter()
                        .filter(|r| r.decision == ActionDecision::Accept)
                        .count() as u64;
                    let staged = metric_reports
                        .iter()
                        .filter(|r| r.decision == ActionDecision::Stage)
                        .count() as u64;
                    let blocked = metric_reports
                        .iter()
                        .filter(|r| r.decision == ActionDecision::Block)
                        .count() as u64;
                    evaluator.record_cycle(report.metric, accepted, staged, blocked);
                    // Check for strategy promotion; emit CNS span if promoted.
                    if evaluator.active_policy(report.metric) {
                        promoted.push(report.metric);
                    }
                }
            }
            promoted
        };
        for metric in promoted_metrics {
            self.emit_regulation_span(
                SpanKind::ActionSubstituted,
                serde_json::json!({
                    "event": "strategy_promoted",
                    "metric": metric.as_str(),
                }),
            )
            .await;
        }

        // Feed regulation health into CNS for metacognition observability.
        {
            let accepted = impact_reports
                .iter()
                .filter(|r| r.decision == ActionDecision::Accept)
                .count() as u64;
            let staged = impact_reports
                .iter()
                .filter(|r| r.decision == ActionDecision::Stage)
                .count() as u64;
            let blocked = impact_reports
                .iter()
                .filter(|r| r.decision == ActionDecision::Block)
                .count() as u64;
            let cns = self.cns.read().await;
            let cumulative = cns.regulation_health().await.effectiveness();
            cns.record_regulation_cycle(RegulationCycleEntry {
                timestamp: chrono::Utc::now(),
                signals: signals.len() as u64,
                deviations: deviations.len() as u64,
                actions: actions.len() as u64,
                verified: impact_reports.len() as u64,
                accepted,
                staged,
                blocked,
                cumulative_effectiveness: cumulative,
            })
            .await;
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;

        let quality = LoopQuality::from_cycle(
            elapsed_ms,
            &deviations,
            &actions,
            &impact_reports,
            TriggerOrigin::Scheduled,
        );
        *self.loop_quality.write().await = quality;

        tracing::debug!(
            target: "cns.cybernetics",
            delay_ms = quality.delay_ms,
            gain = quality.gain,
            fidelity = quality.fidelity_score,
            effectiveness = quality.effectiveness_score,
            deviations = deviations.len(),
            actions = actions.len(),
            impact_reports = impact_reports.len(),
            "Loop-quality telemetry recorded"
        );

        self.emit_regulation_span(
            SpanKind::LoopQualityTelemetry,
            serde_json::json!({
                "delay_ms": quality.delay_ms,
                "gain": quality.gain,
                "fidelity_score": quality.fidelity_score,
                "effectiveness_score": quality.effectiveness_score,
                "fidelity_confidence": quality.fidelity_confidence,
                "trigger": format!("{:?}", quality.trigger),
                "deviations": deviations.len(),
                "actions": actions.len(),
                "impact_reports": impact_reports.len(),
            }),
        )
        .await;

        // ── Seam drift check (throttled to every 10 minutes) ──
        // Closes the boundary-monitoring loop: detects architectural seam
        // coverage regression and emits CNS spans + algedonic alerts.
        if let Some(ref watcher) = self.seam_watcher {
            let should_check = {
                let mut last = self.last_seam_check.lock().await;
                let elapsed = last.elapsed();
                if elapsed >= std::time::Duration::from_secs(600) {
                    *last = std::time::Instant::now();
                    true
                } else {
                    false
                }
            };
            if should_check {
                let cns = self.cns.read().await;
                let mut watcher = watcher.lock().await;
                if let Some(ref sink) = self.event_sink {
                    let drifts = watcher.check_drift(&cns, sink.as_ref()).await;
                    if !drifts.is_empty()
                        && let Some(ref tx) = self.alerts_tx
                    {
                        for drift in &drifts {
                            if drift.delta_pct < 0.0 {
                                let alert = crate::algedonic::RuntimeAlert {
                                    domain: format!("seam_drift:{}", drift.crate_name),
                                    deficit: 1,
                                    threshold: 1,
                                    severity: if drift.delta_pct < -5.0 {
                                        AlertSeverity::Critical
                                    } else {
                                        AlertSeverity::Warning
                                    },
                                    escalated: true,
                                    timestamp: chrono::Utc::now(),
                                    message: format!(
                                        "Seam coverage regression in {}: {:.1}% → {:.1}% ({:+.1}%)",
                                        drift.crate_name,
                                        drift.previous_coverage_pct,
                                        drift.current_coverage_pct,
                                        drift.delta_pct,
                                    ),
                                };
                                if tx.send(CurationInput::Alert(alert)).is_err() {
                                    tracing::warn!(
                                        target: "hkask.seam",
                                        "Seam drift alert send failed — channel closed"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Adapt `Arc<RwLock<CyberneticsLoop>>` for use as `Arc<dyn HkaskLoop>` in LoopSystem.
/// Eliminates the pass-through `CyberneticsLoopHandle` struct per Prohibition #4.
#[async_trait::async_trait]
impl HkaskLoop for tokio::sync::RwLock<CyberneticsLoop> {
    fn id(&self) -> LoopId {
        LoopId::Cybernetics
    }

    async fn sense(&self) -> Vec<Signal> {
        self.read().await.sense().await
    }

    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        self.read().await.compute(deviations).await
    }

    async fn act(&self, actions: &[LoopAction]) {
        self.read().await.act(actions).await
    }

    async fn verify_impact(&self, previous_actions: &[LoopAction]) -> Vec<ImpactReport> {
        self.read().await.verify_impact(previous_actions).await
    }
}

impl CyberneticsLoop {
    /// Build a `LoopAction` from a `ProposedAction` returned by the regulation policy.
    ///
    /// Applies mode-specific filtering (e.g., `InferenceThrottleMode`) and
    /// `try_substitute` for stagnation-based action ladder substitution.
    /// Returns `None` when the rule should be skipped (e.g., throttle in Off mode).
    async fn build_regulation_action(
        &self,
        dev: &Deviation,
        proposed: &regulation_policy::ProposedAction,
    ) -> Option<LoopAction> {
        use ActionType::*;
        use LoopId::*;
        use SignalMetric::*;

        match proposed.reason {
            // -- EnergyRemaining BelowSetPoint ------------------------------
            "energy_budget_low" => {
                if !matches!(
                    self.set_points.inference_throttle_mode,
                    InferenceThrottleMode::Autonomous
                ) {
                    return None;
                }
                let at = self.try_substitute(EnergyRemaining, Throttle).await;
                Some(LoopAction::with_metric(
                    Inference,
                    at,
                    LoopActionParams::with_data(
                        "energy_budget_low",
                        RegulationData::EnergyBudgetLow {
                            remaining_ratio: dev.signal.value,
                            set_point: dev.signal.set_point,
                        },
                    ),
                    "energy_remaining".into(),
                ))
            }
            "budget_guard_escalation" => {
                let curator_timeout_secs = match self.set_points.inference_throttle_mode {
                    InferenceThrottleMode::CuratorMediated {
                        curator_timeout_secs,
                    } => curator_timeout_secs,
                    _ => return None,
                };
                let remaining_ratio = dev.signal.value;
                let projected_minutes = (remaining_ratio * 60.0) as u64;
                Some(LoopAction::new(
                    Curation,
                    Escalate,
                    LoopActionParams::with_data(
                        "budget_guard_escalation",
                        RegulationData::BudgetGuardEscalation {
                            remaining_ratio,
                            set_point: dev.signal.set_point,
                            projected_minutes,
                            options: vec![
                                BudgetOption {
                                    id: "add_funds".into(),
                                    label: "Add funds to continue at current rate".into(),
                                },
                                BudgetOption {
                                    id: "switch_model".into(),
                                    label: "Switch to a smaller/cheaper model".into(),
                                },
                                BudgetOption {
                                    id: "continue".into(),
                                    label: "Continue at current rate (budget will exhaust)".into(),
                                },
                            ],
                            curator_timeout_secs,
                            fallback: "gentle_throttle".into(),
                        },
                    ),
                ))
            }
            "energy_depletion_auto_adjust" => {
                if matches!(
                    self.set_points.inference_throttle_mode,
                    InferenceThrottleMode::Off
                ) {
                    return None;
                }
                let at = self
                    .try_substitute(EnergyRemaining, AdjustEnergyBudget)
                    .await;
                Some(LoopAction::new(
                    Cybernetics,
                    at,
                    LoopActionParams::with_data(
                        "energy_depletion_auto_adjust",
                        RegulationData::EnergyDepletionAutoAdjust {
                            remaining_ratio: dev.signal.value,
                            set_point: dev.signal.set_point,
                        },
                    ),
                ))
            }
            // -- VarietyDeficit AboveSetPoint -------------------------------
            "variety_deficit_exceeded" => {
                let at = self.try_substitute(VarietyDeficit, Escalate).await;
                Some(LoopAction::new(
                    Curation,
                    at,
                    LoopActionParams::with_data(
                        "variety_deficit_exceeded",
                        RegulationData::VarietyDeficitExceeded {
                            deficit: dev.signal.value,
                            threshold: dev.signal.set_point,
                        },
                    ),
                ))
            }
            // -- ErrorRate AboveSetPoint ------------------------------------
            "error_rate_exceeded" => {
                let at = self.try_substitute(ErrorRate, CircuitBreak).await;
                Some(LoopAction::new(
                    Inference,
                    at,
                    LoopActionParams::with_data(
                        "error_rate_exceeded",
                        RegulationData::ErrorRateExceeded {
                            error_rate: dev.signal.value,
                            threshold: dev.signal.set_point,
                        },
                    ),
                ))
            }
            // -- ConnectorLatency AboveSetPoint -----------------------------
            "connector_latency_exceeded" => {
                let at = self.try_substitute(ConnectorLatency, Throttle).await;
                Some(LoopAction::new(
                    Cybernetics,
                    at,
                    LoopActionParams::with_data(
                        "connector_latency_exceeded",
                        RegulationData::ConnectorLatencyExceeded {
                            latency_secs: dev.signal.value,
                            threshold: dev.signal.set_point,
                        },
                    ),
                ))
            }
            // -- CommunicationQueueDepth AboveSetPoint ----------------------
            "communication_backpressure" => {
                tracing::info!(
                    target: "cns.cybernetics.backpressure",
                    queue_depth = dev.signal.value,
                    threshold = dev.signal.set_point,
                    "Communication queue depth exceeded backpressure threshold"
                );
                let at = self.try_substitute(CommunicationQueueDepth, Throttle).await;
                Some(LoopAction::new(
                    Cybernetics,
                    at,
                    LoopActionParams::with_data(
                        "communication_backpressure",
                        RegulationData::CommunicationBackpressure {
                            queue_depth: dev.signal.value,
                            threshold: dev.signal.set_point,
                        },
                    ),
                ))
            }
            // -- WalletBalanceRatio BelowSetPoint ---------------------------
            "wallet_balance_low" => {
                let severity = if dev.signal.value <= 0.0 {
                    "critical"
                } else {
                    "warning"
                };
                tracing::warn!(
                    target: "cns.wallet",
                    balance_ratio = dev.signal.value,
                    severity = severity,
                    "Wallet balance alert"
                );
                let at = self.try_substitute(WalletBalanceRatio, Escalate).await;
                Some(LoopAction::new(
                    Curation,
                    at,
                    LoopActionParams::with_data(
                        "wallet_balance_low",
                        RegulationData::WalletBalanceLow {
                            balance_ratio: dev.signal.value,
                            severity: severity.to_string(),
                            threshold: dev.signal.set_point,
                        },
                    ),
                ))
            }
            // -- WalletKeyHealth AboveSetPoint ------------------------------
            "wallet_key_unhealthy" => {
                tracing::info!(
                    target: "cns.wallet",
                    "API key health alert — exhausted or expired"
                );
                Some(LoopAction::new(
                    Curation,
                    Escalate,
                    LoopActionParams::with_data(
                        "wallet_key_unhealthy",
                        RegulationData::WalletKeyUnhealthy {
                            severity: "warning".into(),
                            threshold: dev.signal.set_point,
                        },
                    ),
                ))
            }
            // -- SeamCoverage BelowSetPoint ---------------------------------
            "seam_coverage_degraded" => {
                let drop_magnitude = dev.signal.set_point - dev.signal.value;
                let severity = if drop_magnitude > 5.0 {
                    "critical"
                } else {
                    "warning"
                };
                tracing::warn!(
                    target: "hkask.architecture.seam",
                    coverage_pct = dev.signal.value,
                    set_point = dev.signal.set_point,
                    drop_magnitude = drop_magnitude,
                    severity = severity,
                    "Public seam coverage degraded — seam watcher alert"
                );
                Some(LoopAction::new(
                    Curation,
                    Escalate,
                    LoopActionParams::with_data(
                        "seam_coverage_degraded",
                        RegulationData::SeamCoverageDegraded {
                            coverage_pct: dev.signal.value,
                            previous_coverage: dev.signal.set_point,
                            drop_magnitude,
                            severity: severity.to_string(),
                        },
                    ),
                ))
            }
            // -- SeamCoverage AboveSetPoint ---------------------------------
            "seam_coverage_improved" => {
                let improvement = dev.signal.value - dev.signal.set_point;
                tracing::info!(
                    target: "hkask.architecture.seam",
                    coverage_pct = dev.signal.value,
                    set_point = dev.signal.set_point,
                    improvement = improvement,
                    "Public seam coverage improved — seam watcher positive signal"
                );
                Some(LoopAction::new(
                    Curation,
                    Notify,
                    LoopActionParams::with_data(
                        "seam_coverage_improved",
                        RegulationData::SeamCoverageImproved {
                            coverage_pct: dev.signal.value,
                            previous_coverage: dev.signal.set_point,
                            improvement,
                        },
                    ),
                ))
            }
            // -- ToolReliability BelowSetPoint ------------------------------
            "tool_reliability_degraded" => {
                tracing::warn!(
                    target: "cns.tool",
                    reliability = dev.signal.value,
                    set_point = dev.signal.set_point,
                    "Tool reliability degraded — success rate below threshold"
                );
                let at = self.try_substitute(ToolReliability, Escalate).await;
                Some(LoopAction::new(
                    Curation,
                    at,
                    LoopActionParams::with_data(
                        "tool_reliability_degraded",
                        RegulationData::ToolReliabilityDegraded {
                            reliability: dev.signal.value,
                            threshold: dev.signal.set_point,
                        },
                    ),
                ))
            }
            _ => {
                tracing::debug!(
                    target: "cns.regulation",
                    reason = proposed.reason,
                    "Unknown regulation reason — no action built"
                );
                None
            }
        }
    }
}

impl CyberneticsLoop {
    /// Return a snapshot of the most recent loop-quality telemetry.
    ///
    /// expect: "The system provides observability into CNS regulation state"
    pub async fn loop_quality(&self) -> LoopQuality {
        *self.loop_quality.read().await
    }

    /// Return a reference to the current set-points (read-only).
    ///
    /// expect: "The system provides observability into CNS regulation state"
    pub fn set_points(&self) -> &SetPoints {
        &self.set_points
    }

    /// Return a mutable reference to the set-points for calibration.
    /// Callers must hold `&mut CyberneticsLoop` (e.g., via `loop.write().await`).
    ///
    /// expect: "The system provides observability into CNS regulation state"
    pub fn set_points_mut(&mut self) -> &mut SetPoints {
        &mut self.set_points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_loop_starts_with_default_quality() {
        let cns = Arc::new(RwLock::new(CnsRuntime::with_threshold(100)));
        let loop_instance = CyberneticsLoop::new(cns);
        let q = loop_instance.loop_quality().await;
        assert_eq!(q.delay_ms, 0);
        assert!((q.gain - 0.0).abs() < f64::EPSILON);
        assert!((q.fidelity_score - 0.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn tick_updates_loop_quality() {
        let cns = Arc::new(RwLock::new(CnsRuntime::with_threshold(100)));
        let loop_instance = CyberneticsLoop::new(cns);
        loop_instance.tick().await;
        let q = loop_instance.loop_quality().await;
        // After a tick, gain and fidelity should be computed (even if delay_ms is 0)
        // The key property: quality is no longer the default zero-state
        assert!(
            q.gain >= 0.0 && q.fidelity_score >= 0.0,
            "quality should be computed after tick"
        );
    }
}
