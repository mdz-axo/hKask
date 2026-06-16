//! SolanaPort — SPL USDC deposit monitoring and withdrawal on Solana.
//!
//! # Feature gate
//! This module is only compiled when the `solana` feature is enabled.
//! Default builds have zero Solana SDK dependencies.
//!
//! # Dependency constraint `[IS-DECL]`
//! `solana-client` depends on openssl via `solana-tls-utils` (forbidden by hKask).
//! Instead, we use `solana-sdk` (types, no openssl) + `solana-rpc-client-api`
//! (RPC request/response types, no openssl) + raw `reqwest` (rustls) HTTP calls
//! to the Solana JSON-RPC endpoint. This gives full RPC functionality without
//! the openssl dependency chain.
//!
//! # Security `[OUGHT-DECL]`
//! - Does NOT hold treasury keys — signing is delegated to `signing.rs`
//! - HTTP client uses rustls (no openssl)
//! - Deposit addresses derived deterministically from treasury public key

use async_trait::async_trait;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
use hkask_types::wallet::{ChainId, TxHash, WalletError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Signature, transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction as spl_token_ix;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crate::chain::{ChainPort, DepositEvent};

/// USDC mint address on Solana mainnet.
const USDC_MINT_MAINNET: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// USDC mint address on Solana devnet.
const USDC_MINT_DEVNET: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

/// Minimum confirmations required for a deposit to be considered final.
const MIN_CONFIRMATIONS: u64 = 32;

/// HTTP request timeout.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Serializable payload for withdrawal transaction building.
/// Carries the instructions and payer so submit_signed_tx can reconstruct.
#[derive(Serialize, Deserialize)]
struct WithdrawalPayload {
    instructions: Vec<Instruction>,
    payer: Pubkey,
}

/// Solana chain port — SPL USDC on Solana via raw JSON-RPC (rustls).
///
/// # Ownership
/// - Owns a `reqwest::Client` for JSON-RPC HTTP requests
/// - Holds the treasury public key for deposit address derivation
/// - Does NOT hold the treasury private key (signing is external)
pub struct SolanaPort {
    /// HTTP client for JSON-RPC calls (rustls, no openssl).
    client: Client,
    /// Solana JSON-RPC endpoint URL.
    rpc_url: String,
    /// Treasury public key — used to derive deposit addresses.
    treasury_pubkey: Pubkey,
    /// USDC token mint address.
    usdc_mint: Pubkey,
    /// Minimum confirmations for deposit finality.
    min_confirmations: u64,
    /// Optional CNS event sink for chain error span emission.
    event_sink: Option<Arc<dyn NuEventSink>>,
}

impl SolanaPort {
    /// Create a new SolanaPort connected to the given RPC endpoint.
    ///
    /// `treasury_pubkey` is the base58-encoded Ed25519 public key of the
    /// treasury account. Deposit addresses are derived from this key.
    /// `usdc_mint` defaults to mainnet USDC if not specified.
    pub fn new(
        rpc_url: &str,
        treasury_pubkey: &str,
        usdc_mint: Option<&str>,
    ) -> Result<Self, WalletError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| {
                WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                    "failed to build HTTP client: {e}"
                )))
            })?;

        let treasury_pubkey = Pubkey::from_str(treasury_pubkey).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "invalid treasury pubkey: {e}"
            )))
        })?;

        let usdc_mint = Pubkey::from_str(usdc_mint.unwrap_or(USDC_MINT_MAINNET)).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "invalid USDC mint address: {e}"
            )))
        })?;

        Ok(SolanaPort {
            client,
            rpc_url: rpc_url.to_string(),
            treasury_pubkey,
            usdc_mint,
            min_confirmations: MIN_CONFIRMATIONS,
            event_sink: None,
        })
    }

    /// Attach a CNS event sink for chain error span emission.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Emit a CNS chain_error span if an event sink is configured.
    fn emit_chain_error(&self, operation: &str, error_msg: &str) {
        if let Some(ref sink) = self.event_sink {
            let span_obj = Span::new(SpanNamespace::from(CnsSpan::WalletChainError), "error");
            let event = NuEvent::new(
                hkask_types::WebID::new(),
                span_obj,
                Phase::Sense,
                serde_json::json!({
                    "chain": "solana",
                    "operation": operation,
                    "error": error_msg,
                }),
                0,
            );
            if let Err(e) = sink.persist(&event) {
                tracing::warn!(target: "hkask.wallet.solana", error = %e, "Failed to persist CNS chain_error span");
            }
        }
    }

    /// Create a SolanaPort for devnet testing.
    pub fn new_devnet(treasury_pubkey: &str) -> Result<Self, WalletError> {
        Self::new(
            "https://api.devnet.solana.com",
            treasury_pubkey,
            Some(USDC_MINT_DEVNET),
        )
    }

    /// Create a SolanaPort for mainnet.
    pub fn new_mainnet(treasury_pubkey: &str) -> Result<Self, WalletError> {
        Self::new(
            "https://api.mainnet-beta.solana.com",
            treasury_pubkey,
            Some(USDC_MINT_MAINNET),
        )
    }

    // ── JSON-RPC helpers ──────────────────────────────────────────────────────

    /// Make a JSON-RPC call to the Solana endpoint.
    async fn rpc_call(&self, method: &str, params: Value) -> Result<Value, WalletError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("RPC HTTP error ({method}): {e}");
                self.emit_chain_error(method, &msg);
                WalletError::ChainError {
                    chain: ChainId::Solana,
                    message: msg,
                }
            })?;

        let json: Value = resp.json().await.map_err(|e| {
            let msg = format!("RPC JSON parse error ({method}): {e}");
            self.emit_chain_error(method, &msg);
            WalletError::ChainError {
                chain: ChainId::Solana,
                message: msg,
            }
        })?;

        if let Some(err) = json.get("error") {
            let msg = format!("RPC error ({method}): {err}");
            self.emit_chain_error(method, &msg);
            return Err(WalletError::ChainError {
                chain: ChainId::Solana,
                message: msg,
            });
        }

        Ok(json["result"].clone())
    }

    /// Get the latest blockhash.
    async fn get_latest_blockhash(&self) -> Result<solana_sdk::hash::Hash, WalletError> {
        let result = self
            .rpc_call(
                "getLatestBlockhash",
                serde_json::json!([{"commitment": "finalized"}]),
            )
            .await?;

        let blockhash_str =
            result["value"]["blockhash"]
                .as_str()
                .ok_or_else(|| WalletError::ChainError {
                    chain: ChainId::Solana,
                    message: "missing blockhash in RPC response".into(),
                })?;

        solana_sdk::hash::Hash::from_str(blockhash_str).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "invalid blockhash: {e}"
            )))
        })
    }

    /// Get signatures for an address.
    async fn get_signatures_for_address(
        &self,
        address: &Pubkey,
    ) -> Result<Vec<serde_json::Value>, WalletError> {
        let result = self
            .rpc_call(
                "getSignaturesForAddress",
                serde_json::json!([address.to_string(), {"limit": 25, "commitment": "finalized"}]),
            )
            .await?;

        Ok(result.as_array().cloned().unwrap_or_default())
    }

    /// Get a parsed transaction by signature.
    async fn get_transaction(&self, signature: &Signature) -> Result<Value, WalletError> {
        self.rpc_call(
            "getTransaction",
            serde_json::json!([
                signature.to_string(),
                {
                    "encoding": "jsonParsed",
                    "commitment": "finalized",
                    "maxSupportedTransactionVersion": 0,
                },
            ]),
        )
        .await
    }

    /// Send a signed transaction.
    async fn send_transaction(&self, signed_tx_bytes: &[u8]) -> Result<Signature, WalletError> {
        // Encode transaction as base58 (Solana wire format)
        let tx_base58 = solana_sdk::bs58::encode(signed_tx_bytes).into_string();

        let result = self
            .rpc_call(
                "sendTransaction",
                serde_json::json!([tx_base58, {"encoding": "base58", "skipPreflight": false}]),
            )
            .await?;

        let sig_str = result.as_str().ok_or_else(|| WalletError::ChainError {
            chain: ChainId::Solana,
            message: "missing signature in sendTransaction response".into(),
        })?;

        Signature::from_str(sig_str).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "invalid signature from RPC: {e}"
            )))
        })
    }

    /// Get signature statuses.
    async fn get_signature_statuses(&self, signatures: &[Signature]) -> Result<Value, WalletError> {
        let sig_strings: Vec<String> = signatures.iter().map(|s| s.to_string()).collect();
        self.rpc_call(
            "getSignatureStatuses",
            serde_json::json!([sig_strings, {"searchTransactionHistory": true}]),
        )
        .await
    }

    /// Derive a deposit address from the treasury key + derivation index.
    ///
    /// Uses PDA (Program Derived Address) derivation with the treasury pubkey
    /// as the program address. The address is deterministic — same treasury
    /// key + same index always produces the same address.
    fn derive_pda(&self, index: u64) -> Result<Pubkey, WalletError> {
        let index_bytes = index.to_le_bytes();
        let seeds: &[&[u8]] = &[b"hkask-deposit", &index_bytes];
        let (pda, _bump) = Pubkey::find_program_address(seeds, &self.treasury_pubkey);
        Ok(pda)
    }
}

