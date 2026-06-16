//! hKask Wallet — rJoule payments, self-custody multi-chain deposits, API key issuance.
//!
//! # Specialized sub-wallet `[OUGHT-DECL]`
//! The hKask wallet is a specialized sub-wallet — one of several crypto wallets
//! the user holds. It only does what hKask needs:
//! - Receive deposits (USDC → rJoules)
//! - Track rJoule balances
//! - Issue API key capability tokens
//! - Process withdrawals (rJoules → USDC)
//!
//! The user's primary wallet (Phantom, HashPack, MetaMask) handles key storage,
//! multi-chain asset management, and DeFi interactions.
//!
//! # Self-custody `[OUGHT-DECL]` (P1 — User Sovereignty)
//! hKask derives treasury keys from the user's master key via HKDF. No third
//! party holds the keys. No custodial service. The user controls their funds
//! at all times. Chain ports (`solana.rs`, `hedera.rs`) interact directly with
//! blockchain RPC endpoints — no intermediary API.
//!
//! # Security `[OUGHT-DECL]`
//! - `signing.rs` — isolated security boundary for all key operations
//! - Per-operation key loading: keys derived via HKDF, used, zeroized immediately
//! - No long-lived treasury key material
//! - API key private keys returned to user once, never stored by hKask
//! - `Zeroizing` wrappers on all secret key material
//!
//! # Crate Map
//! - `chain.rs` — `ChainPort` trait + `DepositEvent`
//! - `privacy.rs` — `PrivacyPort` trait + `ShieldedTransfer`
//! - `signing.rs` — Isolated signing module (security boundary)
//! - `manager.rs` — `WalletManager` + deposit reference logic
//! - `issuer.rs` — `ApiKeyIssuer` + `ApiKeyMaterial`
//! - `price_feed.rs` — `PriceFeed` trait + fee estimation
//! - `solana.rs` — `SolanaPort` (feature-gated: "solana")
//! - `hedera.rs` — `HederaPort` (feature-gated: "hedera")
//! - `hinkal.rs` — `HinkalPort` (feature-gated: "hinkal")

pub mod chain;
pub mod issuer;
pub mod manager;
pub mod price_feed;
pub mod privacy;
pub mod signing;

#[cfg(feature = "solana")]
pub mod solana;

#[cfg(feature = "hedera")]
pub mod hedera;

#[cfg(feature = "hinkal")]
pub mod hinkal;

pub use chain::{ChainPort, DepositEvent};
pub use issuer::{ApiKeyIssuer, ApiKeyMaterial};
pub use manager::WalletManager;
pub use price_feed::{
    CoinGeckoPriceFeed, CompositePriceFeed, EodhdPriceFeed, ExchangeRate, PriceFeed,
    StaticPriceFeed, WithdrawalFee, estimate_withdrawal_fee, resolve_price_feed,
};
pub use privacy::{PrivacyPort, ShieldedTransfer};
pub use signing::{sign_capability, sign_withdrawal};
