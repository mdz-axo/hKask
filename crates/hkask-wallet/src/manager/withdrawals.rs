//! Withdrawal pipeline and shield operations.

use hkask_rsolidity as rs;
use super::*;
use crate::signing;

impl WalletManager {
    pub async fn withdraw(
        &self,
        actor: &WebID,
        wallet_id: WalletId,
        amount_rj: RJoule,
        to_address: &str,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<TxHash, WalletError> {
        self.verify_withdrawal_chain(chain, privacy)?;

        let balance = self.store.debit_rjoules(wallet_id, amount_rj)?;
        let amount_usdc_micro = self.rjoules_to_usdc(amount_rj);
        self.emit_conversion_span(actor, wallet_id, amount_rj, amount_usdc_micro);

        let tx_hash_result = self
            .build_and_submit_withdrawal(actor, to_address, amount_usdc_micro, chain, privacy)
            .await;

        let tx_hash = match tx_hash_result {
            Ok(tx_hash) => {
                self.emit_span_with_actor(actor, CnsSpan::WalletWithdrawal, "submitted", Phase::Act,
                    serde_json::json!({"actor": actor.to_string(), "chain": chain.to_string(), "tx_hash": tx_hash.0}));
                tx_hash
            }
            Err(err) => {
                if let Err(refund_err) = self.store.credit_rjoules(wallet_id, amount_rj) {
                    self.emit_chain_error_for_actor(
                        actor,
                        chain,
                        "withdraw_refund_failed",
                        &format!("original_error={err}; refund_error={refund_err}"),
                    );
                    return Err(WalletError::Infra(
                        hkask_types::InfrastructureError::Database(format!(
                            "withdraw failed and refund failed: original={err}; refund={refund_err}"
                        )),
                    ));
                }
                return Err(err);
            }
        };

        self.record_withdrawal_tx(
            wallet_id,
            &tx_hash,
            chain,
            privacy,
            amount_rj,
            amount_usdc_micro,
            balance.rjoules,
        )?;
        Ok(tx_hash)
    }

    fn verify_withdrawal_chain(
        &self,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<(), WalletError> {
        match privacy {
            PrivacyMode::Transparent => {
                self.chains
                    .get(&chain)
                    .ok_or(WalletError::ChainNotEnabled { chain })?;
            }
            PrivacyMode::Shielded => {
                let privacy_port = self
                    .privacy
                    .as_ref()
                    .ok_or(WalletError::PrivacyUnavailable { chain })?;
                if !privacy_port.available_for_chain(chain) {
                    return Err(WalletError::PrivacyUnavailable { chain });
                }
            }
        }
        Ok(())
    }

    fn emit_conversion_span(
        &self,
        actor: &WebID,
        wallet_id: WalletId,
        amount_rj: RJoule,
        amount_usdc_micro: u64,
    ) {
        self.emit_span_with_actor(actor, CnsSpan::WalletConversion, "converted", Phase::Act,
            serde_json::json!({"actor": actor.to_string(), "wallet_id": wallet_id.to_string(),
                "rjoules": amount_rj.as_u64(), "usdc_micro": amount_usdc_micro, "direction": "rj_to_usdc"}));
    }

    async fn build_and_submit_withdrawal(
        &self,
        actor: &WebID,
        to_address: &str,
        amount_usdc_micro: u64,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<TxHash, WalletError> {
        match privacy {
            PrivacyMode::Transparent => {
                let port = self.chains.get(&chain).expect("chain port verified above");
                let tx_bytes = port.build_withdrawal_tx(to_address, amount_usdc_micro)?;
                self.emit_span_with_actor(actor, CnsSpan::WalletWithdrawal, "built", Phase::Act,
                    serde_json::json!({"actor": actor.to_string(), "chain": chain.to_string(),
                        "to_address": to_address, "amount_usdc_micro": amount_usdc_micro, "privacy": "transparent"}));
                let signature = signing::sign_withdrawal(chain, &tx_bytes)?;
                self.emit_span_with_actor(
                    actor,
                    CnsSpan::WalletWithdrawal,
                    "signed",
                    Phase::Act,
                    serde_json::json!({"actor": actor.to_string(), "chain": chain.to_string()}),
                );
                let mut signed_tx = tx_bytes;
                signed_tx.extend_from_slice(&signature);
                port.submit_signed_tx(actor, &signed_tx).await
            }
            PrivacyMode::Shielded => {
                let privacy_port = self.privacy.as_ref().expect("privacy port verified above");
                let tx_bytes = privacy_port.build_unshield_tx(to_address, amount_usdc_micro)?;
                if chain == ChainId::Hinkal {
                    privacy_port.submit_signed_tx(actor, &tx_bytes).await
                } else {
                    let signature = signing::sign_withdrawal(chain, &tx_bytes)?;
                    let mut signed_tx = tx_bytes;
                    signed_tx.extend_from_slice(&signature);
                    privacy_port.submit_signed_tx(actor, &signed_tx).await
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn record_withdrawal_tx(
        &self,
        wallet_id: WalletId,
        tx_hash: &TxHash,
        chain: ChainId,
        privacy: PrivacyMode,
        amount_rj: RJoule,
        amount_usdc_micro: u64,
        balance_after: u64,
    ) -> Result<(), WalletError> {
        self.store.record_transaction(&WalletTransaction {
            id: 0,
            wallet_id,
            tx_type: TransactionType::Withdrawal {
                chain,
                privacy,
                tx_hash: tx_hash.0.clone(),
                amount_usdc_micro,
            },
            rjoules_delta: -(amount_rj.as_u64() as i64),
            balance_after,
            timestamp: Utc::now(),
        })
    }

    pub async fn shield_assets(
        &self,
        wallet_id: WalletId,
        amount_usdc_micro: u64,
        chain: ChainId,
    ) -> Result<TxHash, WalletError> {
        let privacy_port = self
            .privacy
            .as_ref()
            .ok_or(WalletError::PrivacyUnavailable { chain })?;
        if !privacy_port.available_for_chain(chain) {
            return Err(WalletError::PrivacyUnavailable { chain });
        }

        let tx_bytes = privacy_port.build_shield_tx(amount_usdc_micro, chain)?;

        self.emit_span(
            CnsSpan::WalletWithdrawal,
            "shield_built",
            Phase::Act,
            serde_json::json!({
                "chain": chain.to_string(),
                "amount_usdc_micro": amount_usdc_micro,
                "operation": "shield",
            }),
        );

        let actor = Self::default_actor();
        let tx_hash = if chain == ChainId::Hinkal {
            privacy_port.submit_signed_tx(&actor, &tx_bytes).await?
        } else {
            let signature = signing::sign_withdrawal(chain, &tx_bytes)?;
            let mut signed_tx = tx_bytes;
            signed_tx.extend_from_slice(&signature);
            privacy_port.submit_signed_tx(&actor, &signed_tx).await?
        };

        self.emit_span(
            CnsSpan::WalletWithdrawal,
            "shield_submitted",
            Phase::Act,
            serde_json::json!({
                "chain": chain.to_string(),
                "tx_hash": tx_hash.0,
                "operation": "shield",
            }),
        );

        self.store.record_transaction(&WalletTransaction {
            id: 0,
            wallet_id,
            tx_type: TransactionType::Shield {
                chain,
                tx_hash: tx_hash.0.clone(),
                amount_usdc_micro,
            },
            rjoules_delta: 0,
            balance_after: 0,
            timestamp: Utc::now(),
        })?;

        Ok(tx_hash)
    }
}
