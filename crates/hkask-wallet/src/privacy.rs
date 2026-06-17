//! PrivacyPort — abstract interface for shielded deposits and withdrawals.
//!
//! # Implementations
//! - `HinkalPort` — Hinkal Shared Privacy Protocol (feature-gated: "hinkal")
//!
//! # Graceful degradation `[OUGHT-DECL]`
//! When Hinkal is not deployed on a chain, `available_for_chain()` returns false.
//! Shielded operations return `PrivacyUnavailable` error. Transparent path works normally.

use async_trait::async_trait;

use chrono::{DateTime, Utc};
use hkask_types::WebID;
use hkask_types::wallet::{ChainId, TxHash, WalletError, WalletId};

/// A shielded transfer detected in the privacy pool.
#[derive(Debug, Clone)]
pub struct ShieldedTransfer {
    /// zkSNARK commitment hash
    pub commitment: String,
    /// Sender's shielded address
    pub from_shielded: String,
    /// Our (hKask's) shielded address
    pub to_shielded: String,
    /// Amount in micro-USDC (1 = 0.000001 USDC)
    pub amount_usdc_micro: u64,
    /// Settlement chain (e.g., Solana for Hinkal)
    pub chain: ChainId,
    /// Deposit reference memo (if provided)
    pub memo: Option<String>,
    pub block_time: DateTime<Utc>,
}

/// Abstract interface for privacy-preserving deposits and withdrawals.
///
/// # Hinkal integration `[IS-DECL]`
/// Hinkal uses a Shielded Pool with zkSNARKs (Groth16), stealth addresses,
/// and relayers. Users shield assets into the pool, transfer privately,
/// and unshield to public addresses. hKask monitors pool events for deposits.
///
/// # Security `[OUGHT-DECL]`
/// Implementations verify relayer responses independently where possible.
/// Quantstamp audit finding: no integrity check on `encryptedOutputs`.
/// hKask's deposit reference scheme provides a second factor.

#[async_trait::async_trait]
pub trait PrivacyPort: Send + Sync {
    /// hKask's own shielded address in the privacy pool.
    fn our_shielded_address(&self) -> Result<String, WalletError>;

    /// Generate a shielded deposit address for a user wallet.
    fn shielded_deposit_address(&self, wallet_id: WalletId) -> Result<String, WalletError>;

    /// Monitor the Shielded Pool for incoming transfers to our shielded address.
    /// Decrypts `encryptedOutputs` from pool events, extracts memos.
    async fn monitor_shielded_transfers(
        &self,
        actor: &WebID,
    ) -> Result<Vec<ShieldedTransfer>, WalletError>;

    /// Build an unsigned shielding transaction (for private withdrawals).
    /// Signing happens in the isolated `signing.rs` module.
    fn build_shield_tx(
        &self,
        amount_usdc_micro: u64,
        chain: ChainId,
    ) -> Result<Vec<u8>, WalletError>;

    /// Build an unsigned unshielding transaction (to a public address).
    fn build_unshield_tx(
        &self,
        to_public: &str,
        amount_usdc_micro: u64,
    ) -> Result<Vec<u8>, WalletError>;

    /// Submit a signed transaction through the privacy layer.
    async fn submit_signed_tx(
        &self,
        actor: &WebID,
        signed_tx_bytes: &[u8],
    ) -> Result<TxHash, WalletError>;

    /// Check if the privacy layer is available for a given chain.
    fn available_for_chain(&self, chain: ChainId) -> bool;
}
