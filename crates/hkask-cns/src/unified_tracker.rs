//! Unified variety tracker — single SENSE point for all CNS observation domains
//!
//! Consolidates domain variety (4.1), bot metrics (4.3), sovereignty events (4.4),
//! and goal variety into a single variety accounting structure. All SENSE subloops
//! feed into this one tracker, ensuring consistent variety accounting per Ashby's Law.
//!
//! # Design Rationale
//!
//! The previous design used independent variety-tracking structures:
//! - `VarietyMonitor` (Loop 4.1 — domain-based variety)
//! - `SovereigntyObserver` (Loop 4.4 — sovereignty event variety) — removed in Phase 11a
//! - `GoalVarietyMonitor` (Loop 4.4 — goal variety) — removed in Phase 11a
//! - `BotMetricsCollector` (Loop 4.3 — bot health variety) — removed in Phase 11b
//!
//! Each tracked variety independently, which meant:
//! 1. Inconsistent variety accounting (different windows, different reset policies)
//! 2. Duplicate state management (multiple HashMaps where one suffices)
//! 3. Complex CnsState (multiple fields where one suffices)
//!
//! The unified tracker uses domain-prefixed keys so all variety counting
//! goes through a single `VarietyMonitor`, while preserving the domain-specific
//! methods each subloop needs.

use crate::algedonic::AlgedonicManager;
use crate::bot_metrics::{BotEvaluationMetrics, BotHealthStatus, CapabilityGap};
use crate::observers::sovereignty::{
    SovereigntyEvent, SovereigntyEventType, SovereigntyObserverState,
};
use crate::variety::VarietyMonitor;
use hkask_types::WebID;
use hkask_types::event::SpanCategory;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{error, warn};

/// Domain prefixes for unified variety counting.
///
/// All variety counters use domain-prefixed keys to avoid collisions
/// between different observation domains while sharing a single tracker.
pub mod domains {
    /// Sovereignty acquisition attempts: `sovereignty:acq:{webid}`
    pub const SOVEREIGNTY_ACQ: &str = "sovereignty:acq";
    /// Sovereignty kill zone alerts: `sovereignty:kz:{webid}`
    pub const SOVEREIGNTY_KZ: &str = "sovereignty:kz";
    /// Sovereignty boundary violations: `sovereignty:bv:{webid}`
    pub const SOVEREIGNTY_BV: &str = "sovereignty:bv";
    /// Bot variety tracking: `bot:{webid}:{category}`
    pub const BOT: &str = "bot";
    /// Goal variety tracking: `goal:{webid}`
    pub const GOAL: &str = "goal";
}

/// Acquisition attempt threshold before algedonic alert
const DEFAULT_ACQUISITION_THRESHOLD: u64 = 5;
/// Boundary violation threshold before algedonic alert
const DEFAULT_VIOLATION_THRESHOLD: u64 = 3;
/// Default expected variety for goal counting
const DEFAULT_GOAL_THRESHOLD: u64 = 10;
/// Default expected variety for bots
const DEFAULT_BOT_EXPECTED_VARIETY: u64 = 100;

/// Unified variety tracker for all CNS observation domains.
///
/// A single structure that tracks variety across all SENSE subloops:
/// - Loop 4.1: Domain-based variety (inference, memory, governance, etc.)
/// - Loop 4.3: Bot health metrics (per-WebID evaluation)
/// - Loop 4.4: Sovereignty events (acquisition, violations, kill zone)
/// - Goal variety (per-WebID goal counting)
///
/// All variety counting goes through a single `VarietyMonitor`, ensuring
/// consistent windowing and reset behavior. The algedonic manager is shared
/// so all domains can trigger alerts through the same escalation path.
pub struct UnifiedVarietyTracker {
    /// Single variety monitor for all domains
    variety: VarietyMonitor,
    /// Shared algedonic manager for alert escalation
    algedonic: Arc<RwLock<AlgedonicManager>>,
    /// Sovereignty event state (per-WebID counters)
    sovereignty_state: SovereigntyObserverState,
    /// Per-bot metrics (evaluation data)
    bot_metrics: HashMap<WebID, BotEvaluationMetrics>,
    /// Per-bot success tracking (observe_count, outcome_count)
    bot_success: HashMap<WebID, (u64, u64)>,
    /// Per-bot name mapping
    bot_names: HashMap<WebID, String>,
    /// Per-WebID goal counts
    goal_counts: HashMap<WebID, u64>,
    /// Default goal threshold for algedonic alert
    goal_threshold: u64,
    /// Acquisition attempt threshold for algedonic alert
    acquisition_threshold: u64,
    /// Boundary violation threshold for algedonic alert
    violation_threshold: u64,
    /// Default expected variety for bot evaluation
    bot_expected_variety: u64,
}

