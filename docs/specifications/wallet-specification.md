---
title: "hKask Wallet Crate — Architectural Specification"
audience: [architects, developers]
last_updated: 2026-06-18
version: "0.31.0"
status: "Active"
domain: "Application"
mds_categories: [domain, composition, trust, lifecycle]
---

# hKask Wallet Crate — Architectural Specification

**Date:** 2026-06-12
**Project:** hKask v0.28.0
**Status:** Phases 1–8 complete ✅ — Full wallet subsystem (types, storage, keystore, wallet crate, CNS, services, CLI, API) built and tested
**Skills applied:** idiomatic-rust, essentialist, pragmatic-semantics, pragmatic-cybernetics, coding-guidelines

---

## 0. Epistemic Frame

Every statement is classified per pragmatic-semantics:

| Tag | Meaning |
|-----|---------|
| `[IS-DECL]` | Direct measurement or self-evident fact |
| `[IS-PROB]` | Probabilistic inference from data |
| `[IS-SUBJ]` | What-if projection |
| `[OUGHT-DECL]` | Prescriptive rule or requirement |

---

## 1. Purpose & Scope

### 1.1 What hKask Wallet Is `[OUGHT-DECL]`

The hKask wallet is a **specialized sub-wallet** — one of several crypto wallets the user holds. It only does what hKask needs:

- Receive deposits (USDC → rJoules) on Hedera
- Track rJoule balances in SQLite (SQLCipher-encrypted)
- Issue Ed25519-signed API key capability tokens
- Process withdrawals (rJoules → USDC) back to user's primary wallet
- Support optional shielded deposits/withdrawals via Hinkal privacy protocol

### 1.2 What hKask Wallet Is NOT `[OUGHT-DECL]`

- NOT a general-purpose crypto wallet — user's primary wallet (Phantom, HashPack, MetaMask) handles key storage, multi-chain asset management, DeFi
- NOT a key generator for users — treasury keys are derived from hKask's master passphrase, not user keys
- NOT a KYC/AML platform — headless constraint, P1 sovereignty
- NOT a zkSNARK proof generator — Hinkal SDK handles this
- NOT an on-chain rJoule token — rJoule is an internal accounting unit in SQLite

---

## 2. Crate Architecture

### 2.1 Crate Map

```
hkask-wallet/
├── Cargo.toml              — Feature gates: hedera, hinkal
├── src/
│   ├── lib.rs              — Crate docs, module declarations, re-exports
│   ├── chain.rs            — ChainPort trait (7 public items) + DepositEvent
│   ├── privacy.rs          — PrivacyPort trait (7 public items) + ShieldedTransfer
│   ├── signing.rs          — Isolated security boundary (2 public functions)
│   ├── manager.rs          — WalletManager (12 methods, justified) + deposit reference logic
│   ├── issuer.rs           — ApiKeyIssuer (6 public items) + ApiKeyMaterial re-export
│   ├── hedera.rs           — HederaPort (feature-gated: "hedera")
│   └── hinkal.rs           — HinkalPort (feature-gated: "hinkal")
```

### 2.2 Module Dependency Graph

```mermaid
graph TD
    subgraph "hkask-wallet"
        CHAIN["chain.rs<br/>ChainPort trait"]
        PRIV["privacy.rs<br/>PrivacyPort trait"]
        SIGN["signing.rs<br/>Security boundary"]
        MGR["manager.rs<br/>WalletManager"]
        ISS["issuer.rs<br/>ApiKeyIssuer"]
        HED["hedera.rs<br/>(feature: hedera)"]
        HINK["hinkal.rs<br/>(feature: hinkal)"]
    end

    subgraph "Dependencies (workspace)"
        TYPES["hkask-types"]
        KS["hkask-keystore"]
        STORE["hkask-storage"]
    end

    CHAIN --> TYPES
    PRIV --> TYPES
    SIGN --> KS
    SIGN --> TYPES
    MGR --> CHAIN
    MGR --> PRIV
    MGR --> SIGN
    MGR --> STORE
    MGR --> TYPES
    ISS --> SIGN
    ISS --> STORE
    ISS --> KS
    ISS --> TYPES
    SOL --> CHAIN
    HED --> CHAIN
    HINK --> PRIV

    style SIGN fill:#7c3aed,color:#fff
    style MGR fill:#2563eb,color:#fff
    style ISS fill:#2563eb,color:#fff
```