#[async_trait]
impl ChainPort for SolanaPort {
    fn chain_id(&self) -> ChainId {
        ChainId::Solana
    }

    fn derive_deposit_address(&self, index: u64) -> Result<String, WalletError> {
        let pda = self.derive_pda(index)?;
        Ok(pda.to_string())
    }

    async fn monitor_deposits(
        &self,
        addresses: &[String],
    ) -> Result<Vec<DepositEvent>, WalletError> {
        let mut events = Vec::new();

        for addr_str in addresses {
            let addr = Pubkey::from_str(addr_str).map_err(|e| {
                WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                    "invalid address {addr_str}: {e}"
                )))
            })?;

            // Get recent signatures for this address
            let sigs = self.get_signatures_for_address(&addr).await?;

            for sig_info in sigs {
                let confirmations: u64 = sig_info
                    .get("confirmations")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                if confirmations < self.min_confirmations {
                    continue;
                }

                let sig_str = sig_info["signature"].as_str().unwrap_or("");
                if sig_str.is_empty() {
                    continue;
                }

                let sig = match Signature::from_str(sig_str) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                // Get the full transaction to parse token transfers
                let tx = match self.get_transaction(&sig).await {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                // Parse SPL token transfers from transaction metadata
                if let Some(meta) = tx.get("meta") {
                    let pre_balances = meta["preTokenBalances"].as_array();
                    let post_balances = meta["postTokenBalances"].as_array();

                    if let (Some(pre), Some(post)) = (pre_balances, post_balances) {
                        for (i, pre_balance) in pre.iter().enumerate() {
                            let mint = pre_balance["mint"].as_str().unwrap_or("");
                            if mint != self.usdc_mint.to_string() {
                                continue;
                            }

                            let pre_amount = pre_balance["uiTokenAmount"]["amount"]
                                .as_str()
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);

                            let post_amount = post
                                .get(i)
                                .and_then(|b| {
                                    b["uiTokenAmount"]["amount"]
                                        .as_str()
                                        .and_then(|s| s.parse::<f64>().ok())
                                })
                                .unwrap_or(0.0);

                            let delta = post_amount - pre_amount;
                            if delta > 0.0 {
                                let amount_usdc_micro = (delta * 1_000_000.0) as u64;
                                if amount_usdc_micro > 0 {
                                    let block_time = tx["blockTime"]
                                        .as_i64()
                                        .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                                        .unwrap_or_else(chrono::Utc::now);

                                    events.push(DepositEvent {
                                        tx_hash: TxHash(sig_str.to_string()),
                                        from_address: "unknown".into(),
                                        to_address: addr_str.clone(),
                                        amount_usdc_micro,
                                        confirmations,
                                        block_time,
                                    });
                                }
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
        to_address: &str,
        amount_usdc_micro: u64,
    ) -> Result<Vec<u8>, WalletError> {
        let destination = Pubkey::from_str(to_address).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "invalid destination address: {e}"
            )))
        })?;

        // Get the treasury's Associated Token Account (ATA) for USDC
        let treasury_ata = get_associated_token_address(&self.treasury_pubkey, &self.usdc_mint);
        let dest_ata = get_associated_token_address(&destination, &self.usdc_mint);

        let mut instructions = Vec::new();

        // Create destination ATA if it doesn't exist.
        // The ATA program is idempotent — if the ATA already exists,
        // this instruction is a no-op (it won't fail).
        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &self.treasury_pubkey, // payer
                &destination,          // owner
                &self.usdc_mint,       // mint
                &spl_token::id(),      // token program
            );
        instructions.push(create_ata_ix);

        // Build SPL token transfer instruction
        let transfer_ix = spl_token_ix::transfer(
            &spl_token::id(),
            &treasury_ata,
            &dest_ata,
            &self.treasury_pubkey,
            &[],
            amount_usdc_micro,
        )
        .map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "failed to build transfer instruction: {e}"
            )))
        })?;
        instructions.push(transfer_ix);

        // Serialize instructions + payer for signing.rs to sign.
        // The full transaction with blockhash is assembled at submission time.
        let payload = WithdrawalPayload {
            instructions,
            payer: self.treasury_pubkey,
        };
        Ok(bincode::serialize(&payload).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "failed to serialize withdrawal payload: {e}"
            )))
        })?)
    }

    async fn submit_signed_tx(&self, signed_tx_bytes: &[u8]) -> Result<TxHash, WalletError> {
        // The signing.rs module appends the Ed25519 signature (64 bytes) to the payload.
        if signed_tx_bytes.len() < 64 {
            let msg = "signed transaction too short".to_string();
            self.emit_chain_error("submit_signed_tx", &msg);
            return Err(WalletError::Infra(
                hkask_types::InfrastructureError::Database(msg),
            ));
        }

        let (payload_bytes, sig_bytes) = signed_tx_bytes.split_at(signed_tx_bytes.len() - 64);
        let payload: WithdrawalPayload = bincode::deserialize(payload_bytes).map_err(|e| {
            let msg = format!("failed to deserialize withdrawal payload: {e}");
            self.emit_chain_error("submit_signed_tx", &msg);
            WalletError::Infra(hkask_types::InfrastructureError::Database(msg))
        })?;

        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(sig_bytes);
        let signature = Signature::from(sig_arr);

        // Get fresh blockhash
        let blockhash = self.get_latest_blockhash().await?;

        // Build message and unsigned transaction
        let message = solana_sdk::message::Message::new_with_blockhash(
            &payload.instructions,
            Some(&payload.payer),
            &blockhash,
        );
        let mut tx = Transaction::new_unsigned(message);
        tx.signatures.push(signature);

        // Serialize the full transaction for submission
        let full_tx_bytes = bincode::serialize(&tx).map_err(|e| {
            let msg = format!("failed to serialize transaction: {e}");
            self.emit_chain_error("submit_signed_tx", &msg);
            WalletError::Infra(hkask_types::InfrastructureError::Database(msg))
        })?;

        // Submit via RPC
        let rpc_sig = self.send_transaction(&full_tx_bytes).await?;
        Ok(TxHash(rpc_sig.to_string()))
    }

    async fn confirmations(&self, tx_hash: &TxHash) -> Result<u64, WalletError> {
        let sig = Signature::from_str(&tx_hash.0).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "invalid signature: {e}"
            )))
        })?;

        let result = self.get_signature_statuses(&[sig]).await?;
        let statuses = result["value"].as_array();

        Ok(statuses
            .and_then(|arr| arr.first())
            .and_then(|s| s.get("confirmations"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0))
    }
}

