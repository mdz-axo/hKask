//! Chain-related wallet types — ChainId, PrivacyMode, deposit/transaction identifiers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::id::WalletId;

// ── ChainId — supported blockchain networks ────────────────────────────────────

/// Supported blockchain networks for deposits and withdrawals.
///
/// # Extensibility `[OUGHT-DECL]`
/// Adding a new chain requires: a variant here, a `ChainPort` implementation,
/// a treasury key derivation context, and a storage migration.
/// This is intentional — adding a chain is an architectural commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainId {
    /// Solana — SPL USDC, ~$0.00001/tx, 400ms blocks, Hinkal privacy planned
    Solana,
    /// Hedera — HTS USDC, $0.0001/tx fixed, 2s blocks, deterministic finality
    Hedera,
    /// Hinkal — privacy-preserving shielded transactions across multiple chains
    Hinkal,
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainId::Solana => write!(f, "solana"),
            ChainId::Hedera => write!(f, "hedera"),
            ChainId::Hinkal => write!(f, "hinkal"),
        }
    }
}

impl FromStr for ChainId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "solana" => Ok(ChainId::Solana),
            "hedera" => Ok(ChainId::Hedera),
            "hinkal" => Ok(ChainId::Hinkal),
            other => Err(format!("unknown chain: {other}")),
        }
    }
}

// ── PrivacyMode — deposit and API key privacy level ────────────────────────────

/// Deposit and API key privacy mode.
///
/// # Semantic distinction from bool `[OUGHT-DECL]`
/// `PrivacyMode::Transparent` and `PrivacyMode::Shielded` carry meaning.
/// A bare `bool` (`is_private: true`) would be "boolean blindness" —
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrivacyMode {
    /// Direct on-chain deposit/withdrawal — visible to public explorers
    Transparent,
    /// Via Hinkal Shielded Pool — wallet addresses and amounts not visible on-chain
    Shielded,
}

impl fmt::Display for PrivacyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrivacyMode::Transparent => write!(f, "transparent"),
            PrivacyMode::Shielded => write!(f, "shielded"),
        }
    }
}

impl FromStr for PrivacyMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "transparent" => Ok(PrivacyMode::Transparent),
            "shielded" => Ok(PrivacyMode::Shielded),
            other => Err(format!("unknown privacy mode: {other}")),
        }
    }
}

// ── Ed25519PublicKey — type-safe key material ──────────────────────────────────

/// Ed25519 public key — 32 bytes.
///
/// Newtype to prevent accidental mixing with other 32-byte values
/// (hashes, secrets, UUIDs). Conversion to/from `ed25519_dalek::VerifyingKey`
/// lives in `hkask-keystore` where the crypto dependency exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ed25519PublicKey(pub [u8; 32]);

impl Ed25519PublicKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Ed25519PublicKey(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

// ── TxHash — on-chain transaction hash ─────────────────────────────────────────

/// On-chain transaction hash — newtype to prevent confusion with other hex strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxHash(pub String);

impl fmt::Display for TxHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── DepositAddress — validated deposit destination ─────────────────────────────

/// A deposit address with chain and privacy metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepositAddress {
    pub address: String,
    pub chain: ChainId,
    pub privacy_mode: PrivacyMode,
}

impl fmt::Display for DepositAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.chain, self.address)
    }
}

// ── DepositReference — one-time shielded deposit identifier ────────────────────

/// A one-time reference for shielded (Hinkal) deposits.
///
/// # Privacy property `[IS-DECL]`
/// Derived via HKDF from the wallet seed + nonce + expiry.
/// Appears random on-chain but hKask can verify it belongs to a specific wallet.
///
/// # Anti-replay `[OUGHT-DECL]`
/// References are burned on use (consumed in `WalletStore`).
/// References expire after `validity_duration` (default 24h).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositReference {
    /// The reference string (hex-encoded HKDF output)
    pub reference: String,
    pub wallet_id: WalletId,
    pub chain: ChainId,
    /// Random nonce for uniqueness
    pub nonce: [u8; 16],
    /// When this reference expires
    pub expires_at: DateTime<Utc>,
}

impl fmt::Display for DepositReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dep_{} (wallet: {}, chain: {}, expires: {})",
            self.reference, self.wallet_id, self.chain, self.expires_at
        )
    }
}
