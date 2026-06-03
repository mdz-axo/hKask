//! Energy budget enforcement — Loop 1 guard
//!
//! Every operation costs energy. When the budget is exhausted, the operation
//! is rejected. This is the enforcement gate that closes Loop 1.

use serde::{Deserialize, Serialize};

/// Energy budget allocation
#[derive(Serialize, Deserialize)]
pub struct EnergyBudget {
    pub cap: u64,
    pub remaining: u64,
    pub cost_per_token: f64,
    pub alert_threshold: f64,
    pub hard_limit: bool,
}

impl EnergyBudget {
    pub fn new(cap: u64) -> Self {
        Self {
            cap,
            remaining: cap,
            cost_per_token: 0.25,
            alert_threshold: 0.8,
            hard_limit: true,
        }
    }

    pub fn with_cost_per_token(mut self, cost: f64) -> Self {
        self.cost_per_token = cost;
        self
    }

    pub fn with_alert_threshold(mut self, threshold: f64) -> Self {
        self.alert_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_hard_limit(mut self, hard: bool) -> Self {
        self.hard_limit = hard;
        self
    }

    pub fn calculate_cost(&self, tokens: u64) -> u64 {
        ((tokens as f64) * self.cost_per_token) as u64
    }

    pub fn calculate_tokens(&self, energy: u64) -> u64 {
        if self.cost_per_token > 0.0 {
            (energy as f64 / self.cost_per_token) as u64
        } else {
            0
        }
    }

    pub(crate) fn allocate(&mut self, tokens: u64) -> Result<u64, EnergyError> {
        let cost = self.calculate_cost(tokens);
        if cost > self.remaining && self.hard_limit {
            return Err(EnergyError::BudgetExceeded {
                requested: cost,
                remaining: self.remaining,
            });
        }
        self.remaining = self.remaining.saturating_sub(cost);
        Ok(cost)
    }

    pub(crate) fn try_consume(&mut self, estimated_tokens: u64) -> Result<u64, EnergyError> {
        let cost = self.calculate_cost(estimated_tokens);
        if self.hard_limit && cost > self.remaining {
            return Err(EnergyError::BudgetExceeded {
                requested: cost,
                remaining: self.remaining,
            });
        }
        self.remaining = self.remaining.saturating_sub(cost);
        Ok(cost)
    }

    /// Check whether an operation can proceed without consuming energy.
    ///
    /// This is the replacement for rate-limit checks: instead of asking
    /// "am I within my rate window?", ask "do I have enough energy budget?".
    /// Returns `true` if the estimated cost fits within the remaining budget.
    pub fn can_proceed(&self, estimated_tokens: u64) -> bool {
        let cost = self.calculate_cost(estimated_tokens);
        cost <= self.remaining || !self.hard_limit
    }

    /// Acquire budget for an operation, consuming energy if available.
    ///
    /// Returns `Ok(cost)` if the budget was acquired, `Err` if insufficient.
    /// This is the atomic check-and-consume: it both checks AND deducts.
    pub(crate) fn acquire_budget(&mut self, estimated_tokens: u64) -> Result<u64, EnergyError> {
        self.try_consume(estimated_tokens)
    }

    /// Replenish energy budget by a given amount.
    ///
    /// Energy budgets replenish over time (analogous to rate limit window resets),
    /// but the replenishment is continuous rather than discretized into windows.
    pub fn replenish(&mut self, amount: u64) {
        self.remaining = (self.remaining + amount).min(self.cap);
    }

    pub fn should_alert(&self) -> bool {
        self.usage_ratio() >= self.alert_threshold
    }

    pub fn usage_ratio(&self) -> f64 {
        1.0 - (self.remaining as f64 / self.cap.max(1) as f64)
    }
}

impl std::fmt::Debug for EnergyBudget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnergyBudget")
            .field("cap", &self.cap)
            .field("remaining", &self.remaining)
            .field("hard_limit", &self.hard_limit)
            .finish()
    }
}

impl Clone for EnergyBudget {
    fn clone(&self) -> Self {
        Self {
            cap: self.cap,
            remaining: self.remaining,
            cost_per_token: self.cost_per_token,
            alert_threshold: self.alert_threshold,
            hard_limit: self.hard_limit,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum EnergyError {
    #[error("Energy budget exceeded: requested {requested}, remaining {remaining}")]
    BudgetExceeded { requested: u64, remaining: u64 },
}
