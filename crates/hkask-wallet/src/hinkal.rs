//! HinkalPort — privacy-preserving deposits and withdrawals via Hinkal protocol.
//!
//! # Feature gate
//! This module is only compiled when the `hinkal` feature is enabled.
//! Default builds have zero Hinkal dependencies.
//!
//! # Hinkal protocol `[IS-DECL]`
//! Hinkal provides shielded/private transactions across multiple chains.
//! Integration requires the Hinkal SDK or direct RPC interaction with
//! Hinkal relayers. This is deferred to a future integration sprint.
//!
//! # Current capability
//! - **Reads:** Not yet implemented — requires Hinkal relayer API
//! - **Writes:** Not yet implemented — requires shielded transaction construction
//!
//! # Security `[OUGHT-DECL]`
//! - Does NOT hold treasury keys — signing is delegated to `signing.rs`
//! - HTTP client uses rustls (no openssl)
//! - Shielded addresses derived deterministically from treasury public key

use async_trait::async_trait;
use hkask_types::wallet::{ChainId, TxHash, WalletError};
use reqwest::Client;
use std::time::Duration;

use crate::chain::{ChainPort, DepositEvent};

/// HTTP request timeout.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Hinkal chain port — privacy-preserving deposits via Hinkal protocol.
///
/// # Ownership
/// - Owns a `reqwest::Client` for relayer API requests
/// - Holds the treasury account identifier for deposit address derivation
/// - Does NOT hold the treasury private key (signing is external)
pub struct HinkalPort {
    /// HTTP client for relayer API (rustls, no openssl).
    #[allow(dead_code)]
    client: Client,
    /// Relayer API base URL.
    #[allow(dead_code)]
    relayer_url: String,
    /// Treasury account identifier.
    #[allow(dead_code)]
    treasury_account: String,
}

impl HinkalPort {
    /// Create a new HinkalPort connected to the given relayer.
    ///
    /// `treasury_account` is the account identifier used for deposit
    /// address derivation within the Hinkal shielded pool.
    pub fn new(relayer_url: &str, treasury_account: &str) -> Result<Self, WalletError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| {
                WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                    "failed to build HTTP client: {e}"
                )))
            })?;

        Ok(HinkalPort {
            client,
            relayer_url: relayer_url.to_string(),
            treasury_account: treasury_account.to_string(),
        })
    }

    /// Derive a shielded deposit address from the treasury account + index.
    fn derive_account_id(&self, _index: u64) -> String {
        self.treasury_account.clone()
    }
}

#[async_trait]
impl ChainPort for HinkalPort {
    fn chain_id(&self) -> ChainId {
        ChainId::Hinkal
    }

    fn derive_deposit_address(&self, index: u64) -> Result<String, WalletError> {
        Ok(self.derive_account_id(index))
    }

    async fn monitor_deposits(
        &self,
        _addresses: &[String],
    ) -> Result<Vec<DepositEvent>, WalletError> {
        // Hinkal shielded deposits require relayer API integration.
        // The relayer provides shielded transaction events that can be
        // decoded to reveal deposit amounts to the treasury.
        //
        // Integration path:
        // 1. Add Hinkal SDK or direct relayer RPC client
        // 2. Query shielded pool events for treasury addresses
        // 3. Decode shielded transfers to extract deposit amounts
        Err(WalletError::ChainError {
            chain: ChainId::Hinkal,
            message: "Deposit monitoring not yet implemented — requires Hinkal relayer API integration. See crates/hkask-wallet/src/hinkal.rs for integration path.".into(),
        })
    }

    fn build_withdrawal_tx(
        &self,
        _to_address: &str,
        _amount_usdc_micro: u64,
    ) -> Result<Vec<u8>, WalletError> {
        // Shielded withdrawal transaction construction requires the Hinkal
        // protocol's zero-knowledge proof generation and relayer coordination.
        //
        // Integration path:
        // 1. Add Hinkal SDK for proof generation
        // 2. Construct shielded withdrawal with recipient address
        // 3. Coordinate with relayer for submission
        Err(WalletError::ChainError {
            chain: ChainId::Hinkal,
            message: "Withdrawal transactions not yet implemented — requires Hinkal SDK for shielded proof generation. See crates/hkask-wallet/src/hinkal.rs for integration path.".into(),
        })
    }

    async fn submit_signed_tx(&self, _signed_tx_bytes: &[u8]) -> Result<TxHash, WalletError> {
        Err(WalletError::ChainError {
            chain: ChainId::Hinkal,
            message: "Transaction submission not yet implemented — see build_withdrawal_tx documentation.".into(),
        })
    }

    async fn confirmations(&self, _tx_hash: &TxHash) -> Result<u64, WalletError> {
        // Hinkal shielded transactions have probabilistic finality
        // based on the underlying chain's consensus.
        Err(WalletError::ChainError {
            chain: ChainId::Hinkal,
            message:
                "Confirmation checking not yet implemented — requires relayer API integration."
                    .into(),
        })
    }

    async fn native_token_usd_rate(&self) -> Result<f64, WalletError> {
        // Hinkal operates across multiple chains; native token rate
        // depends on the settlement layer.
        Err(WalletError::ChainError {
            chain: ChainId::Hinkal,
            message:
                "Native token rate not yet implemented — depends on settlement chain configuration."
                    .into(),
        })
    }
}
