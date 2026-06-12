//! HederaPort — HTS USDC deposit monitoring and withdrawal on Hedera.
//!
//! # Feature gate
//! This module is only compiled when the `hedera` feature is enabled.
//! Default builds have zero Hedera dependencies.
//!
//! # SDK constraint `[IS-DECL]`
//! The `hiero-sdk` crate (v0.45.0) depends on `openssl`, which is forbidden
//! by hKask's design constraints (rustls only). The `hiero-sdk-proto` crate
//! (protobuf definitions) has no openssl dependency and could be used with
//! `tonic` + `rustls` for full gRPC transaction submission. This is deferred
//! to a future integration sprint.
//!
//! # Current capability
//! - **Reads:** Mirror node REST API (account info, transaction history, token balances)
//! - **Writes:** Not yet implemented — requires gRPC transaction submission
//!
//! # Security `[OUGHT-DECL]`
//! - Does NOT hold treasury keys — signing is delegated to `signing.rs`
//! - HTTP client uses rustls (no openssl)
//! - Account IDs derived deterministically from treasury public key

use async_trait::async_trait;
use hkask_types::wallet::{ChainId, TxHash, WalletError};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::chain::{ChainPort, DepositEvent};

/// Hedera mainnet mirror node REST API endpoint.
const MIRROR_NODE_MAINNET: &str = "https://mainnet-public.mirrornode.hedera.com";

/// Hedera testnet mirror node REST API endpoint.
const MIRROR_NODE_TESTNET: &str = "https://testnet.mirrornode.hedera.com";

/// HTS USDC token ID on Hedera mainnet.
const USDC_TOKEN_MAINNET: &str = "0.0.456858";

/// HTS USDC token ID on Hedera testnet.
const USDC_TOKEN_TESTNET: &str = "0.0.2276698";

/// HTTP request timeout.
const REQUEST_TIMEOUT_SECS: u64 = 30;

// ── Mirror node REST API response types ──────────────────────────────────────

#[derive(Debug, Deserialize)]
struct MirrorTransactionsResponse {
    transactions: Vec<MirrorTransaction>,
}

#[derive(Debug, Deserialize)]
struct MirrorTransaction {
    transaction_id: String,
    name: String,
    transfers: Option<Vec<MirrorTransfer>>,
    #[serde(rename = "consensus_timestamp")]
    consensus_timestamp: String,
}

#[derive(Debug, Deserialize)]
struct MirrorTransfer {
    account: String,
    amount: i64,
    #[serde(rename = "token_id")]
    token_id: Option<String>,
}

// These types are defined for future use (account balance queries).
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MirrorAccountResponse {
    account: String,
    balance: Option<MirrorBalance>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MirrorBalance {
    balance: u64,
    tokens: Option<Vec<MirrorTokenBalance>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MirrorTokenBalance {
    #[serde(rename = "token_id")]
    token_id: String,
    balance: u64,
}

/// Hedera chain port — HTS USDC on Hedera via mirror node REST API.
///
/// # Ownership
/// - Owns a `reqwest::Client` for mirror node HTTP requests
/// - Holds the treasury account ID for deposit address derivation
/// - Does NOT hold the treasury private key (signing is external)
pub struct HederaPort {
    /// HTTP client for mirror node REST API (rustls, no openssl).
    client: Client,
    /// Mirror node base URL.
    mirror_node_url: String,
    /// Treasury account ID (0.0.XXXXX format).
    treasury_account: String,
    /// HTS USDC token ID.
    usdc_token: String,
}

impl HederaPort {
    /// Create a new HederaPort connected to the given mirror node.
    ///
    /// `treasury_account` is the Hedera account ID (0.0.XXXXX) of the
    /// treasury. Deposit addresses are derived from this account.
    /// `usdc_token` defaults to mainnet USDC if not specified.
    pub fn new(
        mirror_node_url: &str,
        treasury_account: &str,
        usdc_token: Option<&str>,
    ) -> Result<Self, WalletError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| {
                WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                    "failed to build HTTP client: {e}"
                )))
            })?;

        Ok(HederaPort {
            client,
            mirror_node_url: mirror_node_url.to_string(),
            treasury_account: treasury_account.to_string(),
            usdc_token: usdc_token.unwrap_or(USDC_TOKEN_MAINNET).to_string(),
        })
    }

    /// Create a HederaPort for testnet.
    pub fn new_testnet(treasury_account: &str) -> Result<Self, WalletError> {
        Self::new(
            MIRROR_NODE_TESTNET,
            treasury_account,
            Some(USDC_TOKEN_TESTNET),
        )
    }

    /// Create a HederaPort for mainnet.
    pub fn new_mainnet(treasury_account: &str) -> Result<Self, WalletError> {
        Self::new(
            MIRROR_NODE_MAINNET,
            treasury_account,
            Some(USDC_TOKEN_MAINNET),
        )
    }

    /// Derive a deposit address from the treasury account + index.
    ///
    /// Hedera account IDs are in the format `shard.realm.num`.
    /// For deposit addresses, we use the treasury account directly —
    /// Hedera doesn't have PDAs like Solana. Multiple indices can be
    /// supported by creating sub-accounts or using memo fields.
    fn derive_account_id(&self, _index: u64) -> String {
        // For now, all deposits go to the treasury account directly.
        // Multi-account derivation (using HKDF from treasury key) is a
        // future enhancement.
        self.treasury_account.clone()
    }
}

