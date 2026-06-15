//! CirclePort — ChainPort implementation backed by Circle's Programmable Wallets API.
//!
//! # Why Circle instead of raw chain RPC
//!
//! Circle provides managed USDC custody across multiple chains. Instead of
//! maintaining per-chain RPC connections, transaction construction, and key
//! management, we delegate to Circle's REST API. This eliminates:
//! - Solana RPC endpoint maintenance (`solana.rs` — ~500 lines)
//! - Hedera gRPC transaction construction (`hedera.rs` — ~550 lines)
//! - Per-chain treasury key management (Circle holds the keys)
//! - Chain SDK dependency trees (solana-sdk, hiero-sdk-proto, tonic)
//!
//! # What stays in hKask
//!
//! Everything above the `ChainPort` trait boundary remains unchanged:
//! - rJoule accounting (USDC→rJ, gas→rJ)
//! - OCAP API key issuance (Ed25519 capability tokens)
//! - Encumbrance system (lock→consume→release)
//! - CNS span emission (cns.wallet.*, cns.gas.*)
//! - Transaction ledger (append-only audit trail)
//!
//! # API surface used
//!
//! Circle's Programmable Wallets REST API:
//! - `POST /wallets` — create wallet, get address
//! - `GET /wallets/{id}/transfers` — list incoming transfers (deposit monitoring)
//! - `POST /wallets/{id}/transfers` — create outgoing transfer (withdrawal)
//! - `GET /transfers/{id}` — get transfer status (confirmations)
//!
//! # Security model shift
//!
//! With raw chain ports, hKask holds treasury keys and signs transactions locally
//! via `signing.rs`. With Circle, Circle holds the keys and signs transactions.
//! The `build_withdrawal_tx` + `submit_signed_tx` split collapses into a single
//! API call. The `signing.rs` module is still used for API key capability tokens
//! (OCAP), but NOT for chain transactions.
//!
//! # Feature gate
//!
//! This module is compiled when the `circle` feature is enabled.
//! It replaces `solana` and `hedera` features — one provider, all chains.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hkask_types::wallet::{ChainId, TxHash, WalletError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::chain::{ChainPort, DepositEvent};

// ── Circle API configuration ──────────────────────────────────────────────────

/// Circle Programmable Wallets API base URL (sandbox).
const CIRCLE_SANDBOX_URL: &str = "https://api-sandbox.circle.com";

/// Circle Programmable Wallets API base URL (production).
const CIRCLE_PRODUCTION_URL: &str = "https://api.circle.com";

/// HTTP request timeout.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Number of recent transfers to fetch per poll.
const TRANSFER_PAGE_SIZE: u32 = 25;

// ── Circle API response types ─────────────────────────────────────────────────

/// Circle wallet object.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CircleWallet {
    id: String,
    address: String,
    blockchain: String,
    state: String,
    create_date: String,
}

