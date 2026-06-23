# hkask-wallet

Multi-chain crypto wallet for hKask. Manages wallet lifecycle, key derivation, and chain port selection across Hedera and optional Hinkal.

## Core Components

- `WalletManager` — Wallet lifecycle: create, import, export, backup
- `ChainPortSelector` — Selects the appropriate chain adapter
- `KeyDerivation` — HKDF-SHA256 deterministic key derivation per WebID

## Supported Chains

| Chain | Status | Adapter |
|-------|--------|---------|
| Hedera | ✅ Core | `hedera-sdk` |
| Hinkal | ⚠️ Optional | Feature-gated |

## See Also

- [`hkask-wallet-types`](../hkask-wallet-types/README.md) — Wallet value types
- [`hkask-keystore`](../hkask-keystore/README.md) — OS keychain integration
- [`PRINCIPLES.md`](../../docs/architecture/core/PRINCIPLES.md) §P1 — User Sovereignty
