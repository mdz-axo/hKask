//! Withdrawal and shielding.

use super::*;


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
        // 1. Verify chain/privacy availability before debiting.
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

        // 2. Debit rJoules (only after confirming the path is available).
        let balance = self.store.debit_rjoules(wallet_id, amount_rj)?;
        let amount_usdc_micro = self.rjoules_to_usdc(amount_rj);

        // CNS span: conversion
        self.emit_span_with_actor(
            actor,
            CnsSpan::WalletConversion,
            "converted",
            Phase::Act,
            serde_json::json!({
                "actor": actor.to_string(),
                "wallet_id": wallet_id.to_string(),
                "rjoules": amount_rj.as_u64(),
                "usdc_micro": amount_usdc_micro,
                "direction": "rj_to_usdc",
            }),
        );

        // 3. Build/sign/submit transaction (chain already verified).
        let tx_hash_result: Result<TxHash, WalletError> = async {
            match privacy {
                PrivacyMode::Transparent => {
                    let port = self.chains.get(&chain).expect("chain port verified above");
                    let tx_bytes = port.build_withdrawal_tx(to_address, amount_usdc_micro)?;

                    // CNS span: withdrawal built
                    self.emit_span_with_actor(
                        actor,
                        CnsSpan::WalletWithdrawal,
                        "built",
                        Phase::Act,
                        serde_json::json!({
                            "actor": actor.to_string(),
                            "chain": chain.to_string(),
                            "to_address": to_address,
                            "amount_usdc_micro": amount_usdc_micro,
                            "privacy": "transparent",
                        }),
                    );

                    let signature = signing::sign_withdrawal(chain, &tx_bytes)?;

                    // CNS span: withdrawal signed
                    self.emit_span_with_actor(
                        actor,
                        CnsSpan::WalletWithdrawal,
                        "signed",
                        Phase::Act,
                        serde_json::json!({
                            "actor": actor.to_string(),
                            "chain": chain.to_string(),
                        }),
                    );

                    // Combine tx_bytes + signature (chain-specific format)
                    let mut signed_tx = tx_bytes;
                    signed_tx.extend_from_slice(&signature);
                    let tx_hash = port.submit_signed_tx(actor, &signed_tx).await?;
                    Ok(tx_hash)
                }
                PrivacyMode::Shielded => {
                    let privacy_port = self.privacy.as_ref().expect("privacy port verified above");
                    let tx_bytes = privacy_port.build_unshield_tx(to_address, amount_usdc_micro)?;

                    if chain == ChainId::Hinkal {
                        // Hinkal submit path signs the protocol withdraw message internally.
                        // Avoid appending a redundant signature over the serialized request payload.
                        let tx_hash = privacy_port.submit_signed_tx(actor, &tx_bytes).await?;
                        Ok(tx_hash)
                    } else {
                        let signature = signing::sign_withdrawal(chain, &tx_bytes)?;
                        let mut signed_tx = tx_bytes;
                        signed_tx.extend_from_slice(&signature);
                        let tx_hash = privacy_port.submit_signed_tx(actor, &signed_tx).await?;
                        Ok(tx_hash)
                    }
                }
            }
        }
        .await;

        let tx_hash = match tx_hash_result {
            Ok(tx_hash) => {
                // CNS span: withdrawal submitted
                self.emit_span_with_actor(
                    actor,
                    CnsSpan::WalletWithdrawal,
                    "submitted",
                    Phase::Act,
                    serde_json::json!({
                        "actor": actor.to_string(),
                        "chain": chain.to_string(),
                        "tx_hash": tx_hash.0,
                    }),
                );
                tx_hash
            }
            Err(err) => {
                // Compensating action: refund debited rJoules when submit path fails.
                if let Err(refund_err) = self.store.credit_rjoules(wallet_id, amount_rj) {
                    self.emit_chain_error_for_actor(
                        actor,
                        chain,
                        "withdraw_refund_failed",
                        &format!("original_error={err}; refund_error={refund_err}"),
                    );
                    return Err(WalletError::Infra(
                        hkask_types::InfrastructureError::Database(format!(
                            "withdraw failed and refund failed (wallet state may be inconsistent): original={err}; refund={refund_err}"
                        )),
                    ));
                }
                return Err(err);
            }
        };

        // 4. Record transaction
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
            balance_after: balance.rjoules,
            timestamp: Utc::now(),
        })?;

        Ok(tx_hash)
    }

    // ── Shield ───────────────────────────────────────────────────────────────

    /// Shield transparently-held USDC into the Hinkal privacy pool.
    ///
    /// This moves assets from the transparent treasury into the shielded pool
    /// without affecting rJoule balances. The assets were already credited as
    /// rJoules when the transparent deposit was detected; this is a pure asset
    /// layer transition for privacy.
    ///
    /// # Flow
    /// 1. Verify privacy port availability
    /// 2. Build unsigned shield transaction
    /// 3. Sign (or pass raw for Hinkal which signs internally)
    /// 4. Submit via privacy port
    /// 5. Record transaction in ledger (zero rJoule delta)
    pub async fn shield_assets(
        &self,
        wallet_id: WalletId,
        amount_usdc_micro: u64,
        chain: ChainId,
    ) -> Result<TxHash, WalletError> {
        // 1. Verify privacy port availability.
        let privacy_port = self
            .privacy
            .as_ref()
            .ok_or(WalletError::PrivacyUnavailable { chain })?;
        if !privacy_port.available_for_chain(chain) {
            return Err(WalletError::PrivacyUnavailable { chain });
        }

        // 2. Build unsigned shield transaction.
        let tx_bytes = privacy_port.build_shield_tx(amount_usdc_micro, chain)?;

        // CNS span: shield built
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

        // 3. Sign and submit.
        let actor = Self::default_actor();
        let tx_hash = if chain == ChainId::Hinkal {
            // Hinkal signs the protocol message internally via sign_message.
            privacy_port.submit_signed_tx(&actor, &tx_bytes).await?
        } else {
            let signature = signing::sign_withdrawal(chain, &tx_bytes)?;
            let mut signed_tx = tx_bytes;
            signed_tx.extend_from_slice(&signature);
            privacy_port.submit_signed_tx(&actor, &signed_tx).await?
        };

        // CNS span: shield submitted
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

        // 4. Record transaction (zero rJoule delta — pure asset layer transition).
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

    // ── Deposit address ──────────────────────────────────────────────────────

    /// Get or derive a deposit address for a wallet on a specific chain.
    pub fn get_deposit_address(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<DepositAddress, WalletError> {
        let port = self
            .chains
            .get(&chain)
            .ok_or(WalletError::ChainNotEnabled { chain })?;
        // Use derivation index 0 for the primary address
        let address = port.derive_deposit_address(0)?;
        self.store
            .store_deposit_address(wallet_id, chain, &address, 0, privacy)?;

        // CNS span: deposit address derived
        self.emit_span(
            CnsSpan::WalletDeposit,
            "derived",
            Phase::Act,
            serde_json::json!({
                "wallet_id": wallet_id.to_string(),
                "chain": chain.to_string(),
                "privacy": privacy.to_string(),
            }),
        );

        Ok(DepositAddress {
            address,
            chain,
            privacy_mode: privacy,
        })
    }

    // ── Gas ↔ rJoule conversion ──────────────────────────────────────────────

    /// Convert gas units to rJoules.
    ///
    /// REQ: P9-wallet-mgr-gas-to-rjoules
    /// pre:  gas is a non-negative integer
    /// post: returns RJoule equivalent using the current gas_per_rjoule rate
    pub fn gas_to_rjoules(&self, gas: u64) -> RJoule {
        // Integer division: gas / gas_per_rjoule
        // Minimum 1 rJ if gas > 0 (sub-rJoule operations round up to 1 rJ)
        if gas == 0 {
            RJoule::ZERO
        } else {
            let rate = self.gas_per_rjoule.load(Ordering::Relaxed);
            let rj = gas / rate;
            RJoule::new(if rj == 0 { 1 } else { rj })
        }
    }

    /// Convert rJoules to gas units.
    ///
    /// REQ: P9-wallet-mgr-rjoules-to-gas
    /// pre:  rj is a non-negative RJoule
    /// post: returns gas equivalent using the current gas_per_rjoule rate
    pub fn rjoules_to_gas(&self, rj: RJoule) -> u64 {
        rj.as_u64() * self.gas_per_rjoule.load(Ordering::Relaxed)
    }

    /// Current gas→rJoule conversion rate.
    ///
    /// REQ: P9-wallet-mgr-gas-per-rjoule
    /// post: returns the manager's current gas_per_rjoule rate
    pub fn gas_per_rjoule(&self) -> u64 {
        self.gas_per_rjoule.load(Ordering::Relaxed)
    }

    /// Update the gas→rJoule conversion rate at runtime.
    ///
    /// REQ: GAS-CALIB-005 — runtime calibration of wallet gas conversion rate
    /// pre:  rate > 0
    /// post: subsequent gas_to_rjoules/rjoules_to_gas use the new rate
    pub fn set_gas_per_rjoule(&self, rate: u64) {
        let rate = rate.max(1);
        self.gas_per_rjoule.store(rate, Ordering::Relaxed);
    }

    /// Estimate network withdrawal fee in rJoules/native units/USDC using configured PriceFeed.
    ///
    /// REQ: P9-wallet-mgr-fee-estimate
    /// \[P9\] Motivating: Homeostatic Self-Regulation — fee estimate enables cost-aware withdrawal
    /// \[P8\] Constraining: Semantic Grounding — derived from live/native USD rate
    /// pre:  chain is a valid ChainId
    /// post: returns fee estimate derived from live/native USD rate when available
    /// post: returns Err if configured price feed cannot provide a rate
    pub async fn estimate_withdrawal_fee(
        &self,
        chain: ChainId,
    ) -> Result<WithdrawalFee, WalletError> {
        let rate = self
            .price_feed
            .get_rate(chain)
            .await
            .map_err(|e| {
                WalletError::Infra(hkask_types::InfrastructureError::Database(
                    e.to_string(),
                ))
            })?;
        Ok(estimate_withdrawal_fee(chain, &rate, self.config.rj_per_usdc))
    }
}
