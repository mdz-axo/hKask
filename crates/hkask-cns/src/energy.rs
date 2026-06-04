//! Gas budget enforcement — preventing infinite loops (analogous to Ethereum gas)
//!
//! Every operation costs gas. When the budget is exhausted, the operation
//! is rejected. Gas replenishes periodically, managed by the Cybernetics Loop.
//!
//! The gas model is deliberately simple: each MCP tool invocation has a
//! configured cost, and budgets refill at a steady rate. This prevents
//! runaway agents while keeping the implementation minimal.

use serde::{Deserialize, Serialize};

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
    pub cap: u64,
    /// Currently available gas.
    pub remaining: u64,
    /// Gas units replenished per replenishment cycle.
    /// Set to 0 to disable automatic replenishment (one-shot budget).
    pub replenish_rate: u64,
    /// Alert threshold as a ratio (0.0–1.0). Alert when usage ≥ threshold.
    pub alert_threshold: f64,
    /// Whether to hard-reject when budget is exhausted.
    /// If false, operations proceed but emit depletion warnings.
    pub hard_limit: bool,
    /// Amount reserved by in-flight operations (hold-settle pattern).
    /// Gas that has been reserved but not yet settled.
    #[serde(default)]
    pub reserved: u64,
}

impl GasBudget {
    /// Create a new gas budget with the given cap.
    ///
    /// Defaults: replenish_rate = cap / 10, alert_threshold = 0.8, hard_limit = true.
    pub fn new(cap: u64) -> Self {
        Self {
            remaining: cap,
            replenish_rate: cap / 10,
            alert_threshold: 0.8,
            hard_limit: true,
            reserved: 0,
            cap,
        }
    }

