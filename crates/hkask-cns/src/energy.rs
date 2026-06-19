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

// ── Domain newtypes ──────────────────────────────────────────────────────

/// Gas cost of a single tool invocation.
///
/// Newtype wrapper around `u64` that prevents accidental confusion with
/// other unsigned quantities in the gas subsystem (cap, remaining, rate).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct EnergyCost(pub u64);

impl EnergyCost {
    /// Zero energy cost — used for free/internal operations.
    pub const ZERO: EnergyCost = EnergyCost(0);

    /// Create an energy cost from a raw `u64`.
    pub fn from_raw(value: u64) -> Self {
        EnergyCost(value)
    }

    /// Return the raw `u64` value.
    pub fn as_raw(self) -> u64 {
        self.0
    }
}

impl From<u64> for EnergyCost {
    fn from(value: u64) -> Self {
        EnergyCost(value)
    }
}

impl From<EnergyCost> for u64 {
    fn from(cost: EnergyCost) -> Self {
        cost.0
    }
}

impl Deref for EnergyCost {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EnergyCost {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for EnergyCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} gas", self.0)
    }
}

// ── Lazy Universe: EnergyDelta ───────────────────────────────────────────

/// A measurement of system energy change between two states.
///
/// A negative delta means the system moved toward lower energy
/// (lazy universe compliance — the system found a lower-action path).
/// A positive delta means the system moved away from minimal representation
/// and triggers algedonic alert after `ALERT_THRESHOLD` consecutive positives.
///
/// # CNS span: `cns.evolution.energy_delta`
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct EnergyDelta(pub f64);

impl EnergyDelta {
    /// Zero energy change — system at stationary point.
    pub const ZERO: EnergyDelta = EnergyDelta(0.0);

    /// Create an energy delta from a raw `f64`.
    pub fn from_raw(value: f64) -> Self {
        EnergyDelta(value)
    }

    /// Return the raw `f64` value.
    pub fn as_raw(self) -> f64 {
        self.0
    }

    /// Returns true if the system moved toward lower energy (lazy universe satisfied).
    /// Zero delta (stationary point) is also considered descending — the system
    /// has found its minimal-action configuration.
    pub fn is_descending(&self) -> bool {
        self.0 <= 0.0
    }

    /// Returns true if the system moved toward higher energy (anti-lazy — alert candidate).
    pub fn is_ascending(&self) -> bool {
        self.0 > 0.0
    }

    /// The algedonic threshold: how many consecutive positive deltas before alert.
    /// After ALERT_THRESHOLD consecutive ascending deltas, the CNS emits:
    /// "System moving away from minimal representation — anti-lazy drift detected."
    pub const ALERT_THRESHOLD: usize = 5;
}

impl fmt::Display for EnergyDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let direction = if self.is_descending() { "↓" } else { "↑" };
        write!(f, "{direction}{:.4}", self.0.abs())
    }
}

impl From<f64> for EnergyDelta {
    fn from(value: f64) -> Self {
        EnergyDelta(value)
    }
}

impl From<EnergyDelta> for f64 {
    fn from(delta: EnergyDelta) -> Self {
        delta.0
    }
}

/// Default energy budget alert threshold — alert when 80% of gas is consumed.
pub const DEFAULT_ENERGY_ALERT_THRESHOLD: f64 = 0.8;

/// Default priority for serde default.
const fn default_priority() -> f64 {
    1.0
}

/// Gas budget allocation.
///
/// Gas units are dimensionless — they represent computational cost on a
/// shared scale. Inference tools are more expensive than internal tools.
/// The `EnergyEstimator` trait maps each (server, tool) pair to an energy cost.
///
/// Gas replenishes periodically via `replenish()`, called by the
/// Cybernetics Loop on its regulation cycle.
///
/// # Invariants (enforced structurally by private fields)
/// - `remaining + reserved ≤ cap`
/// - `remaining ≥ 0`, `reserved ≥ 0`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyBudget {
    /// Maximum gas capacity.
    cap: EnergyCost,
    /// Currently available gas.
    remaining: EnergyCost,
    /// Gas units replenished per replenishment cycle.
    /// Set to 0 to disable automatic replenishment (one-shot budget).
    replenish_rate: EnergyCost,
    /// Alert threshold as a ratio (0.0–1.0). Alert when usage ≥ threshold.
    alert_threshold: f64,
    /// Whether to hard-reject when budget is exhausted.
    /// If false, operations proceed but emit depletion warnings.
    hard_limit: bool,
    /// Amount reserved by in-flight operations (hold-settle pattern).
    /// Gas that has been reserved but not yet settled.
    #[serde(default)]
    reserved: EnergyCost,
    /// Priority weight for replenishment scaling (0.0–1.0).
    /// Higher priority agents receive a larger share of replenishment.
    /// Defaults to 1.0 (full replenishment).
    #[serde(default = "default_priority")]
    priority: f64,
}

