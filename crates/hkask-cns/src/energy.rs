//! Gas budget enforcement — preventing infinite loops (analogous to Ethereum gas)
//
//! Every operation costs gas. When the budget is exhausted, the operation
//! is rejected. Gas replenishes periodically, managed by the Cybernetics Loop.
//
//! The gas model is deliberately simple: each MCP tool invocation has a
//! configured cost, and budgets refill at a steady rate. This prevents
//! runaway agents while keeping the implementation minimal.

use std::fmt;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

// ── Domain newtypes (P2.3) ──────────────────────────────────────────────────

/// Gas cost of a single tool invocation.
///
/// Newtype wrapper around `u64` that prevents accidental confusion with
/// other unsigned quantities in the gas subsystem (cap, remaining, rate).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct GasCost(pub u64);

impl GasCost {
    /// Zero gas cost — used for free/internal operations.
    pub const ZERO: GasCost = GasCost(0);

    /// Create a gas cost from a raw `u64`.
    pub fn from_raw(value: u64) -> Self {
        GasCost(value)
    }

    /// Return the raw `u64` value.
    pub fn as_raw(self) -> u64 {
        self.0
    }
}

impl From<u64> for GasCost {
    fn from(value: u64) -> Self {
        GasCost(value)
    }
}

impl From<GasCost> for u64 {
    fn from(cost: GasCost) -> Self {
        cost.0
    }
}

impl Deref for GasCost {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GasCost {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for GasCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} gas", self.0)
    }
}

/// Default gas budget alert threshold — alert when 80% of gas is consumed.
pub const DEFAULT_GAS_ALERT_THRESHOLD: f64 = 0.8;

/// Threshold for R̄ (confidence) in the curation gate's transition zone.
///
/// Newtype wrapper around `f64` that prevents accidental confusion with
/// other floating-point quantities (priority weight, usage ratio, etc.).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RBarThreshold(pub f64);

impl RBarThreshold {
    /// Create an R̄ threshold, clamped to [0.0, 1.0].
    pub fn new(value: f64) -> Self {
        RBarThreshold(value.clamp(0.0, 1.0))
    }

    /// Default upper threshold for the Proceed zone.
    pub const DEFAULT_UPPER: RBarThreshold = RBarThreshold(0.8);
    /// Default lower threshold for the Suppress zone.
    pub const DEFAULT_LOWER: RBarThreshold = RBarThreshold(0.3);

    /// Return the raw `f64` value.
    pub fn as_raw(self) -> f64 {
        self.0
    }
}

impl Deref for RBarThreshold {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RBarThreshold {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for RBarThreshold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "R̄={:.2}", self.0)
    }
}

/// Communication queue depth for backpressure regulation.
///
/// Newtype wrapper that prevents accidental confusion with other numeric
/// thresholds in `SetPoints` (gas, variety deficit, error rate).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct QueueDepth(pub f64);

impl QueueDepth {
    /// Create a queue depth threshold.
    pub fn new(value: f64) -> Self {
        QueueDepth(value.max(0.0))
    }

    /// Default backpressure threshold: 100 messages.
    pub const DEFAULT_BACKPRESSURE: QueueDepth = QueueDepth(100.0);

    /// Return the raw `f64` value.
    pub fn as_raw(self) -> f64 {
        self.0
    }
}

impl Deref for QueueDepth {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for QueueDepth {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for QueueDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "depth={:.0}", self.0)
    }
}

/// Default priority for serde default.
const fn default_priority() -> f64 {
    1.0
}

