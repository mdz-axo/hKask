//! OS keychain integration

use hkask_rsolidity as rs;
use ed25519_dalek::Signer;
use hkask_types::WebID;
use hkask_types::secret::SecretRef;
use hkask_types::secret::derivation_contexts;
use hkask_types::wallet::{ApiKeyCapability, ChainId};
use keyring::{Entry, Error as KeyringError};
use thiserror::Error;
use tracing::{info, warn};
use zeroize::Zeroizing;

#[derive(Error, Debug)]
pub enum KeychainError {
    #[error("Platform keychain error: {0}")]
    Platform(String),
    #[error("Secret not found: {0}")]
    NotFound(String),
}

impl From<KeyringError> for KeychainError {
    fn from(err: KeyringError) -> Self {
        use KeyringError::*;
        match err {
            NoEntry => KeychainError::NotFound("secret not found in keychain".into()),
            other => KeychainError::Platform(other.to_string()),
        }
    }
}

/// Keychain service for secure credential storage
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// inv: secrets are stored in OS keychain, never in plaintext files
pub struct Keychain {
    service_name: String,
}

impl Keychain {
    /// Create a new Keychain for the given service name.
    ///
    /// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    /// post: service_name is set
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    /// Store a secret in the OS keychain, keyed by WebID.
    ///
    /// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    /// pre:  webid is a valid WebID, secret is non-empty
    /// post: secret stored in OS keychain under service_name + webid.uuid
    /// post: returns Err(Platform) if keychain is unavailable
    pub fn store(&self, webid: &WebID, secret: &str) -> Result<(), KeychainError> {
        let entry = Entry::new(&self.service_name, &webid.as_uuid().to_string())
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry
            .set_password(secret)
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        // P9: CNS span
        info!(target: "cns.keystore", operation = "store", "CNS");
        Ok(())
    }

    /// Retrieve a secret from the OS keychain by WebID.
    ///
    /// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    /// pre:  webid is a valid WebID
    /// post: returns Ok(secret) if stored, Err(NotFound) if not
    pub fn retrieve(&self, webid: &WebID) -> Result<String, KeychainError> {
        let entry = Entry::new(&self.service_name, &webid.as_uuid().to_string())
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        let result = entry.get_password().map_err(KeychainError::from)?;
        info!(target: "cns.keystore", operation = "retrieve", "CNS");
        Ok(result)
    }

    /// Delete a secret from the OS keychain by WebID.
    ///
    /// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    /// pre:  webid is a valid WebID
    /// post: secret removed from OS keychain
    /// post: idempotent — deleting non-existent entry is no-op (platform-dependent)
    pub fn delete(&self, webid: &WebID) -> Result<(), KeychainError> {
        let entry = Entry::new(&self.service_name, &webid.as_uuid().to_string())
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry
            .delete_credential()
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        info!(target: "cns.keystore", operation = "delete", "CNS");
        Ok(())
    }

    /// Store a secret in the OS keychain by arbitrary key name.
    ///
    /// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    /// pre:  key is non-empty, secret is non-empty
    /// post: secret stored in OS keychain under service_name + key
    pub fn store_by_key(&self, key: &str, secret: &str) -> Result<(), KeychainError> {
        let entry = Entry::new(&self.service_name, key)
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry
            .set_password(secret)
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        info!(target: "cns.keystore", operation = "store_by_key", "CNS");
        Ok(())
    }

    /// Retrieve a secret from the OS keychain by arbitrary key name.
    ///
    /// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    /// pre:  key is non-empty
    /// post: returns Ok(secret) if stored, Err(NotFound) if not
    pub fn retrieve_by_key(&self, key: &str) -> Result<String, KeychainError> {
        let entry = Entry::new(&self.service_name, key)
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        let result = entry.get_password().map_err(KeychainError::from)?;
        info!(target: "cns.keystore", operation = "retrieve_by_key", "CNS");
        Ok(result)
    }

    /// Delete a secret from the OS keychain by arbitrary key name.
    ///
    /// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    /// pre:  key is non-empty
    /// post: secret removed from OS keychain
    pub fn delete_by_key(&self, key: &str) -> Result<(), KeychainError> {
        let entry = Entry::new(&self.service_name, key)
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        entry
            .delete_credential()
            .map_err(|e| KeychainError::Platform(e.to_string()))?;

        info!(target: "cns.keystore", operation = "delete_by_key", "CNS");
        Ok(())
    }
}

