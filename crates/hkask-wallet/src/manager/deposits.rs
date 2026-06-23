//! Deposit monitoring — background polling for transparent and shielded deposits.

use super::*;

impl WalletManager {
    pub async fn start_deposit_monitor(&self, interval_secs: u64) -> Result<(), WalletError> {
        loop {
            self.poll_deposits_once().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
        }
    }

    pub(crate) async fn poll_deposits_once(&self) {
        let wallet_ids = match self.store.list_wallet_ids() {
            Ok(ids) => ids,
            Err(_) => return,
        };
        for wallet_id in &wallet_ids {
            for chain_id in &self.config.enabled_chains {
                if let Some(port) = self.chains.get(chain_id) {
                    let addresses: Vec<String> = match self.store.get_deposit_addresses(*wallet_id)
                    {
                        Ok(addrs) => addrs
                            .iter()
                            .filter(|a| {
                                a.chain == *chain_id && a.privacy_mode == PrivacyMode::Transparent
                            })
                            .map(|a| a.address.clone())
                            .collect(),
                        Err(_) => continue,
                    };
                    if !addresses.is_empty() {
                        let actor = Self::default_actor();
                        match port.monitor_deposits(&actor, &addresses).await {
                            Ok(events) => {
                                for event in events {
                                    let _ = self.process_deposit(event).await;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(target: "hkask.wallet", error = %e, chain = %chain_id, "Deposit monitor error");
                            }
                        }
                    }
                }
            }
        }
        if let Some(ref privacy_port) = self.privacy {
            let actor = Self::default_actor();
            match privacy_port.monitor_shielded_transfers(&actor).await {
                Ok(transfers) => {
                    for transfer in transfers {
                        let _ = self.process_shielded_deposit(transfer).await;
                    }
                }
                Err(e) => {
                    tracing::warn!(target: "hkask.wallet", error = %e, "Privacy monitor error");
                }
            }
        }
    }

    async fn process_deposit(&self, event: DepositEvent) -> Result<(), WalletError> {
        if self.store.transaction_exists_by_hash(&event.tx_hash.0)? {
            tracing::debug!(
                target: "hkask.wallet",
                tx_hash = %event.tx_hash.0,
                "Deposit already processed — skipping"
            );
            return Ok(());
        }

        let wallet_id = self
            .store
            .resolve_wallet_for_address(&event.to_address)?
            .unwrap_or_else(|| {
                tracing::warn!(
                    target: "hkask.wallet",
                    to_address = %event.to_address,
                    "Deposit to unknown address — crediting default wallet"
                );
                WalletId::default()
            });

        self.emit_span(
            CnsSpan::WalletDeposit,
            "detected",
            Phase::Sense,
            serde_json::json!({
                "chain": "hedera",
                "amount_usdc_micro": event.amount_usdc_micro,
                "tx_hash": event.tx_hash.0,
                "privacy": "transparent",
            }),
        );

        let rj_amount = self.usdc_to_rjoules(event.amount_usdc_micro);
        self.store.credit_rjoules(wallet_id, rj_amount)?;
        let balance = self
            .store
            .get_balance(wallet_id)?
            .expect("balance exists for active wallet");
        self.store.record_transaction(&WalletTransaction {
            id: 0,
            wallet_id,
            tx_type: TransactionType::Deposit {
                chain: event.tx_hash.0.parse().unwrap_or(ChainId::Hinkal),
                privacy: PrivacyMode::Transparent,
                tx_hash: event.tx_hash.0.clone(),
                amount_usdc_micro: event.amount_usdc_micro,
            },
            rjoules_delta: rj_amount.as_u64() as i64,
            balance_after: balance.rjoules,
            timestamp: Utc::now(),
        })?;

        self.emit_span(
            CnsSpan::WalletBalance,
            "credited",
            Phase::Act,
            serde_json::json!({
                "wallet_id": wallet_id.to_string(),
                "rjoules_credited": rj_amount.as_u64(),
                "balance_after": balance.rjoules,
            }),
        );

        self.emit_span(
            CnsSpan::WalletDeposit,
            "deposit_credited",
            Phase::Act,
            serde_json::json!({
                "wallet_id": wallet_id.to_string(),
                "amount_rj": rj_amount.as_u64(),
                "amount_usdc_micro": event.amount_usdc_micro,
                "tx_hash": event.tx_hash.0,
                "chain": "hedera",
                "balance_after_rj": balance.rjoules,
            }),
        );

        Ok(())
    }

    async fn process_shielded_deposit(
        &self,
        transfer: ShieldedTransfer,
    ) -> Result<(), WalletError> {
        if self
            .store
            .transaction_exists_by_hash(&transfer.commitment)?
        {
            tracing::debug!(
                target: "hkask.wallet",
                commitment = %transfer.commitment,
                "Shielded deposit already processed — skipping"
            );
            return Ok(());
        }

        let memo = match transfer.memo {
            Some(ref m) => m.clone(),
            None => {
                tracing::warn!(target: "hkask.wallet", "Shielded transfer without deposit reference memo — cannot attribute");
                return Ok(());
            }
        };
        let wallet_id = match self.store.consume_deposit_reference(&memo)? {
            Some(id) => id,
            None => {
                tracing::warn!(target: "hkask.wallet", reference = %memo, "Deposit reference not found or already spent");
                return Ok(());
            }
        };

        self.emit_span(
            CnsSpan::WalletDepositShielded,
            "detected",
            Phase::Sense,
            serde_json::json!({
                "amount_usdc_micro": transfer.amount_usdc_micro,
                "commitment": transfer.commitment,
                "privacy": "shielded",
            }),
        );

        let rj_amount = self.usdc_to_rjoules(transfer.amount_usdc_micro);
        self.store.credit_rjoules(wallet_id, rj_amount)?;
        let balance = self
            .store
            .get_balance(wallet_id)?
            .expect("balance exists for active wallet");
        let commitment = transfer.commitment.clone();
        self.store.record_transaction(&WalletTransaction {
            id: 0,
            wallet_id,
            tx_type: TransactionType::Deposit {
                chain: transfer.chain,
                privacy: PrivacyMode::Shielded,
                tx_hash: transfer.commitment,
                amount_usdc_micro: transfer.amount_usdc_micro,
            },
            rjoules_delta: rj_amount.as_u64() as i64,
            balance_after: balance.rjoules,
            timestamp: Utc::now(),
        })?;

        self.emit_span(
            CnsSpan::WalletBalance,
            "credited",
            Phase::Act,
            serde_json::json!({
                "wallet_id": wallet_id.to_string(),
                "rjoules_credited": rj_amount.as_u64(),
                "balance_after": balance.rjoules,
            }),
        );

        self.emit_span(
            CnsSpan::WalletDeposit,
            "deposit_credited",
            Phase::Act,
            serde_json::json!({
                "wallet_id": wallet_id.to_string(),
                "amount_rj": rj_amount.as_u64(),
                "amount_usdc_micro": transfer.amount_usdc_micro,
                "commitment": commitment,
                "privacy": "shielded",
                "balance_after_rj": balance.rjoules,
            }),
        );

        Ok(())
    }
}
