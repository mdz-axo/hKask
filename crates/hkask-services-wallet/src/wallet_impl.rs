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

use hkask_services_core::ServiceError;

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
    ///
    pub fn new(manager: Arc<WalletManager>, issuer: Arc<ApiKeyIssuer>) -> Self {
        Self {
            manager,
            issuer,
            cybernetics: None,
            consent_manager: None,
        }
    }

    /// Attach a CyberneticsLoop for CNS wallet budget registration.
    ///
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
    ///
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_consent_manager(mut self, cm: Arc<ConsentManager>) -> Self {
        self.consent_manager = Some(cm);
        self
    }

    /// Access the underlying WalletManager (for orchestration: ensure_wallet, deposit monitor).
    ///
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
        let mut chains: HashMap<ChainId, Arc<dyn hkask_wallet::ChainPort>> = HashMap::new();

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
                    chains.insert(ChainId::Solana, Arc::new(port));
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
                    chains.insert(ChainId::Hedera, Arc::new(port));
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
        let mut privacy: Option<Arc<dyn hkask_wallet::PrivacyPort>> = None;

        #[cfg(feature = "hinkal")]
        if config.privacy_enabled {
            let relayer_url = config
                .hinkal_relayer_url
                .clone()
                .or_else(|| std::env::var("HINKAL_RELAYER_URL").ok());
            let treasury_account = std::env::var("HINKAL_TREASURY_ACCOUNT").ok();

            match (relayer_url, treasury_account) {
                (Some(relayer_url), Some(treasury_account)) => {
                    // Construct a single HinkalPort shared between chain and privacy roles.
                    // This avoids double session creation and duplicate HTTP clients.
                    match hkask_wallet::hinkal::HinkalPort::new(&relayer_url, &treasury_account) {
                        Ok(hinkal) => {
                            let hinkal = Arc::new(hinkal.with_event_sink(Arc::clone(&event_sink)));
                            tracing::info!(
                                target: "cns.wallet.chain",
                                chain = "hinkal",
                                relayer_url = %relayer_url,
                                "HinkalPort initialized (shared chain + privacy adapter)"
                            );
                            chains.insert(
                                ChainId::Hinkal,
                                Arc::clone(&hinkal) as Arc<dyn hkask_wallet::ChainPort>,
                            );
                            privacy = Some(hinkal as Arc<dyn hkask_wallet::PrivacyPort>);
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "cns.wallet.chain",
                                chain = "hinkal",
                                error = %e,
                                "Failed to initialize HinkalPort"
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
    ///
    pub fn get_balance(&self, wallet_id: WalletId) -> Result<WalletBalance, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "get_balance", wallet_id = %wallet_id, "CNS");
        self.manager.get_balance(wallet_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Check if a wallet can afford a given rJoule cost.
    ///
    pub fn can_afford(&self, wallet_id: WalletId, cost_rj: RJoule) -> Result<bool, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "can_afford", wallet_id = %wallet_id, cost_rj = %cost_rj, "CNS");
        self.manager.can_afford(wallet_id, cost_rj).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                source: Some(Box::new(e)),
                message: msg,
            }
        })
    }

    /// Ensure a wallet row exists (idempotent — creates if missing).
    ///
    pub fn ensure_wallet(&self, wallet_id: WalletId) -> Result<(), ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "ensure_wallet", wallet_id = %wallet_id, "CNS");
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
    ///
    pub fn get_deposit_address(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<DepositAddress, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "get_deposit_address", wallet_id = %wallet_id, chain = ?chain, "CNS");
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
    ///
    pub fn generate_deposit_reference(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        validity_hours: i64,
    ) -> Result<DepositReference, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.wallet_svc", operation = "generate_deposit_reference", wallet_id = %wallet_id, chain = ?chain, "CNS");
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
    ///
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

    // ── Withdrawal ───────────────────────────────────────────────────────────

    /// Withdraw rJoules as USDC to a user's primary wallet address.
    ///
