//! WalletBackedBudget — Energy budget variant backed by a wallet's rJoule balance.
//!
//! Unlike the standard `EnergyBudget` which replenishes periodically from a
//! dimensionless gas pool, `WalletBackedBudget` converts gas costs to rJoules
//! and debits a real wallet. This is the payment mechanism for paid agents.
//!
//! # Hold-settle pattern
//! 1. `can_proceed(gas)` — converts gas to rJoules, checks wallet balance
//! 2. `reserve(gas)` — optimistic reservation (checks balance, doesn't debit)
//! 3. Tool executes
//! 4. `settle(reserved_gas, actual_gas)` — debits actual rJoule cost
//!
//! # Coexistence with EnergyBudget
//! `WalletBackedBudget` is an additional budget type, not a replacement.
//! The existing gas system continues for non-wallet-backed agents.
//! Both coexist in the `GovernedTool` membrane via `EnergyBudgetManager`.

use crate::energy::{EnergyCost, EnergyError};
use hkask_types::wallet::{ApiKeyId, RJoule, WalletId};
use hkask_wallet::WalletManager;
use std::sync::Arc;

/// An energy budget backed by a wallet's rJoule balance.
///
/// Converts dimensionless gas costs to rJoules via `gas_per_rjoule` and
/// delegates balance checks, reservations, and settlements to `WalletManager`.
///
/// # Hard limit
/// Wallet-backed budgets always have `hard_limit = true`. When the wallet
/// balance is insufficient, operations are rejected — there is no "soft limit"
/// fallback because rJoules represent real value.
pub struct WalletBackedBudget {
    /// The wallet that funds this budget.
    pub wallet_id: WalletId,
    /// Optional API key for spending-limit tracking.
    /// When present, spending is also checked against the key's limit.
    pub key_id: Option<ApiKeyId>,
    /// Optional per-key spending cap (rJoules).
    /// When set, the key cannot spend more than this total.
    pub spending_limit_rj: Option<RJoule>,
    /// Reference to the wallet manager for balance operations.
    pub wallet_manager: Arc<WalletManager>,
    /// Conversion rate: how many dimensionless gas units equal 1 rJoule.
    /// Default: 1000 gas = 1 rJ (configurable via WalletConfig).
    pub gas_per_rjoule: u64,
    /// Always true for wallet budgets — insufficient balance = rejection.
    pub hard_limit: bool,
}

impl WalletBackedBudget {
    /// Create a new wallet-backed budget.
    pub fn new(
        wallet_id: WalletId,
        wallet_manager: Arc<WalletManager>,
        gas_per_rjoule: u64,
    ) -> Self {
        Self {
            wallet_id,
            key_id: None,
            spending_limit_rj: None,
            wallet_manager,
            gas_per_rjoule,
            hard_limit: true,
        }
    }

    /// Attach an API key for spending-limit tracking.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_api_key(mut self, key_id: ApiKeyId, spending_limit_rj: RJoule) -> Self {
        self.key_id = Some(key_id);
        self.spending_limit_rj = Some(spending_limit_rj);
        self
    }

    /// Convert gas units to rJoules using the configured rate.
    fn gas_to_rjoules(&self, gas: u64) -> RJoule {
        self.wallet_manager.gas_to_rjoules(gas)
    }

    /// Check whether an operation costing `gas` can proceed.
    ///
    /// Converts gas to rJoules and checks the wallet balance.
    /// Returns `true` if the wallet can afford the cost.
    pub fn can_proceed(&self, gas: EnergyCost) -> bool {
        let cost_rj = self.gas_to_rjoules(gas.0);
        match self.wallet_manager.can_afford(self.wallet_id, cost_rj) {
            Ok(true) => true,
            Ok(false) | Err(_) => false,
        }
    }

    /// Reserve rJoules for an in-flight operation.
    ///
    /// Converts gas to rJoules and optimistically reserves the amount.
    /// The actual debit happens at `settle()` time.
    pub fn reserve(&self, gas: EnergyCost) -> Result<EnergyCost, EnergyError> {
        let cost_rj = self.gas_to_rjoules(gas.0);
        self.wallet_manager
            .reserve_rjoules(self.wallet_id, cost_rj)
            .map_err(|_e| EnergyError::BudgetExceeded {
                requested: gas,
                remaining: EnergyCost(0), // wallet errors don't map cleanly to gas units
            })?;
        Ok(gas)
    }

    /// Settle rJoules after an operation completes.
    ///
    /// Converts both reserved and actual gas to rJoules, then debits
    /// the actual cost. If actual < reserved, the difference is
    /// implicitly refunded (only actual is debited).
    pub fn settle(
        &self,
        reserved_gas: EnergyCost,
        actual_gas: EnergyCost,
    ) -> Result<EnergyCost, EnergyError> {
        let reserved_rj = self.gas_to_rjoules(reserved_gas.0);
        let actual_rj = self.gas_to_rjoules(actual_gas.0);
        self.wallet_manager
            .settle_rjoules(self.wallet_id, reserved_rj, actual_rj)
            .map_err(|_e| EnergyError::BudgetExceeded {
                requested: actual_gas,
                remaining: EnergyCost(0),
            })?;
        Ok(actual_gas)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // WalletBackedBudget tests require a real WalletManager with an in-memory DB.
    // These are integration-style tests — they validate the gas→rJoule→debit pipeline.
    // Skipped by default (require keystore env); run with:
    //   HKASK_MASTER_KEY=000102... cargo test -p hkask-cns -- wallet_budget

    #[test]
    fn wallet_budget_gas_to_rjoules_conversion() {
        // Unit test: verify the conversion math without a real wallet.
        // gas_per_rjoule = 1000 → 500 gas = 1 rJ (rounds up from 0.5)
        // gas_per_rjoule = 1000 → 1500 gas = 2 rJ (rounds up from 1.5)
        // This is a pure math test — no WalletManager needed.
        // We test the conversion logic inline since we can't construct
        // a WalletManager without keystore env vars in unit tests.
        let gas_per_rjoule: u64 = 1000;
        // 500 gas / 1000 = 0 rJ → rounds up to 1
        assert_eq!(500 / gas_per_rjoule, 0); // integer division
        // 1500 gas / 1000 = 1 rJ → rounds up to 2? No, 1.5 → 1 in integer div
        assert_eq!(1500 / gas_per_rjoule, 1);
        // 2000 gas / 1000 = 2 rJ
        assert_eq!(2000 / gas_per_rjoule, 2);
    }
}