/// Gas budget allocation.
///
/// Gas units are dimensionless — they represent computational cost on a
/// shared scale. Inference tools are more expensive than internal tools.
/// The `GasEstimator` trait maps each (server, tool) pair to a gas cost.
///
/// Gas replenishes periodically via `replenish()`, called by the
/// Cybernetics Loop on its regulation cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasBudget {
    /// Maximum gas capacity.
    pub cap: GasCost,
    /// Currently available gas.
    pub remaining: GasCost,
    /// Gas units replenished per replenishment cycle.
    /// Set to 0 to disable automatic replenishment (one-shot budget).
    pub replenish_rate: GasCost,
    /// Alert threshold as a ratio (0.0–1.0). Alert when usage ≥ threshold.
    pub alert_threshold: f64,
    /// Whether to hard-reject when budget is exhausted.
    /// If false, operations proceed but emit depletion warnings.
    pub hard_limit: bool,
    /// Amount reserved by in-flight operations (hold-settle pattern).
    /// Gas that has been reserved but not yet settled.
    #[serde(default)]
    pub reserved: GasCost,
    /// Priority weight for replenishment scaling (0.0–1.0).
    /// Higher priority agents receive a larger share of replenishment.
    /// Defaults to 1.0 (full replenishment).
    #[serde(default = "default_priority")]
    pub priority: f64,
}

impl GasBudget {
    /// Create a new gas budget with the given cap.
    ///
    /// Defaults: replenish_rate = cap / 10, alert_threshold = DEFAULT_GAS_ALERT_THRESHOLD, hard_limit = true.
    pub fn new(cap: GasCost) -> Self {
        let cap_raw = cap.0;
        Self {
            remaining: cap,
            replenish_rate: GasCost(cap_raw / 10),
            alert_threshold: DEFAULT_GAS_ALERT_THRESHOLD,
            hard_limit: true,
            reserved: GasCost::ZERO,
            priority: 1.0,
            cap,
        }
    }

    /// Create a gas budget with unlimited capacity (u64::MAX).
    ///
    /// Useful for agents that should never be throttled. The budget still
    /// tracks usage for observability but never hard-rejects.
    pub fn unlimited() -> Self {
        Self::new(GasCost(u64::MAX)).with_hard_limit(false)
    }

    /// Set the replenishment rate (gas units per cycle).
    pub fn with_replenish_rate(mut self, rate: GasCost) -> Self {
        self.replenish_rate = rate;
        self
    }