// ── Integration tests (manual — requires Solana devnet) ────────────────────────
//
// These tests validate the full withdrawal flow against Solana devnet.
// They require:
//   1. A funded treasury keypair on devnet with USDC
//   2. Network access to api.devnet.solana.com
//
// Run manually with:
//   cargo test -p hkask-wallet --features solana -- solana_integration --ignored
//
// Set environment variables:
//   SOLANA_TREASURY_PUBKEY=<base58 pubkey>
//   HKASK_MASTER_KEY=<32-byte hex seed>

#[cfg(test)]
#[cfg(feature = "solana")]
mod integration_tests {
    use super::*;
    use crate::signing;

    /// Build a SolanaPort for devnet testing.
    fn devnet_port() -> SolanaPort {
        let pubkey = std::env::var("SOLANA_TREASURY_PUBKEY")
            .unwrap_or_else(|_| "11111111111111111111111111111111".to_string());
        SolanaPort::new_devnet(&pubkey).expect("Failed to create devnet SolanaPort")
    }

    // REQ: solana-int-001 — build_withdrawal_tx produces valid serialized payload
    #[test]
    fn build_withdrawal_tx_produces_valid_payload() {
        let port = devnet_port();
        let dest = "2xNpeZTxL7oZTD3B8mGV6KqjyrV8qN1XJqY7B8Z9nK1W"; // random valid base58
        let payload_bytes = port
            .build_withdrawal_tx(dest, 1_000_000) // 1 USDC
            .expect("build_withdrawal_tx should succeed");

        // Payload should contain 2 instructions (create ATA + transfer)
        let payload: WithdrawalPayload =
            bincode::deserialize(&payload_bytes).expect("payload should deserialize");
        assert_eq!(
            payload.instructions.len(),
            2,
            "should have create ATA + transfer"
        );
        assert_eq!(payload.payer, port.treasury_pubkey);
    }