### 2.3 Essentialist Review Summary

| Gate | Result |
|------|--------|
| **G1 — Exist** | 3 items pruned: `error.rs` (pass-through), `deposit_ref.rs` (merged into manager.rs), `TxHash` (moved to hkask-types). All surviving components encode behavior beyond direct calls. |
| **G2 — Surface** | `chain.rs`: 7 (at threshold). `privacy.rs`: 7 (at threshold). `manager.rs`: 13 (justified — each method has distinct caller). `issuer.rs`: 6. `signing.rs`: 2. |
| **G3 — Contract** | 0 pass-through abstractions. All traits add behavior beyond direct dependency calls. |

---

## 3. Type System

### 3.1 Types in `hkask-wallet-types` (Phase 1)

| Type | Kind | Security Constraints |
|------|------|---------------------|
| `RJoule(u64)` | Newtype | Copy, Clone — value unit, not secret |
| `ChainId` | Enum (Hedera, Hinkal) | Copy, Clone |
| `PrivacyMode` | Enum (Transparent, Shielded) | Copy, Clone |
| `Ed25519PublicKey([u8; 32])` | Newtype | Copy, Clone — public key |
| `DepositAddress` | Struct | Clone — no secrets |
| `WalletConfig` | Struct | Clone — configuration |
| `WalletBalance` | Struct | Clone — public state |
| `ApiKeyCapability` | Struct | Clone — public metadata |
| `ApiKeyMaterial` | Struct | **NO Clone** — contains `private_key_hex` |
| `TransactionType` | Enum | Clone |
| `WalletTransaction` | Struct | Clone |
| `DepositReference` | Struct | Clone |
| `TxHash(String)` | Newtype | Clone — public tx hash |
| `WalletError` (10 variants) | thiserror::Error | Typed errors with context |

### 3.2 Key Material Types — Internal to signing.rs

| Type | Copy? | Clone? | Zeroize? | Rationale |
|------|-------|--------|----------|-----------|
| `LoadedKey` (internal) | ❌ | ❌ | ✅ `Zeroizing<[u8; 32]>` | Secret — MUST zeroize (MUST-8) |
| Treasury key (internal) | ❌ | ❌ | ✅ `Zeroizing<Vec<u8>>` | Secret — MUST zeroize |
| Wallet seed (internal) | ❌ | ❌ | ✅ `Zeroizing<[u8; 32]>` | Secret — MUST zeroize |
| API key private key | ❌ | ❌ | N/A (user-held) | Returned once, never stored (MUST-5) |
| Signature (public output) | ✅ Copy | ✅ Clone | ❌ | Public output |

### 3.3 Debug Redaction (MUST-2)

```rust
impl std::fmt::Debug for LoadedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedKey")
            .field("bytes", &"[REDACTED]")
            .finish()
    }
}
```

---

## 4. Security Architecture

### 4.1 Signing Module — The Security Boundary

```mermaid
graph TD
    subgraph "Security Boundary: signing.rs"
        SK["sign_withdrawal(chain, tx_bytes) → Signature"]
        SC["sign_capability(capability) → Signature"]
        LK["LoadedKey<br/>Zeroizing<[u8; 32]>"]
        ZD["Zeroize on Drop"]
    end

    subgraph "Outside Boundary"
        WM["WalletManager"]
        ISS["ApiKeyIssuer"]
    end

    WM -->|"tx_bytes"| SK
    SK -->|"signature (no key material)"| WM
    SK --> LK
    LK --> ZD
    ISS --> SC

    style SK fill:#7c3aed,color:#fff
    style SC fill:#7c3aed,color:#fff
    style LK fill:#dc2626,color:#fff
    style ZD fill:#059669,color:#fff
```

**Invariant `[OUGHT-DECL]`:** No un-zeroized key material ever leaves `signing.rs`. Keys are loaded per-operation via HKDF, wrapped in `LoadedKey` (redacted Debug), used for signing, and zeroized on drop. The caller receives only the signature.

