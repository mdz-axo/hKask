//! WalletService — Composes WalletManager, ApiKeyIssuer, and CNS integration.
//!
//! Provides a clean interface for CLI and API surfaces. Hides the internal
//! `Arc<>` sharing pattern so callers don't repeat boilerplate at every call site.
//!
//! # Composition
//! - `WalletManager` — balance, deposits, withdrawals, gas conversion
//! - `ApiKeyIssuer` — API key creation, revocation, listing
//! - `CyberneticsLoop` (optional) — CNS wallet budget registration
//!
//! # Construction
//! Use `WalletService::build()` to construct from config + store + event sink.
//! This encapsulates chain port assembly, price feed resolution, and CNS wiring
//! — keeping `context.rs` focused on orchestration, not wallet internals.

use std::collections::HashMap;
use std::sync::Arc;

use hkask_agents::consent::ConsentManager;
use hkask_cns::CyberneticsLoop;
use hkask_storage::WalletStore;
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use hkask_types::sovereignty::DataCategory;
use hkask_types::wallet::{
    ApiKeyCapability, ApiKeyId, ApiKeyMaterial, ChainId, DepositAddress, DepositReference,
    PrivacyMode, RJoule, TxHash, WalletBalance, WalletConfig, WalletError, WalletId,
    WalletTransaction,
};
#[cfg(test)]
use hkask_wallet::price_feed::StaticPriceFeed;
use hkask_wallet::{ApiKeyIssuer, WalletManager, WithdrawalFee, resolve_price_feed};
use tokio::sync::RwLock;

use crate::ServiceError;

/// Service for wallet operations — balance, deposits, withdrawals, API keys.
///
/// Wraps `WalletManager` and `ApiKeyIssuer` behind a clean interface.
/// Optionally integrates with CNS for wallet-backed energy budget registration.
/// Optionally enforces P2 affirmative consent for withdrawal signing (MUST-4).
/// Constructed during startup — never created directly by surfaces.
#[derive(Clone)]
pub struct WalletService {
    manager: Arc<WalletManager>,
    issuer: Arc<ApiKeyIssuer>,
    /// Optional CNS loop for registering wallet-backed budgets.
    cybernetics: Option<Arc<RwLock<CyberneticsLoop>>>,
    /// Optional consent manager for P2 affirmative consent (MUST-4).
    /// When `None`, withdrawal proceeds without consent check (backward compatible).
    consent_manager: Option<Arc<ConsentManager>>,
}

impl WalletService {
    /// Create a new WalletService from its components.
    pub fn new(manager: Arc<WalletManager>, issuer: Arc<ApiKeyIssuer>) -> Self {
        Self {
            manager,
            issuer,
            cybernetics: None,
            consent_manager: None,
        }
    }