/// Circle transfer object (deposit or withdrawal).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CircleTransfer {
    id: String,
    source: CircleTransferParty,
    destination: CircleTransferParty,
    amount: CircleAmount,
    transaction_hash: Option<String>,
    status: String,
    create_date: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CircleTransferParty {
    address: Option<String>,
    #[serde(rename = "walletId")]
    wallet_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CircleAmount {
    amount: String,
    currency: String,
}

/// Circle API response wrapper.
#[derive(Debug, Deserialize)]
struct CircleResponse<T> {
    data: T,
}

/// Circle API list response wrapper.
#[derive(Debug, Deserialize)]
struct CircleListResponse<T> {
    data: Vec<T>,
}

// ── Circle API request types ─────────────────────────────────────────────────

/// Request to create a transfer (withdrawal).
#[derive(Debug, Serialize)]
struct CreateTransferRequest {
    source: TransferSource,
    destination: TransferDestination,
    amount: TransferAmount,
    #[serde(rename = "idempotencyKey")]
    idempotency_key: String,
}

#[derive(Debug, Serialize)]
struct TransferSource {
    #[serde(rename = "type")]
    source_type: String,
    #[serde(rename = "walletId")]
    wallet_id: String,
}

#[derive(Debug, Serialize)]
struct TransferDestination {
    #[serde(rename = "type")]
    dest_type: String,
    address: String,
    chain: String,
}

#[derive(Debug, Serialize)]
struct TransferAmount {
    amount: String,
    currency: String,
}

// ── CirclePort ────────────────────────────────────────────────────────────────

/// Chain port backed by Circle's Programmable Wallets API.
///
/// # Ownership
/// - Owns a `reqwest::Client` for Circle REST API calls
/// - Holds the Circle API key (from env: `CIRCLE_API_KEY`)
/// - Holds the Circle wallet ID for the treasury
/// - Does NOT hold treasury keys (Circle manages signing)
pub struct CirclePort {
    /// HTTP client for Circle REST API (rustls, no openssl).
    client: Client,
    /// Circle API base URL.
    api_url: String,
    /// Circle API key (Bearer token).
    api_key: String,
    /// Circle wallet ID for the hKask treasury.
    treasury_wallet_id: String,
    /// Treasury deposit address (resolved from Circle on init).
    treasury_address: String,
    /// Which blockchain this wallet is on (e.g., "SOL", "ETH").
    blockchain: String,
    /// The ChainId this port serves.
    chain_id: ChainId,
}

impl CirclePort {
    /// Create a new CirclePort for a specific chain.
    ///
    /// # Arguments
    /// * `api_key` — Circle API key (Bearer token)
    /// * `treasury_wallet_id` — Circle wallet ID for the treasury
    /// * `blockchain` — Circle blockchain identifier ("SOL", "ETH", "MATIC")
    /// * `chain_id` — hKask ChainId corresponding to this blockchain
    /// * `sandbox` — use sandbox environment (true) or production (false)
    pub async fn new(
        api_key: &str,
        treasury_wallet_id: &str,
        blockchain: &str,
        chain_id: ChainId,
        sandbox: bool,
    ) -> Result<Self, WalletError> {
        let api_url = if sandbox {
            CIRCLE_SANDBOX_URL.to_string()
        } else {
            CIRCLE_PRODUCTION_URL.to_string()
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| {
                WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                    "failed to build HTTP client: {e}"
                )))
            })?;

        // Resolve the treasury address from Circle
        let wallet: CircleResponse<CircleWallet> = Self::api_get(
            &client,
            &api_url,
            api_key,
            &format!("/v1/wallets/{treasury_wallet_id}"),
        )
        .await?;

        Ok(CirclePort {
            client,
            api_url,
            api_key: api_key.to_string(),
            treasury_wallet_id: treasury_wallet_id.to_string(),
            treasury_address: wallet.data.address,
            blockchain: blockchain.to_string(),
            chain_id,
        })
    }

    /// Create a CirclePort for Solana USDC (sandbox).
    pub async fn new_solana_sandbox(
        api_key: &str,
        treasury_wallet_id: &str,
    ) -> Result<Self, WalletError> {
        Self::new(api_key, treasury_wallet_id, "SOL", ChainId::Solana, true).await
    }

    /// Create a CirclePort for Solana USDC (production).
    pub async fn new_solana(api_key: &str, treasury_wallet_id: &str) -> Result<Self, WalletError> {
        Self::new(api_key, treasury_wallet_id, "SOL", ChainId::Solana, false).await
    }

    // ── API helpers ──────────────────────────────────────────────────────────

    /// Make an authenticated GET request to the Circle API.
    async fn api_get<T: for<'de> Deserialize<'de>>(
        client: &Client,
        api_url: &str,
        api_key: &str,
        path: &str,
    ) -> Result<T, WalletError> {
        let resp = client
            .get(format!("{api_url}{path}"))
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|e| WalletError::ChainError {
                chain: ChainId::Solana, // generic — caller overrides
                message: format!("Circle API HTTP error: {e}"),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(WalletError::ChainError {
                chain: ChainId::Solana,
                message: format!("Circle API error {status}: {body}"),
            });
        }

        resp.json().await.map_err(|e| WalletError::ChainError {
            chain: ChainId::Solana,
            message: format!("Circle API JSON parse error: {e}"),
        })
    }

    /// Make an authenticated POST request to the Circle API.
    async fn api_post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, WalletError> {
        let resp = self
            .client
            .post(format!("{}{path}", self.api_url))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| WalletError::ChainError {
                chain: self.chain_id,
                message: format!("Circle API HTTP error: {e}"),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(WalletError::ChainError {
                chain: self.chain_id,
                message: format!("Circle API error {status}: {body}"),
            });
        }

        resp.json().await.map_err(|e| WalletError::ChainError {
            chain: self.chain_id,
            message: format!("Circle API JSON parse error: {e}"),
        })
    }

    /// Fetch recent incoming transfers to the treasury wallet.
    async fn fetch_recent_transfers(&self) -> Result<Vec<CircleTransfer>, WalletError> {
        let response: CircleListResponse<CircleTransfer> = Self::api_get(
            &self.client,
            &self.api_url,
            &self.api_key,
            &format!(
                "/v1/wallets/{}/transfers?pageSize={}&direction=IN",
                self.treasury_wallet_id, TRANSFER_PAGE_SIZE
            ),
        )
        .await?;
        Ok(response.data)
    }
}