### 4.2 Key Material Lifecycle

```mermaid
sequenceDiagram
    participant Caller
    participant Signing as signing.rs
    participant Keystore as hkask-keystore
    participant Memory

    Caller->>Signing: sign_withdrawal(Hedera, tx_bytes)
    Signing->>Keystore: resolve_treasury_key(Hedera)
    Keystore-->>Signing: Zeroizing<Vec<u8>> (32 bytes)
    Note over Signing,Memory: Key material exists in memory
    Signing->>Signing: LoadedKey::from_zeroizing → Zeroizing<[u8; 32]>
    Signing->>Signing: Ed25519 signing
    Signing-->>Caller: Signature (64 bytes)
    Note over Memory: LoadedKey drops → Zeroizing zeroes
    Note over Memory: Key material gone from memory
```

### 4.3 Defense in Depth

| Layer | Mechanism | Protects Against |
|-------|-----------|-----------------|
| **Type system** | `Zeroizing<[u8; 32]>` — no Copy, no Clone, zeroize on drop | Memory dumps, use-after-free, accidental copies |
| **Module boundary** | `signing.rs` — only module that loads key material | Scattered key handling, audit difficulty |
| **Per-operation loading** | Keys loaded per call via HKDF, not held long-term | Long-lived key material in memory |
| **Debug redaction** | `LoadedKey` Debug shows `[REDACTED]` | Key leakage in logs, error messages |
| **Constant-time ops** | `subtle` crate for sensitive comparisons | Timing side channels |
| **Feature gates** | Chain SDKs behind Cargo features | Reduced compile-time attack surface |
| **Mock ports** | All chain/privacy interactions testable without real keys | Test coverage of security-critical paths |

### 4.4 Security Invariant Checklist

#### MUST (Inviolable) `[OUGHT-DECL]`

| # | Invariant | Status |
|---|-----------|--------|
| MUST-1 | Seed never in plain memory beyond Zeroizing scope | ✅ `Zeroizing<Vec<u8>>` on all derived key material |
| MUST-2 | Seed never in logs, error messages, or Debug output | ✅ `LoadedKey` Debug shows `[REDACTED]` |
| MUST-3 | Seed derivation always uses domain-separated HKDF contexts | ✅ `TREASURY_HEDERA`, `TREASURY_HINKAL`, `WALLET_SEED` |
| MUST-4 | Signing requires user consent (P2 Affirmative Consent) | 🔶 Deferred to Phase 6 (ConsentManager gate) |
| MUST-5 | Private keys never serialized to disk unencrypted | ✅ API key private keys returned once, never stored |
| MUST-6 | All cryptographic comparisons use constant-time equality | 🔶 Deferred (subtle crate available, not yet wired) |
| MUST-7 | No branching on secret data | ✅ Signing path is linear, no secret-dependent branches |
| MUST-8 | Zeroize on drop for all types containing key material | ✅ `Zeroizing` on `LoadedKey`, treasury key, wallet seed |
| MUST-9 | No Clone on secret-bearing types | ✅ `LoadedKey` has no Clone; `Zeroizing` prevents Copy |
| MUST-10 | Balance invariant: sum(ledger deltas) == current_balance | 🔶 Deferred to property test (proptest) |
| MUST-11 | No key material leaves signing.rs | ✅ API returns `Vec<u8>` (signature), never key bytes |

#### SHOULD (Strongly Recommended) `[OUGHT-DECL]`

| # | Invariant | Status |
|---|-----------|--------|
| SHOULD-1 | `mlock()` on in-memory key material | 🔶 Deferred (platform-specific) |
| SHOULD-2 | Subprocess isolation for signing | 🔶 Deferred (defense-in-depth) |
| SHOULD-3 | Anti-ptrace / anti-coredump | 🔶 Deferred (platform-specific) |
| SHOULD-4 | Key material cache with ≤30s TTL | 🔶 Deferred (performance optimization) |
| SHOULD-5 | `cargo-deny` in CI | 🔶 Deferred (CI configuration) |
| SHOULD-6 | Pinned dependency versions in `Cargo.lock` | ✅ Committed to repository |
| SHOULD-7 | No proc-macro deps beyond thiserror/serde | ✅ Only thiserror, serde, async-trait |

