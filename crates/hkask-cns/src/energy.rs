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
pub struct EnergyCost(pub u64);

impl EnergyCost {
    /// Zero energy cost — used for free/internal operations.
    pub const ZERO: EnergyCost = EnergyCost(0);

    /// Create a energy cost from a raw `u64`.
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
    /// Useful for agents that should never be throttled. The budget still
    /// tracks usage for observability but never hard-rejects.
    pub fn unlimited() -> Self {
        Self::new(EnergyCost(u64::MAX)).with_hard_limit(false)
    }

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
    /// Returns `Ok(reserved)` if gas was reserved, `Err` if insufficient.
    /// Reserved gas is deducted from available but not from remaining until
    /// `settle()` is called.
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
    /// Since `reserve()` only tracks reserved gas without deducting from remaining,
    /// settlement simply removes the reservation and deducts the actual cost.
    /// If actual < reserved, the remaining budget was never reduced for the
    /// reservation, so the difference is implicitly refunded.
    ///
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
    /// For operations where the cost is known exactly at call time
    /// (no hold-settle needed).
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
    /// Called by the Cybernetics Loop on its regulation cycle.
    /// Never exceeds cap.
    pub fn replenish(&mut self) {
        if self.replenish_rate.0 > 0 {
            self.remaining = EnergyCost((self.remaining.0 + self.replenish_rate.0).min(self.cap.0));
        }
    }

    /// Replenish energy budget by a specific amount (used by CuratorDirective::ReplenishBudget).
    pub fn replenish_by(&mut self, amount: EnergyCost) {
        self.remaining = EnergyCost((self.remaining.0 + amount.0).min(self.cap.0));
    }

    /// Replenish energy budget by `amount * priority`, weighted by the given priority.
    ///
    /// The effective replenishment is `(amount * priority).round()`, never exceeding cap.
    /// If `amount * priority` rounds to 0, at least 1 unit is replenished (so
    /// low-priority directives still have effect).
    pub fn replenish_by_weighted(&mut self, amount: EnergyCost, priority: f64) -> EnergyCost {
        let scaled = (amount.0 as f64 * priority.clamp(0.0, 1.0)).round() as u64;
        let effective = scaled.max(1);
        let before = self.remaining.0;
        self.remaining = EnergyCost((self.remaining.0 + effective).min(self.cap.0));
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
/// for use by the `cns_energy` MCP tool and InferenceLoop gas sync.
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