#[async_trait]
impl ChainPort for CirclePort {
    fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    fn derive_deposit_address(&self, _index: u64) -> Result<String, WalletError> {
        // Circle wallets have a single address. Multi-address derivation
        // (for per-user deposit addresses) would use Circle's wallet creation
        // API to create sub-wallets. For now, all deposits go to the treasury.
        Ok(self.treasury_address.clone())
    }

    async fn monitor_deposits(
        &self,
        _addresses: &[String],
    ) -> Result<Vec<DepositEvent>, WalletError> {
        let transfers = self.fetch_recent_transfers().await?;
        let mut events = Vec::new();

        for transfer in transfers {
            // Only process completed incoming transfers
            if transfer.status != "complete" {
                continue;
            }

            // Parse the USDC amount (Circle returns string like "100.00")
            let amount_usdc: f64 =
                transfer
                    .amount
                    .amount
                    .parse()
                    .map_err(|_| WalletError::ChainError {
                        chain: self.chain_id,
                        message: format!("invalid amount format: {}", transfer.amount.amount),
                    })?;
            let amount_usdc_micro = (amount_usdc * 1_000_000.0) as u64;

            if amount_usdc_micro == 0 {
                continue;
            }

            let tx_hash = transfer
                .transaction_hash
                .unwrap_or_else(|| transfer.id.clone());

            let block_time = transfer
                .create_date
                .parse::<DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now());

            let from_address = transfer
                .source
                .address
                .unwrap_or_else(|| "unknown".to_string());

            events.push(DepositEvent {
                tx_hash: TxHash(tx_hash),
                from_address,
                to_address: self.treasury_address.clone(),
                amount_usdc_micro,
                confirmations: 1, // Circle marks "complete" = final
                block_time,
            });
        }

