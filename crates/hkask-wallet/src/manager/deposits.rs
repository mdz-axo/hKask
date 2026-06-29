//! Deposit monitoring — background polling for transparent and shielded deposits.

use super::*;

fn repair_max_derivation_index() -> u64 {
    const DEFAULT_MAX_INDEX: u64 = 5;
    const MAX_ALLOWED_INDEX: u64 = 100;
    std::env::var("HKASK_DEPOSIT_REPAIR_MAX_INDEX")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.min(MAX_ALLOWED_INDEX))
        .unwrap_or(DEFAULT_MAX_INDEX)
}

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
                                    let tx_hash = event.tx_hash.0.clone();
                                    if let Err(err) = self.process_deposit(*chain_id, event).await {
                                        tracing::warn!(
                                            target: "hkask.wallet",
                                            error = %err,
                                            tx_hash = %tx_hash,
                                            "Deposit processing failed"
                                        );
                                        if let Ok(slot) = self.self_heal_hook.lock()
                                            && let Some(ref healer) = *slot
                                        {
                                            healer.heal("wallet.deposit.process", &err.to_string());
                                        }
                                    }
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
    }

    fn attempt_deposit_address_repair(
        &self,
        chain: ChainId,
        privacy_mode: PrivacyMode,
        address: &str,
    ) -> Result<Option<WalletId>, WalletError> {
        // General self-healing pattern:
        // 1) Keep repairs deterministic and idempotent.
        // 2) Never guess across multiple owners.
        // 3) Emit explicit CNS self-heal spans for attempt/success/failure.
        // 4) If repair can't be proven safe, return None and let Curator escalate.
        // 5) Centralize repeated patterns in a service-layer SelfHealer when
        //    cross-domain coordination or backoff is needed.
        let repair_max_index = repair_max_derivation_index();
        let wallet_ids = self.store.list_wallet_ids()?;
        if wallet_ids.len() != 1 {
            return Ok(None);
        }
        let wallet_id = wallet_ids[0];
        // Repair is conservative: only when a single wallet exists, and we
        // can prove the address matches a bounded derivation index.
        let port = match self.chains.get(&chain) {
            Some(port) => port,
            None => return Ok(None),
        };
        for index in 0..=repair_max_index {
            if let Ok(derived) = port.derive_deposit_address(index)
                && derived == address
            {
                self.store
                    .store_deposit_address(wallet_id, address, index, chain, privacy_mode)?;
                return Ok(Some(wallet_id));
            }
        }
        Ok(None)
    }

    pub fn repair_deposit_address_mapping(&self, address: &str) -> Result<bool, WalletError> {
        for chain in &self.config.enabled_chains {
            if let Ok(Some(_wallet_id)) =
                self.attempt_deposit_address_repair(*chain, PrivacyMode::Transparent, address)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn process_deposit(
        &self,
        chain: ChainId,
        event: DepositEvent,
    ) -> Result<(), WalletError> {
        if self.store.transaction_exists_by_hash(&event.tx_hash.0)? {
            tracing::debug!(
                target: "hkask.wallet",
                tx_hash = %event.tx_hash.0,
                "Deposit already processed — skipping"
            );
            return Ok(());
        }

        let wallet_id = match self.store.resolve_wallet_for_address(
            &event.to_address,
            chain,
            PrivacyMode::Transparent,
        )? {
            Some(wallet_id) => wallet_id,
            None => {
                tracing::warn!(
                    target: "hkask.wallet",
                    to_address = %event.to_address,
                    "Deposit address unresolvable — attempting auto-repair"
                );
                self.emit_span(
                    CnsSpan::WalletDeposit,
                    "unresolvable_address",
                    CyclePhase::Sense,
                    serde_json::json!({
                        "chain": chain.to_string(),
                        "amount_usdc_micro": event.amount_usdc_micro,
                        "tx_hash": event.tx_hash.0,
                        "privacy": PrivacyMode::Transparent.to_string(),
                        "to_address": event.to_address,
                    }),
                );
                self.emit_span(
                    CnsSpan::SelfHeal,
                    "wallet_deposit_address_unresolvable",
                    CyclePhase::Sense,
                    serde_json::json!({
                        "chain": chain.to_string(),
                        "tx_hash": event.tx_hash.0,
                        "to_address": event.to_address,
                        "action": "rebuild_wallet_address_index",
                        "note": "deposit address not resolvable from wallet store; attempting minimal auto-repair",
                    }),
                );

                match self.attempt_deposit_address_repair(
                    chain,
                    PrivacyMode::Transparent,
                    &event.to_address,
                ) {
                    Ok(Some(repaired_wallet_id)) => {
                        self.emit_span(
                            CnsSpan::SelfHeal,
                            "wallet_deposit_address_repaired",
                            CyclePhase::Act,
                            serde_json::json!({
                                "chain": chain.to_string(),
                                "to_address": event.to_address,
                                "wallet_id": repaired_wallet_id.to_string(),
                            }),
                        );
                        repaired_wallet_id
                    }
                    Ok(None) => {
                        self.emit_span(
                            CnsSpan::SelfHeal,
                            "wallet_deposit_address_repair_deferred",
                            CyclePhase::Sense,
                            serde_json::json!({
                                "chain": chain.to_string(),
                                "to_address": event.to_address,
                                "reason": "multi_wallet_or_no_wallet",
                            }),
                        );
                        return Err(WalletError::DepositAddressUnresolvable {
                            address: event.to_address,
                        });
                    }
                    Err(err) => {
                        self.emit_span(
                            CnsSpan::SelfHeal,
                            "wallet_deposit_address_repair_failed",
                            CyclePhase::Sense,
                            serde_json::json!({
                                "chain": chain.to_string(),
                                "to_address": event.to_address,
                                "error": err.to_string(),
                            }),
                        );
                        return Err(WalletError::Infra(
                            hkask_types::InfrastructureError::Database(err.to_string()),
                        ));
                    }
                }
            }
        };

        self.emit_span(
            CnsSpan::WalletDeposit,
            "detected",
            CyclePhase::Sense,
            serde_json::json!({
                "chain": chain.to_string(),
                "amount_usdc_micro": event.amount_usdc_micro,
                "tx_hash": event.tx_hash.0,
                "privacy": PrivacyMode::Transparent.to_string(),
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
                chain,
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
            CyclePhase::Act,
            serde_json::json!({
                "wallet_id": wallet_id.to_string(),
                "rjoules_credited": rj_amount.as_u64(),
                "balance_after": balance.rjoules,
            }),
        );

        self.emit_span(
            CnsSpan::WalletDeposit,
            "deposit_credited",
            CyclePhase::Act,
            serde_json::json!({
                "wallet_id": wallet_id.to_string(),
                "amount_rj": rj_amount.as_u64(),
                "amount_usdc_micro": event.amount_usdc_micro,
                "tx_hash": event.tx_hash.0,
                "chain": chain.to_string(),
                "balance_after_rj": balance.rjoules,
            }),
        );

        Ok(())
    }
}
