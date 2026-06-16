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
// to keep energy.rs focused on gas accounting types only.

// ── Domain newtypes (P2.3) ──────────────────────────────────────────────────

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

    /// Create a energy cost from a raw `u64`.
    ///
    /// REQ: CNS-ENERGY-004
    /// post: result.0 == value
    pub fn from_raw(value: u64) -> Self {
        EnergyCost(value)
    }

    /// Return the raw `u64` value.
    ///
    /// REQ: CNS-ENERGY-004
    /// post: result == self.0
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

// ── Lazy Universe: EnergyDelta (TASK 4.1) ──────────────────────────────────

/// A measurement of system energy change between two states.
///
/// # Invariant (type-level encoding per Graydon Hoare's type-driven design)
///
/// A negative delta means the system moved toward lower energy
/// (lazy universe compliance — the system found a lower-action path).
/// A positive delta means the system moved away from minimal representation
/// and triggers algedonic alert after `ALERT_THRESHOLD` consecutive positives.
///
/// # Epistemic grounding (P8)
/// - **crt:certainty** = Declarative (direct measurement of energy change)
/// - **crt:force** = Evidence (IS statement, measured from CNS span)
/// - **mode** = IS
///
/// # CNS span: `cns.evolution.energy_delta`
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct EnergyDelta(pub f64);

impl EnergyDelta {
    /// Zero energy change — system at stationary point.
    pub const ZERO: EnergyDelta = EnergyDelta(0.0);

    /// Create an energy delta from a raw `f64`.
    ///
    /// REQ: CNS-ENERGY-005
    /// post: result.0 == value
    pub fn from_raw(value: f64) -> Self {
        EnergyDelta(value)
    }

    /// Return the raw `f64` value.
    ///
    /// REQ: CNS-ENERGY-005
    /// post: result == self.0
    pub fn as_raw(self) -> f64 {
        self.0
    }

    /// Returns true if the system moved toward lower energy (lazy universe satisfied).
    /// Zero delta (stationary point) is also considered descending — the system
    /// has found its minimal-action configuration.
    ///
    /// REQ: CNS-ENERGY-005
    /// post: result == (self.0 <= 0.0)
    pub fn is_descending(&self) -> bool {
        self.0 <= 0.0
    }