impl EnergyBudget {
    // ── Constructors ────────────────────────────────────────────────────

    /// Create a new energy budget with the given cap.
    ///
    /// Defaults: replenish_rate = cap / 10, alert_threshold = 0.8, hard_limit = true.
    ///
    /// # Panics
    /// Panics if `cap` is zero.
    pub fn new(cap: EnergyCost) -> Self {
        assert!(cap.0 > 0, "cap must be positive");
        let cap_raw = cap.0;
        Self {
            remaining: cap,
            replenish_rate: EnergyCost(cap_raw / 10),
            alert_threshold: DEFAULT_ENERGY_ALERT_THRESHOLD,
            hard_limit: true,
            reserved: EnergyCost::ZERO,
            priority: 1.0,
            cap,
        }
    }

    /// Create an energy budget with unlimited capacity (u64::MAX).
    ///
    /// Tracks usage for observability but never hard-rejects.
    pub fn unlimited() -> Self {
        Self::new(EnergyCost(u64::MAX)).with_hard_limit(false)
    }

    // ── Builder methods ─────────────────────────────────────────────────

    /// Set the replenishment rate (gas units per cycle).
    pub fn with_replenish_rate(mut self, rate: EnergyCost) -> Self {
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

    // ── Accessors ───────────────────────────────────────────────────────

    /// Maximum gas capacity.
    pub fn cap(&self) -> EnergyCost {
        self.cap
    }

    /// Currently available gas.
    pub fn remaining(&self) -> EnergyCost {
        self.remaining
    }

    /// Gas reserved by in-flight operations.
    pub fn reserved(&self) -> EnergyCost {
        self.reserved
    }

    /// Replenishment rate (gas units per cycle).
    pub fn replenish_rate(&self) -> EnergyCost {
        self.replenish_rate
    }

    /// Whether the budget hard-rejects on exhaustion.
    pub fn hard_limit(&self) -> bool {
        self.hard_limit
    }

    /// Alert threshold ratio (0.0–1.0).
    pub fn alert_threshold(&self) -> f64 {
        self.alert_threshold
    }

    /// Priority weight for replenishment scaling (0.0–1.0).
    pub fn priority(&self) -> f64 {
        self.priority
    }

    // ── Mutation for external budget management ─────────────────────────

    /// Reset the budget cap and remaining to a new value.
    ///
    /// Used by curator overrides to set a new budget ceiling.
    /// The invariant `remaining ≤ cap` is maintained by setting both fields.
    /// Reserved gas is cleared (in-flight operations should be re-reserved).
    pub fn reset_to(&mut self, new_cap: EnergyCost) {
        assert!(new_cap.0 > 0, "new cap must be positive");
        self.cap = new_cap;
        self.remaining = new_cap;
        self.reserved = EnergyCost::ZERO;
        tracing::warn!(
            target: "cns.gas",
            operation = "reset_to",
            new_cap = %new_cap.0,
            "CNS"
        );
    }

    // ── Gas operations ──────────────────────────────────────────────────

    /// Check whether an operation costing `gas` can proceed.
    ///
    /// Returns `true` if the gas fits within available (remaining - reserved) budget.
    pub fn can_proceed(&self, gas: EnergyCost) -> bool {
        let available = self.available();
        gas.0 <= available.0 || !self.hard_limit
    }

    /// Available gas = remaining - reserved.
    pub fn available(&self) -> EnergyCost {
        EnergyCost(self.remaining.0.saturating_sub(self.reserved.0))
    }

    /// Reserve gas for an in-flight operation (hold-settle pattern).
    ///
    /// Reserved gas is deducted from available but not from remaining until
    /// `settle()` is called.
    pub fn reserve(&mut self, gas: EnergyCost) -> Result<EnergyCost, EnergyError> {
        let available = self.available();
        if self.hard_limit && gas.0 > available.0 {
            tracing::warn!(
                target: "cns.gas",
                operation = "reserve",
                remaining = %self.remaining.0,
                reserved = %self.reserved.0,
                cap = %self.cap.0,
                requested = %gas.0,
                outcome = "budget_exceeded",
                "CNS"
            );
            return Err(EnergyError::BudgetExceeded {
                requested: gas,
                remaining: available,
            });
        }
        self.reserved = EnergyCost(self.reserved.0.saturating_add(gas.0));
        debug_assert!(
            self.reserved.0 <= self.remaining.0,
            "invariant: reserved ≤ remaining"
        );
        tracing::info!(
            target: "cns.gas",
            operation = "reserve",
            remaining = %self.remaining.0,
            reserved = %self.reserved.0,
            cap = %self.cap.0,
            requested = %gas.0,
            outcome = "ok",
            "CNS"
        );
        Ok(gas)
    }

    /// Settle a reserved operation: deduct actual cost from remaining.
    ///
    /// If actual < reserved, the difference is implicitly refunded
    /// (remaining was never reduced for the reservation).
    /// If actual > reserved (under-estimation), the extra is deducted from
    /// remaining as well.
    pub fn settle(
        &mut self,
        reserved_gas: EnergyCost,
        actual_gas: EnergyCost,
    ) -> Result<EnergyCost, EnergyError> {
        // Remove the reservation
        self.reserved = EnergyCost(self.reserved.0.saturating_sub(reserved_gas.0));

        // Deduct actual cost from remaining
        if self.hard_limit && actual_gas.0 > self.remaining.0 {
            tracing::warn!(
                target: "cns.gas",
                operation = "settle",
                remaining = %self.remaining.0,
                reserved = %self.reserved.0,
                cap = %self.cap.0,
                actual = %actual_gas.0,
                outcome = "budget_exceeded",
                "CNS"
            );
            return Err(EnergyError::BudgetExceeded {
                requested: actual_gas,
                remaining: self.remaining,
            });
        }
        self.remaining = EnergyCost(self.remaining.0.saturating_sub(actual_gas.0));
        debug_assert!(
            self.remaining.0 + self.reserved.0 <= self.cap.0,
            "invariant: remaining + reserved ≤ cap"
        );
        tracing::info!(
            target: "cns.gas",
            operation = "settle",
            remaining = %self.remaining.0,
            reserved = %self.reserved.0,
            cap = %self.cap.0,
            actual = %actual_gas.0,
            outcome = "ok",
            "CNS"
        );
        Ok(actual_gas)
    }

    /// Consume gas immediately (non-reserved path).
    ///
    /// For operations where the cost is known exactly at call time
    /// (no hold-settle needed).
    pub fn consume(&mut self, gas: EnergyCost) -> Result<EnergyCost, EnergyError> {
        if self.hard_limit && gas.0 > self.remaining.0 {
            tracing::warn!(
                target: "cns.gas",
                operation = "consume",
                remaining = %self.remaining.0,
                reserved = %self.reserved.0,
                cap = %self.cap.0,
                requested = %gas.0,
                outcome = "budget_exceeded",
                "CNS"
            );
            return Err(EnergyError::BudgetExceeded {
                requested: gas,
                remaining: self.remaining,
            });
        }
        self.remaining = EnergyCost(self.remaining.0.saturating_sub(gas.0));
        tracing::info!(
            target: "cns.gas",
            operation = "consume",
            remaining = %self.remaining.0,
            reserved = %self.reserved.0,
            cap = %self.cap.0,
            consumed = %gas.0,
            outcome = "ok",
            "CNS"
        );
        Ok(gas)
    }

    // ── Replenishment ───────────────────────────────────────────────────

    /// Replenish energy budget by the configured replenish_rate.
    ///
    /// Called by the Cybernetics Loop on its regulation cycle.
    /// Never exceeds cap.
    pub fn replenish(&mut self) {
        if self.replenish_rate.0 > 0 {
            self.remaining = EnergyCost(
                self.remaining
                    .0
                    .saturating_add(self.replenish_rate.0)
                    .min(self.cap.0),
            );
        }
        debug_assert!(
            self.remaining.0 <= self.cap.0,
            "postcondition: remaining never exceeds cap"
        );
    }

    /// Replenish energy budget by a specific amount (used by CuratorDirective::ReplenishBudget).
    pub fn replenish_by(&mut self, amount: EnergyCost) {
        self.remaining = EnergyCost(self.remaining.0.saturating_add(amount.0).min(self.cap.0));
        debug_assert!(
            self.remaining.0 <= self.cap.0,
            "postcondition: remaining never exceeds cap"
        );
    }

    /// Replenish energy budget by `amount * priority`, weighted by the given priority.
    ///
    /// If `amount * priority` rounds to 0, at least 1 unit is replenished
    /// (so low-priority directives still have effect).
    pub fn replenish_by_weighted(&mut self, amount: EnergyCost, priority: f64) -> EnergyCost {
        let scaled = (amount.0 as f64 * priority.clamp(0.0, 1.0)).round() as u64;
        let effective = scaled.max(1);
        let before = self.remaining.0;
        self.remaining = EnergyCost(self.remaining.0.saturating_add(effective).min(self.cap.0));
        let delta = EnergyCost(self.remaining.0 - before);
        debug_assert!(
            self.remaining.0 <= self.cap.0,
            "postcondition: remaining never exceeds cap"
        );
        delta
    }

    /// Usage ratio: 0.0 = full budget, 1.0 = empty.
    pub(crate) fn usage_ratio(&self) -> f64 {
        1.0 - (self.remaining.0 as f64 / self.cap.0.max(1) as f64)
    }
}

// ── AgentEnergyStatus (read-only snapshot) ───────────────────────────────

/// Read-only snapshot of an agent's energy budget status.
///
/// Returned by `CyberneticsLoop::agent_gas_status()` and `CnsRuntime::agent_gas_status()`
/// for use by the CNS service and InferenceLoop gas sync.
pub struct AgentEnergyStatus {
    /// Maximum gas capacity.
    pub cap: EnergyCost,
    /// Currently available gas (total remaining, including reserved).
    pub remaining: EnergyCost,
    /// Gas reserved by in-flight operations.
    pub reserved: EnergyCost,
    /// Available gas = remaining - reserved.
    pub available: EnergyCost,
    /// Usage ratio: 0.0 = full budget, 1.0 = empty.
    pub usage_ratio: f64,
    /// Whether the agent will be hard-rejected on exhaustion.
    pub hard_limit: bool,
    /// The ratio at which alerts fire (0.0–1.0).
    pub alert_threshold: f64,
}

impl From<&EnergyBudget> for AgentEnergyStatus {
    fn from(budget: &EnergyBudget) -> Self {
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

// ── EnergyError ──────────────────────────────────────────────────────────

/// Gas budget error — returned when gas operations fail.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EnergyError {
    #[error("Gas budget exceeded: {requested}, remaining {remaining}")]
    BudgetExceeded {
        requested: EnergyCost,
        remaining: EnergyCost,
    },
}

// ── Property-based tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy: generate a random EnergyCost in a reasonable range.
    fn arbitrary_cost() -> BoxedStrategy<EnergyCost> {
        (1u64..1000u64).prop_map(EnergyCost).boxed()
    }

