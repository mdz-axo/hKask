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

    pub fn allocate(&mut self, tokens: u64) -> Result<u64, EnergyError> {
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

    pub fn try_consume(
        &mut self,
        _operation: &str,
        estimated_tokens: u64,
    ) -> Result<u64, EnergyError> {
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
    pub fn acquire_budget(&mut self, estimated_tokens: u64) -> Result<u64, EnergyError> {
        self.try_consume("acquire_budget", estimated_tokens)
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
pub enum EnergyError {
    #[error("Energy budget exceeded: requested {requested}, remaining {remaining}")]
    BudgetExceeded { requested: u64, remaining: u64 },
}

/// Energy account for tracking costs
#[derive(Debug, Clone)]
pub struct EnergyAccount {
    pub id: String,
    pub budget: EnergyBudget,
    pub total_allocated: u64,
    pub total_consumed: u64,
    pub opportunity_costs: Vec<OpportunityCost>,
}

impl EnergyAccount {
    pub fn new(id: &str, cap: u64) -> Self {
        Self {
            id: id.to_string(),
            budget: EnergyBudget::new(cap),
            total_allocated: 0,
            total_consumed: 0,
            opportunity_costs: vec![],
        }
    }

    pub fn allocate(&mut self, tokens: u64) -> Result<u64, EnergyError> {
        let cost = self.budget.allocate(tokens)?;
        self.total_allocated = self.total_allocated.saturating_add(cost);
        Ok(cost)
    }

    pub fn consume(&mut self, cost: u64) {
        self.total_consumed = self.total_consumed.saturating_add(cost);
    }

    pub fn record_opportunity(&mut self, opportunity: OpportunityCost) {
        self.opportunity_costs.push(opportunity);
    }

    pub fn total_opportunity_cost(&self) -> u64 {
        self.opportunity_costs.iter().map(|o| o.cost).sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityCost {
    pub operation: String,
    pub actual_cost: u64,
    pub alternative_cost: u64,
    pub cost: u64,
    pub timestamp: i64,
}

impl OpportunityCost {
    pub fn new(operation: &str, actual: u64, alternative: u64) -> Self {
        Self {
            operation: operation.to_string(),
            actual_cost: actual,
            alternative_cost: alternative,
            cost: alternative.saturating_sub(actual),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}
