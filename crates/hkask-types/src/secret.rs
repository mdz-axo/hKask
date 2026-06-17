//! Secret derivation — Loop 6 (Cybernetics): key management
//!
//! Secret derivation contexts are used by the Cybernetics Access Guard (6.1)
//! and the keystore for capability token signing and verification.

use serde::{Deserialize, Serialize};

/// Loop: Cybernetics
/// Declarative reference to a secret's source.
///
/// Each variant specifies how to resolve a secret value at runtime.
/// The resolution priority (in `hkask_keystore::resolve`) is:
///
/// 1. `Env` — read from an environment variable
/// 2. `Keychain` — read from the OS keychain
/// 3. `Derived` — deterministically derive from a master key via HKDF-SHA256
/// 4. `Generated` — random bytes (⚠️ not reproducible across restarts; avoid in production)
///
/// \[NORMATIVE\] **Prefer `Derived` over `Generated`** for any secret that must survive
/// process restarts (HMAC signing keys, capability tokens, etc.) (P4 — Clear Boundaries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretRef {
    /// Read secret from an environment variable.
    Env(String),

    /// Read secret from the OS keychain by service name.
    Keychain(String),

    /// Deterministically derive a sub-key from a master key using HKDF-SHA256.
    ///
    /// The `master_key_env` field names the environment variable (or keychain key)
    /// holding the master key. The `context` string provides domain separation —
    /// different contexts produce different sub-keys from the same master key.
    ///
    /// Resolution chain: env var → keychain → HKDF-SHA256(master_key, context)
    Derived {
        master_key_env: String,
        context: String,
    },

    /// Generate random bytes of the given length.
    ///
    /// **⚠️ Not reproducible.** Every call produces a different value.
    /// Use only for one-time secrets (salts, nonces) that don't need to
    /// survive a restart. For persistent secrets, use `Derived`.
    ///
    /// \[NORMATIVE\] Only available in debug builds — never use in production (P4 — Clear Boundaries).
    #[cfg(debug_assertions)]
    Generated(u32),
}

impl SecretRef {
    /// Reference a secret stored in an environment variable.
    pub fn env(name: &str) -> Self {
        Self::Env(name.to_string())
    }

    /// Reference a secret stored in the OS keychain.
    pub fn keychain(service: &str) -> Self {
        Self::Keychain(service.to_string())
    }

    /// Deterministically derive a sub-key from a master key using HKDF-SHA256.
    ///
    /// The `master_key_env` names the env var or keychain key that holds
    /// the master key. The `context` provides domain separation:
    /// different contexts yield cryptographically independent sub-keys
    /// from the same master key.
    pub fn derived(master_key_env: &str, context: &str) -> Self {
        Self::Derived {
            master_key_env: master_key_env.to_string(),
            context: context.to_string(),
        }
    }

    /// Generate random bytes. **Not reproducible across restarts.**
    /// \[NORMATIVE\] Only available in debug builds — never use in production (P4 — Clear Boundaries).
    #[cfg(debug_assertions)]
    pub fn generated(length: u32) -> Self {
        Self::Generated(length)
    }
}

/// Well-known derivation contexts for hKask internal secrets.
///
/// Each context produces a cryptographically independent 256-bit sub-key
/// from the same master key. This ensures that a compromise of one
/// derived secret does not compromise the others or the master key.
pub mod derivation_contexts {
    /// A2A (Agent-to-Agent Protocol) HMAC signing secret.
    pub const A2A_SECRET: &str = "hkask:a2a-secret";

    /// API capability token signing key.
    pub const CAPABILITY_KEY: &str = "hkask:capability-key";

    /// MCP security gateway HMAC key.
    pub const MCP_SECURITY_KEY: &str = "hkask:mcp-security-key";

    /// MCP dispatch and tool invocation signing key.
    /// Used for DelegationToken minting in tool dispatch paths
    /// (/invoke, tool-augmented chat). Derived from the A2A master key
    /// via HKDF-SHA256, same chain as resolve_a2a_secret().
    pub const MCP_SECRET: &str = "hkask:mcp-secret";

    /// OCAP capability token signing secret.
    pub const OCAP_SECRET: &str = "hkask:ocap-secret";

    /// Master key environment variable name.
    pub const MASTER_KEY_ENV: &str = "HKASK_MASTER_KEY";

    /// Solana treasury keypair derivation context.
    pub const TREASURY_SOLANA: &str = "hkask:treasury-solana";

    /// Hedera treasury keypair derivation context.
    pub const TREASURY_HEDERA: &str = "hkask:treasury-hedera";

    /// Hinkal treasury keypair derivation context.
    pub const TREASURY_HINKAL: &str = "hkask:treasury-hinkal";

    /// Wallet seed for HD derivation, deposit references, and API key signing.
    pub const WALLET_SEED: &str = "hkask:wallet-seed";
}

/// A `Vec<u8>` wrapper that zeroizes its contents on drop.
///
/// \[DECLARATIVE\] Used for secrets (A2A keys, capability tokens) that must not
/// linger in memory after use. Derefs to `&[u8]` for pass-through
/// to functions that accept byte slices.
#[derive(Clone)]
pub struct ZeroizingSecret(Vec<u8>);

impl ZeroizingSecret {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::ops::Deref for ZeroizingSecret {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for ZeroizingSecret {
    fn drop(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.0);
    }
}
