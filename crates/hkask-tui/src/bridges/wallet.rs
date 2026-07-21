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

#[derive(Debug, Clone)]
pub struct WalletReady {
    pub rjoules: u64,
    pub usdc_micro: u64,
    pub gas_equivalent: u64,
    pub gas_per_rjoule: u64,
    pub transactions: Vec<WalletTxSummary>,
    pub transaction_count: u64,
}

#[derive(Debug, Clone)]
pub enum WalletSnapshot {
    Unavailable { reason: String },
    Ready(WalletReady),
    Failed { error: String },
}

/// Trait for querying wallet state without exposing wallet service dependencies.
pub trait WalletDataBridge: Send + Sync {
    fn snapshot(&self, transaction_limit: usize) -> WalletSnapshot;
}

/// A mock implementation for TUI development and testing.
pub struct MockWalletBridge {
    pub rjoules: u64,
    pub usdc_micro: u64,
    pub gas_equiv: u64,
    pub gas_per_rj: u64,
    pub txs: Vec<WalletTxSummary>,
}

impl Default for MockWalletBridge {
    fn default() -> Self {
        Self::new()
    }
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
    fn snapshot(&self, transaction_limit: usize) -> WalletSnapshot {
        WalletSnapshot::Ready(WalletReady {
            rjoules: self.rjoules,
            usdc_micro: self.usdc_micro,
            gas_equivalent: self.gas_equiv,
            gas_per_rjoule: self.gas_per_rj,
            transactions: self.txs.iter().take(transaction_limit).cloned().collect(),
            transaction_count: self.txs.len() as u64,
        })
    }
}
