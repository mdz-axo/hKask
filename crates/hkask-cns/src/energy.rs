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

// Re-export domain newtypes that live in the substrate crate (hkask-types).
pub use hkask_types::cns::{QueueDepth, RBarThreshold};

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

#[cfg(test)]
mod tests {
    use super::*;

    // ── GasCost ──────────────────────────────────────────────────────────

    #[test]
    fn gas_cost_zero_is_zero() {
        assert_eq!(GasCost::ZERO.0, 0);
    }

    #[test]
    fn gas_cost_from_raw_round_trips() {
        let cost = GasCost::from_raw(42);
        assert_eq!(cost.as_raw(), 42);
    }

    #[test]
    fn gas_cost_from_u64() {
        let cost: GasCost = 99u64.into();
        assert_eq!(cost.0, 99);
    }

    #[test]
    fn gas_cost_into_u64() {
        let cost = GasCost(77);
        let raw: u64 = cost.into();
        assert_eq!(raw, 77);
    }

    #[test]
    fn gas_cost_display() {
        assert_eq!(format!("{}", GasCost(50)), "50 gas");
    }

    #[test]
    fn gas_cost_ordering() {
        assert!(GasCost(1) < GasCost(2));
        assert!(GasCost(100) > GasCost(50));
    }

    // ── GasBudget creation ─────────────────────────────────────────────

    #[test]
    fn budget_new_sets_remaining_to_cap() {
        let budget = GasBudget::new(GasCost(1000));
        assert_eq!(budget.remaining, GasCost(1000));
        assert_eq!(budget.cap, GasCost(1000));
    }

    #[test]
    fn budget_new_default_replenish_rate() {
        let budget = GasBudget::new(GasCost(1000));
        assert_eq!(budget.replenish_rate, GasCost(100)); // cap / 10
    }

    #[test]
    fn budget_new_hard_limit_default_true() {
        let budget = GasBudget::new(GasCost(1000));
        assert!(budget.hard_limit);
    }

    #[test]
    fn budget_unlimited_never_hard_rejects() {
        let budget = GasBudget::unlimited();
        assert!(!budget.hard_limit);
        assert_eq!(budget.cap, GasCost(u64::MAX));
    }

    // ── GasBudget can_proceed ───────────────────────────────────────────

    #[test]
    fn can_proceed_when_sufficient() {
        let budget = GasBudget::new(GasCost(1000));
        assert!(budget.can_proceed(GasCost(500)));
    }

    #[test]
    fn can_proceed_rejects_when_exhausted_hard_limit() {
        let mut budget = GasBudget::new(GasCost(100));
        budget.remaining = GasCost(50);
        assert!(!budget.can_proceed(GasCost(60)));
    }

    #[test]
    fn can_proceed_allows_when_soft_limit() {
        let mut budget = GasBudget::new(GasCost(100)).with_hard_limit(false);
        budget.remaining = GasCost(50);
        assert!(budget.can_proceed(GasCost(200)));
    }

    #[test]
    fn can_proceed_accounts_for_reserved() {
        let mut budget = GasBudget::new(GasCost(100));
        budget.remaining = GasCost(100);
        budget.reserved = GasCost(60);
        // available = 100 - 60 = 40
        assert!(!budget.can_proceed(GasCost(50)));
        assert!(budget.can_proceed(GasCost(40)));
    }

    // ── GasBudget consume ───────────────────────────────────────────────

