//! Gas conversion and CNS integration — gas↔rJoule conversion, budget registration, encumbrance.

use std::sync::Arc;

use super::WalletService;
use hkask_services_core::ServiceError;
use hkask_types::id::{ApiKeyId, WalletId};
use hkask_wallet::RJoule;

impl WalletService {
    /// Convert gas units to rJoules.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  gas must be >= 0
    /// post: returns RJoule equivalent using manager's conversion rate
    pub fn gas_to_rjoules(&self, gas: u64) -> RJoule {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "gas_to_rjoules", gas = gas, "CNS");
        self.manager.gas_to_rjoules(gas)
    }

    /// Convert rJoules to gas units.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  rj must be >= 0
    /// post: returns u64 gas equivalent using manager's conversion rate
    pub fn rjoules_to_gas(&self, rj: RJoule) -> u64 {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "rjoules_to_gas", rj = %rj, "CNS");
        self.manager.rjoules_to_gas(rj)
    }

    // ── CNS Integration ─────────────────────────────────────────────────────

    /// Register a wallet-backed energy budget for an agent in the CNS.
    ///
    /// The agent's tool invocations will debit rJoules from the wallet
    /// instead of consuming from the dimensionless gas pool.
    /// The gas→rJoule conversion rate is taken from the WalletManager's config.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  cybernetics must be attached via with_cybernetics(); agent must be a valid WebID; wallet_id must be valid
    /// post: wallet-backed budget is registered in CNS for the agent; Err(Wallet) if cybernetics not attached
    pub async fn register_wallet_budget(
        &self,
        agent: hkask_types::WebID,
        wallet_id: WalletId,
    ) -> Result<(), ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "register_wallet_budget", agent = %agent, wallet_id = %wallet_id, "CNS");
        let loop_ = self
            .cybernetics
            .as_ref()
            .ok_or_else(|| ServiceError::Wallet {
                source: None,
                message: "CyberneticsLoop not attached to WalletService — call with_cybernetics() during construction".into(),
            })?;
        let budget = hkask_cns::WalletBackedBudget::new(wallet_id, Arc::clone(&self.manager));
        loop_
            .read()
            .await
            .register_wallet_budget(agent, budget)
            .await;
        Ok(())
    }

    /// Register a wallet-backed energy budget with an API key for encumbrance tracking.
    ///
    /// Unlike `register_wallet_budget`, this attaches the API key so that
    /// gas consumption is debited from the key's encumbrance (not raw wallet
    /// balance). The spending limit is also tracked per-key.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  cybernetics must be attached; agent must be valid; wallet_id and key_id must be valid; spending_limit_rj must be >= 0
    /// post: wallet-backed budget with API key tracking is registered in CNS; Err(Wallet) if cybernetics not attached
    pub async fn register_wallet_budget_for_key(
        &self,
        agent: hkask_types::WebID,
        wallet_id: WalletId,
        key_id: ApiKeyId,
        spending_limit_rj: RJoule,
    ) -> Result<(), ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "register_wallet_budget_for_key", agent = %agent, wallet_id = %wallet_id, key_id = %key_id, "CNS");
        let loop_ = self
            .cybernetics
            .as_ref()
            .ok_or_else(|| ServiceError::Wallet {
                source: None,
                message: "CyberneticsLoop not attached to WalletService — call with_cybernetics() during construction".into(),
            })?;
        let budget = hkask_cns::WalletBackedBudget::new(wallet_id, Arc::clone(&self.manager))
            .with_api_key(key_id, spending_limit_rj);
        loop_
            .read()
            .await
            .register_wallet_budget(agent, budget)
            .await;
        Ok(())
    }

    // ── Encumbrance ──────────────────────────────────────────────────────────

    /// Encumber rJoules from a wallet for an API key's allocation.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid with sufficient balance; key_id must be valid; amount must be > 0
    /// post: rJoules are encumbered from wallet to key; Err(Wallet) on manager error
    pub fn encumber_key(
        &self,
        wallet_id: WalletId,
        key_id: ApiKeyId,
        amount: RJoule,
    ) -> Result<(), ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "encumber_key", wallet_id = %wallet_id, key_id = %key_id, amount = %amount, "CNS");
        self.manager
            .encumber(wallet_id, key_id, amount)
            .map_err(|e| {
                let msg = e.to_string();
                ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })
    }

    /// Release an encumbrance, returning unspent rJoules to the wallet.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must have an active encumbrance
    /// post: encumbrance is released; unspent rJoules returned to wallet; Err(Wallet) on manager error
    pub fn release_encumbrance(&self, key_id: ApiKeyId) -> Result<(), ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "release_encumbrance", key_id = %key_id, "CNS");
        self.manager.release_encumbrance(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Atomically consume rJoules from an API key's encumbrance.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must have sufficient encumbered balance; gas_rj must be > 0
    /// post: rJoules are atomically debited from key's encumbrance; Err(Wallet) on manager error or insufficient balance
    pub fn consume_gas(&self, key_id: ApiKeyId, gas_rj: RJoule) -> Result<(), ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "consume_gas", key_id = %key_id, gas_rj = %gas_rj, "CNS");
        self.manager.consume(key_id, gas_rj).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Get the encumbrance for an API key.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must be valid
    /// post: returns Some(Encumbrance) if key has active encumbrance; None if none; Err(Wallet) on manager error
    pub fn get_encumbrance(
        &self,
        key_id: ApiKeyId,
    ) -> Result<Option<hkask_wallet::Encumbrance>, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "get_encumbrance", key_id = %key_id, "CNS");
        self.manager.get_encumbrance(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Emit a CNS algedonic alert for API key health events.
    ///
    /// Delegates to `WalletManager::emit_key_alert`. When the manager has
    /// no event sink configured, this is a no-op (graceful degradation).
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must be valid; exhausted and expired are boolean flags
    /// post: CNS alert emitted if event sink configured; no-op otherwise
    pub fn emit_key_alert(&self, key_id: ApiKeyId, exhausted: bool, expired: bool) {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "emit_key_alert", key_id = %key_id, exhausted = exhausted, expired = expired, "CNS");
        self.manager.emit_key_alert(key_id, exhausted, expired);
    }
}