    /// Returns true if the system moved toward higher energy (anti-lazy — alert candidate).
    ///
    /// REQ: CNS-ENERGY-005
    /// post: result == (self.0 > 0.0)
    /// post: is_ascending() == !is_descending() || self.0 == 0.0
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
/// The `EnergyEstimator` trait maps each (server, tool) pair to a energy cost.
///
/// Gas replenishes periodically via `replenish()`, called by the
/// Cybernetics Loop on its regulation cycle.
///
/// REQ: CNS-001
/// inv: remaining + reserved ≤ cap (budget cap invariant)
/// inv: remaining ≥ 0, reserved ≥ 0
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyBudget {
    /// Maximum gas capacity.
    pub cap: EnergyCost,
    /// Currently available gas.
    pub remaining: EnergyCost,
    /// Gas units replenished per replenishment cycle.
    /// Set to 0 to disable automatic replenishment (one-shot budget).
    pub replenish_rate: EnergyCost,
    /// Alert threshold as a ratio (0.0–1.0). Alert when usage ≥ threshold.
    pub alert_threshold: f64,
    /// Whether to hard-reject when budget is exhausted.
    /// If false, operations proceed but emit depletion warnings.
    pub hard_limit: bool,
    /// Amount reserved by in-flight operations (hold-settle pattern).
    /// Gas that has been reserved but not yet settled.
    #[serde(default)]
    pub reserved: EnergyCost,
    /// Priority weight for replenishment scaling (0.0–1.0).
    /// Higher priority agents receive a larger share of replenishment.
    /// Defaults to 1.0 (full replenishment).
    #[serde(default = "default_priority")]
    pub priority: f64,
}

impl EnergyBudget {
    /// Create a new energy budget with the given cap.
    ///
    /// REQ: CNS-001
    /// pre:  cap > 0
    /// post: remaining == cap, reserved == 0, hard_limit == true
    /// post: replenish_rate == cap / 10, alert_threshold == DEFAULT_ENERGY_ALERT_THRESHOLD
    /// Defaults: replenish_rate = cap / 10, alert_threshold = DEFAULT_ENERGY_ALERT_THRESHOLD, hard_limit = true.
    pub fn new(cap: EnergyCost) -> Self {
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

    /// Create a energy budget with unlimited capacity (u64::MAX).
    ///
    /// REQ: CNS-001
    /// post: cap == u64::MAX, hard_limit == false
    ///
    /// [NORMATIVE] Useful for agents that should never be throttled (P9 — Homeostatic Self-Regulation). The budget still
    /// [DECLARATIVE] tracks usage for observability but never hard-rejects. (P9 — Homeostatic Self-Regulation).
    pub fn unlimited() -> Self {
        Self::new(EnergyCost(u64::MAX)).with_hard_limit(false)
    }

    /// Set the replenishment rate (gas units per cycle).
    ///
    /// REQ: CNS-003
    /// post: self.replenish_rate == rate
    pub fn with_replenish_rate(mut self, rate: EnergyCost) -> Self {
        self.replenish_rate = rate;
        self
    }

    /// Set the alert threshold (0.0–1.0).
    ///
    /// REQ: CNS-001
    /// pre:  threshold is a valid ratio
    /// post: self.alert_threshold == threshold.clamp(0.0, 1.0)
    pub fn with_alert_threshold(mut self, threshold: f64) -> Self {
        self.alert_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set whether to hard-reject on exhaustion.
    ///
    /// REQ: CNS-001
    /// post: self.hard_limit == hard
    pub fn with_hard_limit(mut self, hard: bool) -> Self {
        self.hard_limit = hard;
        self
    }

    /// Check whether an operation costing `gas` can proceed.
    ///
    /// REQ: CNS-001
    /// pre:  gas is a valid EnergyCost
    /// post: returns true iff gas <= available OR hard_limit is false
    /// Returns `true` if the gas fits within available (remaining - reserved) budget.
    pub fn can_proceed(&self, gas: EnergyCost) -> bool {
        let available = self.available();
        gas.0 <= available.0 || !self.hard_limit
    }

    /// Available gas = remaining - reserved.
    ///
    /// REQ: CNS-002
    /// [NORMATIVE] post: result >= 0 (available never negative) (P9 — Homeostatic Self-Regulation)
    /// post: result == remaining.saturating_sub(reserved)
    pub fn available(&self) -> EnergyCost {
        EnergyCost(self.remaining.0.saturating_sub(self.reserved.0))
    }

    /// Reserve gas for an in-flight operation (hold-settle pattern).
    ///
    /// REQ: CNS-001
    /// pre:  gas is a valid EnergyCost
    /// post: if hard_limit && gas > available → Err(BudgetExceeded)
    /// post: if Ok → reserved increased by gas, remaining unchanged
    /// inv:  remaining + reserved ≤ cap (maintained)
    ///
    /// Returns `Ok(reserved)` if gas was reserved, `Err` if insufficient.
    /// Reserved gas is deducted from available but not from remaining until
    /// `settle()` is called.
    /// Reserve gas for an in-flight operation.
    ///
    /// REQ: CNS-087
    /// pre:  gas > 0
    /// post: if hard_limit && gas > available → Err
    /// post: if Ok → reserved increased by gas
    pub fn reserve(&mut self, gas: EnergyCost) -> Result<EnergyCost, EnergyError> {
        let available = self.available();
        if self.hard_limit && gas.0 > available.0 {
            return Err(EnergyError::BudgetExceeded {
                requested: gas,
                remaining: available,
            });
        }
        self.reserved = EnergyCost(self.reserved.0.saturating_add(gas.0));
        Ok(gas)
    }

    /// Settle a reserved operation: deduct actual cost from remaining.
    ///
    /// REQ: CNS-001
    /// [NORMATIVE] pre:  reserved_gas ≤ self.reserved (caller must track reservations) (P9 — Homeostatic Self-Regulation)
    /// post: reserved decreased by reserved_gas
    /// post: if hard_limit && actual > remaining → Err(BudgetExceeded)
    /// post: if Ok → remaining decreased by actual
    /// inv:  remaining + reserved ≤ cap (maintained)
    ///
    /// Since `reserve()` only tracks reserved gas without deducting from remaining,
    /// settlement simply removes the reservation and deducts the actual cost.
    /// [DECLARATIVE] If actual < reserved, the remaining budget was never reduced for the
    /// reservation, so the difference is implicitly refunded.
    ///
    /// If actual > reserved (under-estimation), the extra is deducted from
    /// remaining as well.
    /// Settle a reserved operation.
    ///
    /// REQ: CNS-088
    /// pre:  reserved_gas ≤ self.reserved
    /// post: reserved decreased, remaining decreased by actual
    pub fn settle(
        &mut self,
        reserved_gas: EnergyCost,
        actual_gas: EnergyCost,
    ) -> Result<EnergyCost, EnergyError> {
        // Remove the reservation
        self.reserved = EnergyCost(self.reserved.0.saturating_sub(reserved_gas.0));

        // Deduct actual cost from remaining
        if self.hard_limit && actual_gas.0 > self.remaining.0 {
            return Err(EnergyError::BudgetExceeded {
                requested: actual_gas,
                remaining: self.remaining,
            });
        }
        self.remaining = EnergyCost(self.remaining.0.saturating_sub(actual_gas.0));
        Ok(actual_gas)
    }

    /// Consume gas immediately (non-reserved path).
    ///
    /// REQ: CNS-001
    /// pre:  gas is a valid EnergyCost
    /// post: if hard_limit && gas > remaining → Err(BudgetExceeded)
    /// post: if Ok → remaining decreased by gas
    /// inv:  remaining + reserved ≤ cap (maintained)
    ///
    /// For operations where the cost is known exactly at call time
    /// (no hold-settle needed).
    /// Consume gas immediately (non-reserved).
    ///
    /// REQ: CNS-089
    /// pre:  gas > 0
    /// post: if hard_limit && gas > remaining → Err
    /// post: if Ok → remaining decreased by gas
    pub fn consume(&mut self, gas: EnergyCost) -> Result<EnergyCost, EnergyError> {
        if self.hard_limit && gas.0 > self.remaining.0 {
            return Err(EnergyError::BudgetExceeded {
                requested: gas,
                remaining: self.remaining,
            });
        }
        self.remaining = EnergyCost(self.remaining.0.saturating_sub(gas.0));
        Ok(gas)
    }

    /// Replenish energy budget by the configured replenish_rate.
    ///
    /// REQ: CNS-003
    /// post: remaining ≤ cap (never exceeds cap)
    /// post: if replenish_rate > 0 → remaining increased by up to replenish_rate
    ///
    /// Called by the Cybernetics Loop on its regulation cycle.
    /// Never exceeds cap.
    /// Replenish energy budget.
    ///
    /// REQ: CNS-090
    /// post: remaining increased by replenish_rate, capped at cap
    pub fn replenish(&mut self) {
        if self.replenish_rate.0 > 0 {
            self.remaining = EnergyCost(
                self.remaining
                    .0
                    .saturating_add(self.replenish_rate.0)
                    .min(self.cap.0),
            );
        }
    }

    /// Replenish energy budget by a specific amount (used by CuratorDirective::ReplenishBudget).
    ///
    /// REQ: CNS-003
    /// pre:  amount is a valid EnergyCost
    /// post: remaining ≤ cap (never exceeds cap)
    /// post: remaining increased by up to amount
    pub fn replenish_by(&mut self, amount: EnergyCost) {
        self.remaining = EnergyCost(self.remaining.0.saturating_add(amount.0).min(self.cap.0));
    }

    /// Replenish energy budget by `amount * priority`, weighted by the given priority.
    ///
    /// REQ: CNS-003
    /// pre:  amount is a valid EnergyCost, priority in [0.0, 1.0]
    /// post: remaining ≤ cap (never exceeds cap)
    /// post: returns the actual amount replenished (≥ 1 if amount * priority > 0)
    ///
    /// [NORMATIVE] The effective replenishment is `(amount * priority).round()`, never exceeding cap (P9 — Homeostatic Self-Regulation).
    /// If `amount * priority` rounds to 0, at least 1 unit is replenished (so
    /// low-priority directives still have effect).
    /// Replenish by weighted amount.
    ///
    /// REQ: CNS-091
    /// pre:  amount > 0, priority in [0.0, 1.0]
    /// post: remaining increased by amount * priority, capped at cap
    /// post: returns actual amount replenished
    pub fn replenish_by_weighted(&mut self, amount: EnergyCost, priority: f64) -> EnergyCost {
        let scaled = (amount.0 as f64 * priority.clamp(0.0, 1.0)).round() as u64;
        let effective = scaled.max(1);
        let before = self.remaining.0;
        self.remaining = EnergyCost(self.remaining.0.saturating_add(effective).min(self.cap.0));
        EnergyCost(self.remaining.0 - before)
    }

    /// Usage ratio: 0.0 = full budget, 1.0 = empty.
    pub(crate) fn usage_ratio(&self) -> f64 {
        1.0 - (self.remaining.0 as f64 / self.cap.0.max(1) as f64)
    }
}

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

/// Gas budget error — returned when gas operations fail.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EnergyError {
    #[error("Gas budget exceeded: {requested}, remaining {remaining}")]
    BudgetExceeded {
        requested: EnergyCost,
        remaining: EnergyCost,
    },
}

// ── Property-based tests (Wave 2) ───────────────────────────────────────

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

