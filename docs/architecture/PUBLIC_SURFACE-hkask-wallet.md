---
title: "Public Surface Justification — hkask-wallet"
audience: [architects, developers]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-wallet

**Crate:** `hkask-wallet`  
**Public items in lib.rs:** 22  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-wallet` is the **multi-chain wallet and energy accounting crate** — rJoule energy tokens, on-chain settlement across Solana/Hedera/Hinkal, privacy-preserving shielded operations, API key lifecycle management, and live price feeds. Its surface is large because it spans multiple financial concerns:

1. **Multi-chain settlement** — Solana (SPL USDC via JSON-RPC), Hedera (HTS USDC via mirror node + gRPC), Hinkal (shielded pool via REST API).
2. **rJoule accounting** — Energy token minting, burning, conversion, and balance tracking.
3. **API key lifecycle** — Key issuance, revocation, expiration, spending limit enforcement, encumbrance.
4. **Privacy layer** — Shielded deposits/withdrawals via Hinkal Shared Privacy Protocol (zkSNARKs, stealth addresses, relayers).
5. **Price feed** — Multi-source live exchange rates (EODHD primary, CoinGecko fallback), user-configurable via `PriceFeedConfig`, composite with caching and stale fallback.
6. **CNS integration** — Chain error spans on all RPC/gRPC/HTTP failure paths across all three chain ports.

## Public Surface (lib.rs exports)

| Export | Kind | Purpose |
|--------|------|---------|
| `WalletManager` | struct | Orchestrates chain ports, privacy layer, rJoule accounting |
| `ApiKeyIssuer` | struct | API key creation, revocation, listing |
| `ApiKeyMaterial` | struct | Issued key material (private key hex, capability) |
| `ChainPort` | trait | Abstract chain interface (deposit monitoring, withdrawal, confirmations) |
| `DepositEvent` | struct | Detected on-chain deposit |
| `PrivacyPort` | trait | Abstract privacy interface (shielded deposits, unshield withdrawals) |
| `ShieldedTransfer` | struct | Shielded transfer detected in privacy pool (with chain attribution) |
| `PriceFeed` | trait | Abstract exchange rate interface |
| `ExchangeRate` | struct | USD per native token with update timestamp |
| `StaticPriceFeed` | struct | Hardcoded rates for dev/test |
| `EodhdPriceFeed` | struct | EOD Historical Data API (primary canonical source) |
| `CoinGeckoPriceFeed` | struct | CoinGecko free public API (fallback) |
| `CompositePriceFeed` | struct | Multi-source orchestrator with caching and fallback |
| `WithdrawalFee` | struct | Estimated fee in rJoules, native units, micro-USDC |
| `estimate_withdrawal_fee` | fn | Fee estimation from chain + rate + rj_per_usdc |
| `resolve_price_feed` | fn | Factory: `PriceFeedConfig` → `Arc<dyn PriceFeed>` |
| `sign_withdrawal` | fn | Sign withdrawal tx bytes (isolated security boundary) |
| `sign_capability` | fn | Sign API key capability token |
| `sign_message` | fn | Sign arbitrary message (Hinkal session/auth) |

Feature-gated submodules:
| `solana` | mod | `SolanaPort` — SPL USDC via raw JSON-RPC (rustls) |
| `hedera` | mod | `HederaPort` — HTS USDC via mirror node + gRPC (rustls) |
| `hinkal` | mod | `HinkalPort` — Shielded pool via Hinkal REST API |

## Mitigations

- **Submodule organization:** `manager.rs` (wallet operations), `issuer.rs` (key lifecycle), `signing.rs` (isolated security boundary), `price_feed.rs` (exchange rates), `privacy.rs` (shielded operations), per-chain modules (`solana.rs`, `hedera.rs`, `hinkal.rs`).
- **Newtype safety:** `RJoule` prevents accidental mixing with gas units. `TxHash` wraps chain-specific transaction identifiers.
- **Feature gates:** Chain SDKs (solana-sdk, hiero-sdk-proto) are optional — default build has zero chain dependencies.
- **Security boundary:** `signing.rs` is the ONLY module where treasury key material is loaded, used, and zeroized. All other modules operate on already-signed data or public keys.

## Deletion Test

Delete `hkask-wallet` and HD key derivation, rJoule accounting, API key lifecycle management, multi-chain deposit/withdrawal tracking, privacy-preserving shielded operations, and live price feed resolution reappear scattered across CNS energy budget, API metering, and service orchestration. The crate earns its existence.
