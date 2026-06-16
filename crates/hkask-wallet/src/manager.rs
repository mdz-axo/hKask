//! WalletManager — orchestrates chain ports, privacy layer, and rJoule accounting.
//!
//! # Deposit reference logic merged here per essentialist G1
//! The deposit reference scheme (generate, verify, consume) was originally a
//! separate 2-function module. Merged into WalletManager because it is tightly
//! coupled to wallet_seed and WalletStore — a separate module added no behavior
//! beyond what inline functions provide.

use chrono::{Duration, Utc};
use hkask_keystore::resolve_wallet_seed;
use hkask_storage::WalletStore;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
#[cfg(test)]
use hkask_types::wallet::EncumbranceStatus;
use hkask_types::wallet::{
    ApiKeyId, ChainId, DepositAddress, DepositReference, Encumbrance, PrivacyMode, RJoule,
    TransactionType, TxHash, WalletBalance, WalletConfig, WalletError, WalletId, WalletTransaction,
};
use std::collections::HashMap;
use std::sync::Arc;
use zeroize::Zeroizing;

use crate::chain::{ChainPort, DepositEvent};
use crate::price_feed::PriceFeed;
use crate::privacy::{PrivacyPort, ShieldedTransfer};
use crate::signing;

/// Orchestrates chain ports, privacy layer, and rJoule accounting.
///
/// # Ownership `[OUGHT-DECL]`
/// - Sole-owns `ChainPort` and `PrivacyPort` implementations
/// - Shares `Arc<WalletStore>` with CNS for algedonic monitoring
/// - Holds `wallet_seed` in `Zeroizing` for deposit reference generation
/// - Does NOT hold treasury keys (loaded per-operation in signing.rs)
///
/// REQ: WALLET-001
/// inv: wallet_seed is zeroized on drop (Zeroizing wrapper)
/// inv: chains map is non-empty after successful build
pub struct WalletManager {
    config: WalletConfig,
    store: Arc<WalletStore>,
    chains: HashMap<ChainId, Box<dyn ChainPort>>,
    privacy: Option<Box<dyn PrivacyPort>>,
    wallet_seed: Zeroizing<[u8; 32]>,
    /// Optional CNS event sink for span emission (Phase 5).
    /// When present, wallet operations emit cns.wallet.* spans.
    event_sink: Option<Arc<dyn NuEventSink>>,
    /// Price feed for native token USD rates (fee estimation).
    /// Resolved from user's `PriceFeedConfig` at build time.
    price_feed: Arc<dyn PriceFeed>,
}

