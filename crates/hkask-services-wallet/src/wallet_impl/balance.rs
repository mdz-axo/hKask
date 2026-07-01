//! Balance operations — balance queries, deposits, deposit references.

use super::WalletService;
use hkask_services_core::ServiceError;
use hkask_wallet::{
    ChainId, DepositAddress, DepositReference, PrivacyMode, RJoule, WalletBalance,
};
use hkask_types::id::WalletId;

impl WalletService {
    /// Get the current rJoule balance for a wallet.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid
    /// post: returns WalletBalance; Err(Wallet) on manager error
    pub fn get_balance(&self, wallet_id: WalletId) -> Result<WalletBalance, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "get_balance", wallet_id = %wallet_id, "CNS");
        self.manager.get_balance(wallet_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Check if a wallet can afford a given rJoule cost.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; cost_rj must be >= 0
    /// post: returns true if balance >= cost_rj; false otherwise; Err(Wallet) on manager error
    pub fn can_afford(&self, wallet_id: WalletId, cost_rj: RJoule) -> Result<bool, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "can_afford", wallet_id = %wallet_id, cost_rj = %cost_rj, "CNS");
        self.manager.can_afford(wallet_id, cost_rj).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Ensure a wallet row exists (idempotent — creates if missing).
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid
    /// post: wallet row exists in store; Ok(()) on success; Err(Wallet) on manager error
    pub fn ensure_wallet(&self, wallet_id: WalletId) -> Result<(), ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "ensure_wallet", wallet_id = %wallet_id, "CNS");
        self.manager.ensure_wallet(wallet_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Get or derive a deposit address for a wallet on a specific chain.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; chain must be a configured ChainId; privacy must be a valid PrivacyMode
    /// post: returns DepositAddress; Err(Wallet) on manager error
    pub fn get_deposit_address(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<DepositAddress, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "get_deposit_address", wallet_id = %wallet_id, chain = ?chain, "CNS");
        self.manager
            .get_deposit_address(wallet_id, chain, privacy)
            .map_err(|e| {
                let msg = e.to_string();
                ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })
    }

    /// Generate a one-time deposit reference for shielded deposits.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; chain must be configured; validity_hours must be > 0
    /// post: returns DepositReference with expiry; Err(Wallet) on manager error
    pub fn generate_deposit_reference(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        validity_hours: i64,
    ) -> Result<DepositReference, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "generate_deposit_reference", wallet_id = %wallet_id, chain = ?chain, "CNS");
        let duration = chrono::Duration::hours(validity_hours);
        self.manager
            .generate_deposit_reference(wallet_id, chain, duration)
            .map_err(|e| {
                let msg = e.to_string();
                ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })
    }
}