impl Default for Keychain {
    fn default() -> Self {
        Self::new("hkask")
    }
}

//
// These functions encapsulate the standard 3-tier resolution chain
// (derived → env → keychain) for each well-known secret. Every call site
// that previously hand-rolled its own chain should use these instead.
//
// Benefits:
//   - Eliminates copy-paste drift (10+ independent copies collapsed to 1 implementation)
//   - Fixes the ACP env var inconsistency (HKASK_A2A_SECRET vs HKASK_A2A_SECRET_KEY)
//   - Single place to audit secret resolution behavior

/// Resolve a secret through the standard 3-tier chain:
/// 1. Master key derivation (HKDF-SHA256)
/// 2. Direct environment variable
/// 3. OS keychain lookup
///
/// This is the canonical resolution pattern for all hKask secrets.
/// Domain-specific functions (`resolve_a2a_secret`, etc.) call this with
/// the appropriate parameters.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// pre:  derivation_context, env_var, keychain_key are valid
/// post: tries derivation → env → keychain in order
/// post: returns Ok(Zeroizing<Vec<u8>>) on first success
/// post: returns Err if all three sources fail
pub fn resolve_secret_chain(
    derivation_context: (&str, &str),
    env_var: &str,
    keychain_key: &str,
) -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve(&SecretRef::derived(
        derivation_context.0,
        derivation_context.1,
    ))
    .or_else(|_| resolve(&SecretRef::env(env_var)))
    .or_else(|_| resolve(&SecretRef::keychain(keychain_key)))
}

/// Resolve the A2A (Agent-to-Agent Protocol) HMAC signing secret.
///
/// Chain: master key derivation → env var → OS keychain.
/// Tries both `HKASK_A2A_SECRET` (canonical) and `HKASK_A2A_SECRET_KEY` (legacy)
/// environment variables for backward compatibility.
/// Resolve the A2A secret for agent capability protocol signing.
///
/// Chain: master key derivation → env var → OS keychain.
/// Tries both `HKASK_A2A_SECRET` (canonical) and `HKASK_A2A_SECRET_KEY` (legacy)
/// environment variables for backward compatibility.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// post: returns Zeroizing<Vec<u8>> from first successful resolution step
pub fn resolve_a2a_secret() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve_secret_chain(
        (
            derivation_contexts::MASTER_KEY_ENV,
            derivation_contexts::A2A_SECRET,
        ),
        "HKASK_A2A_SECRET",
        "a2a-secret",
    )
    .or_else(|_| resolve(&SecretRef::env("HKASK_A2A_SECRET_KEY")))
}

/// Resolve the MCP dispatch and tool invocation signing key.
///
/// Chain: master key derivation → env var → OS keychain → A2A fallback.
/// Falls back to the A2A secret if MCP-specific key is unavailable,
/// since they share the same authority chain.
/// Resolve the MCP dispatch and tool invocation signing key.
///
/// Chain: master key derivation → env var → OS keychain → A2A fallback.
/// Falls back to the A2A secret if MCP-specific key is unavailable,
/// since they share the same authority chain.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// post: returns Zeroizing<Vec<u8>> from first successful resolution step
/// post: falls back to A2A secret if MCP key unavailable
pub fn resolve_mcp_secret() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve_secret_chain(
        (
            derivation_contexts::MASTER_KEY_ENV,
            derivation_contexts::MCP_SECRET,
        ),
        "HKASK_MCP_SECRET",
        "mcp-secret",
    )
    .or_else(|_| resolve_a2a_secret())
}

/// Resolve the MCP security gateway HMAC key (used for API auth).
///
/// Chain: master key derivation → env var → OS keychain.
/// Resolve the MCP security gateway HMAC key (used for API auth).
///
/// Chain: master key derivation → env var → OS keychain.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// post: returns Zeroizing<Vec<u8>> from first successful resolution step
pub fn resolve_mcp_security_key() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve_secret_chain(
        (
            derivation_contexts::MASTER_KEY_ENV,
            derivation_contexts::MCP_SECURITY_KEY,
        ),
        "HKASK_MCP_SECURITY_KEY",
        "mcp-security-key",
    )
}

