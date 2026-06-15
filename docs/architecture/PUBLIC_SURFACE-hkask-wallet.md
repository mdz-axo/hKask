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
**Public items in lib.rs:** 13  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-wallet` is the **HD wallet and energy accounting crate** — rJoule energy tokens, on-chain settlement, and API key lifecycle management. Its surface is large because it spans multiple financial concerns:

1. **HD wallet** — BIP32 hierarchical deterministic wallet with Ed25519 key derivation.
2. **rJoule accounting** — Energy token minting, burning, conversion, and balance tracking.
3. **API key lifecycle** — Key issuance, revocation, expiration, and spending limit enforcement.
4. **Multi-chain** — ChainId abstraction for cross-chain deposit/withdrawal.
5. **Privacy** — Shielded/unshielded balance operations with PrivacyMode.

## Mitigations

- **Submodule organization:** manager.rs (wallet operations), issuer.rs (key lifecycle), types in hkask-types.
- **Newtype safety:** `RJoule` prevents accidental mixing with gas units.

## Deletion Test

Delete `hkask-wallet` and HD key derivation, rJoule accounting, API key lifecycle management, and multi-chain deposit tracking reappear scattered across CNS energy budget and API metering. The crate earns its existence.