    /// Attach a CyberneticsLoop for CNS wallet budget registration.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_cybernetics(mut self, loop_: Arc<RwLock<CyberneticsLoop>>) -> Self {
        self.cybernetics = Some(loop_);
        self
    }

    /// Attach a ConsentManager for P2 affirmative consent enforcement (MUST-4).
    ///
    /// When configured, withdrawal operations require explicit user consent
    /// via `DataCategory::Custom("wallet_withdrawal")`. Without a consent manager,
    /// withdrawals proceed unchecked (backward compatible for standalone mode).
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_consent_manager(mut self, cm: Arc<ConsentManager>) -> Self {
        self.consent_manager = Some(cm);
        self
    }

    /// Access the underlying WalletManager (for orchestration: ensure_wallet, deposit monitor).
    pub fn manager(&self) -> &Arc<WalletManager> {
        &self.manager
    }

    /// Build a fully-wired WalletService from config, store, and CNS infrastructure.
    ///
    /// Encapsulates chain port assembly (Solana, Hedera, Hinkal), price feed
    /// resolution, WalletManager construction, and ApiKeyIssuer creation.
    /// This is the single entry point for production wallet construction —
    /// `context.rs` calls this and handles only orchestration (replicant binding,
    /// deposit monitor spawning).
    ///
    /// # Parameters
    /// - `config`: Wallet subsystem configuration (chains, privacy, price feed)
    /// - `store`: Shared wallet store for balances, keys, transactions
    /// - `event_sink`: CNS event sink for span emission (chain errors, key alerts)
    /// - `cybernetics`: CNS loop for wallet-backed energy budget registration
    pub fn build(
        config: &WalletConfig,
        store: Arc<WalletStore>,
        event_sink: Arc<dyn NuEventSink>,
        cybernetics: Arc<RwLock<CyberneticsLoop>>,
    ) -> Result<Arc<Self>, ServiceError> {
        // ── Build chain ports from environment ────────────────────────────
        #[allow(unused_mut)]
        let mut chains: HashMap<ChainId, Box<dyn hkask_wallet::ChainPort>> = HashMap::new();

        // Solana — self-custody via raw JSON-RPC
        #[cfg(feature = "solana")]
        if let Ok(rpc_url) = std::env::var("SOLANA_RPC_URL")
            && let Ok(treasury_pubkey) = std::env::var("SOLANA_TREASURY_PUBKEY")
        {
            match hkask_wallet::solana::SolanaPort::new(&rpc_url, &treasury_pubkey, None) {
                Ok(port) => {
                    let port = port.with_event_sink(Arc::clone(&event_sink));
                    tracing::info!(
                        target: "cns.wallet.chain",
                        chain = "solana",
                        rpc_url = %rpc_url,
                        "SolanaPort initialized"
                    );
                    chains.insert(ChainId::Solana, Box::new(port));
                }
                Err(e) => {
                    tracing::warn!(
                        target: "cns.wallet.chain",
                        chain = "solana",
                        error = %e,
                        "Failed to initialize SolanaPort"
                    );
                }
            }
        }

        // Hedera — self-custody via mirror node + gRPC
        #[cfg(feature = "hedera")]
        if let Ok(treasury_account) = std::env::var("HEDERA_TREASURY_ACCOUNT") {
            let mirror_url = std::env::var("HEDERA_MIRROR_NODE_URL")
                .unwrap_or_else(|_| "https://mainnet-public.mirrornode.hedera.com".to_string());
            let consensus_url = std::env::var("HEDERA_CONSENSUS_NODE_URL")
                .unwrap_or_else(|_| "https://35.232.244.145:50211".to_string());
            match hkask_wallet::hedera::HederaPort::new(
                &mirror_url,
                &treasury_account,
                None,
                &consensus_url,
            ) {
                Ok(port) => {
                    let port = port.with_event_sink(Arc::clone(&event_sink));
                    tracing::info!(
                        target: "cns.wallet.chain",
                        chain = "hedera",
                        mirror_url = %mirror_url,
                        consensus_url = %consensus_url,
                        "HederaPort initialized"
                    );
                    chains.insert(ChainId::Hedera, Box::new(port));
                }
                Err(e) => {
                    tracing::warn!(
                        target: "cns.wallet.chain",
                        chain = "hedera",
                        error = %e,
                        "Failed to initialize HederaPort"
                    );
                }
            }
        }

        // Optional Hinkal privacy adapter
        #[allow(unused_mut)]
        let mut privacy: Option<Box<dyn hkask_wallet::PrivacyPort>> = None;

        #[cfg(feature = "hinkal")]
        if config.privacy_enabled {
            let relayer_url = config
                .hinkal_relayer_url
                .clone()
                .or_else(|| std::env::var("HINKAL_RELAYER_URL").ok());
            let treasury_account = std::env::var("HINKAL_TREASURY_ACCOUNT").ok();

            match (relayer_url, treasury_account) {
                (Some(relayer_url), Some(treasury_account)) => {
                    match hkask_wallet::hinkal::HinkalPort::new(&relayer_url, &treasury_account) {
                        Ok(chain_port) => {
                            let chain_port = chain_port.with_event_sink(Arc::clone(&event_sink));
                            tracing::info!(
                                target: "cns.wallet.chain",
                                chain = "hinkal",
                                relayer_url = %relayer_url,
                                "HinkalPort initialized for chain routing"
                            );
                            chains.insert(ChainId::Hinkal, Box::new(chain_port));
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "cns.wallet.chain",
                                chain = "hinkal",
                                error = %e,
                                "Failed to initialize Hinkal chain adapter"
                            );
                        }
                    }

                    match hkask_wallet::hinkal::HinkalPort::new(&relayer_url, &treasury_account) {
                        Ok(privacy_port) => {
                            let privacy_port =
                                privacy_port.with_event_sink(Arc::clone(&event_sink));
                            tracing::info!(
                                target: "cns.wallet.chain",
                                chain = "hinkal",
                                relayer_url = %relayer_url,
                                "Hinkal privacy adapter initialized"
                            );
                            privacy = Some(Box::new(privacy_port));
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "cns.wallet.chain",
                                chain = "hinkal",
                                error = %e,
                                "Failed to initialize Hinkal privacy adapter"
                            );
                        }
                    }
                }
                _ => {
                    tracing::warn!(
                        target: "cns.wallet.chain",
                        "Privacy enabled but Hinkal env incomplete"
                    );
                }
            }
        }

        if chains.is_empty() {
            tracing::info!(
                target: "cns.wallet.chain",
                "No chain ports configured — wallet running in read-only mode"
            );
        }

        // ── Resolve price feed from user config ──────────────────────────
        let price_feed =
            resolve_price_feed(&config.price_feed).map_err(|e| ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: "Failed to resolve price feed".into(),
            })?;

        // ── Build WalletManager ──────────────────────────────────────────
        let manager = Arc::new(
            WalletManager::build(
                config.clone(),
                Arc::clone(&store),
                chains,
                privacy,
                price_feed,
            )
            .map_err(|e| ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: "Failed to build WalletManager".into(),
            })?
            .with_event_sink(Arc::clone(&event_sink)),
        );

        // ── Build ApiKeyIssuer ───────────────────────────────────────────
        let issuer = Arc::new(
            ApiKeyIssuer::new(Arc::clone(&store))
                .map_err(|e| ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: "Failed to build ApiKeyIssuer".into(),
                })?
                .with_event_sink(Arc::clone(&event_sink)),
        );

        Ok(Arc::new(
            Self::new(manager, issuer).with_cybernetics(cybernetics),
        ))
    }

    // ── Balance ──────────────────────────────────────────────────────────────

    /// Get the current rJoule balance for a wallet.
    pub fn get_balance(&self, wallet_id: WalletId) -> Result<WalletBalance, ServiceError> {
        self.manager.get_balance(wallet_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Check if a wallet can afford a given rJoule cost.
    pub fn can_afford(&self, wallet_id: WalletId, cost_rj: RJoule) -> Result<bool, ServiceError> {
        self.manager.can_afford(wallet_id, cost_rj).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Ensure a wallet row exists (idempotent — creates if missing).
    pub fn ensure_wallet(&self, wallet_id: WalletId) -> Result<(), ServiceError> {
        self.manager.ensure_wallet(wallet_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    // ── Deposit ──────────────────────────────────────────────────────────────

    /// Get or derive a deposit address for a wallet on a specific chain.
    pub fn get_deposit_address(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<DepositAddress, ServiceError> {
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
    pub fn generate_deposit_reference(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        validity_hours: i64,
    ) -> Result<DepositReference, ServiceError> {
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

    /// Get paginated transaction history for a wallet.
    pub fn get_transactions(
        &self,
        wallet_id: WalletId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<WalletTransaction>, ServiceError> {
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

    // ── Withdrawal ───────────────────────────────────────────────────────────

    /// Withdraw rJoules as USDC to a user's primary wallet address.
    ///
    /// REQ: MUST-4 — requires P2 affirmative consent when ConsentManager is configured.
    /// pre:  webid identifies the user requesting the withdrawal
    /// post: if consent_manager is Some and consent denied → Err(ConsentDenied)
    /// post: if consent_manager is None → proceeds without consent check (backward compat)
    pub async fn withdraw(
        &self,
        webid: &WebID,
        wallet_id: WalletId,
        amount_rj: RJoule,
        to_address: &str,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<TxHash, ServiceError> {
        // REQ: MUST-4 — P2 affirmative consent gate for withdrawal signing
        if let Some(ref cm) = self.consent_manager {
            let category = DataCategory::Custom("wallet_withdrawal".into());
            let has_consent = cm.has_consent(&webid.to_string(), &category).map_err(|e| {
                ServiceError::ConsentDenied {
                    message: format!(
                        "Consent check failed for {}: {e}. Denying wallet withdrawal by default",
                        webid
                    ),
                }
            })?;
            if !has_consent {
                return Err(ServiceError::ConsentDenied {
                    message: format!(
                        "User {} has not granted consent for wallet withdrawal. \
                         Grant consent with: kask sovereignty grant {} wallet_withdrawal",
                        webid, webid
                    ),
                });
            }
        }

        self.manager
            .withdraw(wallet_id, amount_rj, to_address, chain, privacy)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                // REQ: P9 — emit chain_error span for CNS feedback loop closure
                if matches!(
                    e,
                    WalletError::ChainNotEnabled { .. }
                        | WalletError::ChainError { .. }
                        | WalletError::PrivacyUnavailable { .. }
                ) {
                    self.manager.emit_chain_error(chain, "withdraw", &msg);
                }
                ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })
    }

    /// Estimate network withdrawal fee for a chain using configured price feed.
    pub async fn estimate_withdrawal_fee(
        &self,
        chain: ChainId,
    ) -> Result<WithdrawalFee, ServiceError> {
        self.manager
            .estimate_withdrawal_fee(chain)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })
    }

    // ── API Keys ─────────────────────────────────────────────────────────────

    /// Create a new API key with the specified limits, scope, and purpose.
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
        rate_limit: Option<hkask_types::wallet::RateLimitConfig>,
    ) -> Result<ApiKeyMaterial, ServiceError> {
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
    pub fn revoke_key(&self, key_id: ApiKeyId) -> Result<(), ServiceError> {
        self.issuer.revoke_key(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// List active (non-revoked) API keys for a wallet.
    pub fn list_keys(&self, wallet_id: WalletId) -> Result<Vec<ApiKeyCapability>, ServiceError> {
        self.issuer.list_keys(wallet_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Get a single API key capability by key ID.
    pub fn get_api_key(&self, key_id: ApiKeyId) -> Result<Option<ApiKeyCapability>, ServiceError> {
        self.manager.get_api_key(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    // ── Gas conversion ──────────────────────────────────────────────────────

    /// Convert gas units to rJoules.
    pub fn gas_to_rjoules(&self, gas: u64) -> RJoule {
        self.manager.gas_to_rjoules(gas)
    }

    /// Convert rJoules to gas units.
    pub fn rjoules_to_gas(&self, rj: RJoule) -> u64 {
        self.manager.rjoules_to_gas(rj)
    }

    // ── CNS Integration ─────────────────────────────────────────────────────

    /// Register a wallet-backed energy budget for an agent in the CNS.
    ///
    /// The agent's tool invocations will debit rJoules from the wallet
    /// instead of consuming from the dimensionless gas pool.
    /// The gas→rJoule conversion rate is taken from the WalletManager's config.
    pub async fn register_wallet_budget(
        &self,
        agent: hkask_types::WebID,
        wallet_id: WalletId,
    ) -> Result<(), ServiceError> {
        let loop_ = self
            .cybernetics
            .as_ref()
            .ok_or_else(|| ServiceError::Wallet {
                source: None,
                message: "CyberneticsLoop not attached to WalletService — call with_cybernetics() during construction".into(),
            })?;
        let budget = hkask_cns::WalletBackedBudget::new(wallet_id, Arc::clone(&self.manager));
        loop_
            .read()
            .await
            .register_wallet_budget(agent, budget)
            .await;
        Ok(())
    }

    /// Register a wallet-backed energy budget with an API key for encumbrance tracking.
    ///
    /// Unlike `register_wallet_budget`, this attaches the API key so that
    /// gas consumption is debited from the key's encumbrance (not raw wallet
    /// balance). The spending limit is also tracked per-key.
    pub async fn register_wallet_budget_for_key(
        &self,
        agent: hkask_types::WebID,
        wallet_id: WalletId,
        key_id: ApiKeyId,
        spending_limit_rj: RJoule,
    ) -> Result<(), ServiceError> {
        let loop_ = self
            .cybernetics
            .as_ref()
            .ok_or_else(|| ServiceError::Wallet {
                source: None,
                message: "CyberneticsLoop not attached to WalletService — call with_cybernetics() during construction".into(),
            })?;
        let budget = hkask_cns::WalletBackedBudget::new(wallet_id, Arc::clone(&self.manager))
            .with_api_key(key_id, spending_limit_rj);
        loop_
            .read()
            .await
            .register_wallet_budget(agent, budget)
            .await;
        Ok(())
    }

    // ── Encumbrance ──────────────────────────────────────────────────────────

    /// Encumber rJoules from a wallet for an API key's allocation.
    pub fn encumber_key(
        &self,
        wallet_id: WalletId,
        key_id: ApiKeyId,
        amount: RJoule,
    ) -> Result<(), ServiceError> {
        self.manager
            .encumber(wallet_id, key_id, amount)
            .map_err(|e| {
                let msg = e.to_string();
                ServiceError::Wallet {
                    source: Some(Box::new(e)),
                    message: msg,
                }
            })
    }

    /// Release an encumbrance, returning unspent rJoules to the wallet.
    pub fn release_encumbrance(&self, key_id: ApiKeyId) -> Result<(), ServiceError> {
        self.manager.release_encumbrance(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Atomically consume rJoules from an API key's encumbrance.
    pub fn consume_gas(&self, key_id: ApiKeyId, gas_rj: RJoule) -> Result<(), ServiceError> {
        self.manager.consume(key_id, gas_rj).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Get the encumbrance for an API key.
    pub fn get_encumbrance(
        &self,
        key_id: ApiKeyId,
    ) -> Result<Option<hkask_types::wallet::Encumbrance>, ServiceError> {
        self.manager.get_encumbrance(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Emit a CNS algedonic alert for API key health events.
    ///
    /// Delegates to `WalletManager::emit_key_alert`. When the manager has
    /// no event sink configured, this is a no-op (graceful degradation).
    pub fn emit_key_alert(&self, key_id: ApiKeyId, exhausted: bool, expired: bool) {
        self.manager.emit_key_alert(key_id, exhausted, expired);
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::WalletStore;
    use hkask_storage::database::in_memory_db;
    use hkask_types::wallet::WalletConfig;

    fn make_service() -> WalletService {
        // Set master key for keystore resolution
        // SAFETY: test-only — sets master key env var in isolated test process.
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let db = in_memory_db();
        let store = Arc::new(WalletStore::new(db.conn_arc()));
        let config = WalletConfig::default();
        let manager = Arc::new(
            WalletManager::build(
                config,
                Arc::clone(&store),
                Default::default(),
                None,
                Arc::new(StaticPriceFeed),
            )
            .unwrap(),
        );
        let issuer = Arc::new(ApiKeyIssuer::new(Arc::clone(&store)).unwrap());
        WalletService::new(manager, issuer)
    }

    // REQ: svc-wallet-001 — get_balance returns zero for new wallet
    #[test]
    fn get_balance_returns_zero_for_new_wallet() {
        let svc = make_service();
        let wallet = WalletId::new();
        // ensure_wallet is needed before balance query
        // (WalletService delegates to WalletManager which calls get_balance directly)
        let balance = svc.get_balance(wallet).unwrap();
        assert_eq!(balance.rjoules, 0);
    }

    // REQ: svc-wallet-002 — gas_to_rjoules conversion
    #[test]
    fn gas_to_rjoules_conversion() {
        let svc = make_service();
        // Default gas_per_rjoule = 1000
        assert_eq!(svc.gas_to_rjoules(0).as_u64(), 0);
        assert_eq!(svc.gas_to_rjoules(500).as_u64(), 1); // rounds up
        assert_eq!(svc.gas_to_rjoules(2000).as_u64(), 2);
    }

    // REQ: svc-wallet-003 — rjoules_to_gas conversion
    #[test]
    fn rjoules_to_gas_conversion() {
        let svc = make_service();
        assert_eq!(svc.rjoules_to_gas(RJoule::new(0)), 0);
        assert_eq!(svc.rjoules_to_gas(RJoule::new(5)), 5000);
    }

    // REQ: svc-wallet-007 — estimate_withdrawal_fee returns positive fee
    #[tokio::test]
    async fn estimate_withdrawal_fee_returns_positive_fee() {
        let svc = make_service();
        let fee = svc
            .estimate_withdrawal_fee(ChainId::Solana)
            .await
            .expect("fee estimate");
        assert!(fee.rjoules > 0);
        assert!(fee.usdc_micro > 0);
        assert!(fee.native_units > 0.0);
    }

    // REQ: svc-wallet-004 — create_key produces valid material
    #[test]
    fn create_key_produces_valid_material() {
        let svc = make_service();
        let wallet = WalletId::new();
        svc.manager.ensure_wallet(wallet).expect("ensure_wallet");

        let material = svc
            .create_key(
                wallet,
                RJoule::new(5000),
                None,
                PrivacyMode::Transparent,
                None,
                vec!["read-specs".to_string()],
                "test key".to_string(),
                None,
            )
            .unwrap();
        assert_eq!(material.private_key_hex.len(), 64);
        assert!(material.capability.spending_limit_rj.as_u64() == 5000);
    }

    // REQ: svc-wallet-005 — list_keys returns created keys
    #[test]
    fn list_keys_returns_created_keys() {
        let svc = make_service();
        let wallet = WalletId::new();
        svc.manager.ensure_wallet(wallet).expect("ensure_wallet");

        svc.create_key(
            wallet,
            RJoule::new(1000),
            None,
            PrivacyMode::Transparent,
            None,
            vec!["read-specs".to_string()],
            "list test 1".to_string(),
            None,
        )
        .unwrap();
        svc.create_key(
            wallet,
            RJoule::new(2000),
            None,
            PrivacyMode::Shielded,
            Some(ChainId::Solana),
            vec!["embed-corpus".to_string()],
            "list test 2".to_string(),
            None,
        )
        .unwrap();

        let keys = svc.list_keys(wallet).unwrap();
        assert_eq!(keys.len(), 2);
    }

    // REQ: svc-wallet-006 — revoke_key removes from active list
    #[test]
    fn revoke_key_removes_from_active_list() {
        let svc = make_service();
        let wallet = WalletId::new();
        svc.manager.ensure_wallet(wallet).expect("ensure_wallet");

        let material = svc
            .create_key(
                wallet,
                RJoule::new(1000),
                None,
                PrivacyMode::Transparent,
                None,
                vec!["read-specs".to_string()],
                "revoke test".to_string(),
                None,
            )
            .unwrap();

        assert_eq!(svc.list_keys(wallet).unwrap().len(), 1);
        svc.revoke_key(material.key_id).unwrap();
        assert_eq!(svc.list_keys(wallet).unwrap().len(), 0);
    }
}
