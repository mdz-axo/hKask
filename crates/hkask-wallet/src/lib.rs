//! hKask Wallet — rJoule payments, Circle-backed USDC custody, API key issuance.
//!
//! # Architecture (post-cleanup)
//!
//! The wallet has two layers separated by the `ChainPort` trait boundary:
//!
//! **Above the boundary (hKask-specific, always compiled):**
//! - `manager.rs` — `WalletManager` orchestrates rJoule accounting, deposits, withdrawals
//! - `issuer.rs` — `ApiKeyIssuer` generates Ed25519 OCAP capability tokens
//! - `signing.rs` — Isolated security boundary for API key capability signing
//! - `price_feed.rs` — `PriceFeed` trait + `StaticPriceFeed` for fee estimation
//! - `chain.rs` — `ChainPort` trait (the plug boundary)
//! - `privacy.rs` — `PrivacyPort` trait for shielded transfers
//!
//! **Below the boundary (pluggable chain ports):**
//! - `circle.rs` — **Primary.** Circle Programmable Wallets REST API (default feature)
//! - `hinkal.rs` — Privacy layer via Hinkal protocol (feature: `hinkal`)
//!
//! **Archived (reference only, not in default builds):**
//! - `solana.rs` — Raw Solana JSON-RPC (feature: `archive-solana`)
//! - `hedera.rs` — Hedera mirror node + gRPC (feature: `archive-hedera`)
//!
//! # Security `[OUGHT-DECL]`
//! - `signing.rs` — isolated security boundary for all key operations
//! - Per-operation key loading: keys derived via HKDF, used, zeroized immediately
//! - No long-lived treasury key material (Circle manages chain-level keys)
//! - API key private keys returned to user once, never stored by hKask
//! - `Zeroizing` wrappers on all secret key material

pub mod chain;
pub mod issuer;
pub mod manager;
pub mod price_feed;
pub mod privacy;
pub mod signing;

// Primary chain port (default feature)
#[cfg(feature = "circle")]
pub mod circle;

// Privacy layer (separate concern)
#[cfg(feature = "hinkal")]
pub mod hinkal;

// Archived — reference implementations, not in default builds
#[cfg(feature = "archive-solana")]
pub mod solana;

#[cfg(feature = "archive-hedera")]
pub mod hedera;

pub use chain::{ChainPort, DepositEvent};
pub use issuer::{ApiKeyIssuer, ApiKeyMaterial};
pub use manager::WalletManager;
pub use price_feed::{PriceFeed, StaticPriceFeed, WithdrawalFee, estimate_withdrawal_fee};
pub use privacy::{PrivacyPort, ShieldedTransfer};
pub use signing::{sign_capability, sign_withdrawal};