    /// Strategy: generate a random EnergyBudget with hard_limit=true.
    fn arbitrary_budget() -> BoxedStrategy<EnergyBudget> {
        (1u64..10000u64)
            .prop_map(|cap| EnergyBudget::new(EnergyCost(cap)))
            .boxed()
    }

    /// After any sequence of reserve/settle/consume operations,
    /// remaining + reserved never exceeds cap.
    proptest! {
        #[test]
        fn budget_never_exceeds_cap(
            mut budget in arbitrary_budget(),
            operations in prop::collection::vec((arbitrary_cost(), arbitrary_cost()), 0..20),
        ) {
            let cap = budget.cap();
            for (reserve_gas, actual_gas) in &operations {
                if let Ok(reserved) = budget.reserve(*reserve_gas) {
                    let _ = budget.settle(reserved, *actual_gas);
                }
            }
            let total = EnergyCost(budget.remaining().0 + budget.reserved().0);
            prop_assert!(total <= cap,
                "remaining {} + reserved {} = {} > cap {}",
                budget.remaining().0, budget.reserved().0, total.0, cap.0);
        }
    }

    /// available() = remaining - reserved, must never be negative.
    proptest! {
        #[test]
        fn available_never_negative(
            mut budget in arbitrary_budget(),
            operations in prop::collection::vec(arbitrary_cost(), 0..20),
        ) {
            for cost in &operations {
                let _ = budget.reserve(*cost);
                let _ = budget.consume(*cost);
            }
            let available = budget.available();
            prop_assert!(available.0 <= budget.remaining().0,
                "available {} > remaining {}", available.0, budget.remaining().0);
        }
    }

    /// After replenishment, remaining never exceeds cap.
    proptest! {
        #[test]
        fn replenish_never_exceeds_cap(
            mut budget in arbitrary_budget(),
            cycles in 0u32..100u32,
        ) {
            // Drain budget by consuming all remaining gas
            let _ = budget.consume(budget.remaining());
            for _ in 0..cycles {
                budget.replenish();
            }
            prop_assert!(budget.remaining() <= budget.cap(),
                "remaining {} > cap {} after {} cycles",
                budget.remaining().0, budget.cap().0, cycles);
        }
    }
}
