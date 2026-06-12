//! SolanaPort — SPL USDC deposit monitoring and withdrawal on Solana.
//!
//! # Feature gate
//! This module is only compiled when the `solana` feature is enabled.
//! Default builds have zero Solana SDK dependencies.
//!
//! # Security `[OUGHT-DECL]`
//! - Does NOT hold treasury keys — signing is delegated to `signing.rs`
//! - RPC client uses rustls (no openssl)
//! - Deposit addresses derived deterministically from treasury public key

use async_trait::async_trait;
use hkask_types::wallet::{ChainId, TxHash, WalletError};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Signature, signer::Signer,
    transaction::Transaction,
};
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

/// Solana RPC timeout.
const RPC_TIMEOUT_SECS: u64 = 30;

/// Solana chain port — SPL USDC on Solana.
///
/// # Ownership
/// - Owns an `RpcClient` connected to a Solana RPC endpoint
/// - Holds the treasury public key for deposit address derivation
/// - Does NOT hold the treasury private key (signing is external)
pub struct SolanaPort {
    /// Solana RPC client (rustls, no openssl).
    rpc: Arc<RpcClient>,
    /// Treasury public key — used to derive deposit addresses.
    treasury_pubkey: Pubkey,
    /// USDC token mint address.
    usdc_mint: Pubkey,
    /// Minimum confirmations for deposit finality.
    min_confirmations: u64,
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
        let rpc =
            RpcClient::new_with_timeout(rpc_url.to_string(), Duration::from_secs(RPC_TIMEOUT_SECS));

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
            rpc: Arc::new(rpc),
            treasury_pubkey,
            usdc_mint,
            min_confirmations: MIN_CONFIRMATIONS,
        })
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

    /// Derive a deposit address from the treasury key + derivation index.
    ///
    /// Uses PDA (Program Derived Address) derivation: finds a valid bump seed
    /// that produces an off-curve address. The address is deterministic —
    /// same treasury key + same index always produces the same address.
    fn derive_pda(&self, index: u64) -> Result<Pubkey, WalletError> {
        let index_bytes = index.to_le_bytes();
        // Try bump seeds from 255 down to 0 (standard PDA derivation)
        for bump in (0..=255).rev() {
            let seeds: &[&[u8]] = &[b"hkask-deposit", &index_bytes, &[bump]];
            if let Ok((pda, _bump)) = Pubkey::find_program_address(seeds, &self.treasury_pubkey) {
                // find_program_address already returns the canonical bump
                return Ok(pda);
            }
        }
        Err(WalletError::Infra(
            hkask_types::InfrastructureError::Database(
                "failed to derive PDA for deposit address".into(),
            ),
        ))
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
            let sigs = self.rpc.get_signatures_for_address(&addr).map_err(|e| {
                WalletError::ChainError {
                    chain: ChainId::Solana,
                    message: format!("RPC error getting signatures: {e}"),
                }
            })?;

            for sig_info in sigs {
                // Skip if already at sufficient confirmations (optimization)
                if sig_info.confirmations.unwrap_or(0) < self.min_confirmations {
                    continue;
                }

                // Get the full transaction to parse token transfers
                let tx = self
                    .rpc
                    .get_transaction(
                        &sig_info.signature,
                        solana_client::rpc_config::RpcTransactionConfig {
                            encoding: Some(
                                solana_transaction_status::UiTransactionEncoding::JsonParsed,
                            ),
                            commitment: Some(
                                solana_sdk::commitment_config::CommitmentConfig::finalized(),
                            ),
                            max_supported_transaction_version: Some(0),
                        },
                    )
                    .map_err(|e| WalletError::ChainError {
                        chain: ChainId::Solana,
                        message: format!("RPC error getting transaction: {e}"),
                    })?;

                // Parse SPL token transfers from transaction metadata
                if let Some(meta) = tx.transaction.meta {
                    // Check for USDC token transfers in post-token-balances
                    for (i, pre_balance) in meta.pre_token_balances.iter().enumerate() {
                        if pre_balance.mint == self.usdc_mint.to_string() {
                            let post_balance = meta.post_token_balances.get(i);
                            let pre_amount = pre_balance
                                .ui_token_amount
                                .amount
                                .parse::<f64>()
                                .unwrap_or(0.0);
                            let post_amount = post_balance
                                .and_then(|b| b.ui_token_amount.amount.parse::<f64>().ok())
                                .unwrap_or(0.0);

                            let delta = post_amount - pre_amount;
                            if delta > 0.0 {
                                // This is a deposit (USDC received)
                                let amount_usdc_micro = (delta * 1_000_000.0) as u64;
                                if amount_usdc_micro > 0 {
                                    let block_time = tx
                                        .block_time
                                        .map(|ts| chrono::DateTime::from_timestamp(ts, 0))
                                        .flatten()
                                        .unwrap_or_else(chrono::Utc::now);

                                    events.push(DepositEvent {
                                        tx_hash: TxHash(sig_info.signature.to_string()),
                                        from_address: "unknown".into(), // parsed from tx later
                                        to_address: addr_str.clone(),
                                        amount_usdc_micro,
                                        confirmations: sig_info.confirmations.unwrap_or(0),
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

        // Get the treasury's USDC token account
        // The treasury pubkey owns an Associated Token Account (ATA) for USDC
        let treasury_ata = spl_token::solana_program::program_pack::Pack::unpack_unchecked;
        let treasury_ata = spl_associated_token_account::get_associated_token_address(
            &self.treasury_pubkey,
            &self.usdc_mint,
        );

        // Build SPL token transfer instruction
        let transfer_ix = spl_token_ix::transfer(
            &spl_token::id(),
            &treasury_ata,
            &spl_associated_token_account::get_associated_token_address(
                &destination,
                &self.usdc_mint,
            ),
            &self.treasury_pubkey,
            &[],
            amount_usdc_micro,
        )?;

        // Get recent blockhash
        let blockhash = self
            .rpc
            .get_latest_blockhash()
            .map_err(|e| WalletError::ChainError {
                chain: ChainId::Solana,
                message: format!("RPC error getting blockhash: {e}"),
            })?;

        // Build unsigned transaction
        let tx = Transaction::new_signed_with_payer(
            &[transfer_ix],
            Some(&self.treasury_pubkey),
            &[], // no signers yet — signing happens in signing.rs
            blockhash,
        );

        // Serialize to bytes for signing
        Ok(bincode::serialize(&tx).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "failed to serialize transaction: {e}"
            )))
        })?)
    }

    async fn submit_signed_tx(&self, signed_tx_bytes: &[u8]) -> Result<TxHash, WalletError> {
        // Deserialize the signed transaction
        let tx: Transaction = bincode::deserialize(signed_tx_bytes).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "failed to deserialize signed transaction: {e}"
            )))
        })?;

        // Submit and confirm
        let signature =
            self.rpc
                .send_and_confirm_transaction(&tx)
                .map_err(|e| WalletError::ChainError {
                    chain: ChainId::Solana,
                    message: format!("RPC error submitting transaction: {e}"),
                })?;

        Ok(TxHash(signature.to_string()))
    }

    async fn confirmations(&self, tx_hash: &TxHash) -> Result<u64, WalletError> {
        let sig = Signature::from_str(&tx_hash.0).map_err(|e| {
            WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                "invalid signature: {e}"
            )))
        })?;

        let statuses =
            self.rpc
                .get_signature_statuses(&[sig])
                .map_err(|e| WalletError::ChainError {
                    chain: ChainId::Solana,
                    message: format!("RPC error getting signature status: {e}"),
                })?;

        Ok(statuses.value[0]
            .as_ref()
            .and_then(|s| s.confirmations)
            .unwrap_or(0))
    }

    async fn native_token_usd_rate(&self) -> Result<f64, WalletError> {
        // SOL/USD rate — for production, use a price feed oracle.
        // For now, return a reasonable estimate.
        // TODO: Integrate with a price feed (Pyth, Switchboard, or CoinGecko API)
        Ok(150.0) // ~$150 SOL
    }
}