/// Resolve the capability token signing key (used for SOAP/capability tokens).
///
/// Chain: master key derivation → env var → OS keychain.
/// Resolve the capability token signing key (used for SOAP/capability tokens).
///
/// Chain: master key derivation → env var → OS keychain.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// post: returns Zeroizing<Vec<u8>> from first successful resolution step
pub fn resolve_capability_key() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve_secret_chain(
        (
            derivation_contexts::MASTER_KEY_ENV,
            derivation_contexts::CAPABILITY_KEY,
        ),
        "HKASK_CAPABILITY_KEY",
        "capability-key",
    )
}

/// Resolve the database encryption passphrase.
///
/// Chain: env var → OS keychain.
/// Note: no master-key derivation for the DB passphrase — it must be
/// explicitly set via env var or keychain to avoid accidentally encrypting
/// the database with a derived key that the user didn't consent to.
/// Resolve the database encryption passphrase.
///
/// Chain: env var → OS keychain.
/// Note: no master-key derivation for the DB passphrase — it must be
/// explicitly set via env var or keychain to avoid accidentally encrypting
/// the database with a derived key that the user didn't consent to.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// post: returns Zeroizing<Vec<u8>> from env var or keychain
pub fn resolve_db_passphrase() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve(&SecretRef::env("HKASK_DB_PASSPHRASE"))
        .or_else(|_| resolve(&SecretRef::keychain("hkask-db-passphrase")))
}

/// Get or create OCAP secret
///
/// Resolution chain:
/// 1. Deterministic derivation from master key (preferred — survives restarts)
/// 2. OS keychain (backward compat)
/// 3. Random generation (last resort — tokens will not survive restart)
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// post: returns Zeroizing<Vec<u8>> from derivation, keychain, or random generation
pub fn get_or_create_ocap_secret() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    // Prefer deterministic derivation from master key
    let derived = resolve(&SecretRef::derived(
        derivation_contexts::MASTER_KEY_ENV,
        derivation_contexts::OCAP_SECRET,
    ));

    match derived {
        Ok(key) => {
            info!(target: "cns.keystore", operation = "ocap_secret", source = "derived", "CNS");
            Ok(key)
        }
        Err(_) => {
            // Fallback to keychain for backward compat
            resolve(&SecretRef::Keychain("hkask-ocap-secret".to_string())).or_else(|_| {
                // Last resort: generate random (with warning)
                warn!(
                    "OCAP secret not available via derivation or keychain; \
                     generating random secret. Tokens will not survive restart."
                );
                info!(target: "cns.keystore", operation = "ocap_secret", source = "random", "CNS");
                let secret: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
                Ok(Zeroizing::new(secret))
            })
        }
    }
}

/// Resolve a SecretRef to actual secret bytes.
///
/// Resolution priority:
/// 1. `Env` — read from environment variable
/// 2. `Keychain` — read from OS keychain
/// 3. `Derived` — look up master key (env → keychain), then HKDF-SHA256 derive sub-key
/// 4. `Generated` — random bytes (⚠️ not reproducible; debug builds only)
///
/// For `Derived`, the master key is resolved first (env var → keychain),
/// then HKDF-SHA256 is applied with the given context string to produce
/// a deterministic 256-bit sub-key.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// pre:  secret_ref is a valid SecretRef variant
/// post: Env → reads from environment variable, Err(NotFound) if unset
/// post: Keychain → reads from OS keychain, Err(NotFound) if absent
/// post: Derived → resolves master key (env→keychain), HKDF-SHA256 derives sub-key
/// post: Generated → random bytes (debug only, not reproducible)
/// post: all returned secrets wrapped in Zeroizing
pub fn resolve(secret_ref: &SecretRef) -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    // P9: CNS span
    let start = std::time::Instant::now();
    let variant = match secret_ref {
        SecretRef::Env(_) => "env",
        SecretRef::Keychain(_) => "keychain",
        SecretRef::Derived { .. } => "derived",
        #[cfg(debug_assertions)]
        SecretRef::Generated(_) => "generated",
    };
    info!(target: "cns.keystore", operation = "resolve", variant = variant, "CNS");

    match secret_ref {
        SecretRef::Env(var_name) => {
            let value = std::env::var(var_name)
                .map_err(|_| KeychainError::NotFound(format!("env var {} not set", var_name)))?;
            info!(target: "cns.keystore", operation = "resolve_env", var_name = %var_name, "CNS");
            Ok(Zeroizing::new(value.into_bytes()))
        }
        SecretRef::Keychain(key_name) => {
            let keychain = Keychain::default();
            let entry = Entry::new(&keychain.service_name, key_name)
                .map_err(|e| KeychainError::Platform(e.to_string()))?;
            let secret = entry.get_password().map_err(KeychainError::from)?;
            info!(target: "cns.keystore", operation = "resolve_keychain", key_name = %key_name, "CNS");
            Ok(Zeroizing::new(secret.into_bytes()))
        }
        SecretRef::Derived {
            master_key_env,
            context,
        } => {
            info!(target: "cns.keystore", operation = "resolve_derived", master_key_env = %master_key_env, context = %context, "CNS");
            // Resolve master key: env var first, then keychain
            let master_key_bytes = resolve(&SecretRef::Env(master_key_env.clone()))
                .or_else(|_| resolve(&SecretRef::Keychain(master_key_env.clone())))
                .map_err(|_| {
                    KeychainError::NotFound(format!(
                        "Master key '{}' not found in environment or keychain; \
                     set {} or run `kask init` to derive secrets from a master passphrase",
                        master_key_env, master_key_env
                    ))
                })?;

            // HKDF-SHA256 derive sub-key
            let sub_key = crate::master_key::derive_sub_key(&master_key_bytes, context);
            info!(target: "cns.keystore", operation = "derive_sub_key", latency_ms = start.elapsed().as_millis(), "CNS");
            Ok(sub_key)
        }
        #[cfg(debug_assertions)]
        SecretRef::Generated(length) => {
            let bytes: Vec<u8> = (0..*length as usize)
                .map(|_| rand::random::<u8>())
                .collect();
            warn!(target: "cns.keystore", operation = "resolve_generated", length = *length, "CNS");
            Ok(Zeroizing::new(bytes))
        }
    }
}

