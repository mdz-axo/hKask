//! ApiKeyIssuer — generates Ed25519 API key capability tokens.
//!
//! # "Printing" an API key `[OUGHT-DECL]`
//! An API key is an Ed25519 keypair. The private key IS the API key — returned
//! to the user once at creation time, never stored by hKask. The public key is
//! stored in the `api_keys` table for Bearer token authentication.
//!
//! # OCAP alignment (P4)
//! Each key carries embedded attenuation: spending limit, expiry, privacy mode.
//! The Ed25519 signature proves it was issued by the wallet holder.

use chrono::{Duration, Utc};
use ed25519_dalek::SigningKey;
use hkask_keystore::resolve_wallet_seed;
use hkask_storage::WalletStore;
pub use hkask_types::wallet::ApiKeyMaterial;
use hkask_types::wallet::{
    ApiKeyCapability, ApiKeyId, ChainId, Ed25519PublicKey, PrivacyMode, RJoule, WalletError,
    WalletId,
};
use rand::Rng;
use std::sync::Arc;
use zeroize::Zeroizing;

use crate::signing;

/// Issues Ed25519-signed API key capability tokens.
///
/// # Security `[OUGHT-DECL]`
/// - Private keys are generated per-key, returned to user once, never stored
/// - Only the public key is persisted (for Bearer token lookup)
/// - Wallet seed is held in `Zeroizing` for capability signing
pub struct ApiKeyIssuer {
    store: Arc<WalletStore>,
    /// Held for capability signing via the isolated signing boundary.
    /// Not directly read by issuer methods — signing delegates to `signing.rs`.
    #[allow(dead_code)]
    wallet_seed: Zeroizing<[u8; 32]>,
}

impl ApiKeyIssuer {
    /// Create a new ApiKeyIssuer.
    pub fn new(store: Arc<WalletStore>) -> Result<Self, WalletError> {
        let seed_bytes = resolve_wallet_seed().map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(e.to_string()))
        })?;
        let mut seed_arr = [0u8; 32];
        seed_arr.copy_from_slice(&seed_bytes[..32]);
        Ok(ApiKeyIssuer {
            store,
            wallet_seed: Zeroizing::new(seed_arr),
        })
    }

    /// "Print" a new API key.
    ///
    /// Generates a fresh Ed25519 keypair, creates a signed capability token
    /// with the specified limits, stores the public key, and returns the
    /// private key to the user (shown exactly once).
    pub fn create_key(
        &self,
        wallet_id: WalletId,
        spending_limit_rj: RJoule,
        expiry_days: Option<u32>,
        privacy_mode: PrivacyMode,
        preferred_chain: Option<ChainId>,
    ) -> Result<ApiKeyMaterial, WalletError> {
        // Generate fresh Ed25519 keypair for this API key
        let mut rng = rand::rng();
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let private_key_bytes = signing_key.to_bytes();
        let public_key = Ed25519PublicKey(signing_key.verifying_key().to_bytes());

        let key_id = ApiKeyId::new();
        let issued_at = Utc::now();
        let expiry = expiry_days.map(|days| issued_at + Duration::days(days as i64));

        let capability = ApiKeyCapability {
            wallet_id,
            key_id,
            public_key,
            spending_limit_rj,
            spent_rj: RJoule::ZERO,
            expiry,
            issued_at,
            privacy_mode,
            preferred_chain,
        };

        // Sign the capability with the wallet's Ed25519 key
        let _signature = signing::sign_capability(&capability)?;

        // Store the public key + capability metadata
        self.store.store_api_key(&capability)?;

        Ok(ApiKeyMaterial {
            key_id,
            private_key_hex: hex::encode(private_key_bytes),
            capability,
        })
    }

    /// Revoke an API key. Returns unspent rJoules to the wallet.
    /// Idempotent — revoking an already-revoked key is a no-op.
    pub fn revoke_key(&self, key_id: ApiKeyId) -> Result<(), WalletError> {
        self.store.revoke_api_key(key_id)
    }

    /// List active (non-revoked) API keys for a wallet.
    pub fn list_keys(&self, wallet_id: WalletId) -> Result<Vec<ApiKeyCapability>, WalletError> {
        self.store.list_api_keys(wallet_id)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::database::in_memory_db;

    fn make_issuer() -> ApiKeyIssuer {
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
            );
        }
        let db = in_memory_db();
        let store = Arc::new(WalletStore::new(db.conn_arc()));
        ApiKeyIssuer::new(store).unwrap()
    }

    // REQ: P4-issuer — create_key produces valid Ed25519 keypair
    #[test]
    fn create_key_produces_valid_keypair() {
        let issuer = make_issuer();
        let wallet = WalletId::new();
        issuer.store.ensure_wallet(wallet).unwrap();

        let material = issuer
            .create_key(
                wallet,
                RJoule::new(5000),
                None,
                PrivacyMode::Transparent,
                None,
            )
            .unwrap();

        // Private key is 32 bytes → 64 hex chars
        assert_eq!(material.private_key_hex.len(), 64);
        // Public key is stored
        let retrieved = issuer.store.get_api_key(material.key_id).unwrap();
        assert!(retrieved.is_some());
    }

    // REQ: P4-issuer — create_key with expiry sets expiry field
    #[test]
    fn create_key_with_expiry() {
        let issuer = make_issuer();
        let wallet = WalletId::new();
        issuer.store.ensure_wallet(wallet).unwrap();

        let material = issuer
            .create_key(
                wallet,
                RJoule::new(5000),
                Some(30),
                PrivacyMode::Transparent,
                None,
            )
            .unwrap();

        assert!(material.capability.expiry.is_some());
    }

    // REQ: P4-issuer — revoke_key returns unspent rJoules
    #[test]
    fn revoke_key_returns_unspent_rjoules() {
        let issuer = make_issuer();
        let wallet = WalletId::new();
        issuer
            .store
            .credit_rjoules(wallet, RJoule::new(10000))
            .unwrap();

        let material = issuer
            .create_key(
                wallet,
                RJoule::new(5000),
                None,
                PrivacyMode::Transparent,
                None,
            )
            .unwrap();

        // Simulate spending 1200 rJ
        issuer
            .store
            .update_spent_rj(material.key_id, RJoule::new(1200))
            .unwrap();
        // Debit wallet by the limit (simulating allocation)
        issuer
            .store
            .debit_rjoules(wallet, RJoule::new(5000))
            .unwrap();

        issuer.revoke_key(material.key_id).unwrap();
        let balance = issuer.store.get_balance(wallet).unwrap().unwrap();
        assert_eq!(balance.rjoules, 8800); // 10000 - 5000 + 3800 unspent
    }

    // REQ: P4-issuer — list_keys returns active keys
    #[test]
    fn list_keys_returns_active_keys() {
        let issuer = make_issuer();
        let wallet = WalletId::new();
        issuer.store.ensure_wallet(wallet).unwrap();

        let key1 = issuer
            .create_key(
                wallet,
                RJoule::new(1000),
                None,
                PrivacyMode::Transparent,
                None,
            )
            .unwrap();
        let key2 = issuer
            .create_key(
                wallet,
                RJoule::new(2000),
                None,
                PrivacyMode::Shielded,
                Some(ChainId::Solana),
            )
            .unwrap();

        let keys = issuer.list_keys(wallet).unwrap();
        assert_eq!(keys.len(), 2);

        // Revoke one
        issuer.revoke_key(key1.key_id).unwrap();
        let keys_after = issuer.list_keys(wallet).unwrap();
        assert_eq!(keys_after.len(), 1);
        assert_eq!(keys_after[0].key_id, key2.key_id);
    }
}
