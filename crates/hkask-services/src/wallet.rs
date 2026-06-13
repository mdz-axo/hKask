//! WalletService — Composes WalletManager, ApiKeyIssuer, and CNS integration.
//!
//! Provides a clean interface for CLI and API surfaces. Hides the internal
//! `Arc<>` sharing pattern so callers don't repeat boilerplate at every call site.
//!
//! # Composition
//! - `WalletManager` — balance, deposits, withdrawals, gas conversion
//! - `ApiKeyIssuer` — API key creation, revocation, listing
//! - `CyberneticsLoop` (optional) — CNS wallet budget registration

use std::sync::Arc;

use hkask_cns::CyberneticsLoop;
use hkask_types::wallet::{
    ApiKeyCapability, ApiKeyId, ApiKeyMaterial, ChainId, DepositAddress, DepositReference,
    PrivacyMode, RJoule, TxHash, WalletBalance, WalletId, WalletTransaction,
};
use hkask_wallet::{ApiKeyIssuer, WalletManager};
use tokio::sync::RwLock;

use crate::ServiceError;

/// Service for wallet operations — balance, deposits, withdrawals, API keys.
///
/// Wraps `WalletManager` and `ApiKeyIssuer` behind a clean interface.
/// Optionally integrates with CNS for wallet-backed energy budget registration.
/// Constructed during startup — never created directly by surfaces.
#[derive(Clone)]
pub struct WalletService {
    manager: Arc<WalletManager>,
    issuer: Arc<ApiKeyIssuer>,
    /// Optional CNS loop for registering wallet-backed budgets.
    cybernetics: Option<Arc<RwLock<CyberneticsLoop>>>,
}

impl WalletService {
    /// Create a new WalletService from its components.
    pub fn new(manager: Arc<WalletManager>, issuer: Arc<ApiKeyIssuer>) -> Self {
        Self {
            manager,
            issuer,
            cybernetics: None,
        }
    }

    /// Attach a CyberneticsLoop for CNS wallet budget registration.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_cybernetics(mut self, loop_: Arc<RwLock<CyberneticsLoop>>) -> Self {
        self.cybernetics = Some(loop_);
        self
    }

    // ── Balance ──────────────────────────────────────────────────────────────

    /// Get the current rJoule balance for a wallet.
    pub fn get_balance(&self, wallet_id: WalletId) -> Result<WalletBalance, ServiceError> {
        self.manager
            .get_balance(wallet_id)
            .map_err(|e| ServiceError::Wallet(e.to_string()))
    }

    /// Check if a wallet can afford a given rJoule cost.
    pub fn can_afford(&self, wallet_id: WalletId, cost_rj: RJoule) -> Result<bool, ServiceError> {
        self.manager
            .can_afford(wallet_id, cost_rj)
            .map_err(|e| ServiceError::Wallet(e.to_string()))
    }

    /// Ensure a wallet row exists (idempotent — creates if missing).
    pub fn ensure_wallet(&self, wallet_id: WalletId) -> Result<(), ServiceError> {
        self.manager
            .ensure_wallet(wallet_id)
            .map_err(|e| ServiceError::Wallet(e.to_string()))
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
            .map_err(|e| ServiceError::Wallet(e.to_string()))
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
            .map_err(|e| ServiceError::Wallet(e.to_string()))
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
            .map_err(|e| ServiceError::Wallet(e.to_string()))
    }

    // ── Withdrawal ───────────────────────────────────────────────────────────

    /// Withdraw rJoules as USDC to a user's primary wallet address.
    pub async fn withdraw(
        &self,
        wallet_id: WalletId,
        amount_rj: RJoule,
        to_address: &str,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<TxHash, ServiceError> {
        self.manager
            .withdraw(wallet_id, amount_rj, to_address, chain, privacy)
            .await
            .map_err(|e| ServiceError::Wallet(e.to_string()))
    }

    // ── API Keys ─────────────────────────────────────────────────────────────

    /// Create a new API key with the specified limits, scope, and purpose.
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
            .map_err(|e| ServiceError::Wallet(e.to_string()))
    }

    /// Revoke an API key. Returns unspent rJoules to the wallet.
    pub fn revoke_key(&self, key_id: ApiKeyId) -> Result<(), ServiceError> {
        self.issuer
            .revoke_key(key_id)
            .map_err(|e| ServiceError::Wallet(e.to_string()))
    }

    /// List active (non-revoked) API keys for a wallet.
    pub fn list_keys(&self, wallet_id: WalletId) -> Result<Vec<ApiKeyCapability>, ServiceError> {
        self.issuer
            .list_keys(wallet_id)
            .map_err(|e| ServiceError::Wallet(e.to_string()))
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
            .ok_or_else(|| ServiceError::Wallet("CyberneticsLoop not attached to WalletService — call with_cybernetics() during construction".into()))?;
        let budget = hkask_cns::WalletBackedBudget::new(wallet_id, Arc::clone(&self.manager));
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
            .map_err(|e| ServiceError::Wallet(e.to_string()))
    }

    /// Release an encumbrance, returning unspent rJoules to the wallet.
    pub fn release_encumbrance(&self, key_id: ApiKeyId) -> Result<(), ServiceError> {
        self.manager
            .release_encumbrance(key_id)
            .map_err(|e| ServiceError::Wallet(e.to_string()))
    }

    /// Atomically consume rJoules from an API key's encumbrance.
    pub fn consume_gas(&self, key_id: ApiKeyId, gas_rj: RJoule) -> Result<(), ServiceError> {
        self.manager
            .consume(key_id, gas_rj)
            .map_err(|e| ServiceError::Wallet(e.to_string()))
    }

    /// Get the encumbrance for an API key.
    pub fn get_encumbrance(
        &self,
        key_id: ApiKeyId,
    ) -> Result<Option<hkask_types::wallet::Encumbrance>, ServiceError> {
        self.manager
            .get_encumbrance(key_id)
            .map_err(|e| ServiceError::Wallet(e.to_string()))
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
                "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
            );
        }
        let db = in_memory_db();
        let store = Arc::new(WalletStore::new(db.conn_arc()));
        let config = WalletConfig::default();
        let manager = Arc::new(
            WalletManager::build(config, Arc::clone(&store), Default::default(), None).unwrap(),
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
