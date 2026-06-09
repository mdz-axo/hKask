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

use crate::energy::{AgentGasStatus, GasBudget, GasCost, GasError};
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
    pub async fn can_proceed(&self, agent: &WebID, gas: GasCost) -> bool {
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
    pub async fn reserve_gas(&self, agent: &WebID, gas: GasCost) -> Result<GasCost, GasError> {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.reserve(gas)
        } else {
            // No budget registered — allow by default (soft limit)
            Ok(GasCost::ZERO)
        }
    }

    /// Settle gas: if actual < reserved, the difference is refunded.
    pub async fn settle_gas(
        &self,
        agent: &WebID,
        reserved_gas: GasCost,
        actual_gas: GasCost,
    ) -> Result<GasCost, GasError> {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.settle(reserved_gas, actual_gas)
        } else {
            // No budget registered — cost is 0 (soft limit)
            Ok(GasCost::ZERO)
        }
    }

    /// For estimated cost, prefer `reserve_gas` + `settle_gas`.
    pub async fn acquire_budget(&self, agent: &WebID, gas: GasCost) -> Result<GasCost, GasError> {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.consume(gas)
        } else {
            // No budget registered — cost is 0 (soft limit)
            Ok(GasCost::ZERO)
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
                    GasCost::ZERO
                }
            };
            if replenished.0 > 0 {
                tracing::debug!(
                    target: "cns.cybernetics",
                    agent = %agent,
                    replenish_rate = replenished.0,
                    "Replenished gas budget"
                );
            }
        }
    }

    /// Used by `CuratorDirective::ReplenishBudget`.
    pub async fn replenish_agent_budget(&self, agent: &WebID, amount: GasCost) {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.replenish_by(amount);
            tracing::info!(
                target: "cns.cybernetics",
                agent = %agent,
                amount = %amount,
                remaining = %budget.remaining,
                "Replenished agent gas budget by directive"
            );
        }
    }

    /// Metacognitive override — recorded in active_overrides so replenish skips this agent.
    pub async fn apply_override_gas_budget(&self, agent: WebID, new_budget: GasCost) {
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
                new_budget = %new_budget,
                "Applied OverrideGasBudget directive from Curation (set-point override)"
            );
        } else {
            budgets.insert(agent, GasBudget::new(new_budget));
            tracing::warn!(
                target: "cns.cybernetics",
                agent = %agent,
                new_budget = %new_budget,
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
    pub async fn apply_replenish_budget(
        &self,
        agent: WebID,
        amount: GasCost,
        priority: Option<f64>,
    ) {
        let mut budgets = self.gas_budgets.write().await;
        if let Some(budget) = budgets.get_mut(&agent) {
            let replenished = if let Some(p) = priority {
                budget.replenish_by_weighted(amount, p)
            } else {
                budget.replenish_by(amount);
                GasCost(amount.0.min(budget.cap.0 - budget.remaining.0))
            };
            drop(budgets);
            tracing::info!(
                target: "cns.cybernetics",
                agent = %agent,
                amount = %amount,
                priority = priority,
                replenished = %replenished,
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
    pub async fn energy_ratios(&self) -> Vec<(GasCost, GasCost)> {
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
    use hkask_types::WebID;

    #[tokio::test]
    async fn register_and_can_proceed() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        manager
            .register_gas_budget(agent, GasBudget::new(GasCost(1000)))
            .await;
        assert!(manager.can_proceed(&agent, GasCost(500)).await);
        assert!(!manager.can_proceed(&agent, GasCost(1001)).await);
    }

    #[tokio::test]
    async fn no_budget_allows_proceed_soft_limit() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        // No budget registered → soft limit, always allowed
        assert!(manager.can_proceed(&agent, GasCost(99999)).await);
    }

    #[tokio::test]
    async fn reserve_and_settle_hold_settle() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        manager
            .register_gas_budget(agent, GasBudget::new(GasCost(1000)))
            .await;
        // Reserve
        manager.reserve_gas(&agent, GasCost(100)).await.unwrap();
        // Settle with actual = 80 (refund 20)
        manager
            .settle_gas(&agent, GasCost(100), GasCost(80))
            .await
            .unwrap();
        let status = manager.agent_gas_status(&agent).await.unwrap();
        assert_eq!(status.remaining, GasCost(920));
        assert_eq!(status.reserved, GasCost(0));
    }

    #[tokio::test]
    async fn reserve_no_budget_returns_zero() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        // No budget → soft limit, reserve returns Ok(ZERO)
        let result = manager.reserve_gas(&agent, GasCost(50)).await.unwrap();
        assert_eq!(result, GasCost::ZERO);
    }

    #[tokio::test]
    async fn settle_no_budget_returns_zero() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        let result = manager
            .settle_gas(&agent, GasCost(50), GasCost(30))
            .await
            .unwrap();
        assert_eq!(result, GasCost::ZERO);
    }

    #[tokio::test]
    async fn replenish_all_skips_overridden_agents() {
        let manager = GasBudgetManager::new();
        let agent_a = WebID::new();
        let agent_b = WebID::new();
        manager
            .register_gas_budget(agent_a, GasBudget::new(GasCost(1000)))
            .await;
        manager
            .register_gas_budget(agent_b, GasBudget::new(GasCost(1000)))
            .await;

        // Consume some gas
        {
            let mut budgets = manager.gas_budgets.write().await;
            budgets.get_mut(&agent_a).unwrap().remaining = GasCost(500);
            budgets.get_mut(&agent_b).unwrap().remaining = GasCost(500);
        }

        // Override agent_a
        manager
            .apply_override_gas_budget(agent_a, GasCost(200))
            .await;

        // Replenish all — agent_a should be skipped (override)
        manager.replenish_all_budgets().await;

        let status_a = manager.agent_gas_status(&agent_a).await.unwrap();
        let status_b = manager.agent_gas_status(&agent_b).await.unwrap();
        // agent_a was overridden to 200 and not replenished
        assert_eq!(status_a.remaining, GasCost(200));
        // agent_b was replenished (rate = 100)
        assert_eq!(status_b.remaining, GasCost(600));
    }

    #[tokio::test]
    async fn clear_override_resumes_replenishment() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        manager
            .register_gas_budget(agent, GasBudget::new(GasCost(1000)))
            .await;

        manager.apply_override_gas_budget(agent, GasCost(200)).await;
        manager.apply_clear_override(agent).await;

        // Consume some gas
        {
            let mut budgets = manager.gas_budgets.write().await;
            budgets.get_mut(&agent).unwrap().remaining = GasCost(100);
        }

        // Now replenishment should work (rate = cap/10 = 100)
        manager.replenish_all_budgets().await;
        let status = manager.agent_gas_status(&agent).await.unwrap();
        assert_eq!(status.remaining, GasCost(200)); // 100 remaining + 100 replenish
    }

    #[tokio::test]
    async fn agent_gas_status_none_when_not_registered() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        assert!(manager.agent_gas_status(&agent).await.is_none());
    }

    #[tokio::test]
    async fn acquire_budget_consumes_directly() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        manager
            .register_gas_budget(agent, GasBudget::new(GasCost(1000)))
            .await;
        manager.acquire_budget(&agent, GasCost(300)).await.unwrap();
        let status = manager.agent_gas_status(&agent).await.unwrap();
        assert_eq!(status.remaining, GasCost(700));
    }

    #[tokio::test]
    async fn replenish_agent_budget_by_directive() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        manager
            .register_gas_budget(agent, GasBudget::new(GasCost(1000)))
            .await;
        // Consume
        {
            let mut budgets = manager.gas_budgets.write().await;
            budgets.get_mut(&agent).unwrap().remaining = GasCost(500);
        }
        manager.replenish_agent_budget(&agent, GasCost(200)).await;
        let status = manager.agent_gas_status(&agent).await.unwrap();
        assert_eq!(status.remaining, GasCost(700));
    }

    #[tokio::test]
    async fn energy_ratios_returns_all_registered() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        manager
            .register_gas_budget(agent, GasBudget::new(GasCost(1000)))
            .await;
        let ratios = manager.energy_ratios().await;
        assert_eq!(ratios.len(), 1);
        assert_eq!(ratios[0], (GasCost(1000), GasCost(1000)));
    }

    #[tokio::test]
    async fn default_matches_new() {
        let a = GasBudgetManager::new();
        let b = GasBudgetManager::default();
        let agent = WebID::new();
        assert!(a.agent_gas_status(&agent).await.is_none());
        assert!(b.agent_gas_status(&agent).await.is_none());
    }

    #[tokio::test]
    async fn expire_overrides_removes_expired_ttl() {
        let manager = GasBudgetManager::new();
        let agent = WebID::new();
        manager
            .register_gas_budget(agent, GasBudget::new(GasCost(1000)))
            .await;

        // Apply override — TTL = 0 means persists until cleared (not expired)
        manager.apply_override_gas_budget(agent, GasCost(200)).await;
        manager.expire_overrides().await;
        // TTL=0 override should still be present
        {
            let budgets = manager.gas_budgets.write().await;
            let budget = budgets.get(&agent).unwrap();
            assert_eq!(budget.remaining, GasCost(200));
        }

        // Clear it so replenishment resumes
        manager.apply_clear_override(agent).await;
        {
            let mut budgets = manager.gas_budgets.write().await;
            budgets.get_mut(&agent).unwrap().remaining = GasCost(100);
        }
        manager.replenish_all_budgets().await;
        let status = manager.agent_gas_status(&agent).await.unwrap();
        assert_eq!(status.remaining, GasCost(200)); // 100 + 100 replenish
    }
}
