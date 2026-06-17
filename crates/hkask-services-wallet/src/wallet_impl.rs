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
    /// REQ: P9-svc-wallet-279
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  manager must be a valid Arc<WalletManager>; issuer must be a valid Arc<ApiKeyIssuer>
    /// post: returns WalletService with manager and issuer wired; cybernetics and consent_manager default to None
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
    /// REQ: P9-svc-wallet-280
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  loop_ must be a valid Arc<RwLock<CyberneticsLoop>>
    /// post: returns self with cybernetics set
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
    /// REQ: P9-svc-wallet-281
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  cm must be a valid Arc<ConsentManager>
    /// post: returns self with consent_manager set
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_consent_manager(mut self, cm: Arc<ConsentManager>) -> Self {
        self.consent_manager = Some(cm);
        self
    }

    /// Access the underlying WalletManager (for orchestration: ensure_wallet, deposit monitor).
    ///
    /// REQ: P9-svc-wallet-282
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be constructed
    /// post: returns &Arc<WalletManager>
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
    /// REQ: P9-svc-wallet-283
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  config must be valid; store must be initialized; event_sink must be valid; cybernetics must be valid
    /// post: returns Arc<WalletService> with chain ports, price feed, WalletManager, and ApiKeyIssuer all wired; Err on construction failure
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
                message: "Failed to build WalletManager".into(),
            })?
            .with_event_sink(Arc::clone(&event_sink)),
        );

        // ── Build ApiKeyIssuer ───────────────────────────────────────────
        let issuer = Arc::new(
            ApiKeyIssuer::new(Arc::clone(&store))
                .map_err(|e| ServiceError::Wallet {
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
    /// REQ: P9-svc-wallet-284
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid
    /// post: returns WalletBalance; Err(Wallet) on manager error
    pub fn get_balance(&self, wallet_id: WalletId) -> Result<WalletBalance, ServiceError> {
        self.manager.get_balance(wallet_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                message: msg,
            }
        })
    }

    /// Check if a wallet can afford a given rJoule cost.
    ///
    /// REQ: P9-svc-wallet-285
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; cost_rj must be >= 0
    /// post: returns true if balance >= cost_rj; false otherwise; Err(Wallet) on manager error
    pub fn can_afford(&self, wallet_id: WalletId, cost_rj: RJoule) -> Result<bool, ServiceError> {
        self.manager.can_afford(wallet_id, cost_rj).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                message: msg,
            }
        })
    }

    /// Ensure a wallet row exists (idempotent — creates if missing).
    ///
    /// REQ: P9-svc-wallet-286
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid
    /// post: wallet row exists in store; Ok(()) on success; Err(Wallet) on manager error
    pub fn ensure_wallet(&self, wallet_id: WalletId) -> Result<(), ServiceError> {
        self.manager.ensure_wallet(wallet_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                message: msg,
            }
        })
    }

    // ── Deposit ──────────────────────────────────────────────────────────────

    /// Get or derive a deposit address for a wallet on a specific chain.
    ///
    /// REQ: P9-svc-wallet-287
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; chain must be a configured ChainId; privacy must be a valid PrivacyMode
    /// post: returns DepositAddress; Err(Wallet) on manager error
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
                    message: msg,
                }
            })
    }

    /// Generate a one-time deposit reference for shielded deposits.
    ///
    /// REQ: P9-svc-wallet-288
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; chain must be configured; validity_hours must be > 0
    /// post: returns DepositReference with expiry; Err(Wallet) on manager error
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
                    message: msg,
                }
            })
    }

    /// Get paginated transaction history for a wallet.
    ///
    /// REQ: P9-svc-wallet-289
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; limit must be > 0
    /// post: returns Vec<WalletTransaction>; empty Vec if no transactions; Err(Wallet) on manager error
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
                    message: msg,
                }
            })
    }

    // ── Withdrawal ───────────────────────────────────────────────────────────

    /// Withdraw rJoules as USDC to a user's primary wallet address.
    ///
    /// REQ: P2-svc-wallet-withdraw-consent — requires P2 affirmative consent when ConsentManager is configured.
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
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
        // REQ: P2-svc-wallet-withdraw-consent-gate — P2 affirmative consent gate for withdrawal signing
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
            .withdraw(webid, wallet_id, amount_rj, to_address, chain, privacy)
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
                    self.manager
                        .emit_chain_error_for_actor(webid, chain, "withdraw", &msg);
                }
                ServiceError::Wallet {
                    message: msg,
                }
            })
    }

    /// Estimate network withdrawal fee for a chain using configured price feed.
    ///
    /// REQ: P9-svc-wallet-290
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  webid must be valid; chain must be configured
    /// post: returns WithdrawalFee estimate; Err(Wallet) on manager error
    pub async fn estimate_withdrawal_fee(
        &self,
        webid: &WebID,
        chain: ChainId,
    ) -> Result<WithdrawalFee, ServiceError> {
        self.manager
            .estimate_withdrawal_fee(webid, chain)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                ServiceError::Wallet {
                    message: msg,
                }
            })
    }

    // ── Shield ───────────────────────────────────────────────────────────────

    /// Shield transparently-held USDC into the Hinkal privacy pool.
    ///
    /// REQ: P9-svc-wallet-291
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; amount_usdc_micro must be > 0; chain must support shielding
    /// post: returns TxHash of shield transaction; Err(Wallet) on failure
    pub async fn shield_assets(
        &self,
        wallet_id: WalletId,
        amount_usdc_micro: u64,
        chain: ChainId,
    ) -> Result<TxHash, ServiceError> {
        self.manager
            .shield_assets(wallet_id, amount_usdc_micro, chain)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                self.manager.emit_chain_error(chain, "shield_assets", &msg);
                ServiceError::Wallet {
                    message: msg,
                }
            })
    }

    // ── API Keys ─────────────────────────────────────────────────────────────

    /// Create a new API key with the specified limits, scope, and purpose.
    ///
    /// REQ: P9-svc-wallet-292
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid; spending_limit_rj must be >= 0; purpose must be non-empty
    /// post: returns ApiKeyMaterial with key secret; Err(Wallet) on issuer error
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
                    message: msg,
                }
            })
    }

    /// Revoke an API key. Returns unspent rJoules to the wallet.
    ///
    /// REQ: P9-svc-wallet-293
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must be a valid, non-revoked key
    /// post: key is revoked; unspent rJoules returned to wallet; Err(Wallet) on issuer error
    pub fn revoke_key(&self, key_id: ApiKeyId) -> Result<(), ServiceError> {
        self.issuer.revoke_key(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                message: msg,
            }
        })
    }

    /// List active (non-revoked) API keys for a wallet.
    ///
    /// REQ: P9-svc-wallet-294
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid
    /// post: returns Vec<ApiKeyCapability> of active keys; empty Vec if none; Err(Wallet) on issuer error
    pub fn list_keys(&self, wallet_id: WalletId) -> Result<Vec<ApiKeyCapability>, ServiceError> {
        self.issuer.list_keys(wallet_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                message: msg,
            }
        })
    }

    /// Get a single API key capability by key ID.
    ///
    /// REQ: P9-svc-wallet-295
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must be valid
    /// post: returns Some(ApiKeyCapability) if found; None if not found; Err(Wallet) on manager error
    pub fn get_api_key(&self, key_id: ApiKeyId) -> Result<Option<ApiKeyCapability>, ServiceError> {
        self.manager.get_api_key(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                message: msg,
            }
        })
    }

    // ── Gas conversion ──────────────────────────────────────────────────────

    /// Convert gas units to rJoules.
    ///
    /// REQ: P9-svc-wallet-296
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  gas must be >= 0
    /// post: returns RJoule equivalent using manager's conversion rate
    pub fn gas_to_rjoules(&self, gas: u64) -> RJoule {
        self.manager.gas_to_rjoules(gas)
    }

    /// Convert rJoules to gas units.
    ///
    /// REQ: P9-svc-wallet-297
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  rj must be >= 0
    /// post: returns u64 gas equivalent using manager's conversion rate
    pub fn rjoules_to_gas(&self, rj: RJoule) -> u64 {
        self.manager.rjoules_to_gas(rj)
    }

    // ── CNS Integration ─────────────────────────────────────────────────────

    /// Register a wallet-backed energy budget for an agent in the CNS.
    ///
    /// The agent's tool invocations will debit rJoules from the wallet
    /// instead of consuming from the dimensionless gas pool.
    /// The gas→rJoule conversion rate is taken from the WalletManager's config.
    ///
    /// REQ: P9-svc-wallet-298
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  cybernetics must be attached via with_cybernetics(); agent must be a valid WebID; wallet_id must be valid
    /// post: wallet-backed budget is registered in CNS for the agent; Err(Wallet) if cybernetics not attached
    pub async fn register_wallet_budget(
        &self,
        agent: hkask_types::WebID,
        wallet_id: WalletId,
    ) -> Result<(), ServiceError> {
        let loop_ = self
            .cybernetics
            .as_ref()
            .ok_or_else(|| ServiceError::Wallet {
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
    ///
    /// REQ: P9-svc-wallet-299
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  cybernetics must be attached; agent must be valid; wallet_id and key_id must be valid; spending_limit_rj must be >= 0
    /// post: wallet-backed budget with API key tracking is registered in CNS; Err(Wallet) if cybernetics not attached
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
    ///
    /// REQ: P9-svc-wallet-300
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  wallet_id must be valid with sufficient balance; key_id must be valid; amount must be > 0
    /// post: rJoules are encumbered from wallet to key; Err(Wallet) on manager error
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
                    message: msg,
                }
            })
    }

    /// Release an encumbrance, returning unspent rJoules to the wallet.
    ///
    /// REQ: P9-svc-wallet-301
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must have an active encumbrance
    /// post: encumbrance is released; unspent rJoules returned to wallet; Err(Wallet) on manager error
    pub fn release_encumbrance(&self, key_id: ApiKeyId) -> Result<(), ServiceError> {
        self.manager.release_encumbrance(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                message: msg,
            }
        })
    }

    /// Atomically consume rJoules from an API key's encumbrance.
    ///
    /// REQ: P9-svc-wallet-302
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must have sufficient encumbered balance; gas_rj must be > 0
    /// post: rJoules are atomically debited from key's encumbrance; Err(Wallet) on manager error or insufficient balance
    pub fn consume_gas(&self, key_id: ApiKeyId, gas_rj: RJoule) -> Result<(), ServiceError> {
        self.manager.consume(key_id, gas_rj).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                message: msg,
            }
        })
    }

    /// Get the encumbrance for an API key.
    ///
    /// REQ: P9-svc-wallet-303
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must be valid
    /// post: returns Some(Encumbrance) if key has active encumbrance; None if none; Err(Wallet) on manager error
    pub fn get_encumbrance(
        &self,
        key_id: ApiKeyId,
    ) -> Result<Option<hkask_types::wallet::Encumbrance>, ServiceError> {
        self.manager.get_encumbrance(key_id).map_err(|e| {
            let msg = e.to_string();
            ServiceError::Wallet {
                message: msg,
            }
        })
    }

    /// Emit a CNS algedonic alert for API key health events.
    ///
    /// Delegates to `WalletManager::emit_key_alert`. When the manager has
    /// no event sink configured, this is a no-op (graceful degradation).
    ///
    /// REQ: P9-svc-wallet-304
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  key_id must be valid; exhausted and expired are boolean flags
    /// post: CNS alert emitted if event sink configured; no-op otherwise
    pub fn emit_key_alert(&self, key_id: ApiKeyId, exhausted: bool, expired: bool) {
        self.manager.emit_key_alert(key_id, exhausted, expired);
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_wallet::ChainPort;

    mod test_support {
        use super::*;
        use hkask_storage::WalletStore;
        use hkask_storage::database::in_memory_db;
        use hkask_types::cns::CnsSpan;
        use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
        use hkask_types::wallet::{TxHash, WalletConfig};
        use hkask_wallet::{
            ChainPort, DepositEvent, ExchangeRate, PriceFeed, PrivacyPort, ShieldedTransfer,
        };
        use std::sync::Mutex;

        const TEST_MASTER_KEY: &str =
            "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX";

        /// Harness sink used by actor-continuity tests.
        ///
        /// Stores all emitted ν-events so tests can assert that adapter/manager
        /// error spans preserve the request-level `WebID` observer identity.
        #[derive(Default)]
        pub(super) struct CaptureSink {
            pub(super) events: Mutex<Vec<NuEvent>>,
        }

        impl NuEventSink for CaptureSink {
            fn persist(&self, event: &NuEvent) -> Result<(), hkask_types::InfrastructureError> {
                self.events.lock().expect("lock").push(event.clone());
                Ok(())
            }
        }

        pub(super) fn set_test_master_key() {
            // SAFETY: test-only env var set in isolated test process.
            unsafe {
                std::env::set_var("HKASK_MASTER_KEY", TEST_MASTER_KEY);
            }
        }

        pub(super) fn build_service_with_harness(
            sink: Arc<CaptureSink>,
            chains: HashMap<ChainId, Arc<dyn ChainPort>>,
            privacy: Option<Arc<dyn PrivacyPort>>,
            price_feed: Arc<dyn PriceFeed>,
        ) -> WalletService {
            let db = in_memory_db();
            let store = Arc::new(WalletStore::new(db.conn_arc()));

            let manager = Arc::new(
                WalletManager::build(
                    WalletConfig::default(),
                    Arc::clone(&store),
                    chains,
                    privacy,
                    price_feed,
                )
                .expect("build manager")
                .with_event_sink(Arc::clone(&sink) as Arc<dyn NuEventSink>),
            );
            let issuer = Arc::new(ApiKeyIssuer::new(Arc::clone(&store)).expect("issuer"));
            WalletService::new(manager, issuer)
        }

        pub(super) fn assert_event_actor(sink: &CaptureSink, operation: &str, actor: &WebID) {
            let events = sink.events.lock().expect("lock");
            let event = events
                .iter()
                .find(|e| e.observation.get("operation") == Some(&serde_json::json!(operation)))
                .unwrap_or_else(|| panic!("event for operation '{operation}' must be emitted"));
            assert_eq!(event.observer_webid.to_string(), actor.to_string());
        }

        pub(super) struct FailingActorChain {
            pub(super) sink: Arc<dyn NuEventSink>,
        }

        pub(super) struct FailingActorPrivacy {
            pub(super) sink: Arc<dyn NuEventSink>,
        }

        pub(super) struct FailingPriceFeed;

        #[async_trait::async_trait]
        impl ChainPort for FailingActorChain {
            fn chain_id(&self) -> ChainId {
                ChainId::Solana
            }

            fn derive_deposit_address(&self, _index: u64) -> Result<String, WalletError> {
                Ok("mock_addr".into())
            }

            async fn monitor_deposits(
                &self,
                _actor: &WebID,
                _addresses: &[String],
            ) -> Result<Vec<DepositEvent>, WalletError> {
                Ok(vec![])
            }

            fn build_withdrawal_tx(
                &self,
                _to_address: &str,
                _amount_usdc_micro: u64,
            ) -> Result<Vec<u8>, WalletError> {
                Ok(b"mock-withdraw-payload".to_vec())
            }

            async fn submit_signed_tx(
                &self,
                actor: &WebID,
                _signed_tx_bytes: &[u8],
            ) -> Result<TxHash, WalletError> {
                let event = NuEvent::new(
                    *actor,
                    Span::new(SpanNamespace::from(CnsSpan::WalletChainError), "error"),
                    Phase::Sense,
                    serde_json::json!({
                        "chain": "solana",
                        "operation": "submit_signed_tx",
                        "error": "forced adapter failure"
                    }),
                    0,
                );
                let _ = self.sink.persist(&event);
                Err(WalletError::ChainError {
                    chain: ChainId::Solana,
                    message: "forced adapter failure".into(),
                })
            }

            async fn confirmations(
                &self,
                _actor: &WebID,
                _tx_hash: &TxHash,
            ) -> Result<u64, WalletError> {
                Ok(0)
            }
        }

        #[async_trait::async_trait]
        impl PrivacyPort for FailingActorPrivacy {
            fn our_shielded_address(&self) -> Result<String, WalletError> {
                Ok("shielded_mock".into())
            }

            fn shielded_deposit_address(
                &self,
                _wallet_id: WalletId,
            ) -> Result<String, WalletError> {
                Ok("shielded_mock".into())
            }

            async fn monitor_shielded_transfers(
                &self,
                _actor: &WebID,
            ) -> Result<Vec<ShieldedTransfer>, WalletError> {
                Ok(vec![])
            }

            fn build_shield_tx(
                &self,
                _amount_usdc_micro: u64,
                _chain: ChainId,
            ) -> Result<Vec<u8>, WalletError> {
                Ok(b"mock-shield".to_vec())
            }

            fn build_unshield_tx(
                &self,
                _to_public: &str,
                _amount_usdc_micro: u64,
            ) -> Result<Vec<u8>, WalletError> {
                Ok(b"mock-unshield".to_vec())
            }

            async fn submit_signed_tx(
                &self,
                actor: &WebID,
                _signed_tx_bytes: &[u8],
            ) -> Result<TxHash, WalletError> {
                let event = NuEvent::new(
                    *actor,
                    Span::new(SpanNamespace::from(CnsSpan::WalletChainError), "error"),
                    Phase::Sense,
                    serde_json::json!({
                        "chain": "hinkal",
                        "operation": "privacy_submit_signed_tx",
                        "error": "forced privacy adapter failure"
                    }),
                    0,
                );
                let _ = self.sink.persist(&event);
                Err(WalletError::ChainError {
                    chain: ChainId::Hinkal,
                    message: "forced privacy adapter failure".into(),
                })
            }

            fn available_for_chain(&self, chain: ChainId) -> bool {
                chain == ChainId::Hinkal
            }
        }

        #[async_trait::async_trait]
        impl PriceFeed for FailingPriceFeed {
            async fn get_rate(&self, _chain: ChainId) -> Result<ExchangeRate, WalletError> {
                Err(WalletError::Infra(
                    hkask_types::InfrastructureError::Database("forced price feed failure".into()),
                ))
            }
        }
    }

    use test_support::*;

    fn make_service() -> WalletService {
        set_test_master_key();
        build_service_with_harness(
            Arc::new(CaptureSink::default()),
            Default::default(),
            None,
            Arc::new(StaticPriceFeed),
        )
    }

    // REQ: P9-svc-wallet-001 — get_balance returns zero for new wallet
    #[test]
    fn get_balance_returns_zero_for_new_wallet() {
        let svc = make_service();
        let wallet = WalletId::new();
        // ensure_wallet is needed before balance query
        // (WalletService delegates to WalletManager which calls get_balance directly)
        let balance = svc.get_balance(wallet).unwrap();
        assert_eq!(balance.rjoules, 0);
    }

    // REQ: P9-svc-wallet-002 — gas_to_rjoules conversion
    #[test]
    fn gas_to_rjoules_conversion() {
        let svc = make_service();
        // Default gas_per_rjoule = 1000
        assert_eq!(svc.gas_to_rjoules(0).as_u64(), 0);
        assert_eq!(svc.gas_to_rjoules(500).as_u64(), 1); // rounds up
        assert_eq!(svc.gas_to_rjoules(2000).as_u64(), 2);
    }

    // REQ: P9-svc-wallet-003 — rjoules_to_gas conversion
    #[test]
    fn rjoules_to_gas_conversion() {
        let svc = make_service();
        assert_eq!(svc.rjoules_to_gas(RJoule::new(0)), 0);
        assert_eq!(svc.rjoules_to_gas(RJoule::new(5)), 5000);
    }

    // REQ: P9-svc-wallet-007 — estimate_withdrawal_fee returns positive fee
    #[tokio::test]
    async fn estimate_withdrawal_fee_returns_positive_fee() {
        let svc = make_service();
        let actor = WebID::from_persona(b"wallet-service-test");
        let fee = svc
            .estimate_withdrawal_fee(&actor, ChainId::Solana)
            .await
            .expect("fee estimate");
        assert!(fee.rjoules > 0);
        assert!(fee.usdc_micro > 0);
        assert!(fee.native_units > 0.0);
    }

    // REQ: P9-svc-wallet-008 — actor continuity from service request to adapter-originated chain_error span
    #[tokio::test]
    async fn withdraw_propagates_actor_into_adapter_chain_error_span() {
        set_test_master_key();
        let sink = Arc::new(CaptureSink::default());

        let mut chains: HashMap<ChainId, Arc<dyn ChainPort>> = HashMap::new();
        chains.insert(
            ChainId::Solana,
            Arc::new(FailingActorChain {
                sink: Arc::clone(&sink) as Arc<dyn NuEventSink>,
            }),
        );

        let svc =
            build_service_with_harness(Arc::clone(&sink), chains, None, Arc::new(StaticPriceFeed));

        let wallet = WalletId::new();
        svc.ensure_wallet(wallet).expect("ensure wallet");

        let actor = WebID::from_persona(b"svc-wallet-actor");
        let err = svc
            .withdraw(
                &actor,
                wallet,
                RJoule::ZERO,
                "some_destination",
                ChainId::Solana,
                PrivacyMode::Transparent,
            )
            .await
            .expect_err("forced adapter failure should bubble up");
        assert!(matches!(err, ServiceError::Wallet { .. }));

        assert_event_actor(&sink, "submit_signed_tx", &actor);
    }

    // REQ: P9-svc-wallet-009 — actor continuity for fee-estimation error span
    #[tokio::test]
    async fn estimate_fee_error_span_preserves_request_actor() {
        set_test_master_key();
        let sink = Arc::new(CaptureSink::default());
        let svc = build_service_with_harness(
            Arc::clone(&sink),
            Default::default(),
            None,
            Arc::new(FailingPriceFeed),
        );

        let actor = WebID::from_persona(b"svc-fee-actor");
        let err = svc
            .estimate_withdrawal_fee(&actor, ChainId::Solana)
            .await
            .expect_err("forced price feed failure should bubble up");
        assert!(matches!(err, ServiceError::Wallet { .. }));

        assert_event_actor(&sink, "estimate_withdrawal_fee", &actor);
    }

    // REQ: P9-svc-wallet-010 — actor continuity for shielded withdraw adapter error span
    #[tokio::test]
    async fn shielded_withdraw_error_span_preserves_request_actor() {
        set_test_master_key();
        let sink = Arc::new(CaptureSink::default());
        let svc = build_service_with_harness(
            Arc::clone(&sink),
            Default::default(),
            Some(Arc::new(FailingActorPrivacy {
                sink: Arc::clone(&sink) as Arc<dyn NuEventSink>,
            }) as Arc<dyn hkask_wallet::PrivacyPort>),
            Arc::new(StaticPriceFeed),
        );

        let wallet = WalletId::new();
        svc.ensure_wallet(wallet).expect("ensure wallet");

        let actor = WebID::from_persona(b"svc-shielded-actor");
        let err = svc
            .withdraw(
                &actor,
                wallet,
                RJoule::ZERO,
                "some_destination",
                ChainId::Hinkal,
                PrivacyMode::Shielded,
            )
            .await
            .expect_err("forced privacy adapter failure should bubble up");
        assert!(matches!(err, ServiceError::Wallet { .. }));

        assert_event_actor(&sink, "privacy_submit_signed_tx", &actor);
    }

    // REQ: P9-svc-wallet-004 — create_key produces valid material
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

    // REQ: P9-svc-wallet-005 — list_keys returns created keys
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

    // REQ: P9-svc-wallet-006 — revoke_key removes from active list
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
