//! API key management — creation, revocation, listing, lookup.

use super::WalletService;
use hkask_services_core::ServiceError;
use hkask_types::id::{ApiKeyId, WalletId};
use hkask_wallet::{ApiKeyCapability, ApiKeyMaterial, ChainId, PrivacyMode, RJoule};

impl WalletService {
    /// Create a new API key with the specified limits, scope, and purpose.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; spending_limit_rj must be >= 0; purpose must be non-empty
    /// post: returns ApiKeyMaterial with key secret; Err(Wallet) on issuer error
    #[allow(clippy::too_many_arguments)]
    pub fn create_key(
        &self,
        wallet_id: WalletId,
        spending_limit_rj: RJoule,
        expiry_days: Option<u32>,
        privacy_mode: PrivacyMode,
        preferred_chain: Option<ChainId>,
        scope: Vec<String>,
        purpose: String,
        rate_limit: Option<hkask_wallet::RateLimitConfig>,
    ) -> Result<ApiKeyMaterial, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "create_key", wallet_id = %wallet_id, purpose = %purpose, "CNS");
        self.issuer
            .create_key(
                wallet_id,
                spending_limit_rj,
                expiry_days,
                privacy_mode,
                preferred_chain,
                scope,
                purpose,
                rate_limit,
            )
            .map_err(|e| {
                let msg = e.to_string();
                ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })
    }

    /// Revoke an API key. Returns unspent rJoules to the wallet.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must be a valid, non-revoked key
    /// post: key is revoked; unspent rJoules returned to wallet; Err(Wallet) on issuer error
    pub fn revoke_key(&self, key_id: ApiKeyId) -> Result<(), ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "revoke_key", key_id = %key_id, "CNS");
        self.issuer.revoke_key(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// List active (non-revoked) API keys for a wallet.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid
    /// post: returns `Vec<ApiKeyCapability>` of active keys; empty Vec if none; Err(Wallet) on issuer error
    pub fn list_keys(&self, wallet_id: WalletId) -> Result<Vec<ApiKeyCapability>, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "list_keys", wallet_id = %wallet_id, "CNS");
        self.issuer.list_keys(wallet_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Get a single API key capability by key ID.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must be valid
    /// post: returns Some(ApiKeyCapability) if found; None if not found; Err(Wallet) on manager error
    pub fn get_api_key(&self, key_id: ApiKeyId) -> Result<Option<ApiKeyCapability>, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "get_api_key", key_id = %key_id, "CNS");
        self.manager.get_api_key(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }
}