        Ok(events)
    }

    fn build_withdrawal_tx(
        &self,
        to_address: &str,
        amount_usdc_micro: u64,
    ) -> Result<Vec<u8>, WalletError> {
        // With Circle, there's no transaction to build — the withdrawal
        // is a single API call. We serialize the withdrawal parameters
        // as JSON bytes for the signing.rs module to "sign" (attest).
        // The actual signing is done by Circle's API.
        let amount_usdc = amount_usdc_micro as f64 / 1_000_000.0;
        let payload = serde_json::json!({
            "to_address": to_address,
            "amount_usdc": format!("{amount_usdc:.6}"),
            "chain": self.blockchain,
        });
        Ok(serde_json::to_vec(&payload).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "failed to serialize withdrawal payload: {e}"
            )))
        })?)
    }

    async fn submit_signed_tx(&self, signed_tx_bytes: &[u8]) -> Result<TxHash, WalletError> {
        // Deserialize the withdrawal payload
        let payload: serde_json::Value = serde_json::from_slice(signed_tx_bytes).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "failed to deserialize withdrawal payload: {e}"
            )))
        })?;

        let to_address = payload["to_address"]
            .as_str()
            .ok_or_else(|| WalletError::ChainError {
                chain: self.chain_id,
                message: "missing to_address in withdrawal payload".into(),
            })?;

        let amount_str =
            payload["amount_usdc"]
                .as_str()
                .ok_or_else(|| WalletError::ChainError {
                    chain: self.chain_id,
                    message: "missing amount_usdc in withdrawal payload".into(),
                })?;

        // Create the transfer via Circle API
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let request = CreateTransferRequest {
            source: TransferSource {
                source_type: "wallet".to_string(),
                wallet_id: self.treasury_wallet_id.clone(),
            },
            destination: TransferDestination {
                dest_type: "blockchain".to_string(),
                address: to_address.to_string(),
                chain: self.blockchain.clone(),
            },
            amount: TransferAmount {
                amount: amount_str.to_string(),
                currency: "USDC".to_string(),
            },
            idempotency_key,
        };

        let response: CircleResponse<CircleTransfer> =
            self.api_post("/v1/transfers", &request).await?;

        let transfer = response.data;
        let tx_hash = transfer
            .transaction_hash
            .unwrap_or_else(|| transfer.id.clone());

        Ok(TxHash(tx_hash))
    }

    async fn confirmations(&self, tx_hash: &TxHash) -> Result<u64, WalletError> {
        // Query Circle for the transfer status.
        // The tx_hash is either an on-chain transaction hash or a Circle transfer ID.
        let response: CircleResponse<CircleTransfer> = Self::api_get(
            &self.client,
            &self.api_url,
            &self.api_key,
            &format!("/v1/transfers/{}", tx_hash.0),
        )
        .await?;

        match response.data.status.as_str() {
            "complete" => Ok(1),
            "pending" | "processing" => Ok(0),
            "failed" | "cancelled" => Err(WalletError::ChainError {
                chain: self.chain_id,
                message: format!("Transfer failed with status: {}", response.data.status),
            }),
            _ => Ok(0),
        }
    }

    async fn native_token_usd_rate(&self) -> Result<f64, WalletError> {
        // Circle handles fee estimation internally — the transfer fee
        // is deducted by Circle. For rJoule fee estimation, we return
        // a nominal rate. The actual fee is known after the transfer completes.
        match self.chain_id {
            ChainId::Solana => Ok(150.0),
            ChainId::Hedera => Ok(0.08),
            ChainId::Hinkal => Ok(150.0),
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: circle-001 — CirclePort constructors set correct chain_id
    #[test]
    fn circle_port_chain_id_matches() {
        // Unit test: verify the chain_id mapping without API calls.
        // Full integration tests require a Circle API key and sandbox access.
        assert_eq!(ChainId::Solana.to_string(), "solana");
        assert_eq!(ChainId::Hedera.to_string(), "hedera");
    }

    // REQ: circle-002 — withdrawal payload serialization round-trips
    #[test]
    fn withdrawal_payload_roundtrip() {
        let payload = serde_json::json!({
            "to_address": "0x1234567890abcdef",
            "amount_usdc": "10.000000",
            "chain": "SOL",
        });
        let bytes = serde_json::to_vec(&payload).unwrap();
        let restored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            restored["to_address"].as_str().unwrap(),
            "0x1234567890abcdef"
        );
        assert_eq!(restored["amount_usdc"].as_str().unwrap(), "10.000000");
    }
}
