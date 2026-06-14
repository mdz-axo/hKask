//! CNS Runtime — minimal observability
//!
//! CnsRuntime is the single entry point for all CNS operations:
//! - Variety counting (Ashby's Law)
//! - Algedonic alerts (deficit > threshold → escalate)
//!
//! # Epistemic grounding (TASK 0)
//! - **crt:certainty** = Declarative (direct sensor readings)
//! - **crt:force** = Evidence (IS statement, measured from runtime state)
//! - **mode** = IS
//!
//! # Cybernetic role (TASK 1)
//! - Sensor: VarietyMonitor.counters() — count distinct agent states
//! - Comparator: AlgedonicManager.check() — compares deficit to threshold
//! - Effector: emit_critical_depletion() — broadcasts DepletionSignal to observers

use crate::algedonic::{
    AlgedonicManager, DEFAULT_EXPECTED_VARIETY, DEFAULT_THRESHOLD, RuntimeAlert, cns_health_check,
};
use crate::energy::{AgentEnergyStatus, EnergyBudget, EnergyCost};

use hkask_types::WebID;
use hkask_types::cns::CnsHealth;
use hkask_types::event::SpanNamespace;
use hkask_types::ports::{BackpressureSignal, CnsObserver, DepletionSignal};
use parking_lot::RwLock as ParkingRwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing;

// ── Variety counter infrastructure ────────────────────────────────────────
// Relocated from variety.rs (TASK 2 deletion test — VarietyMonitor only used
// by CnsRuntime, so depth increases when co-located).

/// Default variety counter window duration (1 minute).
const DEFAULT_VARIETY_WINDOW_SECS: u64 = 60;

/// Variety counter for tracking state diversity in a domain.
///
/// # Epistemic grounding
/// - **crt:certainty** = Subjunctive (sampling, not complete observation)
/// - **crt:force** = Hypothesis (counter is an estimate, not a ground truth)
/// - **mode** = IS
#[derive(Debug, Clone)]
pub(crate) struct VarietyTracker {
    counts: HashMap<String, u64>,
    window_start: Instant,
    window_duration: Duration,
}

impl VarietyTracker {
    pub(crate) fn new() -> Self {
        Self {
            counts: HashMap::new(),
            window_start: Instant::now(),
            window_duration: Duration::from_secs(DEFAULT_VARIETY_WINDOW_SECS),
        }
    }

    pub(crate) fn increment(&mut self, key: &str) {
        self.check_window();
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    pub(crate) fn variety(&self) -> u64 {
        self.counts.len() as u64
    }

    pub(crate) fn deficit(&self, expected_variety: u64) -> u64 {
        expected_variety.saturating_sub(self.variety())
    }

    fn check_window(&mut self) {
        if self.window_start.elapsed() > self.window_duration {
            self.reset();
        }
    }

    pub(crate) fn reset(&mut self) {
        self.counts.clear();
        self.window_start = Instant::now();
    }
}

impl Default for VarietyTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Variety monitor for multiple domains — Ashby's Law tracking at the CNS level.
///
/// # Epistemic grounding
/// - **crt:certainty** = Subjunctive
/// - **crt:force** = Hypothesis
/// - **mode** = IS
///
/// # Cybernetic role (TASK 1)
/// This is the **sensor** in the variety regulation feedback loop:
/// ```text
/// (MCP tool dispatch) → [VarietyMonitor.counter().increment()]
///     → [AlgedonicManager.check()] → [RuntimeAlert]
///     → [emit_critical_depletion()] → (agent behavior change)
/// ```
#[derive(Debug)]
pub struct VarietyMonitor {
    counters: HashMap<String, VarietyTracker>,
}

impl VarietyMonitor {
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    pub(crate) fn counter(&mut self, domain: &str) -> &mut VarietyTracker {
        self.counters.entry(domain.to_string()).or_default()
    }

    pub fn variety_for_domain(&self, domain: &str) -> u64 {
        self.counters.get(domain).map(|c| c.variety()).unwrap_or(0)
    }

    pub fn domains(&self) -> Vec<&str> {
        self.counters.keys().map(|s| s.as_str()).collect()
    }