impl WalletManager {
    /// Build a WalletManager from configuration, store, chain/privacy ports, and price feed.
    ///
    /// REQ: WALLET-001
    /// pre:  config is valid, store is initialized, chains is non-empty
    /// pre:  price_feed is a resolved PriceFeed implementation
    /// post: returns Ok(WalletManager) with resolved wallet_seed
    /// post: returns Err if wallet_seed resolution fails
    pub fn build(
        config: WalletConfig,
        store: Arc<WalletStore>,
        chains: HashMap<ChainId, Box<dyn ChainPort>>,
        privacy: Option<Box<dyn PrivacyPort>>,
        price_feed: Arc<dyn PriceFeed>,
    ) -> Result<Self, WalletError> {
        let seed_bytes = resolve_wallet_seed().map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(e.to_string()))
        })?;
        let mut seed_arr = [0u8; 32];
        seed_arr.copy_from_slice(&seed_bytes[..32]);
        Ok(WalletManager {
            config,
            store,
            chains,
            privacy,
            wallet_seed: Zeroizing::new(seed_arr),
            event_sink: None,
            price_feed,
        })
    }

    /// Attach a CNS event sink for span emission.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Replace the price feed (for testing or runtime reconfiguration).
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_price_feed(mut self, feed: Arc<dyn PriceFeed>) -> Self {
        self.price_feed = feed;
        self
    }

    /// Get a reference to the price feed.
    pub fn price_feed(&self) -> &Arc<dyn PriceFeed> {
        &self.price_feed
    }

    /// Emit a CNS span if an event sink is configured (canonical namespaces only).
    fn emit_span(&self, span: CnsSpan, verb: &str, phase: Phase, obs: serde_json::Value) {
        if let Some(ref sink) = self.event_sink {
            let span_obj = Span::new(SpanNamespace::from(span), verb);
            let event = NuEvent::new(hkask_types::WebID::new(), span_obj, phase, obs, 0);
            if let Err(e) = sink.persist(&event) {
                tracing::warn!(target: "hkask.wallet", namespace = %span, verb = verb, error = %e, "Failed to persist CNS span");
            }
        }
    }

    /// Emit a CNS algedonic alert for API key health events.
    ///
    /// REQ: WALLET-006, MUST-6 (algedonic feedback closure)
    /// pre:  key_id is a valid ApiKeyId
    /// post: if key is expired → emits cns.wallet.key_expired span (Sense phase)
    /// post: if key is exhausted → emits cns.wallet.key_exhausted span (Sense phase)
    /// post: if event_sink is None → no-op (graceful degradation)
    ///
    /// Called by `WalletBackedBudget::can_proceed` when key health checks fail,
    /// providing CNS algedonic visibility into key lifecycle events.
    pub fn emit_key_alert(&self, key_id: ApiKeyId, exhausted: bool, expired: bool) {
        if expired {
            self.emit_span(
                CnsSpan::WalletKeyExpired,
                "expired",
                Phase::Sense,
                serde_json::json!({
                    "key_id": key_id.to_string(),
                }),
            );
        }
        if exhausted {
            self.emit_span(
                CnsSpan::WalletKeyExhausted,
                "exhausted",
                Phase::Sense,
                serde_json::json!({
                    "key_id": key_id.to_string(),
                }),
            );
        }
    }

    /// Emit a CNS span for chain-level errors (RPC failure, tx rejection, etc.).
    ///
    /// REQ: P9 — feedback loop closure for cns.wallet.chain_error
    /// pre:  chain is a valid ChainId
    /// post: emits cns.wallet.chain_error span with error details (Sense phase)
    /// post: if event_sink is None → no-op (graceful degradation)
    pub fn emit_chain_error(&self, chain: ChainId, operation: &str, error_msg: &str) {
        self.emit_span(
            CnsSpan::WalletChainError,
            "error",
            Phase::Sense,
            serde_json::json!({
                "chain": chain.to_string(),
                "operation": operation,
                "error": error_msg,
            }),
        );
    }

    // ── Balance ──────────────────────────────────────────────────────────────

    /// Get the current rJoule balance for a wallet.
    ///
    /// REQ: WALLET-002
    /// pre:  wallet_id is a valid WalletId
    /// post: returns Ok(balance) with rjoules, gas_equivalent, usdc_equivalent_micro
    /// post: gas_equivalent == rjoules * config.gas_per_rjoule
    /// post: balance.rjoules >= 0 (balances are never negative)
    pub fn get_balance(&self, wallet_id: WalletId) -> Result<WalletBalance, WalletError> {
        let mut balance = self.store.get_balance(wallet_id)?.unwrap_or(WalletBalance {
            wallet_id,
            rjoules: 0,
            usdc_equivalent_micro: 0,
            gas_equivalent: 0,
        });
        balance.gas_equivalent = balance.rjoules * self.config.gas_per_rjoule;
        balance.usdc_equivalent_micro =
            (balance.rjoules as u128 * 1_000_000 / self.config.rj_per_usdc as u128) as u64;
        Ok(balance)
    }

    /// Get an API key's capability metadata for CNS health monitoring.
    /// Returns `None` if the key doesn't exist or has been revoked.
    ///
    /// REQ: WALLET-003
    /// pre:  key_id is a valid ApiKeyId
    /// post: returns Ok(Some(capability)) if key exists and is active
    /// post: returns Ok(None) if key doesn't exist or is revoked
    pub fn get_api_key(
        &self,
        key_id: hkask_types::wallet::ApiKeyId,
    ) -> Result<Option<hkask_types::wallet::ApiKeyCapability>, WalletError> {
        self.store.get_api_key(key_id)
    }

    /// Ensure a wallet row exists (idempotent — creates if missing).
    pub fn ensure_wallet(&self, wallet_id: WalletId) -> Result<(), WalletError> {
        self.store.ensure_wallet(wallet_id)
    }

    /// Get paginated transaction history for a wallet.
    pub fn get_transactions(
        &self,
        wallet_id: WalletId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<WalletTransaction>, WalletError> {
        self.store.get_transactions(wallet_id, limit, offset)
    }

    // ── Deposit monitoring ───────────────────────────────────────────────────

    /// Start the background deposit monitoring loop.
    /// Polls all enabled chains and the privacy layer at a configurable interval.
    pub async fn start_deposit_monitor(&self, interval_secs: u64) -> Result<(), WalletError> {
        loop {
            self.poll_deposits_once().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
        }
    }

    /// Run a single deposit poll cycle (test-accessible).
    pub(crate) async fn poll_deposits_once(&self) {
        // Iterate all wallets (multi-wallet support)
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
                        match port.monitor_deposits(&addresses).await {
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
        // Check privacy layer
        if let Some(ref privacy_port) = self.privacy {
            match privacy_port.monitor_shielded_transfers().await {
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

    /// Process a transparent on-chain deposit.
    async fn process_deposit(&self, event: DepositEvent) -> Result<(), WalletError> {
        // Idempotency: skip if this tx_hash was already processed.
        // Prevents double-crediting on monitor restart or chain re-org.
        if self.store.transaction_exists_by_hash(&event.tx_hash.0)? {
            tracing::debug!(
                target: "hkask.wallet",
                tx_hash = %event.tx_hash.0,
                "Deposit already processed — skipping"
            );
            return Ok(());
        }

        // For transparent deposits, the to_address IS the wallet identifier.
        // We look up which wallet owns this deposit address.
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

        // CNS span: deposit detected
        self.emit_span(
            CnsSpan::WalletDeposit,
            "detected",
            Phase::Sense,
            serde_json::json!({
                "chain": "solana",
                "amount_usdc_micro": event.amount_usdc_micro,
                "tx_hash": event.tx_hash.0,
                "privacy": "transparent",
            }),
        );

        let rj_amount = self.usdc_to_rjoules(event.amount_usdc_micro);
        self.store.credit_rjoules(wallet_id, rj_amount)?;
        let balance = self.store.get_balance(wallet_id)?.unwrap();
        self.store.record_transaction(&WalletTransaction {
            id: 0,
            wallet_id,
            tx_type: TransactionType::Deposit {
                chain: event.tx_hash.0.parse().unwrap_or(ChainId::Solana),
                privacy: PrivacyMode::Transparent,
                tx_hash: event.tx_hash.0.clone(),
                amount_usdc_micro: event.amount_usdc_micro,
            },
            rjoules_delta: rj_amount.as_u64() as i64,
            balance_after: balance.rjoules,
            timestamp: Utc::now(),
        })?;

        // CNS span: balance credited
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

        // CNS span: deposit credited (user-noteworthy notification)
        self.emit_span(
            CnsSpan::WalletDeposit,
            "deposit_credited",
            Phase::Act,
            serde_json::json!({
                "wallet_id": wallet_id.to_string(),
                "amount_rj": rj_amount.as_u64(),
                "amount_usdc_micro": event.amount_usdc_micro,
                "tx_hash": event.tx_hash.0,
                "chain": "solana",
                "balance_after_rj": balance.rjoules,
            }),
        );

        Ok(())
    }

    async fn process_shielded_deposit(
        &self,
        transfer: ShieldedTransfer,
    ) -> Result<(), WalletError> {
        // Idempotency: skip if this commitment was already processed.
        // The deposit reference consumption below also provides anti-replay,
        // but this tx_hash check is defense-in-depth.
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

        // Extract deposit reference from memo
        let memo = match transfer.memo {
            Some(ref m) => m.clone(),
            None => {
                tracing::warn!(target: "hkask.wallet", "Shielded transfer without deposit reference memo — cannot attribute");
                return Ok(());
            }
        };
        // Consume the deposit reference to get the wallet_id
        let wallet_id = match self.store.consume_deposit_reference(&memo)? {
            Some(id) => id,
            None => {
                tracing::warn!(target: "hkask.wallet", reference = %memo, "Deposit reference not found or already spent");
                return Ok(());
            }
        };

        // CNS span: shielded deposit detected
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
        let balance = self.store.get_balance(wallet_id)?.unwrap();
        let commitment = transfer.commitment.clone();
        self.store.record_transaction(&WalletTransaction {
            id: 0,
            wallet_id,
            tx_type: TransactionType::Deposit {
                chain: ChainId::Solana, // TODO: get from privacy port
                privacy: PrivacyMode::Shielded,
                tx_hash: transfer.commitment,
                amount_usdc_micro: transfer.amount_usdc_micro,
            },
            rjoules_delta: rj_amount.as_u64() as i64,
            balance_after: balance.rjoules,
            timestamp: Utc::now(),
        })?;

        // CNS span: balance credited
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

        // CNS span: deposit credited (user-noteworthy notification)
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

    // ── Withdrawal ───────────────────────────────────────────────────────────

    /// Withdraw rJoules as USDC to a user's primary wallet address.
    ///
    /// # Flow
    /// 1. Debit rJoules from wallet
    /// 2. Convert rJoules to micro-USDC
    /// 3. Chain port builds unsigned withdrawal transaction
    /// 4. signing.rs signs the transaction (per-operation key loading)
    /// 5. Chain port submits signed transaction
    /// 6. Record transaction in ledger
    pub async fn withdraw(
        &self,
        wallet_id: WalletId,
        amount_rj: RJoule,
        to_address: &str,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<TxHash, WalletError> {
        // 1. Debit rJoules
        let balance = self.store.debit_rjoules(wallet_id, amount_rj)?;
        let amount_usdc_micro = self.rjoules_to_usdc(amount_rj);

        // CNS span: conversion
        self.emit_span(
            CnsSpan::WalletConversion,
            "converted",
            Phase::Act,
            serde_json::json!({
                "wallet_id": wallet_id.to_string(),
                "rjoules": amount_rj.as_u64(),
                "usdc_micro": amount_usdc_micro,
                "direction": "rj_to_usdc",
            }),
        );

        // 2. Build and sign transaction
        let tx_hash = match privacy {
            PrivacyMode::Transparent => {
                let port = self
                    .chains
                    .get(&chain)
                    .ok_or(WalletError::ChainNotEnabled { chain })?;
                let tx_bytes = port.build_withdrawal_tx(to_address, amount_usdc_micro)?;

                // CNS span: withdrawal built
                self.emit_span(
                    CnsSpan::WalletWithdrawal,
                    "built",
                    Phase::Act,
                    serde_json::json!({
                        "chain": chain.to_string(),
                        "to_address": to_address,
                        "amount_usdc_micro": amount_usdc_micro,
                        "privacy": "transparent",
                    }),
                );

                let signature = signing::sign_withdrawal(chain, &tx_bytes)?;

                // CNS span: withdrawal signed
                self.emit_span(
                    CnsSpan::WalletWithdrawal,
                    "signed",
                    Phase::Act,
                    serde_json::json!({
                        "chain": chain.to_string(),
                    }),
                );

                // Combine tx_bytes + signature (chain-specific format)
                let mut signed_tx = tx_bytes;
                signed_tx.extend_from_slice(&signature);
                let tx_hash = port.submit_signed_tx(&signed_tx).await?;

                // CNS span: withdrawal submitted
                self.emit_span(
                    CnsSpan::WalletWithdrawal,
                    "submitted",
                    Phase::Act,
                    serde_json::json!({
                        "chain": chain.to_string(),
                        "tx_hash": tx_hash.0,
                    }),
                );

                tx_hash
            }
            PrivacyMode::Shielded => {
                let privacy_port = self
                    .privacy
                    .as_ref()
                    .ok_or(WalletError::PrivacyUnavailable { chain })?;
                if !privacy_port.available_for_chain(chain) {
                    return Err(WalletError::PrivacyUnavailable { chain });
                }
                let tx_bytes = privacy_port.build_unshield_tx(to_address, amount_usdc_micro)?;

                if chain == ChainId::Hinkal {
                    // Hinkal submit path signs the protocol withdraw message internally.
                    // Avoid appending a redundant signature over the serialized request payload.
                    privacy_port.submit_signed_tx(&tx_bytes).await?
                } else {
                    let signature = signing::sign_withdrawal(chain, &tx_bytes)?;
                    let mut signed_tx = tx_bytes;
                    signed_tx.extend_from_slice(&signature);
                    privacy_port.submit_signed_tx(&signed_tx).await?
                }
            }
        };

        // 3. Record transaction
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
    pub fn gas_to_rjoules(&self, gas: u64) -> RJoule {
        // Integer division: gas / gas_per_rjoule
        // Minimum 1 rJ if gas > 0 (sub-rJoule operations round up to 1 rJ)
        if gas == 0 {
            RJoule::ZERO
        } else {
            let rj = gas / self.config.gas_per_rjoule;
            RJoule::new(if rj == 0 { 1 } else { rj })
        }
    }

    /// Convert rJoules to gas units.
    pub fn rjoules_to_gas(&self, rj: RJoule) -> u64 {
        rj.as_u64() * self.config.gas_per_rjoule
    }

    /// Convert micro-USDC to rJoules.
    fn usdc_to_rjoules(&self, usdc_micro: u64) -> RJoule {
        let rj = (usdc_micro as u128 * self.config.rj_per_usdc as u128 / 1_000_000) as u64;
        RJoule::new(rj)
    }

    /// Convert rJoules to micro-USDC.
    fn rjoules_to_usdc(&self, rj: RJoule) -> u64 {
        (rj.as_u64() as u128 * 1_000_000 / self.config.rj_per_usdc as u128) as u64
    }

    /// Check if a wallet can afford a given rJoule cost.
    ///
    /// REQ: WALLET-004
    /// pre:  wallet_id is a valid WalletId, cost_rj is a valid RJoule
    /// post: returns Ok(true) iff balance.rjoules >= cost_rj
    /// post: returns Ok(false) iff balance.rjoules < cost_rj
    pub fn can_afford(&self, wallet_id: WalletId, cost_rj: RJoule) -> Result<bool, WalletError> {
        let balance = self.get_balance(wallet_id)?;
        Ok(balance.rjoules >= cost_rj.as_u64())
    }

    /// Reserve rJoules for an in-flight operation (optimistic).
    /// The actual debit happens at settle time.
    ///
    /// REQ: WALLET-004
    /// pre:  wallet_id is a valid WalletId, amount is a valid RJoule
    /// post: if can_afford → Ok(()), reservation is optimistic (no debit)
    /// post: if !can_afford → Err(InsufficientBalance)
    pub fn reserve_rjoules(&self, wallet_id: WalletId, amount: RJoule) -> Result<(), WalletError> {
        if !self.can_afford(wallet_id, amount)? {
            let balance = self.get_balance(wallet_id)?;
            return Err(WalletError::InsufficientBalance {
                have: RJoule::new(balance.rjoules),
                need: amount,
            });
        }
        // Reservation is optimistic — we check can_afford but don't debit yet.
        // The actual debit happens in settle_rjoules.
        Ok(())
    }

    /// Settle rJoules after an operation completes.
    /// Debits the actual cost (may be less than reserved on failure).
    ///
    /// REQ: WALLET-004
    /// pre:  wallet_id is a valid WalletId, reserved and actual are valid RJoule
    /// post: wallet balance debited by actual (not reserved)
    /// post: if actual < reserved, difference is implicitly refunded
    pub fn settle_rjoules(
        &self,
        wallet_id: WalletId,
        reserved: RJoule,
        actual: RJoule,
    ) -> Result<(), WalletError> {
        self.store.debit_rjoules(wallet_id, actual)?;
        // If actual < reserved, the difference is implicitly refunded
        // (we only debit actual, not reserved).
        let _ = reserved; // reserved amount is informational
        Ok(())
    }

    // ── Deposit reference scheme (merged from deposit_ref.rs) ─────────────────

    /// Generate a one-time deposit reference for shielded deposits.
    ///
    /// # Privacy property `[IS-DECL]`
    /// Derived via HKDF from the wallet seed + nonce + expiry.
    /// Appears random on-chain but hKask can verify it belongs to a specific wallet.
    ///
    /// # Anti-replay `[OUGHT-DECL]`
    /// References are burned on use (consumed in WalletStore).
    /// References expire after `validity_duration` (default 24h).
    pub fn generate_deposit_reference(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        validity_duration: Duration,
    ) -> Result<DepositReference, WalletError> {
        let nonce: [u8; 16] = rand::random();
        let expiry = Utc::now() + validity_duration;
        // REQ: MUST-3 — HKDF context includes nonce to bind reference to its specific random nonce
        let context = format!(
            "hkask:deposit-ref:{}:{}:{}:{}",
            wallet_id,
            chain,
            expiry.timestamp(),
            hex::encode(nonce)
        );
        // HKDF-expand from wallet seed
        let ref_bytes = hkdf_expand(&*self.wallet_seed, context.as_bytes())?;
        let reference = hex::encode(&ref_bytes[..16]); // 32-char hex string

        let dep_ref = DepositReference {
            reference,
            wallet_id,
            chain,
            nonce,
            expires_at: expiry,
        };
        self.store.store_deposit_reference(&dep_ref)?;
        Ok(dep_ref)
    }

    // ── Encumbrance — rJoule lock/release/consume ────────────────────────────

    /// Encumber rJoules from a wallet for an API key's allocation.
    ///
    /// REQ: WALLET-005
    /// pre:  wallet_id is a valid WalletId, key_id is a valid ApiKeyId, amount > 0
    /// post: amount rJoules locked against wallet for key_id
    /// post: emits cns.wallet.encumbered span if event_sink configured
    /// Locks `amount` rJoules against the wallet balance. The locked rJoules
    /// can only be consumed by the specified API key via `consume()`.
    /// Unspent rJoules are returned to the wallet on `release_encumbrance()`.
    pub fn encumber(
        &self,
        wallet_id: WalletId,
        key_id: ApiKeyId,
        amount: RJoule,
    ) -> Result<(), WalletError> {
        self.store.encumber_rjoules(wallet_id, key_id, amount)?;
        self.emit_span(
            CnsSpan::Gas,
            "encumbered",
            Phase::Act,
            serde_json::json!({
                "key_id": key_id.to_string(),
                "wallet_id": wallet_id.to_string(),
                "amount_rj": amount.as_u64(),
            }),
        );
        Ok(())
    }

    /// Release an encumbrance, returning unspent rJoules to the wallet.
    ///
    /// REQ: WALLET-005
    /// pre:  key_id is a valid ApiKeyId
    /// post: unspent rJoules returned to wallet
    /// post: idempotent — releasing already-released/consumed encumbrance is no-op
    /// Idempotent — releasing an already-released or consumed encumbrance
    /// is a no-op.
    pub fn release_encumbrance(&self, key_id: ApiKeyId) -> Result<(), WalletError> {
        self.store.release_encumbrance(key_id)?;
        self.emit_span(
            CnsSpan::Gas,
            "released",
            Phase::Act,
            serde_json::json!({
                "key_id": key_id.to_string(),
            }),
        );
        Ok(())
    }

    /// Atomically consume rJoules from an API key's encumbrance.
    ///
    /// REQ: WALLET-005
    /// pre:  key_id is a valid ApiKeyId, gas_rj > 0
    /// post: gas_rj deducted from key's active encumbrance (atomic)
    /// post: if encumbrance fully consumed → status transitions to 'consumed'
    /// Deducts `gas_rj` from the key's active encumbrance. This is a single
    /// atomic operation — no separate check+deduct pair. If the encumbrance
    /// is fully consumed, status transitions to 'consumed'.
    pub fn consume(&self, key_id: ApiKeyId, gas_rj: RJoule) -> Result<(), WalletError> {
        self.store.consume_encumbrance(key_id, gas_rj)?;
        Ok(())
    }

    /// Get the encumbrance for an API key.
    ///
    /// REQ: WALLET-005
    /// pre:  key_id is a valid ApiKeyId
    /// post: returns Ok(Some(encumbrance)) if key has active encumbrance
    /// post: returns Ok(None) if key has no encumbrance
    pub fn get_encumbrance(&self, key_id: ApiKeyId) -> Result<Option<Encumbrance>, WalletError> {
        self.store.get_encumbrance(key_id)
    }
}

// ── HKDF helper (minimal, uses hmac + sha2 from workspace) ─────────────────────

fn hkdf_expand(seed: &[u8], info: &[u8]) -> Result<Vec<u8>, WalletError> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(seed).map_err(|e| {
        WalletError::Infra(hkask_types::InfrastructureError::Database(e.to_string()))
    })?;
    mac.update(info);
    mac.update(&[0x01]);
    let result = mac.finalize().into_bytes();
    Ok(result.to_vec())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ApiKeyIssuer;
    use crate::chain::DepositEvent;
    use crate::price_feed::StaticPriceFeed;
    use hkask_storage::database::in_memory_db;

    struct MockChainPort {
        chain: ChainId,
    }

    struct MockPrivacyPort {
        available: bool,
    }

    #[async_trait::async_trait]
    impl PrivacyPort for MockPrivacyPort {
        fn our_shielded_address(&self) -> Result<String, WalletError> {
            Ok("mock_shielded_addr".into())
        }

        fn shielded_deposit_address(&self, _wallet_id: WalletId) -> Result<String, WalletError> {
            Ok("mock_shielded_addr".into())
        }

        async fn monitor_shielded_transfers(&self) -> Result<Vec<ShieldedTransfer>, WalletError> {
            Ok(vec![])
        }

        fn build_shield_tx(
            &self,
            _amount_usdc_micro: u64,
            _chain: ChainId,
        ) -> Result<Vec<u8>, WalletError> {
            Ok(b"mock_shield_tx".to_vec())
        }

        fn build_unshield_tx(
            &self,
            _to_public: &str,
            _amount_usdc_micro: u64,
        ) -> Result<Vec<u8>, WalletError> {
            Ok(b"mock_unshield_tx".to_vec())
        }

        async fn submit_signed_tx(&self, signed_tx_bytes: &[u8]) -> Result<TxHash, WalletError> {
            if signed_tx_bytes != b"mock_unshield_tx" {
                return Err(WalletError::ChainError {
                    chain: ChainId::Hinkal,
                    message: "expected raw unshield payload (no appended signature)".into(),
                });
            }
            Ok(TxHash("mock_privacy_hash".into()))
        }

        fn available_for_chain(&self, chain: ChainId) -> bool {
            self.available && chain == ChainId::Hinkal
        }
    }

    #[async_trait::async_trait]
    impl ChainPort for MockChainPort {
        fn chain_id(&self) -> ChainId {
            self.chain
        }
        fn derive_deposit_address(&self, _index: u64) -> Result<String, WalletError> {
            Ok("mock_address_123".into())
        }
        async fn monitor_deposits(
            &self,
            _addresses: &[String],
        ) -> Result<Vec<DepositEvent>, WalletError> {
            Ok(vec![])
        }
        fn build_withdrawal_tx(&self, _to: &str, _amount: u64) -> Result<Vec<u8>, WalletError> {
            Ok(b"mock_tx".to_vec())
        }
        async fn submit_signed_tx(&self, _tx: &[u8]) -> Result<TxHash, WalletError> {
            Ok(TxHash("mock_hash".into()))
        }
        async fn confirmations(&self, _tx_hash: &TxHash) -> Result<u64, WalletError> {
            Ok(32)
        }
        async fn native_token_usd_rate(&self) -> Result<f64, WalletError> {
            Ok(1.0)
        }
    }

    fn make_manager() -> WalletManager {
        // SAFETY: test-only env var set in single-threaded test context;
        // SAFETY: no other threads read HKASK_MASTER_KEY concurrently.
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let db = in_memory_db();
        let store = Arc::new(WalletStore::new(db.conn_arc()));
        let mut chains = HashMap::new();
        chains.insert(
            ChainId::Solana,
            Box::new(MockChainPort {
                chain: ChainId::Solana,
            }) as Box<dyn ChainPort>,
        );
        WalletManager::build(
            WalletConfig::default(),
            store,
            chains,
            None,
            Arc::new(StaticPriceFeed::new()),
        )
        .unwrap()
    }

    fn make_manager_with_hinkal_privacy() -> WalletManager {
        // SAFETY: test-only env var set in single-threaded test context;
        // SAFETY: no other threads read HKASK_MASTER_KEY concurrently.
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let db = in_memory_db();
        let store = Arc::new(WalletStore::new(db.conn_arc()));
        let chains = HashMap::new();
        let privacy = Some(Box::new(MockPrivacyPort { available: true }) as Box<dyn PrivacyPort>);
        WalletManager::build(
            WalletConfig::default(),
            store,
            chains,
            privacy,
            Arc::new(StaticPriceFeed::new()),
        )
        .unwrap()
    }

    // REQ: P4-manager — gas_to_rjoules converts correctly
    #[test]
    fn gas_to_rjoules_conversion() {
        let mgr = make_manager();
        assert_eq!(mgr.gas_to_rjoules(1000), RJoule::new(1));
        assert_eq!(mgr.gas_to_rjoules(500), RJoule::new(1)); // rounds up
        assert_eq!(mgr.gas_to_rjoules(0), RJoule::ZERO);
    }

    // REQ: P4-manager — rjoules_to_gas converts correctly
    #[test]
    fn rjoules_to_gas_conversion() {
        let mgr = make_manager();
        assert_eq!(mgr.rjoules_to_gas(RJoule::new(1)), 1000);
        assert_eq!(mgr.rjoules_to_gas(RJoule::new(5)), 5000);
    }

    // REQ: P4-manager — can_afford checks balance
    #[test]
    fn can_afford_checks_balance() {
        let mgr = make_manager();
        let wallet = WalletId::new();
        mgr.store.credit_rjoules(wallet, RJoule::new(100)).unwrap();
        assert!(mgr.can_afford(wallet, RJoule::new(50)).unwrap());
        assert!(!mgr.can_afford(wallet, RJoule::new(200)).unwrap());
    }

    // REQ: P4-manager — reserve_rjoules rejects insufficient balance
    #[test]
    fn reserve_rejects_insufficient_balance() {
        let mgr = make_manager();
        let wallet = WalletId::new();
        mgr.store.credit_rjoules(wallet, RJoule::new(10)).unwrap();
        assert!(mgr.reserve_rjoules(wallet, RJoule::new(5)).is_ok());
        assert!(mgr.reserve_rjoules(wallet, RJoule::new(100)).is_err());
    }

    // REQ: P4-manager — settle_rjoules debits actual cost
    #[test]
    fn settle_debits_actual_cost() {
        let mgr = make_manager();
        let wallet = WalletId::new();
        mgr.store.credit_rjoules(wallet, RJoule::new(100)).unwrap();
        mgr.settle_rjoules(wallet, RJoule::new(50), RJoule::new(30))
            .unwrap();
        let balance = mgr.get_balance(wallet).unwrap();
        assert_eq!(balance.rjoules, 70); // 100 - 30
    }

    // REQ: P4-manager — deposit reference generation and verification
    #[test]
    fn deposit_reference_generation() {
        let mgr = make_manager();
        let wallet = WalletId::new();
        mgr.store.ensure_wallet(wallet).unwrap();

        let dep_ref = mgr
            .generate_deposit_reference(wallet, ChainId::Solana, Duration::hours(24))
            .unwrap();
        assert_eq!(dep_ref.reference.len(), 32); // 16 bytes → 32 hex chars
        assert_eq!(dep_ref.wallet_id, wallet);
        assert_eq!(dep_ref.chain, ChainId::Solana);
    }

    // ── Property-based tests ───────────────────────────────────────────────

    use proptest::prelude::*;

    /// Strategy: generate a random RJoule amount in a reasonable range.
    fn arbitrary_rjoule() -> BoxedStrategy<RJoule> {
        (1u64..1000u64).prop_map(RJoule::new).boxed()
    }

    /// Helper: create a minimal API key so encumbrance FK constraint is satisfied.
    fn ensure_key(store: &Arc<WalletStore>, wallet_id: WalletId, key_id: ApiKeyId) {
        use hkask_types::wallet::{ApiKeyCapability, Ed25519PublicKey, PrivacyMode};
        let capability = ApiKeyCapability {
            wallet_id,
            key_id,
            public_key: Ed25519PublicKey([0u8; 32]),
            spending_limit_rj: RJoule::new(1_000_000),
            spent_rj: RJoule::ZERO,
            scope: vec![],
            purpose: "test".into(),
            rate_limit: None,
            expiry: None,
            issued_at: chrono::Utc::now(),
            privacy_mode: PrivacyMode::Transparent,
            preferred_chain: None,
        };
        let _ = store.store_api_key(&capability);
    }

    // REQ: WALLET-PBT-001 — Balance conservation under encumbrance lifecycle (P4, P9)
    // After any sequence of credit, encumber, consume, and release operations:
    // - Wallet balance = total_credited - total_consumed (conservation)
    // - Total consumed ≤ total credited (can't spend more than deposited)
    // - Per key: consumed ≤ encumbered (can't consume more than locked)
    proptest! {
        #![proptest_config(ProptestConfig { max_shrink_iters: 0, .. ProptestConfig::with_cases(64) })]
        #[test]
        fn balance_conservation_under_encumbrance_lifecycle(
            credits in prop::collection::vec(arbitrary_rjoule(), 1..10),
            operations in prop::collection::vec((arbitrary_rjoule(), arbitrary_rjoule()), 0..20),
        ) {
            let mgr = make_manager();
            let wallet = WalletId::new();
            mgr.store.ensure_wallet(wallet).unwrap();

            // Track total credited
            let mut total_credited: u64 = 0;
            for credit in &credits {
                mgr.store.credit_rjoules(wallet, *credit).unwrap();
                total_credited += credit.as_u64();
            }

            // Track per-key encumbrance state (create keys on demand)
            let mut key_encumbered: std::collections::HashMap<ApiKeyId, u64> = std::collections::HashMap::new();
            let mut key_consumed: std::collections::HashMap<ApiKeyId, u64> = std::collections::HashMap::new();

            for (encumber_amount, consume_amount) in &operations {
                let key_id = ApiKeyId::new();
                ensure_key(&mgr.store, wallet, key_id);

                // Encumber: lock rJoules for this key (only if affordable)
                if mgr.can_afford(wallet, *encumber_amount).unwrap_or(false) {
                    let _ = mgr.encumber(wallet, key_id, *encumber_amount);
                    *key_encumbered.entry(key_id).or_insert(0) += encumber_amount.as_u64();
                }

                // Consume: spend from encumbrance (up to encumbered amount)
                let encumbered = *key_encumbered.get(&key_id).unwrap_or(&0);
                let consumed = *key_consumed.get(&key_id).unwrap_or(&0);
                let available = encumbered.saturating_sub(consumed);
                let actual_consume = consume_amount.as_u64().min(available);
                if actual_consume > 0 {
                    let _ = mgr.consume(key_id, RJoule::new(actual_consume));
                    *key_consumed.entry(key_id).or_insert(0) += actual_consume;
                }

                // Release: return unspent to wallet
                let _ = mgr.release_encumbrance(key_id);
            }

            // Invariant 1: balance = credited - consumed (conservation)
            let balance = mgr.get_balance(wallet).unwrap();
            let total_consumed: u64 = key_consumed.values().sum();
            prop_assert_eq!(balance.rjoules, total_credited.saturating_sub(total_consumed),
                "balance {} != credited {} - consumed {}", balance.rjoules, total_credited, total_consumed);

            // Invariant 2: can't consume more than credited
            prop_assert!(total_consumed <= total_credited,
                "consumed {} > credited {}", total_consumed, total_credited);

            // Invariant 3: per-key, consumed ≤ encumbered
            for (key_id, encumbered) in &key_encumbered {
                let consumed = key_consumed.get(key_id).copied().unwrap_or(0);
                prop_assert!(consumed <= *encumbered,
                    "key {}: consumed {} > encumbered {}", key_id, consumed, encumbered);
            }
        }
    }

    // ── Integration: deposit monitor ───────────────────────────────────────

    /// A MockChainPort that returns a pre-configured deposit event.
    struct DepositMockPort {
        chain: ChainId,
        deposit: Option<DepositEvent>,
    }

    #[async_trait::async_trait]
    impl ChainPort for DepositMockPort {
        fn chain_id(&self) -> ChainId {
            self.chain
        }
        fn derive_deposit_address(&self, _index: u64) -> Result<String, WalletError> {
            Ok("mock_deposit_addr_1".into())
        }
        async fn monitor_deposits(
            &self,
            _addresses: &[String],
        ) -> Result<Vec<DepositEvent>, WalletError> {
            Ok(self.deposit.clone().into_iter().collect())
        }
        fn build_withdrawal_tx(&self, _to: &str, _amount: u64) -> Result<Vec<u8>, WalletError> {
            Ok(b"mock_tx".to_vec())
        }
        async fn submit_signed_tx(&self, _tx: &[u8]) -> Result<TxHash, WalletError> {
            Ok(TxHash("mock_hash".into()))
        }
        async fn confirmations(&self, _tx_hash: &TxHash) -> Result<u64, WalletError> {
            Ok(32)
        }
        async fn native_token_usd_rate(&self) -> Result<f64, WalletError> {
            Ok(1.0)
        }
    }

    // REQ: wallet-int-001 — deposit monitor credits balance and is idempotent
    #[tokio::test]
    async fn deposit_monitor_credits_and_is_idempotent() {
        // SAFETY: test-only
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let db = in_memory_db();
        let store = Arc::new(WalletStore::new(db.conn_arc()));
        // Use a deterministic wallet ID so the monitor can find it.
        // WalletId::default() creates a random UUID each call — they won't match.
        let wallet_id = WalletId::from_name("test_wallet");
        store.ensure_wallet(wallet_id).unwrap();

        // Store a deposit address so resolution works
        store
            .store_deposit_address(
                wallet_id,
                ChainId::Solana,
                "mock_deposit_addr_1",
                0,
                PrivacyMode::Transparent,
            )
            .unwrap();

        let deposit_event = DepositEvent {
            tx_hash: TxHash("test_tx_hash_001".into()),
            from_address: "sender_addr".into(),
            to_address: "mock_deposit_addr_1".into(),
            amount_usdc_micro: 1_000_000, // 1 USDC
            confirmations: 32,
            block_time: Utc::now(),
        };

        let mut chains = HashMap::new();
        chains.insert(
            ChainId::Solana,
            Box::new(DepositMockPort {
                chain: ChainId::Solana,
                deposit: Some(deposit_event.clone()),
            }) as Box<dyn ChainPort>,
        );

        let mgr = WalletManager::build(
            WalletConfig::default(),
            Arc::clone(&store),
            chains,
            None,
            Arc::new(StaticPriceFeed::new()),
        )
        .unwrap();

        // Run one monitor cycle
        mgr.poll_deposits_once().await;

        // Verify balance was credited
        let balance = store.get_balance(wallet_id).unwrap().unwrap();
        assert!(
            balance.rjoules > 0,
            "balance should be credited after deposit"
        );

        // Verify idempotency: running again with same tx_hash should not double-credit
        let balance_before = balance.rjoules;
        let mut chains2 = HashMap::new();
        chains2.insert(
            ChainId::Solana,
            Box::new(DepositMockPort {
                chain: ChainId::Solana,
                deposit: Some(deposit_event),
            }) as Box<dyn ChainPort>,
        );
        let mgr2 = WalletManager::build(
            WalletConfig::default(),
            Arc::clone(&store),
            chains2,
            None,
            Arc::new(StaticPriceFeed::new()),
        )
        .unwrap();
        mgr2.poll_deposits_once().await;
        let balance_after = store.get_balance(wallet_id).unwrap().unwrap();
        assert_eq!(
            balance_after.rjoules, balance_before,
            "idempotency: balance should not change on replayed deposit"
        );
    }

    // REQ: wallet-int-006 — poll_deposits_once processes deposits from multiple chains
    #[tokio::test]
    async fn poll_deposits_once_multi_chain() {
        // SAFETY: test-only
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let db = in_memory_db();
        let store = Arc::new(WalletStore::new(db.conn_arc()));
        let wallet_id = WalletId::from_name("multi_chain_wallet");
        store.ensure_wallet(wallet_id).unwrap();

        // Register deposit addresses for both chains
        store
            .store_deposit_address(
                wallet_id,
                ChainId::Solana,
                "solana_deposit_addr",
                0,
                PrivacyMode::Transparent,
            )
            .unwrap();
        store
            .store_deposit_address(
                wallet_id,
                ChainId::Hedera,
                "hedera_deposit_addr",
                1,
                PrivacyMode::Transparent,
            )
            .unwrap();

        let solana_deposit = DepositEvent {
            tx_hash: TxHash("sol_tx_001".into()),
            from_address: "sender_a".into(),
            to_address: "solana_deposit_addr".into(),
            amount_usdc_micro: 1_000_000, // 1 USDC
            confirmations: 32,
            block_time: Utc::now(),
        };
        let hedera_deposit = DepositEvent {
            tx_hash: TxHash("hed_tx_001".into()),
            from_address: "sender_b".into(),
            to_address: "hedera_deposit_addr".into(),
            amount_usdc_micro: 2_000_000, // 2 USDC
            confirmations: 64,
            block_time: Utc::now(),
        };

        let mut chains = HashMap::new();
        chains.insert(
            ChainId::Solana,
            Box::new(DepositMockPort {
                chain: ChainId::Solana,
                deposit: Some(solana_deposit),
            }) as Box<dyn ChainPort>,
        );
        chains.insert(
            ChainId::Hedera,
            Box::new(DepositMockPort {
                chain: ChainId::Hedera,
                deposit: Some(hedera_deposit),
            }) as Box<dyn ChainPort>,
        );

        let mgr = WalletManager::build(
            WalletConfig::default(),
            Arc::clone(&store),
            chains,
            None,
            Arc::new(StaticPriceFeed::new()),
        )
        .unwrap();

        mgr.poll_deposits_once().await;

        // Both chains' deposits should be credited
        let balance = store.get_balance(wallet_id).unwrap().unwrap();
        assert!(
            balance.rjoules > 0,
            "balance should reflect deposits from both chains"
        );

        // Verify two deposit transactions recorded
        let txs = store.get_transactions(wallet_id, 10, 0).unwrap();
        let deposit_count = txs
            .iter()
            .filter(|tx| matches!(tx.tx_type, TransactionType::Deposit { .. }))
            .count();
        assert_eq!(deposit_count, 2, "two deposits should be recorded");
    }

    // REQ: wallet-int-002 — full payment lifecycle: deposit → encumber → consume → report
    #[test]
    fn end_to_end_payment_lifecycle() {
        // SAFETY: test-only
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let mgr = make_manager();
        let wallet_id = WalletId::from_name("e2e_test_wallet");
        mgr.store.ensure_wallet(wallet_id).unwrap();

        // Step 1: Deposit — credit rJoules
        mgr.store
            .credit_rjoules(wallet_id, RJoule::new(10_000))
            .unwrap();
        let balance = mgr.get_balance(wallet_id).unwrap();
        assert_eq!(balance.rjoules, 10_000, "deposit credited");

        // Step 2: Create API key with spending limit
        let issuer = ApiKeyIssuer::new(Arc::clone(&mgr.store)).unwrap();
        let material = issuer
            .create_key(
                wallet_id,
                RJoule::new(5_000),
                None, // no expiry
                PrivacyMode::Transparent,
                None, // no chain preference
                vec![],
                "e2e test key".into(),
                None,
            )
            .unwrap();
        let key_id = material.key_id;

        // Step 3: Encumber rJoules to the key
        mgr.encumber(wallet_id, key_id, RJoule::new(2_000)).unwrap();
        let enc = mgr.get_encumbrance(key_id).unwrap().unwrap();
        assert!(enc.is_active(), "encumbrance should be active");
        assert_eq!(enc.remaining_rj(), 2_000, "full amount available");

        // Step 4: Consume rJoules (simulating tool/inference usage)
        mgr.consume(key_id, RJoule::new(500)).unwrap();
        let enc = mgr.get_encumbrance(key_id).unwrap().unwrap();
        assert_eq!(enc.remaining_rj(), 1_500, "500 consumed");

        // Consume more
        mgr.consume(key_id, RJoule::new(300)).unwrap();
        let enc = mgr.get_encumbrance(key_id).unwrap().unwrap();
        assert_eq!(enc.remaining_rj(), 1_200, "800 total consumed");

        // Step 5: Verify wallet balance unchanged (encumbrance is separate)
        let balance = mgr.get_balance(wallet_id).unwrap();
        assert_eq!(balance.rjoules, 8_000, "wallet: 10_000 - 2_000 encumbered");

        // Step 6: Release encumbrance — returns unspent to wallet
        mgr.release_encumbrance(key_id).unwrap();
        let balance = mgr.get_balance(wallet_id).unwrap();
        assert_eq!(
            balance.rjoules, 9_200,
            "wallet: 8_000 + 1_200 unspent returned"
        );

        // Step 7: Verify encumbrance is released
        let enc = mgr.get_encumbrance(key_id).unwrap().unwrap();
        assert!(!enc.is_active(), "encumbrance should be released");

        // Step 8: Verify key spending limit tracking
        // spent_rj is now synced with encumbrance consumption (consume_encumbrance
        // updates both encumbrances.consumed_rj and api_keys.spent_rj).
        let capability = mgr.get_api_key(key_id).unwrap().unwrap();
        assert_eq!(
            capability.spent_rj.as_u64(),
            800,
            "spent_rj reflects 500 + 300 consumed from encumbrance"
        );
    }

    // REQ: MUST-10 — EncumbranceStatus state machine: Released cannot transition back to Active
    // Proves that once an encumbrance is released, no operation can re-activate it.
    #[test]
    fn encumbrance_status_state_machine_no_released_to_active() {
        // SAFETY: test-only
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let mgr = make_manager();
        let wallet_id = WalletId::from_name("state_machine_test");
        mgr.store.ensure_wallet(wallet_id).unwrap();
        mgr.store
            .credit_rjoules(wallet_id, RJoule::new(10_000))
            .unwrap();

        // Create a key and encumber rJoules
        let issuer = ApiKeyIssuer::new(Arc::clone(&mgr.store)).unwrap();
        let material = issuer
            .create_key(
                wallet_id,
                RJoule::new(5_000),
                None,
                PrivacyMode::Transparent,
                None,
                vec![],
                "state machine test key".into(),
                None,
            )
            .unwrap();
        let key_id = material.key_id;

        // Encumber 2000 rJ
        mgr.encumber(wallet_id, key_id, RJoule::new(2_000)).unwrap();
        let enc = mgr.get_encumbrance(key_id).unwrap().unwrap();
        assert!(enc.is_active(), "initial state: Active");

        // Consume some
        mgr.consume(key_id, RJoule::new(500)).unwrap();
        let enc = mgr.get_encumbrance(key_id).unwrap().unwrap();
        assert!(enc.is_active(), "still Active after partial consume");

        // Release the encumbrance
        mgr.release_encumbrance(key_id).unwrap();
        let enc = mgr.get_encumbrance(key_id).unwrap().unwrap();
        assert!(!enc.is_active(), "state after release: Released");
        assert_eq!(enc.status, EncumbranceStatus::Released);

        // Attempt to consume from Released encumbrance — MUST fail
        let result = mgr.consume(key_id, RJoule::new(100));
        assert!(result.is_err(), "consume on Released encumbrance must fail");
        match result {
            Err(WalletError::EncumbranceNotFound { .. }) => {} // expected
            other => panic!("expected EncumbranceNotFound, got {:?}", other),
        }

        // Verify encumbrance is still Released (not re-activated)
        let enc = mgr.get_encumbrance(key_id).unwrap().unwrap();
        assert_eq!(
            enc.status,
            EncumbranceStatus::Released,
            "status remains Released"
        );

        // Attempt to release again — idempotent, no error
        mgr.release_encumbrance(key_id).unwrap();
        let enc = mgr.get_encumbrance(key_id).unwrap().unwrap();
        assert_eq!(
            enc.status,
            EncumbranceStatus::Released,
            "double release is idempotent"
        );

        // Verify wallet balance: 10_000 - 2_000 (encumbered) + 1_500 (unspent returned) = 9_500
        let balance = mgr.get_balance(wallet_id).unwrap();
        assert_eq!(
            balance.rjoules, 9_500,
            "wallet balance after release + refund"
        );
    }

    // ── Withdrawal pipeline tests ─────────────────────────────────────────

    // REQ: wallet-int-003 — withdraw full pipeline: debit → build → sign → submit → record
    #[tokio::test]
    async fn withdraw_full_pipeline_success() {
        // SAFETY: test-only
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let mgr = make_manager();
        let wallet_id = WalletId::from_name("withdraw_test");
        mgr.store.ensure_wallet(wallet_id).unwrap();
        mgr.store
            .credit_rjoules(wallet_id, RJoule::new(10_000))
            .unwrap();

        let balance_before = mgr.get_balance(wallet_id).unwrap();
        assert_eq!(balance_before.rjoules, 10_000);

        // Execute withdrawal
        let tx_hash = mgr
            .withdraw(
                wallet_id,
                RJoule::new(2_000),
                "recipient_addr_123",
                ChainId::Solana,
                PrivacyMode::Transparent,
            )
            .await
            .unwrap();

        // Verify tx_hash returned
        assert_eq!(tx_hash.0, "mock_hash");

        // Verify balance debited
        let balance_after = mgr.get_balance(wallet_id).unwrap();
        assert_eq!(balance_after.rjoules, 8_000, "10_000 - 2_000 = 8_000");

        // Verify transaction recorded in ledger
        let txs = mgr.get_transactions(wallet_id, 10, 0).unwrap();
        let withdrawal_tx = txs
            .iter()
            .find(|tx| matches!(tx.tx_type, TransactionType::Withdrawal { .. }));
        assert!(
            withdrawal_tx.is_some(),
            "withdrawal transaction should be in ledger"
        );
        let wtx = withdrawal_tx.unwrap();
        assert_eq!(wtx.rjoules_delta, -2000, "debit of 2000 rJ");
        assert_eq!(wtx.balance_after, 8000, "balance after withdrawal");
    }

    // REQ: wallet-int-004 — withdraw rejects insufficient balance
    #[tokio::test]
    async fn withdraw_rejects_insufficient_balance() {
        // SAFETY: test-only
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let mgr = make_manager();
        let wallet_id = WalletId::from_name("insufficient_test");
        mgr.store.ensure_wallet(wallet_id).unwrap();
        mgr.store
            .credit_rjoules(wallet_id, RJoule::new(500))
            .unwrap();

        let result = mgr
            .withdraw(
                wallet_id,
                RJoule::new(10_000), // more than balance
                "recipient_addr",
                ChainId::Solana,
                PrivacyMode::Transparent,
            )
            .await;

        assert!(
            result.is_err(),
            "withdraw should fail with insufficient balance"
        );
        match result {
            Err(WalletError::InsufficientBalance { .. }) => {} // expected
            other => panic!("expected InsufficientBalance, got {:?}", other),
        }

        // Verify balance unchanged
        let balance = mgr.get_balance(wallet_id).unwrap();
        assert_eq!(
            balance.rjoules, 500,
            "balance should be unchanged after failed withdrawal"
        );
    }

    // REQ: wallet-int-005 — withdraw rejects unsupported chain
    #[tokio::test]
    async fn withdraw_rejects_unsupported_chain() {
        // SAFETY: test-only
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let mgr = make_manager();
        let wallet_id = WalletId::from_name("chain_test");
        mgr.store.ensure_wallet(wallet_id).unwrap();
        mgr.store
            .credit_rjoules(wallet_id, RJoule::new(10_000))
            .unwrap();

        // make_manager only registers Solana — Hedera should fail
        let result = mgr
            .withdraw(
                wallet_id,
                RJoule::new(1_000),
                "recipient_addr",
                ChainId::Hedera,
                PrivacyMode::Transparent,
            )
            .await;

        assert!(
            result.is_err(),
            "withdraw to unregistered chain should fail"
        );
        match result {
            Err(WalletError::ChainNotEnabled { .. }) => {} // expected
            other => panic!("expected ChainNotEnabled, got {:?}", other),
        }
    }

    // REQ: wallet-int-006 — shielded Hinkal withdrawal uses privacy adapter path
    #[tokio::test]
    async fn withdraw_shielded_hinkal_uses_privacy_path() {
        let mgr = make_manager_with_hinkal_privacy();
        let wallet_id = WalletId::from_name("shielded_hinkal_test");
        mgr.store.ensure_wallet(wallet_id).unwrap();
        mgr.store
            .credit_rjoules(wallet_id, RJoule::new(10_000))
            .unwrap();

        let tx_hash = mgr
            .withdraw(
                wallet_id,
                RJoule::new(1_500),
                "recipient_addr_hinkal",
                ChainId::Hinkal,
                PrivacyMode::Shielded,
            )
            .await
            .expect("shielded withdraw should route to privacy adapter");

        assert_eq!(tx_hash.0, "mock_privacy_hash");

        let txs = mgr.get_transactions(wallet_id, 10, 0).unwrap();
        let withdrawal_tx = txs
            .iter()
            .find(|tx| matches!(tx.tx_type, TransactionType::Withdrawal { .. }))
            .expect("withdrawal tx should be recorded");
        assert_eq!(withdrawal_tx.rjoules_delta, -1500);
    }
}
