//! Wallet — Per-agent gas/rJoule balance store.
//!
//! Wallets are created by the Curator daemon on replicant registration.
//! Agents spend from wallets via WalletBackedBudget.
//! Wallets draw from Wells via the GasBudgetManager.

use crate::energy::{GasCost, GasError};
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Unique wallet identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WalletID(pub u64);

/// Current balance of an agent's wallet.
#[derive(Debug, Clone, Copy)]
pub struct WalletBalance {
    pub gas: u64,
    pub rjoule: u64,
}

/// Internal wallet state.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Wallet {
    wallet_id: WalletID,
    agent: WebID,
    gas_balance: u64,
    rjoule_balance: u64,
    created_at: String,
}

/// Manages agent wallets — creation, drawing, spending, balance queries.
pub struct WalletManager {
    wallets: Arc<RwLock<HashMap<WebID, Wallet>>>,
    next_id: Arc<RwLock<u64>>,
}

impl WalletManager {
    pub fn new() -> Self {
        Self {
            wallets: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Create a wallet for an agent. Returns the wallet ID.
    /// Called by Curator daemon on replicant registration.
    pub async fn create_wallet(
        &self,
        agent: WebID,
        initial_gas: GasCost,
        initial_rjoule: u64,
    ) -> Result<WalletID, GasError> {
        let mut wallets = self.wallets.write().await;
        if wallets.contains_key(&agent) {
            return Err(GasError::Persistence(format!(
                "Wallet already exists for agent {agent}"
            )));
        }
        let mut next = self.next_id.write().await;
        let id = WalletID(*next);
        *next += 1;
        wallets.insert(
            agent,
            Wallet {
                wallet_id: id,
                agent,
                gas_balance: initial_gas.0,
                rjoule_balance: initial_rjoule,
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        );
        Ok(id)
    }

    /// Deposit gas into an agent's wallet. Called after drawing from a Well.
    pub async fn deposit_gas(&self, agent: &WebID, amount: GasCost) -> Result<GasCost, GasError> {
        let mut wallets = self.wallets.write().await;
        let wallet = wallets
            .get_mut(agent)
            .ok_or_else(|| GasError::Persistence(format!("No wallet for agent {agent}")))?;
        wallet.gas_balance = wallet.gas_balance.saturating_add(amount.0);
        Ok(amount)
    }

    /// Spend gas from an agent's wallet.
    pub async fn spend(&self, agent: &WebID, amount: GasCost) -> Result<GasCost, GasError> {
        let mut wallets = self.wallets.write().await;
        let wallet = wallets
            .get_mut(agent)
            .ok_or_else(|| GasError::Persistence(format!("No wallet for agent {agent}")))?;
        let spent = amount.0.min(wallet.gas_balance);
        wallet.gas_balance = wallet.gas_balance.saturating_sub(spent);
        Ok(GasCost(spent))
    }

    /// Query an agent's wallet balance.
    pub async fn balance(&self, agent: &WebID) -> Result<WalletBalance, GasError> {
        let wallets = self.wallets.read().await;
        let wallet = wallets
            .get(agent)
            .ok_or_else(|| GasError::Persistence(format!("No wallet for agent {agent}")))?;
        Ok(WalletBalance {
            gas: wallet.gas_balance,
            rjoule: wallet.rjoule_balance,
        })
    }

    /// Check if a wallet has sufficient gas.
    pub async fn can_proceed(&self, agent: &WebID, amount: GasCost) -> bool {
        let wallets = self.wallets.read().await;
        wallets
            .get(agent)
            .map(|w| w.gas_balance >= amount.0)
            .unwrap_or(false)
    }

    /// Check if a wallet exists for the agent.
    pub async fn has_wallet(&self, agent: &WebID) -> bool {
        let wallets = self.wallets.read().await;
        wallets.contains_key(agent)
    }
}

impl Default for WalletManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_and_query_wallet() {
        let mgr = WalletManager::new();
        let agent = WebID::from_persona(b"test-agent");
        let id = mgr.create_wallet(agent, GasCost(1000), 500).await.unwrap();
        assert!(id.0 > 0);

        let balance = mgr.balance(&agent).await.unwrap();
        assert_eq!(balance.gas, 1000);
        assert_eq!(balance.rjoule, 500);
    }

    #[tokio::test]
    async fn spend_reduces_balance() {
        let mgr = WalletManager::new();
        let agent = WebID::from_persona(b"test-agent");
        mgr.create_wallet(agent, GasCost(1000), 0).await.unwrap();

        let spent = mgr.spend(&agent, GasCost(300)).await.unwrap();
        assert_eq!(spent.0, 300);

        let balance = mgr.balance(&agent).await.unwrap();
        assert_eq!(balance.gas, 700);
    }

    #[tokio::test]
    async fn spend_capped_at_balance() {
        let mgr = WalletManager::new();
        let agent = WebID::from_persona(b"test-agent");
        mgr.create_wallet(agent, GasCost(100), 0).await.unwrap();

        let spent = mgr.spend(&agent, GasCost(500)).await.unwrap();
        assert_eq!(spent.0, 100);
        assert!(mgr.can_proceed(&agent, GasCost(1)).await == false);
    }

    #[tokio::test]
    async fn deposit_adds_gas() {
        let mgr = WalletManager::new();
        let agent = WebID::from_persona(b"test-agent");
        mgr.create_wallet(agent, GasCost(100), 0).await.unwrap();
        mgr.deposit_gas(&agent, GasCost(200)).await.unwrap();

        let balance = mgr.balance(&agent).await.unwrap();
        assert_eq!(balance.gas, 300);
    }

    #[tokio::test]
    async fn duplicate_wallet_rejected() {
        let mgr = WalletManager::new();
        let agent = WebID::from_persona(b"test-agent");
        mgr.create_wallet(agent, GasCost(100), 0).await.unwrap();
        let result = mgr.create_wallet(agent, GasCost(200), 0).await;
        assert!(result.is_err());
    }
}
