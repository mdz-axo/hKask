//! PriceFeed — abstract interface for fetching native token USD exchange rates.
//!
//! # Implementations
//! - `StaticPriceFeed` — hardcoded rates for testing/development
//! - Future: `CoinGeckoPriceFeed` — live rates from CoinGecko API
//!
//! # Design
//! `PriceFeed` is a trait for capability, not a base class. Each implementation
//! is a standalone struct. The `WalletManager` uses it for fee estimation
//! during withdrawals.

use async_trait::async_trait;
use hkask_types::wallet::{ChainId, WalletError};

/// Exchange rate for a native token in USD.
#[derive(Debug, Clone, Copy)]
pub struct ExchangeRate {
    /// USD per 1 native token (e.g., 150.0 = 1 SOL = $150)
    pub usd_per_token: f64,
    /// When this rate was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Abstract interface for fetching native token USD exchange rates.
///
/// Used by `WalletManager` to estimate withdrawal fees in rJoules.
#[async_trait]
pub trait PriceFeed: Send + Sync {
    /// Get the current USD exchange rate for a chain's native token.
    async fn get_rate(&self, chain: ChainId) -> Result<ExchangeRate, WalletError>;
}

/// Static price feed with hardcoded rates for development and testing.
///
/// Rates are approximate and should not be used for production withdrawals.
pub struct StaticPriceFeed;

impl StaticPriceFeed {
    /// Create a new static price feed.
    pub fn new() -> Self {
        Self
    }

    /// Hardcoded USD rate per native token.
    fn hardcoded_rate(chain: ChainId) -> f64 {
        match chain {
            ChainId::Solana => 150.0, // ~$150/SOL
            ChainId::Hedera => 0.08,  // ~$0.08/HBAR
            ChainId::Hinkal => 150.0, // Uses SOL as settlement layer
        }
    }
}

impl Default for StaticPriceFeed {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PriceFeed for StaticPriceFeed {
    async fn get_rate(&self, chain: ChainId) -> Result<ExchangeRate, WalletError> {
        Ok(ExchangeRate {
            usd_per_token: Self::hardcoded_rate(chain),
            updated_at: chrono::Utc::now(),
        })
    }
}

// ── Fee estimation ────────────────────────────────────────────────────────────

/// Estimated withdrawal fee in rJoules.
///
/// Calculated from: native token fee × USD rate ÷ USDC-per-rJoule rate.
#[derive(Debug, Clone, Copy)]
pub struct WithdrawalFee {
    /// Fee in rJoules
    pub rjoules: u64,
    /// Fee in native token units (for display)
    pub native_units: f64,
    /// Fee in micro-USDC (for display)
    pub usdc_micro: u64,
}

/// Estimate the withdrawal fee for a given chain.
///
/// Uses the chain's typical transaction fee in native tokens, converts to USD
/// via the price feed, then converts to rJoules using the rj_per_usdc rate.
///
/// # Fee model
/// - Solana: ~0.000005 SOL per SPL token transfer (~$0.00075)
/// - Hedera: ~$0.001 USD fixed per HTS transfer
/// - Hinkal: ~0.000005 SOL (settlement layer) + relayer fee
pub fn estimate_withdrawal_fee(
    chain: ChainId,
    rate: &ExchangeRate,
    rj_per_usdc: u64,
) -> WithdrawalFee {
    let native_fee = match chain {
        ChainId::Solana => 0.000005, // ~0.000005 SOL for SPL transfer
        ChainId::Hedera => 0.0125,   // ~$0.001 / $0.08 = 0.0125 HBAR
        ChainId::Hinkal => 0.000010, // ~2× Solana fee (shielded + relayer)
    };

    let usd_fee = native_fee * rate.usd_per_token;
    let usdc_micro_fee = (usd_fee * 1_000_000.0) as u64;
    let rj_fee = (usdc_micro_fee as u128 * rj_per_usdc as u128 / 1_000_000) as u64;

    WithdrawalFee {
        rjoules: rj_fee.max(1), // minimum 1 rJ
        native_units: native_fee,
        usdc_micro: usdc_micro_fee.max(1),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: wallet-price-001 — static price feed returns correct rates
    #[test]
    fn static_price_feed_returns_expected_rates() {
        assert!((StaticPriceFeed::hardcoded_rate(ChainId::Solana) - 150.0).abs() < f64::EPSILON);
        assert!((StaticPriceFeed::hardcoded_rate(ChainId::Hedera) - 0.08).abs() < f64::EPSILON);
    }

    // REQ: wallet-price-002 — fee estimation produces non-zero rJoule fee
    #[test]
    fn fee_estimation_produces_non_zero_fee() {
        let rate = ExchangeRate {
            usd_per_token: 150.0,
            updated_at: chrono::Utc::now(),
        };
        let fee = estimate_withdrawal_fee(ChainId::Solana, &rate, 1000);
        assert!(fee.rjoules > 0);
        assert!(fee.usdc_micro > 0);
    }

    // REQ: wallet-price-003 — fee estimation floors at 1 rJ
    #[test]
    fn fee_estimation_floors_at_one_rj() {
        let rate = ExchangeRate {
            usd_per_token: 0.000001, // extremely low rate
            updated_at: chrono::Utc::now(),
        };
        let fee = estimate_withdrawal_fee(ChainId::Solana, &rate, 1000);
        assert_eq!(fee.rjoules, 1);
    }

    // REQ: wallet-price-004 — different chains produce different fees
    #[test]
    fn different_chains_produce_different_fees() {
        let rate = ExchangeRate {
            usd_per_token: 150.0,
            updated_at: chrono::Utc::now(),
        };
        let sol_fee = estimate_withdrawal_fee(ChainId::Solana, &rate, 1000);
        let hed_fee = estimate_withdrawal_fee(ChainId::Hedera, &rate, 1000);
        // Hedera fee should be higher (0.0125 HBAR × $0.08 vs 0.000005 SOL × $150)
        // But with the same rate of $150, Hedera looks more expensive
        // Actually with rate=150 for both: SOL=0.000005*150=$0.00075, HBAR=0.0125*150=$1.875
        assert!(hed_fee.rjoules > sol_fee.rjoules);
    }
}
