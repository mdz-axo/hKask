//! Withdrawal pipeline and shield operations.

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
        _privacy: PrivacyMode,
    ) -> Result<(), WalletError> {
        self.chains.get(&chain).ok_or(WalletError::ChainError {
            chain: ChainId::Hedera,
            message: "chain not enabled".into(),
        })?;
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
        _privacy: PrivacyMode,
    ) -> Result<TxHash, WalletError> {
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
                chain: ChainId::Hedera,
                privacy: PrivacyMode::Transparent,
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
}
