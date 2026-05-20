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
//! - `cns.energy.actual` — Actual energy consumption with capability context
//! - `cns.energy.quota` — Quota allocation from parent to child
//! - `cns.energy.overflow` — Energy budget exceeded (hard abort or escalate)
//!
//! **Integration:**
//! - Every template render → energy cost
//! - Every manifest execute → energy cost
//! - Every registry write → energy cost
//! - Default cost: 1 energy unit per 4 tokens (configurable)

use hkask_types::{NuEvent, Span, WebID};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

/// Energy span types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergySpanType {
    Allocate,
    Consume,
    Opportunity,
    Deficit,
    Actual,
    Quota,
    Overflow,
}

impl EnergySpanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnergySpanType::Allocate => "cns.energy.allocate",
            EnergySpanType::Consume => "cns.energy.consume",
            EnergySpanType::Opportunity => "cns.energy.opportunity",
            EnergySpanType::Deficit => "cns.energy.deficit",
            EnergySpanType::Actual => "cns.energy.actual",
            EnergySpanType::Quota => "cns.energy.quota",
            EnergySpanType::Overflow => "cns.energy.overflow",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "allocate" | "cns.energy.allocate" => Some(EnergySpanType::Allocate),
            "consume" | "cns.energy.consume" => Some(EnergySpanType::Consume),
            "opportunity" | "cns.energy.opportunity" => Some(EnergySpanType::Opportunity),
            "deficit" | "cns.energy.deficit" => Some(EnergySpanType::Deficit),
            "actual" | "cns.energy.actual" => Some(EnergySpanType::Actual),
            "quota" | "cns.energy.quota" => Some(EnergySpanType::Quota),
            "overflow" | "cns.energy.overflow" => Some(EnergySpanType::Overflow),
            _ => None,
        }
    }
}

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

/// Energy span emitter
pub struct EnergyEmitter {
    observer_webid: WebID,
    account: EnergyAccount,
}

impl EnergyEmitter {
    pub fn new(observer_webid: WebID, account: EnergyAccount) -> Self {
        Self {
            observer_webid,
            account,
        }
    }

    /// Get energy account
    pub fn account(&self) -> &EnergyAccount {
        &self.account
    }

    /// Get mutable energy account
    pub fn account_mut(&mut self) -> &mut EnergyAccount {
        &mut self.account
    }

    /// Emit energy allocate span
    pub fn emit_allocate(&mut self, operation: &str, tokens: u64) -> Result<u64, EnergyError> {
        let cost = self.account.allocate(tokens)?;

        let observation = serde_json::json!({
            "operation": operation,
            "tokens": tokens,
            "cost": cost,
            "remaining": self.account.budget.remaining,
            "usage_ratio": self.account.budget.usage_ratio(),
            "should_alert": self.account.budget.should_alert(),
        });

        self.emit(EnergySpanType::Allocate, observation);
        Ok(cost)
    }

    /// Emit energy consume span
    pub fn emit_consume(&mut self, operation: &str, cost: u64) {
        self.account.consume(cost);

        let observation = serde_json::json!({
            "operation": operation,
            "cost": cost,
            "total_consumed": self.account.total_consumed,
            "remaining": self.account.budget.remaining,
            "usage_ratio": self.account.budget.usage_ratio(),
        });

        self.emit(EnergySpanType::Consume, observation);
    }

    /// Emit energy opportunity span
    pub fn emit_opportunity(&mut self, operation: &str, actual: u64, alternative: u64) {
        let opportunity = OpportunityCost::new(operation, actual, alternative);
        let cost = opportunity.cost;
        self.account.record_opportunity(opportunity);

        let observation = serde_json::json!({
            "operation": operation,
            "actual_cost": actual,
            "alternative_cost": alternative,
            "opportunity_cost": cost,
            "total_opportunity_cost": self.account.total_opportunity_cost(),
        });

        self.emit(EnergySpanType::Opportunity, observation);
    }

