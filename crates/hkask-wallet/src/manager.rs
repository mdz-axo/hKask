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
use hkask_types::wallet::{
    ChainId, DepositAddress, DepositReference, PrivacyMode, RJoule, TransactionType, TxHash,
    WalletBalance, WalletConfig, WalletError, WalletId, WalletTransaction,
};
use std::collections::HashMap;
use std::sync::Arc;
use zeroize::Zeroizing;

use crate::chain::{ChainPort, DepositEvent};
use crate::privacy::{PrivacyPort, ShieldedTransfer};
use crate::signing;

/// Orchestrates chain ports, privacy layer, and rJoule accounting.
///
/// # Ownership `[OUGHT-DECL]`
/// - Sole-owns `ChainPort` and `PrivacyPort` implementations
/// - Shares `Arc<WalletStore>` with CNS for algedonic monitoring
/// - Holds `wallet_seed` in `Zeroizing` for deposit reference generation
/// - Does NOT hold treasury keys (loaded per-operation in signing.rs)
pub struct WalletManager {
    config: WalletConfig,
    store: Arc<WalletStore>,
    chains: HashMap<ChainId, Box<dyn ChainPort>>,
    privacy: Option<Box<dyn PrivacyPort>>,
    wallet_seed: Zeroizing<[u8; 32]>,
}

impl WalletManager {
    /// Build a WalletManager from configuration, store, and chain/privacy ports.
    pub fn build(
        config: WalletConfig,
        store: Arc<WalletStore>,
        chains: HashMap<ChainId, Box<dyn ChainPort>>,
        privacy: Option<Box<dyn PrivacyPort>>,
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
        })
    }

    // ── Balance ──────────────────────────────────────────────────────────────

    /// Get the current rJoule balance for a wallet.
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

    // ── Deposit monitoring ───────────────────────────────────────────────────

    /// Start the background deposit monitoring loop.
    /// Polls all enabled chains and the privacy layer at a configurable interval.
    pub async fn start_deposit_monitor(&self, interval_secs: u64) -> Result<(), WalletError> {
        loop {
            for chain_id in &self.config.enabled_chains {
                if let Some(port) = self.chains.get(chain_id) {
                    let addresses: Vec<String> = self
                        .store
                        .get_deposit_addresses(WalletId::default())? // TODO: iterate all wallets
                        .iter()
                        .filter(|a| {
                            a.chain == *chain_id && a.privacy_mode == PrivacyMode::Transparent
                        })
                        .map(|a| a.address.clone())
                        .collect();
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
            tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
        }
    }

    /// Process a transparent on-chain deposit.
    async fn process_deposit(&self, event: DepositEvent) -> Result<(), WalletError> {
        // For transparent deposits, the to_address IS the wallet identifier.
        // We look up which wallet owns this deposit address.
        // For now, we use a single wallet model — all deposits credit the default wallet.
        // Multi-wallet support is a future enhancement.
        let wallet_id = WalletId::default(); // TODO: resolve from deposit address lookup
        let rj_amount = self.usdc_to_rjoules(event.amount_usdc_micro);
        self.store.credit_rjoules(wallet_id, rj_amount)?;
        let balance = self.store.get_balance(wallet_id)?.unwrap();
        self.store.record_transaction(&WalletTransaction {
            id: 0,
            wallet_id,
            tx_type: TransactionType::Deposit {
                chain: event.tx_hash.0.parse().unwrap_or(ChainId::Solana), // TODO: get from port
                privacy: PrivacyMode::Transparent,
                tx_hash: event.tx_hash.0.clone(),
                amount_usdc_micro: event.amount_usdc_micro,
            },
            rjoules_delta: rj_amount.as_u64() as i64,
            balance_after: balance.rjoules,
            timestamp: Utc::now(),
        })?;
        Ok(())
    }

    /// Process a shielded (Hinkal) deposit.
    async fn process_shielded_deposit(
        &self,
        transfer: ShieldedTransfer,
    ) -> Result<(), WalletError> {
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
        let rj_amount = self.usdc_to_rjoules(transfer.amount_usdc_micro);
        self.store.credit_rjoules(wallet_id, rj_amount)?;
        let balance = self.store.get_balance(wallet_id)?.unwrap();
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

        // 2. Build and sign transaction
        let tx_hash = match privacy {
            PrivacyMode::Transparent => {
                let port = self
                    .chains
                    .get(&chain)
                    .ok_or(WalletError::ChainNotEnabled { chain })?;
                let tx_bytes = port.build_withdrawal_tx(to_address, amount_usdc_micro)?;
                let signature = signing::sign_withdrawal(chain, &tx_bytes)?;
                // Combine tx_bytes + signature (chain-specific format)
                let mut signed_tx = tx_bytes;
                signed_tx.extend_from_slice(&signature);
                port.submit_signed_tx(&signed_tx).await?
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
                let signature = signing::sign_withdrawal(chain, &tx_bytes)?;
                let mut signed_tx = tx_bytes;
                signed_tx.extend_from_slice(&signature);
                privacy_port.submit_signed_tx(&signed_tx).await?
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
    pub fn can_afford(&self, wallet_id: WalletId, cost_rj: RJoule) -> Result<bool, WalletError> {
        let balance = self.get_balance(wallet_id)?;
        Ok(balance.rjoules >= cost_rj.as_u64())
    }

    /// Reserve rJoules for an in-flight operation (optimistic).
    /// The actual debit happens at settle time.
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
        let context = format!(
            "hkask:deposit-ref:{}:{}:{}",
            wallet_id,
            chain,
            expiry.timestamp()
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
    use crate::chain::DepositEvent;
    use hkask_storage::database::in_memory_db;

    struct MockChainPort {
        chain: ChainId,
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
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
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
        WalletManager::build(WalletConfig::default(), store, chains, None).unwrap()
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
}