impl UnifiedVarietyTracker {
    /// Create a new unified tracker with default thresholds.
    pub fn new(algedonic: Arc<RwLock<AlgedonicManager>>) -> Self {
        Self {
            variety: VarietyMonitor::new(),
            algedonic,
            sovereignty_state: SovereigntyObserverState::default(),
            bot_metrics: HashMap::new(),
            bot_success: HashMap::new(),
            bot_names: HashMap::new(),
            goal_counts: HashMap::new(),
            goal_threshold: DEFAULT_GOAL_THRESHOLD,
            acquisition_threshold: DEFAULT_ACQUISITION_THRESHOLD,
            violation_threshold: DEFAULT_VIOLATION_THRESHOLD,
            bot_expected_variety: DEFAULT_BOT_EXPECTED_VARIETY,
        }
    }

    // =========================================================================
    // Loop 4.1 — Domain-based variety (Ashby's Law)
    // =========================================================================

    /// Increment variety counter for a domain.
    pub fn increment_variety(&mut self, domain: &str, state_name: &str) {
        self.variety.counter(domain).increment(state_name);
    }

    /// Get variety count for a specific domain.
    pub fn variety_for_domain(&self, domain: &str) -> u64 {
        self.variety.variety_for_domain(domain)
    }

    /// Get all domain names with variety counters.
    pub fn variety_domains(&self) -> Vec<&str> {
        self.variety.domains()
    }

    /// Check if any domain exceeds the deficit threshold.
    pub fn exceeds_variety_threshold(&self, threshold: u64, expected_variety: u64) -> bool {
        self.variety.exceeds_threshold(threshold, expected_variety)
    }

    /// Get total variety deficit across all domains.
    pub fn total_variety_deficit(&self, expected_per_domain: u64) -> u64 {
        self.variety.total_deficit(expected_per_domain)
    }

    /// Get a reference to the underlying variety monitor.
    pub fn variety_monitor(&self) -> &VarietyMonitor {
        &self.variety
    }

    /// Get a mutable reference to the underlying variety monitor.
    pub fn variety_monitor_mut(&mut self) -> &mut VarietyMonitor {
        &mut self.variety
    }

    // =========================================================================
    // Loop 4.4 — Sovereignty observation
    // =========================================================================

    /// Process a sovereignty event.
    ///
    /// Updates sovereignty state counters and triggers algedonic alerts
    /// when thresholds are exceeded. This replaces the need for a separate
    /// `SovereigntyObserver` struct.
    pub fn process_sovereignty_event(&mut self, event: SovereigntyEvent) {
        self.sovereignty_state.total_events += 1;

        match event.event_type {
            SovereigntyEventType::AcquisitionAttempt => {
                *self
                    .sovereignty_state
                    .acquisition_attempts
                    .entry(event.webid)
                    .or_insert(0) += 1;
                let count = self.sovereignty_state.acquisition_attempts[&event.webid];

                // Also track as variety
                self.variety
                    .counter(&format!("{}:{}", domains::SOVEREIGNTY_ACQ, event.webid))
                    .increment("attempt");

                if count >= self.acquisition_threshold {
                    self.trigger_algedonic_alert(
                        &event.webid,
                        domains::SOVEREIGNTY_ACQ,
                        count,
                        &format!(
                            "WebID {} exceeded acquisition threshold ({} attempts)",
                            event.webid, count
                        ),
                    );
                }
            }
            SovereigntyEventType::KillZoneAlert => {
                *self
                    .sovereignty_state
                    .kill_zone_alerts
                    .entry(event.webid)
                    .or_insert(0) += 1;
                let count = self.sovereignty_state.kill_zone_alerts[&event.webid];

                // Also track as variety
                self.variety
                    .counter(&format!("{}:{}", domains::SOVEREIGNTY_KZ, event.webid))
                    .increment("alert");

                // Kill zone alerts immediately trigger algedonic escalation
                self.trigger_algedonic_alert(
                    &event.webid,
                    domains::SOVEREIGNTY_KZ,
                    count,
                    &format!(
                        "WebID {} entered kill zone (VC investment < threshold)",
                        event.webid
                    ),
                );
            }
            SovereigntyEventType::BoundaryViolation => {
                *self
                    .sovereignty_state
                    .boundary_violations
                    .entry(event.webid)
                    .or_insert(0) += 1;
                let count = self.sovereignty_state.boundary_violations[&event.webid];

                // Also track as variety
                self.variety
                    .counter(&format!("{}:{}", domains::SOVEREIGNTY_BV, event.webid))
                    .increment("violation");

                if count >= self.violation_threshold {
                    self.trigger_algedonic_alert(
                        &event.webid,
                        domains::SOVEREIGNTY_BV,
                        count,
                        &format!(
                            "WebID {} exceeded boundary violation threshold ({} violations)",
                            event.webid, count
                        ),
                    );
                }
            }
            SovereigntyEventType::ConsentGranted | SovereigntyEventType::ConsentRevoked => {
                warn!(
                    target: "cns.tracker.sovereignty",
                    webid = %event.webid,
                    event_type = ?event.event_type,
                    "Consent state changed"
                );
            }
        }
    }