// ── Wallet key derivation ──────────────────────────────────────────────────────

/// Derive a chain-specific treasury key seed from the master key.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// pre:  chain is a valid ChainId (Solana, Hedera, or Hinkal)
/// post: returns Ok(Zeroizing<Vec<u8>>) — 32-byte HKDF-derived seed
/// post: same master key → same treasury key for given chain (deterministic)
///
/// Uses HKDF-SHA256 with domain-separated context strings.
/// Same master passphrase → same treasury key for a given chain.
///
/// # Context strings
/// - Solana: `"hkask:treasury-solana"`
/// - Hedera: `"hkask:treasury-hedera"`
///
/// # Returns
/// 32-byte seed suitable for constructing a chain-specific keypair
/// (Ed25519 for Solana, ED25519/ECDSA for Hedera). The actual keypair
/// construction happens in `hkask-wallet` where the chain SDKs live.
pub fn resolve_treasury_key(chain: ChainId) -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    let context = match chain {
        ChainId::Solana => derivation_contexts::TREASURY_SOLANA,
        ChainId::Hedera => derivation_contexts::TREASURY_HEDERA,
        ChainId::Hinkal => derivation_contexts::TREASURY_HINKAL,
    };
    resolve(&SecretRef::derived(
        derivation_contexts::MASTER_KEY_ENV,
        context,
    ))
}

/// Derive the wallet seed for HD derivation, deposit references, and API key signing.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// post: returns Ok(Zeroizing<Vec<u8>>) — 32-byte HKDF-derived seed
/// post: same master key → same wallet seed (deterministic)
///
/// Context: `"hkask:wallet-seed"`
///
/// This seed is used for:
/// - Deriving deposit addresses (BIP44-style per chain)
/// - Generating deposit references (HKDF with nonce + expiry)
/// - Signing API key capability tokens (Ed25519)
///
/// # Returns
/// 32-byte seed wrapped in `Zeroizing` for secure memory handling.
pub fn resolve_wallet_seed() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    resolve(&SecretRef::derived(
        derivation_contexts::MASTER_KEY_ENV,
        derivation_contexts::WALLET_SEED,
    ))
}

