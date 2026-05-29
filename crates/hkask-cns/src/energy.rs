//! CNS Energy Spans
//!
//! Implements energy cost as CNS primitive for pragmatic composition.
//! Energy tracking enables economic analysis of template/manifest operations.
//!
//! **Span Types:**
//! - `cns.energy.allocate` — Energy budget assignment
//! - `cns.energy.consume` — Operation cost debit
//! - `cns.energy.opportunity` — Alternative cost analysis
//! - `cns.energy.deficit` — Algedonic alert trigger (variety deficit)
//!
//! **Integration:**
//! - Every template render → energy cost
//! - Every manifest execute → energy cost
//! - Every registry write → energy cost
//! - Default cost: 1 energy unit per 4 tokens (configurable)

use serde::{Deserialize, Serialize};

/// Energy budget allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyBudget {
    /// Maximum token budget
    pub cap: u64,
    /// Current remaining budget
    pub remaining: u64,
    /// Cost per token (default: 0.25 energy units)
    pub cost_per_token: f64,
    /// Alert threshold (0.0-1.0, default: 0.8)
    pub alert_threshold: f64,
    /// Hard limit enforcement
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

    /// Calculate energy cost for given token count
    pub fn calculate_cost(&self, tokens: u64) -> u64 {
        ((tokens as f64) * self.cost_per_token) as u64
    }

    /// Calculate token count from energy cost
    pub fn calculate_tokens(&self, energy: u64) -> u64 {
        if self.cost_per_token > 0.0 {
            (energy as f64 / self.cost_per_token) as u64
        } else {
            0
        }
    }

    /// Allocate energy from budget
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

    /// Try to consume energy for a named operation.
    ///
    /// This is the enforcement gate — callers that wish to be quota-gated
    /// pass through this method before performing an operation. If the
    /// budget is exhausted, the operation is rejected with `BudgetExceeded`.
    ///
    /// This turns energy *observation* into energy *regulation* — a complete
    /// cybernetic loop (Observe → Regulate → Outcome).
    pub fn try_consume(
        &mut self,
        operation: &str,
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
        tracing::debug!(
            target: "cns.energy.consume",
            operation = %operation,
            tokens = estimated_tokens,
            cost = cost,
            remaining = self.remaining,
            "Energy consumed"
        );
        Ok(cost)
    }

    /// Check if alert should be triggered
    pub fn should_alert(&self) -> bool {
        let usage_ratio = 1.0 - (self.remaining as f64 / self.cap as f64);
        usage_ratio >= self.alert_threshold
    }

    /// Get usage ratio (0.0-1.0)
    pub fn usage_ratio(&self) -> f64 {
        1.0 - (self.remaining as f64 / self.cap as f64)
    }
}

/// Energy error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum EnergyError {
    #[error("Energy budget exceeded: requested {requested}, remaining {remaining}")]
    BudgetExceeded { requested: u64, remaining: u64 },
    #[error("Invalid energy cost: {0}")]
    InvalidCost(String),
    #[error("Energy deficit detected: variety deficit {deficit}")]
    Deficit { deficit: u64 },
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

    /// Record energy allocation
    pub fn allocate(&mut self, tokens: u64) -> Result<u64, EnergyError> {
        let cost = self.budget.allocate(tokens)?;
        self.total_allocated = self.total_allocated.saturating_add(cost);
        Ok(cost)
    }

    /// Record energy consumption
    pub fn consume(&mut self, cost: u64) {
        self.total_consumed = self.total_consumed.saturating_add(cost);
    }

    /// Record opportunity cost
    pub fn record_opportunity(&mut self, opportunity: OpportunityCost) {
        self.opportunity_costs.push(opportunity);
    }

    /// Get total opportunity cost
    pub fn total_opportunity_cost(&self) -> u64 {
        self.opportunity_costs.iter().map(|o| o.cost).sum()
    }
}

/// Opportunity cost record
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
