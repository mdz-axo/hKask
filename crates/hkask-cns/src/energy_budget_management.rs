//! Gas Budget Management — Registration, reservation, settlement, and replenishment
//!
//! Extracted from `CyberneticsLoop` per the Fowler audit (H8). Two consumers
//! justify the extraction: `CyberneticsLoop` (production regulation) and
//! `GovernedTool` (tool invocation membrane). The hold-settle pattern
//! (reserve → call → settle) is the core contract.
//!
//! # Gas Budget Lifecycle
//!
//! 1. **Register** — `register_energy_budget()` creates a budget for an agent
//! 2. **Reserve** — `reserve_gas()` holds budget for estimated cost
//! 3. **Settle** — `settle_gas()` adjusts to actual cost, refunds difference
//! 4. **Replenish** — `replenish_all_budgets()` / `replenish_agent_budget()` restore capacity
//!
//! # Metacognitive Override
//!
//! Curation can override an agent's budget via `apply_override_energy_budget()`.
//! Overridden agents are skipped during `replenish_all_budgets()` to preserve
//! the Curation directive. `apply_clear_override()` resumes normal replenishment.

use crate::energy::{AgentEnergyStatus, EnergyBudget, EnergyCost, EnergyError};
use crate::wallet_budget::WalletBackedBudget;
use hkask_types::WebID;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Record of an active Curation override on an agent's energy budget.
///
/// When Curation issues an `OverrideEnergyBudget` directive, the override is
/// recorded here so that `replenish_all_budgets()` does not overwrite it
/// on the next regulation cycle. This preserves the metacognitive override
/// mechanism — the core safety feature that lets Curation exceed
/// Cybernetics' set-point range.
struct OverrideRecord {
    /// When this override was issued (for TTL expiry)
    issued_at: chrono::DateTime<chrono::Utc>,
    ttl_secs: u64,
}

/// Gas Budget Manager — registration, reservation, settlement, and replenishment.
///
/// Owns the energy budget map and active override tracking. Extracted from
/// `CyberneticsLoop` to concentrate energy budget logic and allow direct access
/// from `GovernedTool` without going through the full loop.
pub struct EnergyBudgetManager {
    energy_budgets: Arc<RwLock<HashMap<WebID, EnergyBudget>>>,
    /// Wallet-backed budgets — checked before gas budgets.
    /// When an agent has a wallet budget, gas operations debit rJoules
    /// instead of consuming from the dimensionless gas pool.
    wallet_budgets: Arc<RwLock<HashMap<WebID, WalletBackedBudget>>>,
    active_overrides: Arc<RwLock<HashMap<WebID, OverrideRecord>>>,
}