    /// Get current sovereignty state.
    pub fn sovereignty_state(&self) -> &SovereigntyObserverState {
        &self.sovereignty_state
    }

    /// Get acquisition attempt count for a WebID.
    pub fn acquisition_count(&self, webid: &WebID) -> u64 {
        self.sovereignty_state
            .acquisition_attempts
            .get(webid)
            .copied()
            .unwrap_or(0)
    }

    /// Get boundary violation count for a WebID.
    pub fn violation_count(&self, webid: &WebID) -> u64 {
        self.sovereignty_state
            .boundary_violations
            .get(webid)
            .copied()
            .unwrap_or(0)
    }

    /// Reset sovereignty state.
    pub fn reset_sovereignty(&mut self) {
        self.sovereignty_state = SovereigntyObserverState::default();
    }

    /// Trigger an algedonic alert through the shared algedonic manager.
    fn trigger_algedonic_alert(&self, webid: &WebID, domain: &str, deficit: u64, message: &str) {
        let mut manager = match self.algedonic.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!(target: "cns.tracker", error = %e, "AlgedonicManager lock poisoned during trigger_algedonic_alert");
                return;
            }
        };

        error!(
            target: "cns.tracker.sovereignty",
            webid = %webid,
            domain = %domain,
            deficit = deficit,
            "ALGEDONIC ALERT - Sovereignty violation"
        );

        // Create variety counter for this domain
        let mut counter = crate::variety::VarietyTracker::new();
        for _ in 0..deficit {
            counter.increment(domain);
        }

        // Check and generate alert
        if let Some(alert) = manager.check(&counter, domain)
            && alert.should_escalate()
        {
            error!(
                target: "cns.algedonic",
                webid = %webid,
                message = %message,
                "Escalating sovereignty violation to Curator/human"
            );
        }
    }

    // =========================================================================
    // Loop 4.3 — Bot metrics (per-WebID evaluation)
    // =========================================================================

    /// Register a bot in the tracker.
    pub fn register_bot(&mut self, bot_id: WebID, bot_name: String) {
        self.bot_names.insert(bot_id, bot_name.clone());
        self.bot_metrics
            .entry(bot_id)
            .or_insert_with(|| BotEvaluationMetrics::new(bot_id, bot_name));
        self.bot_success.entry(bot_id).or_insert((0, 0));
    }

    /// Record a span observation for a bot.
    pub fn record_bot_span(&mut self, bot_id: &WebID, category: SpanCategory) {
        if let Some(metrics) = self.bot_metrics.get_mut(bot_id) {
            *metrics.span_counts.entry(category).or_insert(0) += 1;
            metrics.last_report = chrono::Utc::now();
        }
        self.variety
            .counter(&format!("{}:{}", domains::BOT, bot_id))
            .increment(category.as_str());
    }

    /// Record a success (Outcome phase) for a bot.
    pub fn record_bot_success(&mut self, bot_id: &WebID) {
        if let Some((_, outcome)) = self.bot_success.get_mut(bot_id) {
            *outcome += 1;
        }
        self.update_bot_success_rate(bot_id);
    }

    /// Record an observation (Observe phase) for a bot.
    pub fn record_bot_observation(&mut self, bot_id: &WebID) {
        if let Some((observe, _)) = self.bot_success.get_mut(bot_id) {
            *observe += 1;
        }
        self.update_bot_success_rate(bot_id);
    }

    /// Record energy consumption for a bot.
    pub fn record_bot_energy(&mut self, bot_id: &WebID, amount: u64) {
        if let Some(metrics) = self.bot_metrics.get_mut(bot_id) {
            metrics.energy_consumed += amount;
            metrics.last_report = chrono::Utc::now();
        }
    }

    /// Set energy budget for a bot.
    pub fn set_bot_energy_budget(&mut self, bot_id: &WebID, budget: u64) {
        if let Some(metrics) = self.bot_metrics.get_mut(bot_id) {
            metrics.energy_budget = budget;
        }
    }

    /// Record an algedonic alert for a bot.
    pub fn record_bot_alert(&mut self, bot_id: &WebID) {
        if let Some(metrics) = self.bot_metrics.get_mut(bot_id) {
            metrics.algedonic_alerts += 1;
            metrics.last_report = chrono::Utc::now();
        }
    }

    /// Record a sovereignty violation for a bot.
    pub fn record_bot_sovereignty_violation(&mut self, bot_id: &WebID) {
        if let Some(metrics) = self.bot_metrics.get_mut(bot_id) {
            metrics.sovereignty_violations += 1;
            metrics.last_report = chrono::Utc::now();
        }
    }

    /// Get evaluation metrics for a specific bot.
    pub fn evaluate_bot(&self, bot_id: &WebID) -> Option<BotEvaluationMetrics> {
        self.bot_metrics.get(bot_id).cloned()
    }

    /// Get evaluation metrics for all bots.
    pub fn evaluate_all_bots(&self) -> Vec<BotEvaluationMetrics> {
        self.bot_metrics.values().cloned().collect()
    }

    /// Get health status for a specific bot.
    pub fn bot_health_status(&self, bot_id: &WebID) -> Option<BotHealthStatus> {
        self.bot_metrics.get(bot_id).map(|m| m.health_status())
    }

    /// Identify capability gaps for a specific bot.
    pub fn identify_bot_gaps(
        &self,
        bot_id: &WebID,
        success_threshold: f64,
        deficit_threshold: u64,
    ) -> Vec<CapabilityGap> {
        if let Some(metrics) = self.bot_metrics.get(bot_id) {
            metrics.capability_gaps(success_threshold, deficit_threshold)
        } else {
            Vec::new()
        }
    }

    /// Identify capability gaps across all bots.
    pub fn identify_all_bot_gaps(
        &self,
        success_threshold: f64,
        deficit_threshold: u64,
    ) -> Vec<CapabilityGap> {
        self.bot_metrics
            .values()
            .flat_map(|m| m.capability_gaps(success_threshold, deficit_threshold))
            .collect()
    }

    /// Update success rate for a bot from tracking data.
    fn update_bot_success_rate(&mut self, bot_id: &WebID) {
        if let Some((observe, outcome)) = self.bot_success.get(bot_id)
            && let Some(metrics) = self.bot_metrics.get_mut(bot_id)
            && *observe > 0
        {
            metrics.success_rate = *outcome as f64 / *observe as f64;
        }
    }

    // =========================================================================
    // Goal variety (per-WebID goal counting)
    // =========================================================================

    /// Register a goal tracker for a WebID.
    pub fn register_goal_tracker(&mut self, webid: WebID) {
        self.goal_counts.entry(webid).or_insert(0);
    }

    /// Update goal count for a WebID.
    pub fn update_goal_count(&mut self, webid: &WebID, count: u64) {
        self.goal_counts.insert(*webid, count);
    }

    /// Increment goal count for a WebID.
    pub fn increment_goal(&mut self, webid: &WebID) {
        *self.goal_counts.entry(*webid).or_insert(0) += 1;
    }

    /// Decrement goal count for a WebID.
    pub fn decrement_goal(&mut self, webid: &WebID) {
        if let Some(count) = self.goal_counts.get_mut(webid) {
            *count = count.saturating_sub(1);
        }
    }

    /// Check if any WebID exceeds the goal threshold.
    pub fn exceeds_goal_threshold(&self) -> bool {
        self.goal_counts
            .values()
            .any(|&count| count > self.goal_threshold)
    }

    /// Set the goal threshold for algedonic alerts.
    pub fn set_goal_threshold(&mut self, threshold: u64) {
        self.goal_threshold = threshold;
    }

    /// Get goal count for a specific WebID.
    pub fn goal_count(&self, webid: &WebID) -> u64 {
        self.goal_counts.get(webid).copied().unwrap_or(0)
    }

    /// Set the acquisition threshold.
    pub fn set_acquisition_threshold(&mut self, threshold: u64) {
        self.acquisition_threshold = threshold;
    }

    /// Set the violation threshold.
    pub fn set_violation_threshold(&mut self, threshold: u64) {
        self.violation_threshold = threshold;
    }

    /// Set the bot expected variety.
    pub fn set_bot_expected_variety(&mut self, expected: u64) {
        self.bot_expected_variety = expected;
    }
}

