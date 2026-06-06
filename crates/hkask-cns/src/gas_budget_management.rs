//! Gas Budget Management — Registration, reservation, settlement, and replenishment
//!
//! Extracted from `CyberneticsLoop` per the Fowler audit (H8). Two consumers
//! justify the extraction: `CyberneticsLoop` (production regulation) and
//! `GovernedTool` (tool invocation membrane). The hold-settle pattern
//! (reserve → call → settle) is the core contract.
//!
//! # Gas Budget Lifecycle
//!
//! 1. **Register** — `register_gas_budget()` creates a budget for an agent
//! 2. **Reserve** — `reserve_gas()` holds budget for estimated cost
//! 3. **Settle** — `settle_gas()` adjusts to actual cost, refunds difference
//! 4. **Replenish** — `replenish_all_budgets()` / `replenish_agent_budget()` restore capacity
//!
//! # Metacognitive Override
//!
//! Curation can override an agent's budget via `apply_override_gas_budget()`.
//! Overridden agents are skipped during `replenish_all_budgets()` to preserve
//! the Curation directive. `apply_clear_override()` resumes normal replenishment.

use crate::energy::{AgentGasStatus, GasBudget, GasError};
use hkask_types::WebID;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Record of an active Curation override on an agent's gas budget.
///
/// When Curation issues an `OverrideGasBudget` directive, the override is
/// recorded here so that `replenish_all_budgets()` does not overwrite it
/// on the next regulation cycle. This preserves the metacognitive override
/// mechanism — the core safety feature that lets Curation exceed
/// Cybernetics' set-point range.
struct OverrideRecord {
    /// When this override was issued (for TTL expiry)
    issued_at: chrono::DateTime<chrono::Utc>,
    /// TTL in seconds (0 = no expiry, must be explicitly cleared)
    ttl_secs: u64,
}

/// Gas Budget Manager — registration, reservation, settlement, and replenishment.
///
/// Owns the gas budget map and active override tracking. Extracted from
/// `CyberneticsLoop` to concentrate gas budget logic and allow direct access
/// from `GovernedTool` without going through the full loop.
pub struct GasBudgetManager {
    gas_budgets: Arc<RwLock<HashMap<WebID, GasBudget>>>,
    active_overrides: Arc<RwLock<HashMap<WebID, OverrideRecord>>>,
}

impl Default for GasBudgetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GasBudgetManager {
    /// Create a new `GasBudgetManager` with empty budget and override maps.
    pub fn new() -> Self {
        Self {
            gas_budgets: Arc::new(RwLock::new(HashMap::new())),
            active_overrides: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a gas budget for an agent.
    pub async fn register_gas_budget(&self, agent: WebID, budget: GasBudget) {
        let mut budgets = self.gas_budgets.write().await;
        budgets.insert(agent, budget);
    }

    /// Check whether an agent can proceed with the given gas cost estimate.
    ///
    /// Returns `true` if the agent has no registered budget (soft limit)
    /// or if the budget has sufficient remaining capacity.
    pub async fn can_proceed(&self, agent: &WebID, gas: u64) -> bool {
        let budgets = self.gas_budgets.read().await;
        if let Some(budget) = budgets.get(agent) {
            budget.can_proceed(gas)
        } else {
            // No budget registered — allow by default (soft limit)
            true
        }
    }

    /// Returns `None` if agent has no registered budget.
    pub async fn agent_gas_status(&self, agent: &WebID) -> Option<AgentGasStatus> {
        let budgets = self.gas_budgets.read().await;
        budgets.get(agent).map(AgentGasStatus::from)
    }

    /// Hold-settle pattern: gas reserved but not consumed. Call `settle_gas()` after.
    pub async fn reserve_gas(&self, agent: &WebID, gas: u64) -> Result<u64, GasError> {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.reserve(gas)
        } else {
            // No budget registered — allow by default (soft limit)
            Ok(0)
        }
    }

    /// Settle gas: if actual < reserved, the difference is refunded.
    pub async fn settle_gas(
        &self,
        agent: &WebID,
        reserved_gas: u64,
        actual_gas: u64,
    ) -> Result<u64, GasError> {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.settle(reserved_gas, actual_gas)
        } else {
            // No budget registered — cost is 0 (soft limit)
            Ok(0)
        }
    }

