//! Transaction operations — get_transactions, withdraw, estimate_withdrawal_fee.

use super::WalletService;
use hkask_services_core::ServiceError;
use hkask_types::DataCategory;
use hkask_types::WebID;
use hkask_types::id::WalletId;
use hkask_wallet::{
    ChainId, PrivacyMode, RJoule, TxHash, WalletError, WalletTransaction, WithdrawalFee,
};

impl WalletService {
    /// Get paginated transaction history for a wallet.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; limit must be > 0
    /// post: returns `Vec<WalletTransaction>`; empty Vec if no transactions; Err(Wallet) on manager error
    pub fn get_transactions(
        &self,
        wallet_id: WalletId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<WalletTransaction>, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "get_transactions", wallet_id = %wallet_id, limit = limit, offset = offset, "CNS");
        self.manager
            .get_transactions(wallet_id, limit, offset)
            .map_err(|e| {
                let msg = e.to_string();
                ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })
    }

    /// Withdraw rJoules as USDC to a user's primary wallet address.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  webid identifies the user requesting the withdrawal
    /// post: if consent_manager is Some and consent denied → Err(ConsentDenied)
    /// post: if consent_manager is None → proceeds without consent check
    pub async fn withdraw(
        &self,
        webid: &WebID,
        wallet_id: WalletId,
        amount_rj: RJoule,
        to_address: &str,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<TxHash, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "withdraw", webid = %webid, wallet_id = %wallet_id, amount_rj = %amount_rj, chain = ?chain, "CNS");
        if let Some(ref cm) = self.consent_manager {
            let category = DataCategory::Custom("wallet_withdrawal".into());
            let has_consent = cm.has_consent(&webid.to_string(), &category).map_err(|e| {
                ServiceError::ConsentDenied {
                    source: None,
                    message: format!(
                        "Consent check failed for {}: {e}. Denying wallet withdrawal by default",
                        webid
                    ),
                }
            })?;
            if !has_consent {
                return Err(ServiceError::ConsentDenied {
                    source: None,
                    message: format!(
                        "User {} has not granted consent for wallet withdrawal. \
                         Grant consent with: kask sovereignty grant {} wallet_withdrawal",
                        webid, webid
                    ),
                });
            }
        }

        self.manager
            .withdraw(webid, wallet_id, amount_rj, to_address, chain, privacy)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if matches!(e, WalletError::ChainError { .. }) {
                    self.manager
                        .emit_chain_error_for_actor(webid, chain, "withdraw", &msg);
                }
                ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })
    }

    /// Estimate network withdrawal fee for a chain using configured price feed.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  webid must be valid; chain must be configured
    /// post: returns WithdrawalFee estimate; Err(Wallet) on manager error
    pub async fn estimate_withdrawal_fee(
        &self,
        webid: &WebID,
        chain: ChainId,
    ) -> Result<WithdrawalFee, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "estimate_withdrawal_fee", webid = %webid, chain = ?chain, "CNS");
        self.manager
            .estimate_withdrawal_fee(webid, chain)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })
    }
}