### 4.5 Supply-Chain Attack Mitigation

**Forbidden dependencies `[OUGHT-DECL]`:**
- ❌ No `openssl` (use `rustls` for TLS)
- ❌ No `libsodium` (use Rust-native crypto: ed25519-dalek + sha2 + hmac)
- ❌ No `ring` (use ed25519-dalek + sha2 + hmac)
- ❌ No proc-macro crates beyond `thiserror`, `serde`, `async-trait` (already vetted)

**Dependency footprint `[IS-DECL]`:**

| Dependency | Justification | Risk |
|-----------|---------------|------|
| `hkask-types` | Domain types (workspace) | None — internal |
| `hkask-keystore` | Key derivation (workspace) | None — internal |
| `hkask-storage` | Persistence (workspace) | None — internal |
| `reqwest` | HTTP for Hedera mirror node / Hinkal relay (feature-gated) | Medium — TLS dep (rustls) |
| `ed25519-dalek` | Key construction from seed bytes | Low — already in keystore |
| `zeroize` | Memory protection | Low — already in keystore |
| `subtle` | Constant-time comparison | Low — well-audited |
| `thiserror` | Error derive | Low — already in types |
| `serde` / `serde_json` | Serialization | Low — already in types |
| `tokio` | Async runtime | Low — already in agents |

---

## 5. Hinkal Integration

### 5.1 What hKask Builds In `[OUGHT-DECL]`

| Affordance | Implementation |
|-----------|---------------|
| Shielded address derivation | `PrivacyPort::shielded_deposit_address(wallet_id)` |
| Event log monitoring | `HinkalPort::monitor_shielded_transfers()` — polls Shielded Pool events, decrypts `encryptedOutputs` |
| Relayer communication | Configurable endpoint in `WalletConfig.hinkal_relayer_url` |
| Relayer response verification | Independent verification where possible (Quantstamp audit finding) |
| Graceful degradation | `PrivacyPort::available_for_chain(chain)` — false when Hinkal not deployed |
| Deposit reference scheme | HKDF-derived one-time references as memo in shielded transfers |
| CircuitBreaker on relay | Fail-open to transparent mode with P2 consent gate |

### 5.2 What hKask Does NOT Build In `[OUGHT-DECL]`

| Non-Affordance | Why Not | Where It Lives |
|---------------|---------|---------------|
| Access Token minting | Requires zkSNARK proof generation (Groth16) — heavy dependency | User mints via Hinkal SDK |
| KYC/AML | Headless constraint, P1 sovereignty | User-side via zkMe/Reclaim |
| zkSNARK proof generation | Requires Circom circuits, trusted setup | Hinkal SDK |
| Chainalysis KYT | Protocol-level feature | Hinkal Shielded Pool contract |

### 5.3 Hinkal Deposit Flow

```mermaid
sequenceDiagram
    participant User
    participant PrimaryWallet as User's Primary Wallet
    participant HinkalSDK as Hinkal SDK
    participant ShieldedPool as Hinkal Shielded Pool
    participant HinkalPort as hKask HinkalPort
    participant Wallet as hKask WalletManager

    Note over User,Wallet: SHIELDED DEPOSIT FLOW

    User->>Wallet: kask wallet deposit-reference --chain hedera --shielded
    Wallet-->>User: dep_ref: "dep_a7f3c..." (valid 24h)

    User->>PrimaryWallet: Shield USDC + set memo = "dep_a7f3c..."
    PrimaryWallet->>HinkalSDK: Shield assets (zkSNARK proof)
    HinkalSDK->>ShieldedPool: Shield assets
    ShieldedPool-->>HinkalSDK: Shielded address + commitment

    User->>PrimaryWallet: Transfer to hKask shielded address
    PrimaryWallet->>HinkalSDK: Shielded transfer
    HinkalSDK->>ShieldedPool: Shielded transfer (via relayer)
    Note over ShieldedPool: Event emitted: encryptedOutputs

    loop Every 30s
        HinkalPort->>ShieldedPool: Poll events
        ShieldedPool-->>HinkalPort: encryptedOutputs
        HinkalPort->>HinkalPort: Decrypt with shielded key
        HinkalPort->>HinkalPort: Extract memo = "dep_a7f3c..."
    end

    HinkalPort->>Wallet: ShieldedTransfer { memo: "dep_a7f3c...", amount: 10 USDC }
    Wallet->>Wallet: consume_deposit_reference("dep_a7f3c...") → wallet_id
    Wallet->>Wallet: credit_rjoules(wallet_id, 10,000 rJ)
```

