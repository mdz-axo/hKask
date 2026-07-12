# hkask-wallet-types

Value types and data structures for the hKask wallet subsystem. Provides the type foundation shared between `hkask-wallet` and wallet-adjacent crates.

## Core Types

- `WalletConfig` — Wallet configuration (chain selection, derivation paths)
- `ChainBalance` — Per-chain balance tracking
- `TransactionRecord` — Wallet transaction history
- `KeyMaterial` — Key derivation parameters and material

## Design

Separated from `hkask-wallet` to prevent circular dependencies. The types crate contains no logic — only data structures, serialization, and validation. This enables:

- Wallet services to depend on types without depending on wallet implementation
- API layer to serialize wallet state without pulling in chain adapters
- CNS spans to reference wallet concepts without importing wallet runtime

## See Also

- [`hkask-wallet`](../hkask-wallet/README.md) — Wallet implementation
- [`hkask-types`](../hkask-types/README.md) — Foundation types