    /// Set the alert threshold (0.0–1.0).
    pub fn with_alert_threshold(mut self, threshold: f64) -> Self {
        self.alert_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set whether to hard-reject on exhaustion.
    pub fn with_hard_limit(mut self, hard: bool) -> Self {
        self.hard_limit = hard;
        self
    }

    /// Check whether an operation costing `gas` can proceed.
    ///
    /// Returns `true` if the gas fits within available (remaining - reserved) budget.
    pub fn can_proceed(&self, gas: GasCost) -> bool {
        let available = self.available();
        gas.0 <= available.0 || !self.hard_limit
    }

    /// Available gas = remaining - reserved.
    pub fn available(&self) -> GasCost {
        GasCost(self.remaining.0.saturating_sub(self.reserved.0))
    }

    /// Reserve gas for an in-flight operation (hold-settle pattern).
    ///
    /// Returns `Ok(reserved)` if gas was reserved, `Err` if insufficient.
    /// Reserved gas is deducted from available but not from remaining until
    /// `settle()` is called.
    pub fn reserve(&mut self, gas: GasCost) -> Result<GasCost, GasError> {
        let available = self.available();
        if self.hard_limit && gas.0 > available.0 {
            return Err(GasError::BudgetExceeded {
                requested: gas,
                remaining: available,
            });
        }
        self.reserved = GasCost(self.reserved.0.saturating_add(gas.0));
        Ok(gas)
    }

    /// Settle a reserved operation: deduct actual cost from remaining.
    ///
    /// Since `reserve()` only tracks reserved gas without deducting from remaining,
    /// settlement simply removes the reservation and deducts the actual cost.
    /// If actual < reserved, the remaining budget was never reduced for the
    /// reservation, so the difference is implicitly refunded.
    ///
    /// If actual > reserved (under-estimation), the extra is deducted from
    /// remaining as well.
    pub fn settle(
        &mut self,
        reserved_gas: GasCost,
        actual_gas: GasCost,
    ) -> Result<GasCost, GasError> {
        // Remove the reservation
        self.reserved = GasCost(self.reserved.0.saturating_sub(reserved_gas.0));

        // Deduct actual cost from remaining
        if self.hard_limit && actual_gas.0 > self.remaining.0 {
            return Err(GasError::BudgetExceeded {
                requested: actual_gas,
                remaining: self.remaining,
            });
        }
        self.remaining = GasCost(self.remaining.0.saturating_sub(actual_gas.0));
        Ok(actual_gas)
    }

    /// Consume gas immediately (non-reserved path).
    ///
    /// For operations where the cost is known exactly at call time
    /// (no hold-settle needed).
    pub fn consume(&mut self, gas: GasCost) -> Result<GasCost, GasError> {
        if self.hard_limit && gas.0 > self.remaining.0 {
            return Err(GasError::BudgetExceeded {
                requested: gas,
                remaining: self.remaining,
            });
        }
        self.remaining = GasCost(self.remaining.0.saturating_sub(gas.0));
        Ok(gas)
    }

    /// Replenish gas budget by the configured replenish_rate.
    ///
    /// Called by the Cybernetics Loop on its regulation cycle.
    /// Never exceeds cap.
    pub fn replenish(&mut self) {
        if self.replenish_rate.0 > 0 {
            self.remaining = GasCost((self.remaining.0 + self.replenish_rate.0).min(self.cap.0));
        }
    }

    /// Replenish gas budget by a specific amount (used by CuratorDirective::ReplenishBudget).
    pub fn replenish_by(&mut self, amount: GasCost) {
        self.remaining = GasCost((self.remaining.0 + amount.0).min(self.cap.0));
    }

    /// Replenish gas budget by `amount * priority`, weighted by the given priority.
    ///
    /// The effective replenishment is `(amount * priority).round()`, never exceeding cap.
    /// If `amount * priority` rounds to 0, at least 1 unit is replenished (so
    /// low-priority directives still have effect).
    pub fn replenish_by_weighted(&mut self, amount: GasCost, priority: f64) -> GasCost {
        let scaled = (amount.0 as f64 * priority.clamp(0.0, 1.0)).round() as u64;
        let effective = scaled.max(1);
        let before = self.remaining.0;
        self.remaining = GasCost((self.remaining.0 + effective).min(self.cap.0));
        GasCost(self.remaining.0 - before)
    }

    /// Usage ratio: 0.0 = full budget, 1.0 = empty.
    pub(crate) fn usage_ratio(&self) -> f64 {
        1.0 - (self.remaining.0 as f64 / self.cap.0.max(1) as f64)
    }
}

/// Read-only snapshot of an agent's gas budget status.
///
/// Returned by `CyberneticsLoop::agent_gas_status()` and `CnsRuntime::agent_gas_status()`
/// for use by the `cns_energy` MCP tool and InferenceLoop gas sync.
pub struct AgentGasStatus {
    /// Maximum gas capacity.
    pub cap: GasCost,
    /// Currently available gas (total remaining, including reserved).
    pub remaining: GasCost,
    /// Gas reserved by in-flight operations.
    pub reserved: GasCost,
    /// Available gas = remaining - reserved.
    pub available: GasCost,
    /// Usage ratio: 0.0 = full budget, 1.0 = empty.
    pub usage_ratio: f64,
    /// Whether the agent will be hard-rejected on exhaustion.
    pub hard_limit: bool,
    /// The ratio at which alerts fire (0.0–1.0).
    pub alert_threshold: f64,
}

impl From<&GasBudget> for AgentGasStatus {
    fn from(budget: &GasBudget) -> Self {
        Self {
            cap: budget.cap,
            remaining: budget.remaining,
            reserved: budget.reserved,
            available: budget.available(),
            usage_ratio: budget.usage_ratio(),
            hard_limit: budget.hard_limit,
            alert_threshold: budget.alert_threshold,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum GasError {
    #[error("Gas budget exceeded: {requested}, remaining {remaining}")]
    BudgetExceeded {
        requested: GasCost,
        remaining: GasCost,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_cost_newtype_basics() {
        let cost = GasCost(100);
        assert_eq!(*cost, 100); // Deref
        assert_eq!(cost.as_raw(), 100);
        assert_eq!(cost.to_string(), "100 gas");
        assert_eq!(GasCost::ZERO, GasCost(0));
    }

    #[test]
    fn gas_cost_from_conversions() {
        let cost: GasCost = 500u64.into();
        assert_eq!(cost, GasCost(500));
        let raw: u64 = cost.into();
        assert_eq!(raw, 500);
    }

    #[test]
    fn gas_cost_display() {
        assert_eq!(GasCost(0).to_string(), "0 gas");
        assert_eq!(GasCost(10_000).to_string(), "10000 gas");
    }

    #[test]
    fn rbar_threshold_display() {
        assert_eq!(RBarThreshold(0.8).to_string(), "R̄=0.80");
        assert_eq!(RBarThreshold::DEFAULT_UPPER.to_string(), "R̄=0.80");
    }

    #[test]
    fn queue_depth_display() {
        assert_eq!(QueueDepth(100.0).to_string(), "depth=100");
        assert_eq!(QueueDepth::DEFAULT_BACKPRESSURE.to_string(), "depth=100");
    }

    #[test]
    fn gas_budget_new_has_defaults() {
        let budget = GasBudget::new(GasCost(1000));
        assert_eq!(budget.cap, GasCost(1000));
        assert_eq!(budget.remaining, GasCost(1000));
        assert_eq!(budget.replenish_rate, GasCost(100)); // cap / 10
        assert_eq!(budget.reserved, GasCost::ZERO);
        assert!(budget.hard_limit);
    }

    #[test]
    fn gas_budget_unlimited() {
        let budget = GasBudget::unlimited();
        assert_eq!(budget.cap, GasCost(u64::MAX));
        assert!(!budget.hard_limit);
    }

    #[test]
    fn gas_budget_consume_basic() {
        let mut budget = GasBudget::new(GasCost(1000));
        assert_eq!(budget.consume(GasCost(100)).unwrap(), GasCost(100));
        assert_eq!(budget.remaining, GasCost(900));
    }

    #[test]
    fn gas_budget_consume_exhausted() {
        let mut budget = GasBudget::new(GasCost(100)).with_hard_limit(true);
        let result = budget.consume(GasCost(200));
        assert!(result.is_err());
    }

    #[test]
    fn gas_budget_reserve_and_settle() {
        let mut budget = GasBudget::new(GasCost(1000));
        // Reserve 100 gas
        assert_eq!(budget.reserve(GasCost(100)).unwrap(), GasCost(100));
        assert_eq!(budget.reserved, GasCost(100));
        assert_eq!(budget.remaining, GasCost(1000)); // Not consumed yet
        assert_eq!(budget.available(), GasCost(900)); // Available = remaining - reserved

        // Settle with actual cost of 80 — refund 20
        assert_eq!(
            budget.settle(GasCost(100), GasCost(80)).unwrap(),
            GasCost(80)
        );
        assert_eq!(budget.reserved, GasCost::ZERO);
        assert_eq!(budget.remaining, GasCost(920)); // 1000 - 80 = 920
    }

    #[test]
    fn gas_budget_reserve_settle_over_cost() {
        let mut budget = GasBudget::new(GasCost(1000));
        assert_eq!(budget.reserve(GasCost(100)).unwrap(), GasCost(100));
        // Actual cost exceeds reservation
        assert_eq!(
            budget.settle(GasCost(100), GasCost(150)).unwrap(),
            GasCost(150)
        );
        assert_eq!(budget.remaining, GasCost(850));
    }

    #[test]
    fn gas_budget_reserve_insufficient_available() {
        let mut budget = GasBudget::new(GasCost(200)).with_hard_limit(true);
        assert_eq!(budget.reserve(GasCost(100)).unwrap(), GasCost(100));
        // Only 100 available (200 - 100 reserved)
        let result = budget.reserve(GasCost(150));
        assert!(result.is_err());
    }

    #[test]
    fn gas_budget_replenish() {
        let mut budget = GasBudget::new(GasCost(1000)).with_replenish_rate(GasCost(100));
        budget.consume(GasCost(500)).unwrap();
        assert_eq!(budget.remaining, GasCost(500));
        budget.replenish();
        assert_eq!(budget.remaining, GasCost(600));
    }

    #[test]
    fn gas_budget_replenish_capped() {
        let mut budget = GasBudget::new(GasCost(1000)).with_replenish_rate(GasCost(500));
        // No consumption — replenish should cap at cap
        budget.replenish();
        assert_eq!(budget.remaining, GasCost(1000));
    }

    #[test]
    fn gas_budget_replenish_by_directive() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.consume(GasCost(800)).unwrap();
        assert_eq!(budget.remaining, GasCost(200));
        budget.replenish_by(GasCost(500));
        assert_eq!(budget.remaining, GasCost(700)); // min(200 + 500, 1000)
    }

    #[test]
    fn gas_budget_soft_limit() {
        let mut budget = GasBudget::new(GasCost(100)).with_hard_limit(false);
        // Soft limit: operations proceed even when budget is exhausted
        let result = budget.consume(GasCost(200));
        assert!(result.is_ok()); // Soft limit allows
        assert_eq!(budget.remaining, GasCost::ZERO); // Saturating sub
    }

    #[test]
    fn gas_budget_usage_ratio() {
        let budget = GasBudget::new(GasCost(1000));
        assert!((budget.usage_ratio() - 0.0).abs() < f64::EPSILON);
    }

    // Priority and weighted replenishment tests

    #[test]
    fn gas_budget_new_has_default_priority() {
        let budget = GasBudget::new(GasCost(1000));
        assert!((budget.priority - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn replenish_by_weighted_full_priority() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.consume(GasCost(800)).unwrap();
        assert_eq!(budget.remaining, GasCost(200));
        let replenished = budget.replenish_by_weighted(GasCost(500), 1.0);
        assert_eq!(replenished, GasCost(500));
        assert_eq!(budget.remaining, GasCost(700)); // min(200 + 500, 1000)
    }

    #[test]
    fn replenish_by_weighted_half_priority() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.consume(GasCost(800)).unwrap();
        assert_eq!(budget.remaining, GasCost(200));
        let replenished = budget.replenish_by_weighted(GasCost(500), 0.5);
        assert_eq!(replenished, GasCost(250)); // 500 * 0.5 = 250
        assert_eq!(budget.remaining, GasCost(450)); // 200 + 250
    }

    #[test]
    fn replenish_by_weighted_minimum_one_unit() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.consume(GasCost(800)).unwrap();
        assert_eq!(budget.remaining, GasCost(200));
        // 5 * 0.1 = 0.5, rounds to 0, but minimum is 1
        let replenished = budget.replenish_by_weighted(GasCost(5), 0.1);
        assert_eq!(replenished, GasCost(1));
    }

    #[test]
    fn replenish_by_weighted_capped_at_cap() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.consume(GasCost(100)).unwrap();
        assert_eq!(budget.remaining, GasCost(900));
        // Request replenish of 500 at full priority, but only 100 fits
        let replenished = budget.replenish_by_weighted(GasCost(500), 1.0);
        assert_eq!(replenished, GasCost(100)); // capped at cap
        assert_eq!(budget.remaining, GasCost(1000));
    }

    #[test]
    fn replenish_by_weighted_clamps_priority() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.consume(GasCost(800)).unwrap();
        // priority > 1.0 is clamped to 1.0
        let replenished = budget.replenish_by_weighted(GasCost(500), 2.0);
        assert_eq!(replenished, GasCost(500));
        assert_eq!(budget.remaining, GasCost(700));
    }

    #[test]
    fn replenish_by_weighted_zero_priority_gives_minimum() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.consume(GasCost(800)).unwrap();
        // Even with priority 0.0, minimum 1 unit is replenished
        let replenished = budget.replenish_by_weighted(GasCost(500), 0.0);
        assert_eq!(replenished, GasCost(1));
        assert_eq!(budget.remaining, GasCost(201));
    }
}