    /// Emit energy deficit span (algedonic alert trigger)
    pub fn emit_deficit(&self, variety_deficit: u64, threshold: u64) {
        let observation = serde_json::json!({
            "variety_deficit": variety_deficit,
            "threshold": threshold,
            "alert_triggered": variety_deficit > threshold,
            "severity": if variety_deficit > threshold * 2 {
                "critical"
            } else if variety_deficit > threshold {
                "high"
            } else {
                "low"
            },
        });

        self.emit(EnergySpanType::Deficit, observation);
    }

    /// Emit energy actual span (actual energy consumption measurement)
    ///
    /// # Arguments
    /// * `operation` - Operation name
    /// * `tokens_actual` - Actual tokens consumed
    /// * `energy_actual` - Actual energy cost
    /// * `capability_id` - Optional capability ID used for authorization (for audit trail)
    pub fn emit_actual(
        &mut self,
        operation: &str,
        tokens_actual: u64,
        energy_actual: u64,
        capability_id: Option<&str>,
    ) {
        self.account.consume(energy_actual);

        let observation = serde_json::json!({
            "operation": operation,
            "tokens_actual": tokens_actual,
            "energy_actual": energy_actual,
            "capability_id": capability_id,
            "total_consumed": self.account.total_consumed,
            "remaining": self.account.budget.remaining,
            "usage_ratio": self.account.budget.usage_ratio(),
        });

        self.emit(EnergySpanType::Actual, observation);
    }

    /// Emit quota allocation span (parent allocates quota to child)
    ///
    /// # Arguments
    /// * `from_manifest` - Parent manifest ID
    /// * `to_manifest` - Child manifest ID receiving quota
    /// * `quota_amount` - Energy quota allocated
    pub fn emit_quota(&self, from_manifest: &str, to_manifest: &str, quota_amount: u64) {
        let observation = serde_json::json!({
            "from_manifest": from_manifest,
            "to_manifest": to_manifest,
            "quota_allocated": quota_amount,
        });

        self.emit(EnergySpanType::Quota, observation);
    }

    /// Emit energy overflow span (budget exceeded)
    ///
    /// # Arguments
    /// * `manifest_id` - Manifest that exceeded budget
    /// * `capability_id` - Capability that was being used
    /// * `budget_allocated` - Original budget allocation
    /// * `budget_consumed` - Actual consumption
    /// * `overflow_action` - Action taken: "hard_abort" or "escalate"
    pub fn emit_overflow(
        &self,
        manifest_id: &str,
        capability_id: &str,
        budget_allocated: u64,
        budget_consumed: u64,
        overflow_action: &str,
    ) {
        let overage = budget_consumed.saturating_sub(budget_allocated);
        let overage_percent = if budget_allocated > 0 {
            (overage as f64 / budget_allocated as f64) * 100.0
        } else {
            0.0
        };

        let observation = serde_json::json!({
            "manifest_id": manifest_id,
            "capability_id": capability_id,
            "budget_allocated": budget_allocated,
            "budget_consumed": budget_consumed,
            "overage": overage,
            "overage_percent": overage_percent,
            "overflow_action": overflow_action,
        });

        self.emit(EnergySpanType::Overflow, observation);
    }

    /// Emit energy span
    fn emit(&self, span_type: EnergySpanType, observation: Value) {
        let span = Span::Energy(span_type.as_str().to_string());
        let event = NuEvent::new(
            self.observer_webid,
            span,
            hkask_types::Phase::Observe,
            observation,
            0,
        );

        info!(
            target: "cns.energy",
            event = ?event.id,
            span_type = span_type.as_str(),
            "Energy span emitted"
        );
    }
}

/// Estimate token count from text (4 chars ≈ 1 token)
pub fn estimate_tokens(text: &str) -> u64 {
    (text.len() as f64 / 4.0).ceil() as u64
}

/// Calculate energy cost for text (default: 0.25 energy per token)
pub fn calculate_energy_cost(text: &str, cost_per_token: f64) -> u64 {
    let tokens = estimate_tokens(text);
    ((tokens as f64) * cost_per_token) as u64
}