    // REQ: solana-int-002 — full withdrawal flow round-trips through signing
    #[test]
    fn withdrawal_payload_signing_roundtrip() {
        // SAFETY: test-only — sets master key env var in isolated test process.
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }

        let port = devnet_port();
        let dest = "2xNpeZTxL7oZTD3B8mGV6KqjyrV8qN1XJqY7B8Z9nK1W";
        let payload_bytes = port
            .build_withdrawal_tx(dest, 1_000_000)
            .expect("build_withdrawal_tx");

        // Sign the payload
        let signature =
            signing::sign_withdrawal(ChainId::Solana, &payload_bytes).expect("sign_withdrawal");
        assert_eq!(signature.len(), 64, "Ed25519 signature is 64 bytes");

        // Combine payload + signature (as submit_signed_tx expects)
        let mut signed_tx = payload_bytes;
        signed_tx.extend_from_slice(&signature);

        // Verify the combined format is parseable
        assert!(signed_tx.len() > 64);
        let (payload_part, sig_part) = signed_tx.split_at(signed_tx.len() - 64);
        let _payload: WithdrawalPayload =
            bincode::deserialize(payload_part).expect("roundtrip deserialize");
        assert_eq!(sig_part.len(), 64);
    }

    // REQ: solana-int-003 — submit_signed_tx against devnet (ignored — needs funded treasury)
    #[test]
    #[ignore = "requires funded treasury on Solana devnet with USDC"]
    fn submit_withdrawal_to_devnet() {
        // SAFETY: test-only
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }

        let port = devnet_port();
        // Use a known devnet USDC holder as destination
        let dest = std::env::var("SOLANA_TEST_DESTINATION")
            .unwrap_or_else(|_| "2xNpeZTxL7oZTD3B8mGV6KqjyrV8qN1XJqY7B8Z9nK1W".to_string());
        let amount = 100; // 0.0001 USDC (minimum test amount)

        let payload_bytes = port
            .build_withdrawal_tx(&dest, amount)
            .expect("build_withdrawal_tx");
        let signature = signing::sign_withdrawal(ChainId::Solana, &payload_bytes).expect("sign");

        let mut signed_tx = payload_bytes;
        signed_tx.extend_from_slice(&signature);

        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let tx_hash = rt
            .block_on(port.submit_signed_tx(&signed_tx))
            .expect("submit_signed_tx should return tx hash");

        println!("Withdrawal submitted: {}", tx_hash.0);
        println!(
            "Check on Solana Explorer: https://explorer.solana.com/tx/{}?cluster=devnet",
            tx_hash.0
        );

        // Wait a few seconds and check confirmations
        std::thread::sleep(std::time::Duration::from_secs(5));
        let confs = rt
            .block_on(port.confirmations(&tx_hash))
            .expect("confirmations");
        println!("Confirmations: {confs}");
    }
}