### 5.4 Privacy Mode Decision Tree

```mermaid
graph TD
    START[User requests withdrawal] --> CHECK_PRIV{Privacy mode?}

    CHECK_PRIV -->|Transparent| TX_BUILD[ChainPort::build_withdrawal_tx]
    TX_BUILD --> TX_SIGN[signing.rs::sign_withdrawal]
    TX_SIGN --> TX_SUBMIT[ChainPort::submit_signed_tx]
    TX_SUBMIT --> DONE[Return TxHash]

    CHECK_PRIV -->|Shielded| CHECK_RELAY{PrivacyPort relay health?}

    CHECK_RELAY -->|Healthy| HINKAL_BUILD[PrivacyPort::build_unshield_tx]
    HINKAL_BUILD --> HINKAL_SIGN[signing.rs::sign_withdrawal]
    HINKAL_SIGN --> HINKAL_SUBMIT[PrivacyPort::submit_signed_tx]
    HINKAL_SUBMIT --> DONE

    CHECK_RELAY -->|Unhealthy - CircuitBreaker open| P2_GATE{User consents to transparent fallback?}

    P2_GATE -->|Yes| TX_BUILD
    P2_GATE -->|No| ERR[Return PrivacyUnavailable error]
```

---

## 6. CNS Integration (Phase 5 — Built ✅)

### 6.1 Span Emission Checklist

All namespaces registered in `CANONICAL_NAMESPACES` (`hkask-types::event`).

| Operation | Module | Span Namespace | Verb | Phase | Status |
|-----------|--------|---------------|------|-------|--------|
| Deposit address derived | `manager.rs` | `cns.wallet.deposit` | `derived` | Act | ✅ |
| Deposit detected (transparent) | `chain.rs` → `manager.rs` | `cns.wallet.deposit` | `detected` | Sense | ✅ |
| Deposit detected (shielded) | `hinkal.rs` → `manager.rs` | `cns.wallet.deposit_shielded` | `detected` | Sense | ✅ |
| Deposit credited | `manager.rs` | `cns.wallet.balance` | `credited` | Act | ✅ |
| Withdrawal built | `chain.rs` | `cns.wallet.withdrawal` | `built` | Act | ✅ |
| Withdrawal signed | `signing.rs` | `cns.wallet.withdrawal` | `signed` | Act | ✅ |
| Withdrawal submitted | `chain.rs` | `cns.wallet.withdrawal` | `submitted` | Act | ✅ |
| USDC ↔ rJoule conversion | `manager.rs` | `cns.wallet.conversion` | `converted` | Act | ✅ |
| API key issued | `issuer.rs` | `cns.wallet.key_issued` | `issued` | Act | ✅ |
| API key revoked | `issuer.rs` | `cns.wallet.key_revoked` | `revoked` | Act | ✅ |
| API key expired | `issuer.rs` | `cns.wallet.key_expired` | `expired` | Sense | 🔶 CNS algedonic |
| API key exhausted | `issuer.rs` | `cns.wallet.key_exhausted` | `exhausted` | Sense | 🔶 CNS algedonic |
| Treasury key loaded | `signing.rs` | `cns.wallet.treasury` | `loaded` | Act | 🔶 Covered by withdrawal.signed |
| Chain error | `chain.rs` | `cns.wallet.chain_error` | `error` | Sense | ⬜ Deferred (needs chain ports) |
| Shielded tx initiated | `hinkal.rs` | `cns.wallet.privacy.shield` | `initiated` | Act | ⬜ Deferred (needs hinkal port) |
| Unshield (transparent fallback) | `privacy.rs` | `cns.wallet.privacy.unshield` | `fallback` | Act | ⬜ Deferred (needs privacy port) |
| Privacy error | `privacy.rs` | `cns.wallet.privacy_error` | `error` | Sense | ⬜ Deferred (needs privacy port) |