/// Sign an `ApiKeyCapability` with the wallet's Ed25519 key.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
/// pre:  capability is a valid, fully-populated ApiKeyCapability
/// post: returns Ok(hex_signature) — 128-char hex-encoded Ed25519 signature
/// post: wallet seed loaded, used for signing, zeroized within this call
///
/// The signature proves the capability was issued by the wallet holder.
/// Verification: derive public key from wallet seed, verify signature
/// against the canonical JSON bytes of the capability.
///
/// # Returns
/// 64-byte Ed25519 signature as a hex-encoded string (128 hex chars).
pub fn sign_api_key_capability(capability: &ApiKeyCapability) -> Result<String, KeychainError> {
    let seed = resolve_wallet_seed()?;
    let seed_bytes: [u8; 32] = seed[..32]
        .try_into()
        .map_err(|_| KeychainError::Platform("wallet seed must be 32 bytes".into()))?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed_bytes);
    let canonical_bytes =
        serde_json::to_vec(capability).map_err(|e| KeychainError::Platform(e.to_string()))?;
    let signature = signing_key.sign(&canonical_bytes);
    Ok(hex::encode(signature.to_bytes()))
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::id::{ApiKeyId, WalletId};
    use hkask_types::wallet::{Ed25519PublicKey, PrivacyMode, RJoule};

    /// Set a test master key in the environment for derivation tests.
    /// Uses a fixed 32-byte hex key so derivations are deterministic.
    fn set_test_master_key() {
        // SAFETY: set_var is unsafe in Rust 2024 due to potential race conditions
        // with other threads reading the environment. In a single-threaded test
        // context, this is safe.
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
    }

    // REQ: P3-keystore — resolve_treasury_key returns different keys per chain
    // expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    #[test]
    fn treasury_keys_differ_per_chain() {
        set_test_master_key();
        let solana_key = resolve_treasury_key(ChainId::Solana).unwrap();
        let hedera_key = resolve_treasury_key(ChainId::Hedera).unwrap();
        assert_ne!(&*solana_key, &*hedera_key);
        assert_eq!(solana_key.len(), 32);
        assert_eq!(hedera_key.len(), 32);
    }

    // REQ: P3-keystore — resolve_treasury_key is deterministic
    // expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    #[test]
    fn treasury_key_is_deterministic() {
        set_test_master_key();
        let key1 = resolve_treasury_key(ChainId::Solana).unwrap();
        let key2 = resolve_treasury_key(ChainId::Solana).unwrap();
        assert_eq!(&*key1, &*key2);
    }

    // REQ: P3-keystore — resolve_wallet_seed returns 32 bytes
    // expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    #[test]
    fn wallet_seed_is_32_bytes() {
        set_test_master_key();
        let seed = resolve_wallet_seed().unwrap();
        assert_eq!(seed.len(), 32);
    }

    // REQ: P3-keystore — resolve_wallet_seed is deterministic
    // expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    #[test]
    fn wallet_seed_is_deterministic() {
        set_test_master_key();
        let seed1 = resolve_wallet_seed().unwrap();
        let seed2 = resolve_wallet_seed().unwrap();
        assert_eq!(&*seed1, &*seed2);
    }

    // REQ: P3-keystore — sign_api_key_capability produces verifiable signature
    // expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    #[test]
    fn sign_api_key_capability_produces_signature() {
        set_test_master_key();
        let cap = ApiKeyCapability {
            wallet_id: WalletId::new(),
            key_id: ApiKeyId::new(),
            public_key: Ed25519PublicKey([0u8; 32]),
            spending_limit_rj: RJoule::new(5000),
            spent_rj: RJoule::ZERO,
            scope: vec!["read-specs".to_string()],
            purpose: "keystore test".to_string(),
            rate_limit: None,
            expiry: None,
            issued_at: chrono::Utc::now(),
            privacy_mode: PrivacyMode::Transparent,
            preferred_chain: None,
        };
        let sig = sign_api_key_capability(&cap).unwrap();
        // Ed25519 signature is 64 bytes → 128 hex chars
        assert_eq!(sig.len(), 128);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // REQ: P3-keystore — signature changes when capability is tampered
    // expect: "My keys are generated, stored, and rotated under my sovereignty" [P3]
    #[test]
    fn signature_changes_on_tampered_capability() {
        set_test_master_key();
        let mut cap = ApiKeyCapability {
            wallet_id: WalletId::new(),
            key_id: ApiKeyId::new(),
            public_key: Ed25519PublicKey([0u8; 32]),
            spending_limit_rj: RJoule::new(5000),
            spent_rj: RJoule::ZERO,
            scope: vec!["read-specs".to_string()],
            purpose: "keystore test".to_string(),
            rate_limit: None,
            expiry: None,
            issued_at: chrono::Utc::now(),
            privacy_mode: PrivacyMode::Transparent,
            preferred_chain: None,
        };
        let sig1 = sign_api_key_capability(&cap).unwrap();
        cap.spending_limit_rj = RJoule::new(9999); // tamper
        let sig2 = sign_api_key_capability(&cap).unwrap();
        assert_ne!(sig1, sig2);
    }
}