    /// Set the replenishment rate (gas units per cycle).
    pub fn with_replenish_rate(mut self, rate: u64) -> Self {
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
    pub fn can_proceed(&self, gas: u64) -> bool {
        let available = self.available();
        gas <= available || !self.hard_limit
    }

    /// Available gas = remaining - reserved.
    pub fn available(&self) -> u64 {
        self.remaining.saturating_sub(self.reserved)
    }

    /// Reserve gas for an in-flight operation (hold-settle pattern).
    ///
    /// Returns `Ok(reserved)` if gas was reserved, `Err` if insufficient.
    /// Reserved gas is deducted from available but not from remaining until
    /// `settle()` is called.
    pub fn reserve(&mut self, gas: u64) -> Result<u64, GasError> {
        let available = self.available();
        if self.hard_limit && gas > available {
            return Err(GasError::BudgetExceeded {
                requested: gas,
                remaining: available,
            });
        }
        self.reserved = self.reserved.saturating_add(gas);
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
    pub fn settle(&mut self, reserved_gas: u64, actual_gas: u64) -> Result<u64, GasError> {
        // Remove the reservation
        self.reserved = self.reserved.saturating_sub(reserved_gas);

        // Deduct actual cost from remaining
        if self.hard_limit && actual_gas > self.remaining {
            return Err(GasError::BudgetExceeded {
                requested: actual_gas,
                remaining: self.remaining,
            });
        }
        self.remaining = self.remaining.saturating_sub(actual_gas);
        Ok(actual_gas)
    }

    /// Consume gas immediately (non-reserved path).
    ///
    /// For operations where the cost is known exactly at call time
    /// (no hold-settle needed).
    pub fn consume(&mut self, gas: u64) -> Result<u64, GasError> {
        if self.hard_limit && gas > self.remaining {
            return Err(GasError::BudgetExceeded {
                requested: gas,
                remaining: self.remaining,
            });
        }
        self.remaining = self.remaining.saturating_sub(gas);
        Ok(gas)
    }

    /// Replenish gas budget by the configured replenish_rate.
    ///
    /// Called by the Cybernetics Loop on its regulation cycle.
    /// Never exceeds cap.
    pub fn replenish(&mut self) {
        if self.replenish_rate > 0 {
            self.remaining = (self.remaining + self.replenish_rate).min(self.cap);
        }
    }

    /// Replenish gas budget by a specific amount (used by CuratorDirective::ReplenishBudget).
    pub fn replenish_by(&mut self, amount: u64) {
        self.remaining = (self.remaining + amount).min(self.cap);
    }

    /// Whether the usage ratio has crossed the alert threshold.
    pub fn should_alert(&self) -> bool {
        self.usage_ratio() >= self.alert_threshold
    }

    /// Usage ratio: 0.0 = full budget, 1.0 = empty.
    pub fn usage_ratio(&self) -> f64 {
        1.0 - (self.remaining as f64 / self.cap.max(1) as f64)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum GasError {
    #[error("Gas budget exceeded: requested {requested}, remaining {remaining}")]
    BudgetExceeded { requested: u64, remaining: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_budget_new_has_defaults() {
        let budget = GasBudget::new(1000);
        assert_eq!(budget.cap, 1000);
        assert_eq!(budget.remaining, 1000);
        assert_eq!(budget.replenish_rate, 100); // cap / 10
        assert_eq!(budget.reserved, 0);
        assert!(budget.hard_limit);
    }

    #[test]
    fn gas_budget_consume_basic() {
        let mut budget = GasBudget::new(1000);
        assert_eq!(budget.consume(100).unwrap(), 100);
        assert_eq!(budget.remaining, 900);
    }

    #[test]
    fn gas_budget_consume_exhausted() {
        let mut budget = GasBudget::new(100).with_hard_limit(true);
        let result = budget.consume(200);
        assert!(result.is_err());
    }

    #[test]
    fn gas_budget_reserve_and_settle() {
        let mut budget = GasBudget::new(1000);
        // Reserve 100 gas
        assert_eq!(budget.reserve(100).unwrap(), 100);
        assert_eq!(budget.reserved, 100);
        assert_eq!(budget.remaining, 1000); // Not consumed yet
        assert_eq!(budget.available(), 900); // Available = remaining - reserved

        // Settle with actual cost of 80 — refund 20
        assert_eq!(budget.settle(100, 80).unwrap(), 80);
        assert_eq!(budget.reserved, 0);
        assert_eq!(budget.remaining, 920); // 1000 - 80 = 920
    }

    #[test]
    fn gas_budget_reserve_settle_over_cost() {
        let mut budget = GasBudget::new(1000);
        assert_eq!(budget.reserve(100).unwrap(), 100);
        // Actual cost exceeds reservation
        assert_eq!(budget.settle(100, 150).unwrap(), 150);
        assert_eq!(budget.remaining, 850);
    }

    #[test]
    fn gas_budget_reserve_insufficient_available() {
        let mut budget = GasBudget::new(200).with_hard_limit(true);
        assert_eq!(budget.reserve(100).unwrap(), 100);
        // Only 100 available (200 - 100 reserved)
        let result = budget.reserve(150);
        assert!(result.is_err());
    }

    #[test]
    fn gas_budget_replenish() {
        let mut budget = GasBudget::new(1000).with_replenish_rate(100);
        budget.consume(500).unwrap();
        assert_eq!(budget.remaining, 500);
        budget.replenish();
        assert_eq!(budget.remaining, 600);
    }

    #[test]
    fn gas_budget_replenish_capped() {
        let mut budget = GasBudget::new(1000).with_replenish_rate(500);
        // No consumption — replenish should cap at cap
        budget.replenish();
        assert_eq!(budget.remaining, 1000);
    }

    #[test]
    fn gas_budget_replenish_by_directive() {
        let mut budget = GasBudget::new(1000);
        budget.consume(800).unwrap();
        assert_eq!(budget.remaining, 200);
        budget.replenish_by(500);
        assert_eq!(budget.remaining, 700); // min(200 + 500, 1000)
    }

    #[test]
    fn gas_budget_soft_limit() {
        let mut budget = GasBudget::new(100).with_hard_limit(false);
        // Soft limit: operations proceed even when budget is exhausted
        let result = budget.consume(200);
        assert!(result.is_ok()); // Soft limit allows
        assert_eq!(budget.remaining, 0); // Saturating sub
    }

    #[test]
    fn gas_budget_should_alert() {
        let mut budget = GasBudget::new(1000).with_alert_threshold(0.8);
        assert!(!budget.should_alert()); // 0% usage
        budget.consume(800).unwrap();
        assert!(budget.should_alert()); // 80% usage
    }

    #[test]
    fn gas_budget_usage_ratio() {
        let budget = GasBudget::new(1000);
        assert!((budget.usage_ratio() - 0.0).abs() < f64::EPSILON);
    }
}