impl Default for EnergyBudgetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyBudgetManager {
    /// Create a new `EnergyBudgetManager` with empty budget and override maps.
    pub fn new() -> Self {
        Self {
            energy_budgets: Arc::new(RwLock::new(HashMap::new())),
            wallet_budgets: Arc::new(RwLock::new(HashMap::new())),
            active_overrides: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a energy budget for an agent.
    pub async fn register_energy_budget(&self, agent: WebID, budget: EnergyBudget) {
        let mut budgets = self.energy_budgets.write().await;
        budgets.insert(agent, budget);
    }

    /// Register a wallet-backed budget for an agent.
    /// Wallet budgets are checked before gas budgets.
    pub async fn register_wallet_budget(&self, agent: WebID, budget: WalletBackedBudget) {
        let mut budgets = self.wallet_budgets.write().await;
        budgets.insert(agent, budget);
    }

    /// Check whether an agent can proceed with the given energy cost estimate.
    ///
    /// Checks wallet budgets first, then gas budgets.
    /// Returns `true` if the agent has no registered budget (soft limit)
    /// or if the budget has sufficient remaining capacity.
    pub async fn can_proceed(&self, agent: &WebID, gas: EnergyCost) -> bool {
        // Check wallet budget first
        let wallet_budgets = self.wallet_budgets.read().await;
        if let Some(budget) = wallet_budgets.get(agent) {
            return budget.can_proceed(gas);
        }
        drop(wallet_budgets);
        // Fall back to gas budget
        let budgets = self.energy_budgets.read().await;
        if let Some(budget) = budgets.get(agent) {
            budget.can_proceed(gas)
        } else {
            true
        }
    }

    /// Returns `None` if agent has no registered budget.
    pub async fn agent_gas_status(&self, agent: &WebID) -> Option<AgentEnergyStatus> {
        let budgets = self.energy_budgets.read().await;
        budgets.get(agent).map(AgentEnergyStatus::from)
    }

    /// Hold-settle pattern: gas reserved but not consumed. Call `settle_gas()` after.
    /// Checks wallet budgets first, then gas budgets.
    pub async fn reserve_gas(
        &self,
        agent: &WebID,
        gas: EnergyCost,
    ) -> Result<EnergyCost, EnergyError> {
        // Check wallet budget first
        let wallet_budgets = self.wallet_budgets.read().await;
        if let Some(budget) = wallet_budgets.get(agent) {
            return budget.reserve(gas);
        }
        drop(wallet_budgets);
        // Fall back to gas budget
        let mut budgets = self.energy_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.reserve(gas)
        } else {
            Ok(EnergyCost::ZERO)
        }
    }

    /// Settle gas: if actual < reserved, the difference is refunded.
    /// Checks wallet budgets first, then gas budgets.
    pub async fn settle_gas(
        &self,
        agent: &WebID,
        reserved_gas: EnergyCost,
        actual_gas: EnergyCost,
    ) -> Result<EnergyCost, EnergyError> {
        // Check wallet budget first
        let wallet_budgets = self.wallet_budgets.read().await;
        if let Some(budget) = wallet_budgets.get(agent) {
            return budget.settle(reserved_gas, actual_gas);
        }
        drop(wallet_budgets);
        // Fall back to gas budget
        let mut budgets = self.energy_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.settle(reserved_gas, actual_gas)
        } else {
            Ok(EnergyCost::ZERO)
        }
    }

