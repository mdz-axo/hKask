//! WalletDataBridge — trait for wallet data in the TUI.
//!
//! Provides rJoule balance, gas conversion, and transaction history
//! to the Wallet window. Implemented by the CLI via WalletService.

use std::sync::Arc;

/// Summary of a single wallet transaction for display.
#[derive(Debug, Clone)]
pub struct WalletTxSummary {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// RJoule delta (positive = credit, negative = debit)
    pub rjoules_delta: i64,
    /// Human-readable transaction type (e.g. "Deposit", "Spend", "Withdrawal")
    pub tx_type: String,
    /// Balance after this transaction
    pub balance_after: u64,
    /// Optional detail (e.g. tool name for Spend, chain for Deposit)
    pub detail: Option<String>,
}

/// Trait for querying wallet state.
///
/// Designed to keep the TUI crate free of wallet service dependencies.
/// All domain types are simple value structs that the Wallet window
/// can render without importing `hkask-wallet` or `hkask-wallet-types`.
pub trait WalletDataBridge: Send + Sync {
    /// Returns (rjoules_balance, usdc_equivalent_micro, gas_equivalent).
    fn wallet_balance(&self) -> (u64, u64, u64);

    /// Returns the most recent transactions, newest first.
    fn wallet_transactions(&self, limit: usize) -> Vec<WalletTxSummary>;

    /// Gas-to-rJoule conversion rate (default: 1000 gas per rJ).
    fn gas_per_rjoule(&self) -> u64;

    /// Total number of transactions for this wallet.
    fn transaction_count(&self) -> u64;
}

/// A mock implementation for TUI development and testing.
pub struct MockWalletBridge {
    pub rjoules: u64,
    pub usdc_micro: u64,
    pub gas_equiv: u64,
    pub gas_per_rj: u64,
    pub txs: Vec<WalletTxSummary>,
}

impl MockWalletBridge {
    pub fn new() -> Self {
        Self {
            rjoules: 0,
            usdc_micro: 0,
            gas_equiv: 0,
            gas_per_rj: 1000,
            txs: Vec::new(),
        }
    }

    pub fn with_balance(mut self, rjoules: u64, usdc_micro: u64) -> Self {
        self.rjoules = rjoules;
        self.usdc_micro = usdc_micro;
        self.gas_equiv = rjoules.saturating_mul(self.gas_per_rj);
        self
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl WalletDataBridge for MockWalletBridge {
    fn wallet_balance(&self) -> (u64, u64, u64) {
        (self.rjoules, self.usdc_micro, self.gas_equiv)
    }

    fn wallet_transactions(&self, limit: usize) -> Vec<WalletTxSummary> {
        self.txs.iter().take(limit).cloned().collect()
    }

    fn gas_per_rjoule(&self) -> u64 {
        self.gas_per_rj
    }

    fn transaction_count(&self) -> u64 {
        self.txs.len() as u64
    }
}