    #[test]
    fn consume_deducts_from_remaining() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.consume(GasCost(300)).unwrap();
        assert_eq!(budget.remaining, GasCost(700));
    }

    #[test]
    fn consume_fails_when_hard_limit_exceeded() {
        let mut budget = GasBudget::new(GasCost(100));
        let err = budget.consume(GasCost(200)).unwrap_err();
        match err {
            GasError::BudgetExceeded {
                requested,
                remaining,
            } => {
                assert_eq!(requested, GasCost(200));
                assert_eq!(remaining, GasCost(100));
            }
        }
    }

    // ── GasBudget reserve + settle (hold-settle pattern) ─────────────────

    #[test]
    fn reserve_adds_to_reserved() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.reserve(GasCost(100)).unwrap();
        assert_eq!(budget.reserved, GasCost(100));
        assert_eq!(budget.remaining, GasCost(1000)); // Not deducted yet
    }

    #[test]
    fn reserve_fails_when_insufficient_available() {
        let mut budget = GasBudget::new(GasCost(100));
        budget.reserved = GasCost(80);
        assert!(budget.reserve(GasCost(30)).is_err());
    }

    #[test]
    fn settle_deducts_actual_from_remaining() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.reserve(GasCost(100)).unwrap();
        budget.settle(GasCost(100), GasCost(80)).unwrap();
        assert_eq!(budget.reserved, GasCost(0)); // Reservation cleared
        assert_eq!(budget.remaining, GasCost(920)); // 1000 - 80
    }

    #[test]
    fn settle_refunds_when_actual_less_than_reserved() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.reserve(GasCost(100)).unwrap();
        budget.settle(GasCost(100), GasCost(50)).unwrap();
        // remaining was never deducted by reserve; settle deducts actual
        assert_eq!(budget.remaining, GasCost(950));
    }

    #[test]
    fn settle_deducts_extra_when_actual_exceeds_reserved() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.reserve(GasCost(50)).unwrap();
        budget.settle(GasCost(50), GasCost(80)).unwrap();
        assert_eq!(budget.remaining, GasCost(920)); // 1000 - 80
    }

    // ── GasBudget replenish ─────────────────────────────────────────────

    #[test]
    fn replenish_adds_rate_capped_at_cap() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.remaining = GasCost(950);
        budget.replenish(); // rate = 100
        assert_eq!(budget.remaining, GasCost(1000)); // Capped at cap
    }

    #[test]
    fn replenish_no_overflow_beyond_cap() {
        let mut budget = GasBudget::new(GasCost(100));
        budget.remaining = GasCost(99);
        budget.replenish();
        assert_eq!(budget.remaining, GasCost(100));
    }

    #[test]
    fn replenish_by_specific_amount() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.remaining = GasCost(500);
        budget.replenish_by(GasCost(200));
        assert_eq!(budget.remaining, GasCost(700));
    }

    #[test]
    fn replenish_by_weighted_scales_by_priority() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.remaining = GasCost(500);
        let replenished = budget.replenish_by_weighted(GasCost(100), 0.5);
        // 100 * 0.5 = 50
        assert_eq!(budget.remaining, GasCost(550));
        assert_eq!(replenished, GasCost(50));
    }

    #[test]
    fn replenish_by_weighted_minimum_one_unit() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.remaining = GasCost(500);
        // Very low priority: 100 * 0.001 = 0.1 → rounds to 0 → min 1
        let replenished = budget.replenish_by_weighted(GasCost(100), 0.001);
        assert_eq!(budget.remaining, GasCost(501));
        assert_eq!(replenished, GasCost(1));
    }

    // ── GasBudget usage_ratio ───────────────────────────────────────────

    #[test]
    fn usage_ratio_full_budget() {
        let budget = GasBudget::new(GasCost(1000));
        assert_eq!(budget.usage_ratio(), 0.0);
    }

    #[test]
    fn usage_ratio_half_consumed() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.remaining = GasCost(500);
        assert_eq!(budget.usage_ratio(), 0.5);
    }

    #[test]
    fn usage_ratio_empty() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.remaining = GasCost(0);
        assert_eq!(budget.usage_ratio(), 1.0);
    }

    // ── GasBudget available ────────────────────────────────────────────

    #[test]
    fn available_equals_remaining_minus_reserved() {
        let mut budget = GasBudget::new(GasCost(1000));
        budget.reserved = GasCost(300);
        assert_eq!(budget.available(), GasCost(700));
    }

    #[test]
    fn available_saturating_sub_no_underflow() {
        let mut budget = GasBudget::new(GasCost(100));
        budget.remaining = GasCost(50);
        budget.reserved = GasCost(80); // More than remaining
        assert_eq!(budget.available(), GasCost(0)); // No underflow
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