    /// For estimated cost, prefer `reserve_gas` + `settle_gas`.
    pub async fn acquire_budget(
        &self,
        agent: &WebID,
        gas: EnergyCost,
    ) -> Result<EnergyCost, EnergyError> {
        let mut budgets = self.energy_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.consume(gas)
        } else {
            // No budget registered — cost is 0 (soft limit)
            Ok(EnergyCost::ZERO)
        }
    }

    /// Replenish all registered budgets, skipping agents with active Curation overrides.
    pub async fn replenish_all_budgets(&self) {
        let budget_ids: Vec<WebID> = {
            let budgets = self.energy_budgets.read().await;
            budgets.keys().cloned().collect()
        };
        let overrides = self.active_overrides.read().await;
        for agent in budget_ids {
            if overrides.contains_key(&agent) {
                // Skip replenishment for agents with active Curation overrides
                continue;
            }
            let replenished = {
                let mut budgets = self.energy_budgets.write().await;
                if let Some(budget) = budgets.get_mut(&agent) {
                    let rate = budget.replenish_rate();
                    budget.replenish();
                    rate
                } else {
                    EnergyCost::ZERO
                }
            };
            if replenished.0 > 0 {
                tracing::debug!(
                    target: "cns.cybernetics",
                    agent = %agent,
                    replenish_rate = replenished.0,
                    "Replenished energy budget"
                );
            }
        }
    }

    /// Used by `CuratorDirective::ReplenishBudget`.
    pub async fn replenish_agent_budget(&self, agent: &WebID, amount: EnergyCost) {
        let mut budgets = self.energy_budgets.write().await;
        if let Some(budget) = budgets.get_mut(agent) {
            budget.replenish_by(amount);
            tracing::info!(
                target: "cns.cybernetics",
                agent = %agent,
                amount = %amount,
                remaining = %budget.remaining(),
                "Replenished agent energy budget by directive"
            );
        }
    }

    /// Metacognitive override — recorded in active_overrides so replenish skips this agent.
    pub async fn apply_override_energy_budget(&self, agent: WebID, new_budget: EnergyCost) {
        // Default TTL of 0 means override persists until explicitly cleared
        let ttl_secs: u64 = 0;
        let mut budgets = self.energy_budgets.write().await;
        if let Some(budget) = budgets.get_mut(&agent) {
            budget.reset_to(new_budget);
            tracing::warn!(
                target: "cns.cybernetics",
                agent = %agent,
                new_budget = %new_budget,
                "Applied OverrideEnergyBudget directive from Curation (set-point override)"
            );
        } else {
            budgets.insert(agent, EnergyBudget::new(new_budget));
            tracing::warn!(
                target: "cns.cybernetics",
                agent = %agent,
                new_budget = %new_budget,
                "Registered new energy budget from OverrideEnergyBudget directive"
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
        amount: EnergyCost,
        priority: Option<f64>,
    ) {
        let mut budgets = self.energy_budgets.write().await;
        if let Some(budget) = budgets.get_mut(&agent) {
            let replenished = if let Some(p) = priority {
                budget.replenish_by_weighted(amount, p)
            } else {
                budget.replenish_by(amount);
                EnergyCost(amount.0.min(budget.cap().0 - budget.remaining().0))
            };
            drop(budgets);
            tracing::info!(
                target: "cns.cybernetics",
                agent = %agent,
                amount = %amount,
                priority = priority,
                replenished = %replenished,
                "Replenished agent energy budget by directive"
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

    /// Iterate over energy budgets to produce energy signals.
    /// Returns `(remaining, cap)` for each registered agent.
    pub async fn energy_ratios(&self) -> Vec<(EnergyCost, EnergyCost)> {
        let budgets = self.energy_budgets.read().await;
        budgets
            .values()
            .map(|budget| (budget.remaining(), budget.cap()))
            .collect()
    }

    /// Iterate over wallet-backed budgets to produce wallet health signals.
    /// Returns `(balance_ratio, cap_ratio)` for each wallet-backed agent.
    /// balance_ratio: 0.0 = empty, 1.0 = full (relative to a nominal capacity).
    pub async fn wallet_balance_ratios(&self) -> Vec<(f64, f64)> {
        let budgets = self.wallet_budgets.read().await;
        let mut ratios = Vec::new();
        for budget in budgets.values() {
            // Get the wallet balance and compute a ratio.
            // We use a nominal capacity of 1_000_000 rJ as the denominator
            // (this is a simplified model — production would use 30-day moving avg).
            match budget.wallet_manager.get_balance(budget.wallet_id) {
                Ok(balance) => {
                    let nominal_cap: f64 = 1_000_000.0; // 1M rJ nominal capacity
                    let ratio = (balance.rjoules as f64 / nominal_cap).clamp(0.0, 1.0);
                    ratios.push((ratio, 1.0));
                }
                Err(_) => {
                    // Wallet error → treat as empty
                    ratios.push((0.0, 1.0));
                }
            }
        }
        ratios
    }

    /// Check API key health for all wallet-backed budgets.
    /// Returns `(agent, reason)` for each key that is exhausted or expired.
    pub async fn wallet_key_alerts(&self) -> Vec<(WebID, String)> {
        let budgets = self.wallet_budgets.read().await;
        let mut alerts = Vec::new();
        for (agent, budget) in budgets.iter() {
            if let Some(health) = budget.check_key_health() {
                if health.exhausted {
                    alerts.push((*agent, "key_exhausted".into()));
                }
                if health.expired {
                    alerts.push((*agent, "key_expired".into()));
                }
            }
        }
        alerts
    }
}