    pub(crate) fn counters(&self) -> &HashMap<String, VarietyTracker> {
        &self.counters
    }
}

impl Default for VarietyMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// CNS state shared between threads
struct CnsState {
    algedonic: Arc<ParkingRwLock<AlgedonicManager>>,
    tracker: VarietyMonitor,
    energy_budgets: Arc<tokio::sync::RwLock<HashMap<WebID, EnergyBudget>>>,
}

impl CnsState {
    fn new(threshold: u64) -> Self {
        let algedonic = Arc::new(ParkingRwLock::new(AlgedonicManager::new(
            threshold,
            DEFAULT_EXPECTED_VARIETY,
        )));
        let tracker = VarietyMonitor::new();
        let energy_budgets = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        Self {
            algedonic,
            tracker,
            energy_budgets,
        }
    }
}

/// CNS runtime — single entry point for observability and regulation
///
/// Cheaply clonable: both fields are `Arc`-wrapped, so cloning only bumps
/// reference counts. All clones share the same inner state (variety tracker,
/// algedonic manager, subscribers).
#[derive(Clone)]
pub struct CnsRuntime {
    state: Arc<RwLock<CnsState>>,
    subscribers: Arc<RwLock<Vec<Arc<dyn CnsObserver>>>>,
}

impl CnsRuntime {
    pub fn with_threshold(threshold: u64) -> Self {
        Self {
            state: Arc::new(RwLock::new(CnsState::new(threshold))),
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // ── Health & Alerts ──

    pub async fn health(&self) -> CnsHealth {
        let state = self.state.read().await;
        {
            let mgr = state.algedonic.read();
            cns_health_check(&mgr)
        }
    }

    pub async fn alerts(&self) -> Vec<RuntimeAlert> {
        let state = self.state.read().await;
        state.algedonic.read().alerts().to_vec()
    }

    /// Get the configured default threshold from the algedonic manager.
    pub async fn default_threshold(&self) -> u64 {
        let state = self.state.read().await;
        state.algedonic.read().default_threshold()
    }

    pub async fn critical_alerts(&self) -> Vec<RuntimeAlert> {
        let state = self.state.read().await;
        {
            state
                .algedonic
                .read()
                .critical_alerts()
                .into_iter()
                .cloned()
                .collect()
        }
    }

    // ── Variety ──

    pub async fn variety(&self) -> Vec<(String, u64)> {
        let state = self.state.read().await;
        let domains: Vec<String> = state
            .tracker
            .domains()
            .iter()
            .map(|s| s.to_string())
            .collect();
        drop(state);

        let mut results = Vec::new();
        for domain in &domains {
            let state = self.state.read().await;
            let count = state.tracker.variety_for_domain(domain);
            drop(state);
            results.push((domain.clone(), count));
        }
        results
    }

    pub async fn variety_for_domain(&self, domain: &str) -> u64 {
        let state = self.state.read().await;
        state.tracker.variety_for_domain(domain)
    }

    /// Synchronous version of variety_for_domain — uses blocking_read() on the
    /// internal tokio RwLock. This enables sync contexts (e.g., metric collectors,
    /// CLI closures) to query CNS variety counters without requiring async.
    pub fn blocking_variety_for_domain(&self, domain: &str) -> u64 {
        let state = self.state.blocking_read();
        state.tracker.variety_for_domain(domain)
    }

    /// Increment variety and check thresholds — the loop closes here.
    /// After persisting variety, notifies subscribers whose interest mask
    /// includes the relevant span namespace.
    pub async fn increment_variety(&self, domain: &str, state_name: &str) {
        {
            let mut state = self.state.write().await;
            state.tracker.counter(domain).increment(state_name);
        }
        let alert = self.check_variety(domain).await;

        // Notify subscribers interested in this domain's span namespace
        if let Some(span_ns) = SpanNamespace::parse(domain) {
            let event = hkask_types::event::NuEvent::new(
                WebID::default(),
                hkask_types::event::Span::new(span_ns.clone(), "variety_incremented"),
                hkask_types::event::Phase::Act,
                serde_json::json!({"domain": domain, "state": state_name}),
                0,
            );
            let subscribers = self.subscribers.read().await;
            for observer in subscribers.iter() {
                if observer.interest_mask().iter().any(|ns| ns == &span_ns) {
                    observer.on_event(&event).await;
                }
            }
            drop(subscribers);

            if let Some(ref a) = alert
                && a.severity == crate::algedonic::AlertSeverity::Critical
            {
                emit_critical_depletion(self, a).await;
            }
        }
    }

    pub async fn check_variety(&self, domain: &str) -> Option<RuntimeAlert> {
        let counter = {
            let state = self.state.read().await;
            state
                .tracker
                .counters()
                .get(domain)
                .cloned()
                .unwrap_or_else(VarietyTracker::new)
        };

        let alert = {
            let state = self.state.write().await;
            let mut mgr = state.algedonic.write();
            mgr.check(&counter, domain).cloned()
        };

        // Depletion signals are now emitted from increment_variety after
        // it receives the alert from check_variety. Kept here for direct
        // callers that don't go through increment_variety.
        if let Some(ref alert) = alert
            && alert.severity == crate::algedonic::AlertSeverity::Critical
        {
            emit_critical_depletion(self, alert).await;
        }

        alert
    }

    pub async fn calibrate_threshold(&self, domain: &str, new_threshold: u64) {
        let state = self.state.write().await;
        {
            state
                .algedonic
                .write()
                .set_expected_variety(domain, new_threshold);
        }
        drop(state);
    }

    // ── Bot Observation (CNS Observer) ──

    /// Register a CnsObserver to receive events matching its interest mask.
    ///
    /// Observers are notified asynchronously when:
    /// - A variety increment matches their interest mask (on_event)
    /// - A depletion signal fires for their agent (on_depletion)
    /// - A backpressure signal fires (on_backpressure)
    ///
    /// Use `subscribe_async` when calling from an async context.
    pub fn subscribe(&self, observer: Arc<dyn CnsObserver>) {
        let mut subscribers = self.subscribers.blocking_write();
        subscribers.push(observer);
    }

    /// Register a CnsObserver to receive events matching its interest mask.
    ///
    /// This is the async version of subscribe, preferred when called from
    /// an async context (e.g., during bootstrap or from the API).
    pub async fn subscribe_async(&self, observer: Arc<dyn CnsObserver>) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.push(observer);
    }

    /// Emit a backpressure signal to all subscribers.
    ///
    /// Called by the Cybernetics Loop when energy budget depletion
    /// reaches critical levels, signaling downstream loops to throttle.
    pub async fn emit_backpressure(&self, signal: BackpressureSignal) {
        let subscribers = self.subscribers.read().await;
        for observer in subscribers.iter() {
            observer.on_backpressure(&signal).await;
        }
    }

    /// Register a energy budget for an agent.
    ///
    /// Called during agent pod creation so the CNS can track and replenish budgets.
    pub async fn register_energy_budget(&self, agent: WebID, budget: EnergyBudget) {
        let state = self.state.read().await;
        let mut budgets = state.energy_budgets.write().await;
        budgets.insert(agent, budget);
    }

    /// Replenish a specific agent's energy budget by a specific amount.
    ///
    /// Returns the new remaining gas after replenishment, or 0 if the agent
    /// has no registered budget.
    pub async fn replenish_agent_budget(&self, agent: &WebID, amount: EnergyCost) -> EnergyCost {
        let state = self.state.read().await;
        let mut budgets = state.energy_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.replenish_by(amount);
            let remaining = budget.remaining;
            tracing::info!(
                target: "cns.runtime",
                agent = %agent,
                amount = amount.0,
                remaining = remaining.0,
                "Replenished agent energy budget via CNS runtime"
            );
            remaining
        } else {
            EnergyCost::ZERO
        }
    }