#[async_trait]
impl ChainPort for HederaPort {
    fn chain_id(&self) -> ChainId {
        ChainId::Hedera
    }

    fn derive_deposit_address(&self, index: u64) -> Result<String, WalletError> {
        Ok(self.derive_account_id(index))
    }

    async fn monitor_deposits(
        &self,
        addresses: &[String],
    ) -> Result<Vec<DepositEvent>, WalletError> {
        let mut events = Vec::new();

        for addr in addresses {
            let url = format!(
                "{}/api/v1/accounts/{}/transactions?limit=25&order=desc&transactiontype=CRYPTOTRANSFER",
                self.mirror_node_url, addr
            );

            let resp = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| WalletError::ChainError {
                    chain: ChainId::Hedera,
                    message: format!("Mirror node HTTP error: {e}"),
                })?;

            if !resp.status().is_success() {
                // Account might not exist yet — skip
                continue;
            }

            let body: MirrorTransactionsResponse =
                resp.json().await.map_err(|e| WalletError::ChainError {
                    chain: ChainId::Hedera,
                    message: format!("Mirror node JSON parse error: {e}"),
                })?;

            for tx in body.transactions {
                // Only process CRYPTOTRANSFER transactions
                if tx.name != "CRYPTOTRANSFER" {
                    continue;
                }

                // Parse transfers to find USDC token transfers TO our address
                if let Some(ref transfers) = tx.transfers {
                    for transfer in transfers {
                        // Check if this is a USDC token transfer to our address
                        if transfer.account == *addr
                            && transfer.amount > 0
                            && transfer.token_id.as_deref() == Some(&self.usdc_token)
                        {
                            // USDC has 6 decimals on Hedera (same as Solana)
                            // Mirror node returns amounts in tinybars (10^-8 HBAR) for HBAR
                            // or in token base units for HTS tokens.
                            // For USDC with 6 decimals, amount is in micro-USDC.
                            let amount_usdc_micro = transfer.amount as u64;

                            if amount_usdc_micro > 0 {
                                let consensus_seconds = tx
                                    .consensus_timestamp
                                    .split('.')
                                    .next()
                                    .and_then(|s| s.parse::<i64>().ok())
                                    .unwrap_or(0);

                                let block_time =
                                    chrono::DateTime::from_timestamp(consensus_seconds, 0)
                                        .unwrap_or_else(chrono::Utc::now);

                                events.push(DepositEvent {
                                    tx_hash: TxHash(tx.transaction_id.clone()),
                                    from_address: transfer.account.clone(),
                                    to_address: addr.clone(),
                                    amount_usdc_micro,
                                    confirmations: 1, // Hedera finality is deterministic
                                    block_time,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }

    fn build_withdrawal_tx(
        &self,
        _to_address: &str,
        _amount_usdc_micro: u64,
    ) -> Result<Vec<u8>, WalletError> {
        // Transaction building requires the Hedera protobuf schema.
        // The `hiero-sdk` crate provides this but depends on openssl (forbidden).
        // The `hiero-sdk-proto` crate (protobuf definitions only) has no openssl
        // dependency and could be used with `tonic` + `rustls` for gRPC submission.
        //
        // Integration path:
        // 1. Add `hiero-sdk-proto` + `tonic` (with rustls TLS) as optional deps
        // 2. Construct `TransactionBody` protobuf with CryptoTransfer
        // 3. Serialize, sign externally, submit via gRPC to consensus node
        Err(WalletError::ChainError {
            chain: ChainId::Hedera,
            message: "Withdrawal transactions not yet implemented — requires hiero-sdk-proto + tonic (rustls) for gRPC submission. See crates/hkask-wallet/src/hedera.rs for integration path.".into(),
        })
    }

    async fn submit_signed_tx(&self, _signed_tx_bytes: &[u8]) -> Result<TxHash, WalletError> {
        Err(WalletError::ChainError {
            chain: ChainId::Hedera,
            message: "Transaction submission not yet implemented — see build_withdrawal_tx documentation.".into(),
        })
    }

    async fn confirmations(&self, tx_hash: &TxHash) -> Result<u64, WalletError> {
        // Hedera has deterministic finality — once a transaction appears
        // in the mirror node, it's final. Check if the transaction exists.
        let url = format!("{}/api/v1/transactions/{}", self.mirror_node_url, tx_hash.0);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WalletError::ChainError {
                chain: ChainId::Hedera,
                message: format!("Mirror node HTTP error: {e}"),
            })?;

        if resp.status().is_success() {
            Ok(1) // Transaction exists → confirmed
        } else {
            Ok(0) // Not found or pending
        }
    }

    async fn native_token_usd_rate(&self) -> Result<f64, WalletError> {
        // HBAR/USD rate — for production, use a price feed.
        // For now, return a reasonable estimate.
        // TODO: Integrate with a price feed (CoinGecko, Hedera mirror node exchange rate API)
        Ok(0.08) // ~$0.08 HBAR
    }
}