    /// For estimated cost, prefer `reserve_gas` + `settle_gas`.
    pub async fn acquire_budget(&self, agent: &WebID, gas: u64) -> Result<u64, GasError> {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.consume(gas)
        } else {
            // No budget registered — cost is 0 (soft limit)
            Ok(0)
        }
    }

    /// Replenish all registered budgets, skipping agents with active Curation overrides.
    pub async fn replenish_all_budgets(&self) {
        let budget_ids: Vec<WebID> = {
            let budgets = self.gas_budgets.read().await;
            budgets.keys().cloned().collect()
        };
        let overrides = self.active_overrides.read().await;
        for agent in budget_ids {
            if overrides.contains_key(&agent) {
                // Skip replenishment for agents with active Curation overrides
                continue;
            }
            let replenished = {
                let mut budgets = self.gas_budgets.write().await;
                if let Some(budget) = budgets.get_mut(&agent) {
                    let rate = budget.replenish_rate;
                    budget.replenish();
                    rate
                } else {
                    0
                }
            };
            if replenished > 0 {
                tracing::debug!(
                    target: "cns.cybernetics",
                    agent = %agent,
                    replenish_rate = replenished,
                    "Replenished gas budget"
                );
            }
        }
    }

    /// Used by `CuratorDirective::ReplenishBudget`.
    pub async fn replenish_agent_budget(&self, agent: &WebID, amount: u64) {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.replenish_by(amount);
            tracing::info!(
                target: "cns.cybernetics",
                agent = %agent,
                amount = amount,
                remaining = budget.remaining,
                "Replenished agent gas budget by directive"
            );
        }
    }

    /// Metacognitive override — recorded in active_overrides so replenish skips this agent.
    pub async fn apply_override_gas_budget(&self, agent: WebID, new_budget: u64) {
        // Default TTL of 0 means override persists until explicitly cleared
        let ttl_secs: u64 = 0;
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(&agent) {
            // Override can set budget above or below set-points
            budget.cap = new_budget;
            budget.remaining = new_budget;
            tracing::warn!(
                target: "cns.cybernetics",
                agent = %agent,
                new_budget = new_budget,
                "Applied OverrideGasBudget directive from Curation (set-point override)"
            );
        } else {
            budgets.insert(agent, GasBudget::new(new_budget));
            tracing::warn!(
                target: "cns.cybernetics",
                agent = %agent,
                new_budget = new_budget,
                "Registered new gas budget from OverrideGasBudget directive"
            );
        }
        drop(budgets);
        // Record the override so replenish_all_budgets() skips this agent
        let mut overrides = self.active_overrides.write().await;
        overrides.insert(
            agent,
            OverrideRecord {
                issued_at: chrono::Utc::now(),
                ttl_secs,
            },
        );
    }

    /// Removes agent from active_overrides, resuming normal replenishment.
    pub async fn apply_clear_override(&self, agent: WebID) {
        let mut overrides = self.active_overrides.write().await;
        if overrides.remove(&agent).is_some() {
            tracing::info!(
                target: "cns.cybernetics",
                agent = %agent,
                "Cleared Curation override — normal replenishment resumes"
            );
        } else {
            tracing::debug!(
                target: "cns.cybernetics",
                agent = %agent,
                "ClearOverride directive received but no active override found"
            );
        }
    }

    /// Priority-scaled: when priority is provided, replenishment is weighted.
    pub async fn apply_replenish_budget(&self, agent: WebID, amount: u64, priority: Option<f64>) {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(&agent) {
            let replenished = if let Some(p) = priority {
                budget.replenish_by_weighted(amount, p)
            } else {
                budget.replenish_by(amount);
                amount.min(budget.cap - budget.remaining)
            };
            drop(budgets);
            tracing::info!(
                target: "cns.cybernetics",
                agent = %agent,
                amount = amount,
                priority = priority,
                replenished = replenished,
                "Replenished agent gas budget by directive"
            );
        }
    }
    /// Expire overrides with non-zero TTL that have passed their expiry time.
    /// Called during the Cybernetics Loop's sense→compare→compute→act cycle.
    pub async fn expire_overrides(&self) {
        let mut overrides = self.active_overrides.write().await;
        let now = chrono::Utc::now();
        overrides.retain(|agent, record| {
            if record.ttl_secs == 0 {
                return true; // No TTL — persists until explicitly cleared
            }
            let expires_at = record.issued_at + chrono::Duration::seconds(record.ttl_secs as i64);
            if now > expires_at {
                tracing::info!(
                    target: "cns.cybernetics",
                    agent = %agent,
                    "Curation override expired — resuming normal replenishment"
                );
                false
            } else {
                true
            }
        });
    }

    /// Iterate over gas budgets to produce energy signals.
    /// Returns `(remaining, cap)` for each registered agent.
    pub async fn energy_ratios(&self) -> Vec<(u64, u64)> {
        let budgets = self.gas_budgets.read().await;
        budgets
            .values()
            .map(|budget| (budget.remaining, budget.cap))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy::GasBudget;

    #[tokio::test]
    async fn register_and_check_gas_budget() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();

        // No budget registered — should allow by default
        assert!(manager.can_proceed(&agent, 100).await);

        // Register budget
        manager
            .register_gas_budget(agent, GasBudget::new(10_000))
            .await;

        // Should allow within budget
        assert!(manager.can_proceed(&agent, 100).await);

        // Should deny over budget
        assert!(!manager.can_proceed(&agent, 20_000).await);
    }

    #[tokio::test]
    async fn reserve_and_settle_gas() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();

        manager
            .register_gas_budget(agent, GasBudget::new(10_000))
            .await;

        // Reserve gas
        let reserved = manager.reserve_gas(&agent, 500).await.unwrap();
        assert!(reserved > 0);

        // Settle with actual cost
        let settled = manager.settle_gas(&agent, 500, 300).await.unwrap();
        assert!(settled > 0);

        // Budget should reflect the actual cost (300), not the estimate (500)
        // remaining = 10000 - 300 = 9700
        assert!(manager.can_proceed(&agent, 9700).await);
        assert!(!manager.can_proceed(&agent, 9701).await);
    }

    #[tokio::test]
    async fn acquire_budget_consumes_directly() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();

        manager
            .register_gas_budget(agent, GasBudget::new(10_000))
            .await;

        let cost = manager.acquire_budget(&agent, 1000).await.unwrap();
        assert!(cost > 0);

        // Budget should reflect direct consumption
        assert!(manager.can_proceed(&agent, 9000).await);
        assert!(!manager.can_proceed(&agent, 9001).await);
    }

    #[tokio::test]
    async fn replenish_all_skips_overridden_agents() {
        let manager = GasBudgetManager::new();
        let agent1 = WebID::new();
        let agent2 = WebID::new();

        // Register two agents with same budget
        manager
            .register_gas_budget(agent1, GasBudget::new(10_000).with_replenish_rate(1_000))
            .await;
        manager
            .register_gas_budget(
                agent2,
                GasBudget::new(10_000)
                    .with_replenish_rate(1_000)
                    .with_hard_limit(true),
            )
            .await;

        // Consume some gas from both
        let _ = manager.acquire_budget(&agent1, 5000).await;
        let _ = manager.acquire_budget(&agent2, 5000).await;

        // Override agent1's budget — replenish should skip it
        manager.apply_override_gas_budget(agent1, 20_000).await;

        // Replenish all
        manager.replenish_all_budgets().await;

        // agent2 should be replenished (remaining went up)
        let status2 = manager.agent_gas_status(&agent2).await.unwrap();
        assert!(status2.remaining > 5000, "agent2 should be replenished");

        // agent1's override should be preserved (cap = 20_000)
        let status1 = manager.agent_gas_status(&agent1).await.unwrap();
        assert_eq!(status1.cap, 20_000, "agent1 override should be preserved");
    }

    #[tokio::test]
    async fn clear_override_resumes_replenishment() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();

        manager
            .register_gas_budget(agent, GasBudget::new(10_000).with_replenish_rate(1_000))
            .await;

        // Override
        manager.apply_override_gas_budget(agent, 5_000).await;

        // Clear override
        manager.apply_clear_override(agent).await;

        // Consume some gas
        let _ = manager.acquire_budget(&agent, 4_000).await;

        // Replenish — should restore up to cap
        manager.replenish_all_budgets().await;

        let status = manager.agent_gas_status(&agent).await.unwrap();
        // remaining was 1000, replenish_rate is 1000, cap is 10000
        // After replenishment: remaining = min(1000 + 1000, 10000) = 2000
        assert_eq!(
            status.remaining, 2_000,
            "remaining should reflect replenishment after clearing override"
        );
    }

    #[tokio::test]
    async fn soft_limit_allows_without_budget() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();

        // No budget registered — all operations should succeed with 0 cost
        assert!(manager.can_proceed(&agent, 100).await);
        assert_eq!(manager.reserve_gas(&agent, 100).await.unwrap(), 0);
        assert_eq!(manager.settle_gas(&agent, 100, 50).await.unwrap(), 0);
        assert_eq!(manager.acquire_budget(&agent, 100).await.unwrap(), 0);
    }
}