    /// Get a read-only snapshot of an agent's energy budget status.
    ///
    /// Returns `None` if the agent has no registered budget.
    /// Used by the CNS service.
    pub async fn agent_gas_status(&self, agent: &WebID) -> Option<AgentEnergyStatus> {
        let state = self.state.read().await;
        let budgets = state.energy_budgets.read().await;
        budgets.get(agent).map(AgentEnergyStatus::from)
    }
}

impl Default for CnsRuntime {
    fn default() -> Self {
        Self::with_threshold(DEFAULT_THRESHOLD)
    }
}

/// Build and broadcast a `DepletionSignal` for a critical algedonic alert.
async fn emit_critical_depletion(runtime: &CnsRuntime, alert: &crate::algedonic::RuntimeAlert) {
    let signal = DepletionSignal {
        agent: WebID::default(),
        remaining: alert.threshold.saturating_sub(alert.deficit),
        cap: alert.threshold,
        usage_ratio: if alert.threshold > 0 {
            alert.deficit as f64 / alert.threshold as f64
        } else {
            1.0
        },
    };
    let subscribers = runtime.subscribers.read().await;
    for observer in subscribers.iter() {
        observer.on_depletion(&signal).await;
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: svc-cns-variety-001 — variety_monitor_tracks_distinct_states
    //
    // TASK 1 cybernetic property: the VarietyMonitor sensor must count
    // distinct tool states per domain for Ashby's Law compliance.
    // A domain with 5 distinct tool invocations must report variety=5.
    #[test]
    fn variety_monitor_tracks_distinct_states() {
        let mut monitor = VarietyMonitor::new();

        // Simulate 5 distinct tool invocations in domain "inference"
        for tool in &["chat", "embed", "generate", "classify", "tokenize"] {
            monitor.counter("inference").increment(tool);
        }

        assert_eq!(monitor.variety_for_domain("inference"), 5);
    }

    // REQ: svc-cns-variety-002 — variety_tracker_deficit_calculation
    //
    // When 3 distinct states exist but 10 are expected, deficit must be 7.
    #[test]
    fn variety_tracker_deficit_calculation() {
        let mut tracker = VarietyTracker::new();
        for i in 0..3 {
            tracker.increment(&format!("state_{}", i));
        }
        assert_eq!(tracker.deficit(10), 7);
        assert_eq!(tracker.variety(), 3);
    }

    // REQ: svc-cns-variety-003 — variety_monitor_multi_domain_isolation
    //
    // Two domains must track variety independently.
    #[test]
    fn variety_monitor_multi_domain_isolation() {
        let mut monitor = VarietyMonitor::new();

        monitor.counter("tools").increment("chat");
        monitor.counter("tools").increment("embed");
        monitor.counter("models").increment("llama3");
        monitor.counter("models").increment("qwen3");
        monitor.counter("models").increment("deepseek");

        assert_eq!(monitor.variety_for_domain("tools"), 2);
        assert_eq!(monitor.variety_for_domain("models"), 3);
        assert_eq!(monitor.variety_for_domain("nonexistent"), 0);
    }
}