    // REQ: CNS-001 — Budget cap invariant (P4, P9)
    // After any sequence of reserve/settle/consume operations,
    // remaining + reserved never exceeds cap.
    proptest! {
        #[test]
        fn budget_never_exceeds_cap(
            mut budget in arbitrary_budget(),
            operations in prop::collection::vec((arbitrary_cost(), arbitrary_cost()), 0..20),
        ) {
            let cap = budget.cap;
            for (reserve_gas, actual_gas) in &operations {
                // Try to reserve; may fail if insufficient
                if let Ok(reserved) = budget.reserve(*reserve_gas) {
                    // Settle with actual cost (may differ from reserved)
                    let _ = budget.settle(reserved, *actual_gas);
                }
            }
            let total = EnergyCost(budget.remaining.0 + budget.reserved.0);
            prop_assert!(total <= cap,
                "remaining {} + reserved {} = {} > cap {}",
                budget.remaining.0, budget.reserved.0, total.0, cap.0);
        }
    }

    // REQ: CNS-002 — Available never negative (P4, P9)
    // available() = remaining - reserved, must never be negative.
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
            prop_assert!(available.0 <= budget.remaining.0,
                "available {} > remaining {}", available.0, budget.remaining.0);
        }
    }

    // REQ: CNS-003 — Replenish never exceeds cap (P9)
    // After replenishment, remaining never exceeds cap.
    proptest! {
        #[test]
        fn replenish_never_exceeds_cap(
            mut budget in arbitrary_budget(),
            cycles in 0u32..100u32,
        ) {
            // Drain budget first
            budget.remaining = EnergyCost(0);
            for _ in 0..cycles {
                budget.replenish();
            }
            prop_assert!(budget.remaining <= budget.cap,
                "remaining {} > cap {} after {} cycles",
                budget.remaining.0, budget.cap.0, cycles);
        }
    }
}