### 6.2 CNS Error Threshold Mapping

| Error Variant | CNS Alert | Threshold |
|---------------|-----------|-----------|
| `InsufficientBalance` | `cns.wallet.balance` — depleted | Warning |
| `SpendingLimitExceeded` | `cns.wallet.key_exhausted` | Warning |
| `KeyExpired` | `cns.wallet.key_expired` | Info |
| `KeyRevoked` | `cns.wallet.key_revoked` | Info |
| `ChainNotEnabled` | `cns.wallet.chain_error` | Warning |
| `PrivacyUnavailable` | `cns.wallet.privacy_error` | Critical (relay down) |
| `DepositReferenceInvalid` | `cns.wallet.deposit` — invalid_ref | Warning |
| `ChainError` | `cns.wallet.chain_error` | Critical (chain RPC down) |
| `PrivacyError` | `cns.wallet.privacy_error` | Critical |
| `Infra` | `cns.wallet.*` (context-dependent) | Critical |

---

## 7. Ownership Architecture

```mermaid
graph TD
    subgraph "AgentService (sole owner)"
        WM["WalletManager<br/>chain_ports: HashMap<ChainId, Box<dyn ChainPort>><br/>privacy_port: Option<Box<dyn PrivacyPort>><br/>wallet_seed: Zeroizing<[u8; 32]>"]
        ISS["ApiKeyIssuer<br/>wallet_store: Arc<WalletStore><br/>wallet_seed: Zeroizing<[u8; 32]>"]
        SIGN["signing.rs (stateless)<br/>no owned key material"]
    end

    subgraph "WalletManager Internals"
        CP["HashMap<ChainId, Box<dyn ChainPort>>"]
        PP["Option<Box<dyn PrivacyPort>>"]
        WS["Arc<WalletStore>"]
    end

    subgraph "Shared"
        CNS["CyberneticsLoop<br/>Arc<WalletStore> (read-only)"]
    end

    subgraph "Surfaces (borrow)"
        CLI["ReplState<br/>&WalletService"]
        API["ApiState<br/>&WalletService"]
    end

    WM --> CP
    WM --> PP
    WM --> WS
    ISS --> WS
    CNS --> WS
    CLI --> WM
    API --> WM
    WM --> SIGN
    ISS --> SIGN

    style SIGN fill:#7c3aed,color:#fff
    style WM fill:#2563eb,color:#fff
    style ISS fill:#2563eb,color:#fff
```

**Key decisions `[OUGHT-DECL]`:**
- `WalletManager` sole-owns `ChainPort` and `PrivacyPort` implementations
- `WalletStore` is `Arc<>` — shared with CNS for algedonic monitoring (justified)
- `signing.rs` is stateless — no owned data, no long-lived keys
- Treasury keys NEVER held long-term — loaded per signing operation, zeroized on drop
- Surfaces borrow `&WalletService`, never own wallet components
- `ApiKeyIssuer` shares `Arc<WalletStore>` with WalletManager (both need write access to API key tables)

---

## 8. Implementation Status

### 8.1 Completed Phases

| Phase | Crate | Status | Tests | Key Deliverables |
|-------|-------|--------|-------|-----------------|
| 1 | `hkask-wallet-types` | ✅ | 11 | `RJoule`, `ChainId`, `PrivacyMode`, `ApiKeyCapability`, `WalletError` (15 variants), `TxHash`, 14 CNS spans, 3 wallet SignalMetrics |
| 2 | `hkask-storage` | ✅ | 34 | `WalletStore` — 5 tables, 16 methods, anti-replay deposit references, MUST-10 property test |
| 3 | `hkask-keystore` | ✅ | 6 | `resolve_treasury_key(chain)`, `resolve_wallet_seed()`, `sign_api_key_capability()` |
| 4 | `hkask-wallet` | ✅ | 13 | `ChainPort`, `PrivacyPort`, `signing.rs` (LoadedKey + redacted Debug), `WalletManager` (13 methods + CNS span emission), `ApiKeyIssuer` (CNS span emission) |
| 5 | `hkask-cns` | ✅ | 11 | `WalletBackedBudget`, `WalletEnergyEstimator`, `EnergyBudgetManager` dual-map, algedonic alerts (balance + key health), CNS span emission wired |
| 6 | `hkask-services` | ✅ | 35 | `WalletService` — 13 methods composing WalletManager + ApiKeyIssuer + CNS budget registration |
| 7 | `hkask-cli` | ✅ | 25 | `kask wallet` — 8 subcommands (balance, deposit-address, deposit-reference, history, key create/list/revoke, withdraw) |
| 8 | `hkask-api` | ✅ | 2 | 8 wallet REST endpoints + `ApiKeyAuthService` middleware (Ed25519 Bearer token verification) |

