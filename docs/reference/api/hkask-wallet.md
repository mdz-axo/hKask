---
title: "hkask-wallet — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

# hkask-wallet — API Reference

**Purpose:** Multi-chain wallet for self-custody deposits, API key issuance, and rJoule management. Supports Hedera (feature-gated) and Hinkal privacy integration.

## Public Modules

| Module | Purpose |
|--------|---------|
| `chain` | Multi-chain port abstraction (`ChainPort` trait) |
| `issuer` | API key issuance and lifecycle management |
| `manager` | Wallet lifecycle, balance tracking, CNS integration |
| `price_feed` | Exchange rate feeds (CoinGecko, EODHD, composite, static) |
| `signing` | Transaction signing (Ed25519) |
| `types` | Wallet-specific types re-exported from `hkask-wallet-types` |
| `cns_span` | CNS span emission for wallet events |
| `hedera` | Hedera chain integration (behind `hedera` feature) |

## Key Types

| Type | Description |
|------|-------------|
| `WalletManager` | Central wallet coordinator — manages deposits, withdrawals, and API key issuance |
| `ChainPort` | Trait for chain-specific operations (deposit detection, withdrawal submission) |
| `DepositEvent` | On-chain deposit detected by a chain port |
| `ApiKeyIssuer` | Issues signed API keys backed by wallet deposits |
| `PriceFeed` | Trait for exchange rate resolution (`CoinGeckoPriceFeed`, `EodhdPriceFeed`, `CompositePriceFeed`, `StaticPriceFeed`) |
| `ExchangeRate` | Currency pair exchange rate with timestamp |
| `WithdrawalFee` | Fee estimation for outgoing transactions |

## Re-exported from `hkask-wallet-types`

| Type | Description |
|------|-------------|
| `RJoule` | Unit of energy consumption (currency of the gas system) |
| `ChainId` | Identifier for a blockchain network |
| `PrivacyMode` | Privacy level for transactions (standard, shielded) |
| `WalletConfig` | Wallet configuration (chains, price feeds, CNS integration) |
| `WalletBalance` | Balance snapshot for a wallet |
| `WalletTransaction` | Deposit or withdrawal record |
| `TransactionType` | Deposit or Withdrawal |
| `WalletError` | Wallet-specific error type |
| `ApiKeyCapability` | Capability scope for an issued API key |
| `ApiKeyMaterial` | Cryptographic material for an API key |
| `Encumbrance` | Hold on funds (reserved but not yet settled) |
| `EncumbranceStatus` | Pending, Settled, or Expired |
| `RateLimitConfig` | Per-key rate limiting configuration |
| `TxHash` | Transaction hash type |
| `GAS_PER_RJOULE` | Conversion constant |
| `RJ_PER_USDC` | Price constant |
| `PriceFeedConfig` | Configuration for price feed selection |

## Key Functions

| Function | Signature |
|----------|-----------|
| `sign_capability` | Signs an API key capability with the wallet's Ed25519 key |
| `sign_withdrawal` | Signs a withdrawal transaction |
| `estimate_withdrawal_fee` | Estimates the fee for a withdrawal on a given chain |
| `resolve_price_feed` | Resolves a `PriceFeedConfig` to a concrete `PriceFeed` implementation |

## Features

| Feature | Effect |
|---------|--------|
| `hedera` | Enables Hedera Hashgraph chain integration |

## CNS Integration

Wallet events emit CNS spans in the `cns.wallet.*` namespace. See `cns_span` module for emission points.