### 8.2 Remaining Phases

| Phase | Scope | Dependencies |
|-------|-------|-------------|
| 4 (chain ports) | `hedera.rs`, `hinkal.rs` — feature-gated implementations | reqwest |

### 8.3 Test Inventory

| Crate | Tests | REQ Tags |
|-------|-------|----------|
| `hkask-wallet-types` | 11 (7 wallet) | `P1-wallet-types` |
| `hkask-storage` | 34 (11 wallet_store) | `P2-wallet-store`, `MUST-10` |
| `hkask-keystore` | 6 (6 wallet) | `P3-keystore` |
| `hkask-wallet` | 13 | `P4-signing`, `P4-manager`, `P4-issuer` |
| `hkask-cns` | 11 (1 wallet_budget) | `P5-cns-wallet` |
| `hkask-services` | 35 (6 wallet) | `svc-wallet-001`–`006` |
| `hkask-cli` | 25 (0 wallet-specific) | (existing CLI tests) |
| `hkask-api` | 2 (0 wallet-specific) | (existing API tests) |
| **Total** | **137** (44 wallet-specific) | |

---

## 9. Open Questions & Resolved Decisions

| Question | Decision | Rationale |
|----------|----------|-----------|
| Q1: Per-operation vs long-lived keys | **Per-operation** ✅ | Research consensus (Turnkey, 1Password). 1μs HKDF overhead negligible. |
| Q2: Hinkal Access Token — mint or accept? | **Accept pre-minted** ✅ | zkSNARK proof generation belongs in Hinkal SDK. hKask is headless and minimal. |
| Q3: hKask wallet scope | **Specialized sub-wallet** ✅ | User's primary wallet handles key storage, multi-chain, DeFi. hKask wallet only does deposits, rJoule tracking, API keys, withdrawals. |
| Q4: Deposit detection strategy | **Polling at 30s intervals** | Low-frequency polling avoids persistent RPC connections. Multiple fallback endpoints. |
| Q5: Multi-chain address format | **Chain-specific native formats** | Hedera: `0.0.XXXXX` account ID. |
| Q6: Gas pre-funding (bootstrapping) | **Deferred** | Initial treasury funded by hKask operator. Users deposit USDC → rJoules credited. |
| Q7: Key revocation — on-chain vs off-chain | **Off-chain (database flag)** | `revoked_at` timestamp in `api_keys` table. Unspent rJoules returned to wallet. |
| Q8: Recovery from seed (P1 sovereignty) | **Deterministic derivation** | All keys derived from master passphrase via HKDF. Same passphrase → same keys. |
| Q9: Hinkal support | **Hedera-only** | Hinkal settles on Hedera. Only `ChainId::Hedera` and `ChainId::Hinkal` are supported. |

---

## 10. Verification Commands

```bash
# Per-crate verification
cargo check -p hkask-wallet-types -p hkask-storage -p hkask-keystore -p hkask-wallet
cargo test -p hkask-wallet-types -p hkask-storage -p hkask-keystore -p hkask-wallet
cargo clippy -p hkask-wallet -- -D warnings

# Full workspace (after all phases)
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings

# Constraint verification
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/hkask-wallet/ && echo "VIOLATION" || echo "CLEAN"
grep -r "\.unwrap()" crates/hkask-wallet/src/ && echo "VIOLATION: unwrap in library code" || echo "CLEAN"
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.28.0 — Wallet Specification 2026-06-12*
