---
title: "Energy, Gas, Payments & API Key System"
audience: [architects, developers, agents]
last_updated: 2026-06-18
version: "0.30.0"
status: "Active"
domain: "Trust"
mds_categories: [domain, trust, lifecycle, curation]
---

# Energy, Gas, Payments & API Key System

**Purpose:** Defines the economic layer of hKask — how energy is measured, consumed, funded, and settled across all interaction surfaces.

**Related:** [`PRINCIPLES.md`](core/PRINCIPLES.md) §P12 (Replicant Host Mandate), [`loop-architecture.md`](loop-architecture.md)

---

## 1. Units and Concepts

| Term | Definition | Unit |
|------|-----------|------|
| **rJoule (rJ)** | The base energy unit of hKask. One rJoule represents the computational cost of one token of inference at the default model. | rJ |
| **Gas** | The consumption metric for operations. Gas is denominated in rJoules. `gas_heuristic` estimates per-turn cost; `gas_cap` sets session limit. | rJ |
| **Wallet** | A replicant's energy account. HD wallet derived from WebID via `hkask-wallet`. Holds rJoule balance. | rJ balance |
| **Encumbrance** | rJoules reserved for a specific API key's use. Not transferred — locked against the wallet balance, deducted as the key consumes gas. | rJ locked |
| **Allocation** | The rJoule budget assigned to an API key at issuance. Drawn from the funding replicant's wallet. | rJ |
| **Settlement** | Periodic on-chain confirmation of gas consumption. Batched every N blocks by 7R7 bots. | on-chain tx |

---

## 2. Energy Flow

```
                    ┌──────────────────────────────────────┐
                    │          Replicant Wallet             │
                    │  (HD wallet from WebID, hkask-wallet) │
                    │  Balance: 1,000,000 rJ                │
                    └──────────┬───────────────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
          ▼                    ▼                    ▼
   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
   │  API Key k1  │   │  API Key k2  │   │  CLI Session │
   │  alloc: 50K  │   │  alloc: 200K │   │  gas_cap: 10K│
   │  scope:      │   │  scope:      │   │  per-turn:   │
   │  embed-corp  │   │  read-specs  │   │  500 rJ      │
   └──────┬───────┘   └──────┬───────┘   └──────┬───────┘
          │                  │                  │
          ▼                  ▼                  ▼
   ┌──────────────────────────────────────────────────────┐
   │                    CNS Metering                       │
   │  cns.api.request spans track per-key consumption     │
   │  cns.session spans track per-replicant consumption   │
   │  EnergyBudgetManager enforces caps                   │
   └──────────────────────────────────────────────────────┘
          │
          ▼
   ┌──────────────────────────────────────────────────────┐
   │              7R7 Bot Settlement                       │
   │  Aggregates consumption → produces batch → submits   │
   │  on-chain tx every N blocks                          │
   └──────────────────────────────────────────────────────┘
```

---

## 3. Gas Model

### 3.1 Consumption Formula

```
gas_consumed = endpoint_weight × (prompt_tokens × token_cost + response_tokens × token_cost)
             + payload_size_surcharge
```

| Parameter | Default | Description |
|-----------|---------|-------------|
| `token_cost` | 1 rJ | Base cost per token at default model |
| `endpoint_weight` | 1.0–5.0 | Heavier endpoints (embed-corpus, compose) cost more |
| `payload_size_surcharge` | 0.1 rJ/KB | Additional cost for large request bodies |

### 3.2 Session Budgets (CLI / REPL)

Per the `/repl` settings:

| Setting | Default | Range |
|---------|---------|-------|
| `gas_heuristic` | 500 rJ | Per-turn reservation |
| `gas_cap` | 10,000 rJ | Total session budget |

When `gas_cap` is exhausted, the session ends. The replicant can increase `gas_cap` via `/repl gas_cap <value>` if their wallet holds sufficient balance.

### 3.3 API Key Budgets

Each API key receives an allocation from the funding replicant's wallet at issuance. The allocation is **encumbered** (locked, not transferred). As the key consumes gas, rJoules are deducted from the encumbrance via `WalletBackedBudget::settle()` → `WalletManager::consume()` → `WalletStore::consume_encumbrance()` (atomic SQL operation).

```
key_allocation_remaining = initial_allocation - gas_consumed_by_key
```

When `key_allocation_remaining ≤ 0`, requests return `402 Payment Required`. The funding replicant can replenish via `kask wallet encumber --key-id <id> --amount <rj>`.

**Implementation status (v0.28.0):** Fully implemented. The `ApiKeyAuthService` middleware registers a `WalletBackedBudget` with the key's encumbrance after Bearer token authentication. `GovernedTool` and `GovernedInference` membranes check wallet budgets before gas budgets, and debit from encumbrance on settle. CNS spans `cns.gas.reserved`, `cns.gas.settled`, `cns.gas.depleted` provide observability.

---

## 4. Wallet System

### 4.1 Wallet Creation

Wallets are created automatically during `AgentService::build()` startup via `WalletService::build()`. Each registered replicant without a wallet gets one created with a deterministic `WalletId` derived from the replicant name via UUID v5 (`WalletId::from_name()`). Existing wallet bindings are preserved (idempotent).

`WalletService::build()` encapsulates all chain port assembly (Hedera, Hinkal), price feed resolution, and CNS wiring — keeping `context.rs` focused on orchestration.

### 4.2 Multi-Wallet Model

Each replicant has its own wallet for deposit/withdrawal isolation. The `ReplicantIdentity.wallet_id` field stores the binding. All CLI commands accept `--wallet <uuid>` and all API routes accept `?wallet_id=<uuid>`. When omitted, the system falls back to the authenticated API key's wallet (from `WalletContext` extension) or the system default.

### 4.3 Multi-Chain Architecture

hKask supports two settlement chains (plus Hinkal privacy layer), each with full deposit monitoring, withdrawal building, signed submission, and CNS error span emission:

| Chain | Protocol | Deposit Monitoring | Withdrawal | CNS Spans |
|-------|----------|-------------------|------------|-----------|
| **Hedera** | HTS USDC via mirror node REST + gRPC (rustls) | `GET /api/v1/accounts/{id}/transactions` → parse `MirrorTransactionsResponse` → filter CRYPTOTRANSFER → match USDC token with sender extraction | Protobuf `CryptoTransferTransactionBody` → `CryptoServiceClient::crypto_transfer` | Mirror node HTTP, gRPC connect/submit/pre-check |
| **Hinkal** | Shielded pool via REST API (privacy layer) | `GET /balance` with session auth → delta detection → `ShieldedTransfer` emission | `POST /withdraw` with session auth + protocol message signing | All HTTP/parse/rejection paths |

Chain ports share a single `Arc<HinkalPort>` instance for both chain routing and privacy adapter roles — one HTTP client, one session cache, one circuit breaker.

### 4.4 Deposit Monitoring

A background deposit monitor (`poll_deposits_once`) iterates all wallets and all enabled chains every 30 seconds (configurable via `HKASK_DEPOSIT_MONITOR_INTERVAL_SECS`). Detected deposits are credited to the correct wallet via `WalletStore::resolve_wallet_for_address()` reverse lookup. Idempotency is enforced via `transaction_exists_by_hash()`. Each credited deposit emits a `cns.wallet.deposit_credited` CNS span.

Shielded deposits are monitored via `PrivacyPort::monitor_shielded_transfers()` which polls the Hinkal balance endpoint and detects positive deltas. Shielded transfers carry a `chain` field for correct attribution in the transaction ledger.

### 4.5 Withdrawal Flow

Withdrawals follow a fail-fast pattern:
1. Verify chain/privacy availability (no debit yet)
2. Debit rJoules from wallet
3. Convert rJoules to micro-USDC
4. Chain port builds unsigned transaction
5. `signing.rs` signs (isolated security boundary)
6. Chain port submits signed transaction
7. Record transaction in ledger

On submission failure, debited rJoules are refunded via compensating credit. Shielded (Hinkal) withdrawals route through `PrivacyPort::build_unshield_tx` → `submit_signed_tx` with internal protocol message signing.

### 4.6 Shield Orchestration

`WalletManager::shield_assets(wallet_id, amount, chain)` moves transparently-held USDC into the Hinkal shielded pool:
1. Verify privacy port availability
2. `build_shield_tx` → unsigned shield payload
3. Sign (or pass raw for Hinkal which signs internally via `sign_message`)
4. `submit_signed_tx` via `POST /deposit`
5. Record `TransactionType::Shield` in ledger (zero rJoule delta — pure asset layer transition)

### 4.7 Balance Operations

| Operation | CLI Command | API Endpoint | Auth |
|-----------|------------|-------------|------|
| Check balance | `kask wallet balance [--wallet]` | `GET /api/wallet/balance?wallet_id=` | Bearer or capability |
| Get deposit address | `kask wallet deposit-address [--wallet]` | `GET /api/wallet/deposit-address` | Bearer or capability |
| Transaction history | `kask wallet history [--wallet]` | `GET /api/wallet/transactions` | Bearer or capability |
| Create API key | `kask wallet key create [--wallet]` | `POST /api/wallet/keys` | Bearer or capability |
| Encumber rJoules | `kask wallet encumber --key-id --amount [--wallet]` | (CLI only) | Replicant session |
| Release encumbrance | `kask wallet release-encumbrance --key-id` | (CLI only) | Replicant session |
| Spending report | `kask wallet report --key-id [--wallet]` | (CLI only) | Replicant session |
| Withdraw | `kask wallet withdraw [--wallet]` | `POST /api/wallet/withdraw` | Bearer or capability |

### 4.8 rJoule Acquisition

rJoules enter the system through:

- **On-chain deposit:** USDC transferred to a derived deposit address on Hedera → detected by chain port monitor → converted to rJoules at `rj_per_usdc` rate
- **Shielded deposit:** Privacy-preserving transfer via Hinkal shielded pool with one-time deposit reference → `consume_deposit_reference()` → credited
- **Shield orchestration:** Transparently-detected USDC can be moved into the Hinkal shielded pool via `shield_assets()` for privacy
- **Initial allocation (future):** Each new replicant receives a genesis allocation at registration
- **Earning (future):** Bots earn rJoules by performing system services

### 4.9 Price Feed

Native token USD exchange rates are provided by a user-configurable multi-source price feed system:

| Source | Type | Requirements |
|--------|------|-------------|
| **EODHD** | Primary canonical | `HKASK_EODHD_API_KEY` env var |
| **CoinGecko** | Free fallback | None (rate-limited to ~30 calls/min) |
| **Static** | Dev/test | Hardcoded rates ($150/SOL, $0.08/HBAR) |

Configuration via `PriceFeedConfig` in `WalletConfig`:
- `{"type": "composite", "sources": ["eodhd", "coingecko"], "cache_ttl_secs": 30}` — default: EODHD → CoinGecko with 30s cache
- `{"type": "eodhd"}` — single source
- `{"type": "static"}` — hardcoded dev/test rates

The `CompositePriceFeed` tries sources in priority order, caches results, and falls back to stale cached rates on total failure. `WalletManager::estimate_withdrawal_fee()` uses the configured feed for pre-withdrawal cost estimation.

---

## 5. API Key Lifecycle

### 5.1 Issuance

```
POST /api/keys/request
{
  "replicant": "Jacques rZuck",
  "scope": ["embed-corpus", "read-specs"],
  "purpose": "Automated nightly documentation quality scoring",
  "allocation_rj": 50000,
  "rate_limit": {"requests_per_minute": 10, "tokens_per_day": 100000}
}

→ 7R7 bot verifies:
  1. Replicant authenticated (UserStore session)
  2. Clean CNS history (no abuse flags, 90 days)
  3. Valid scope (endpoints exist in registry)
  4. Purpose stated (≥20 chars)
  5. Rate limit feasible (≤ scope maximum)
  6. Wallet balance ≥ allocation_rj

→ Returns:
{
  "key_id": "k_7r7_abc123",
  "key_secret": "hk_...",       // shown once
  "scope": ["embed-corpus", "read-specs"],
  "allocation_rj": 50000,
  "expires_at": "2026-09-11T00:00:00Z",
  "rate_limit": {"requests_per_minute": 10, "tokens_per_day": 100000}
}
```

### 5.2 Usage

```
GET /api/specs/{id}
Authorization: Bearer hk_...

→ ApiKeyAuthService middleware:
  1. Extracts Bearer token, derives public key
  2. Looks up ApiKeyCapability in WalletStore
  3. Verifies: not revoked, not expired, spending limit not exceeded
  4. Verifies encumbrance exists with remaining rJoules
  5. Verifies scope: request path must match declared scope (empty scope = unrestricted)
  6. Registers WalletBackedBudget in CNS for encumbrance-gated consumption
→ GovernedTool/GovernedInference membranes debit from encumbrance on each operation
→ Returns response (or 402/403 on failure)
```

**Scope enforcement (v0.28.0):** If the key's `scope` field is non-empty, the request URI path must start with one of the declared scope prefixes. Keys scoped to `["read-specs"]` cannot access `/api/chat`. Returns `403 Forbidden` with `ScopeViolation` error.

### 5.3 Replenishment

```
POST /api/keys/k_7r7_abc123/fund
{
  "replicant": "Jacques rZuck",
  "amount_rj": 25000
}

→ 7R7 bot verifies wallet balance
→ Increases encumbrance by amount_rj
→ Returns updated allocation
```

### 5.4 Expiry & Release

At 90 days:
- Key expires automatically
- 7R7 bot sends renewal notice to funding replicant
- If not renewed within 7 days: key revoked, unspent rJ released to wallet
- If renewed: new 90-day period, allocation carries forward

### 5.5 Revocation

Triggers:
- 3 consecutive CNS abuse alerts for the key
- Key used from >5 distinct IPs within 1 hour
- Key used for endpoints outside declared scope
- Manual: `kask api revoke-key k_7r7_abc123` (Curator authority)

On revocation: unspent rJ released to wallet, key_id added to deny list.

---

## 6. CNS Metering

### 6.1 Span Hierarchy

```
cns.api.request
  ├─ key_id
  ├─ endpoint
  ├─ scope_matched: true/false
  ├─ gas_consumed
  ├─ allocation_remaining
  └─ rate_limit_status: ok/exceeded

cns.session
  ├─ replicant
  ├─ gas_consumed_this_turn
  ├─ gas_remaining_this_session
  └─ wallet_balance
```

### 6.2 Rate Limit Enforcement

| Limit Type | Scope | Default |
|-----------|-------|---------|
| `requests_per_minute` | Per key | 60 |
| `tokens_per_day` | Per key | 1,000,000 |
| `concurrent_requests` | Per key | 5 |
| `unique_endpoints_per_hour` | Per key (variety) | 10 |

### 6.3 Alerts

| Alert | Trigger | Action |
|-------|---------|--------|
| `cns.api.rate_limit_exceeded` | Key exceeds rate limit | 429 response, bot notified |
| `cns.api.allocation_low` | Key allocation < 20% | Funder notified |
| `cns.api.allocation_exhausted` | Key allocation ≤ 0 | 402 response, bot notified |
| `cns.api.anomaly_abuse` | 3 abuse patterns detected | Bot investigates, may revoke |
| `cns.api.scope_violation` | Key used outside scope | 403 response, bot notified |

---

## 7. Settlement (Anticipated)

### 7.1 Batch Settlement

```
7R7 bot aggregates per-key consumption over N blocks
  → produces settlement batch:
    [
      {key_id: k1, rJ_consumed: 1,250, wallet: w_A},
      {key_id: k2, rJ_consumed: 8,400, wallet: w_B},
    ]
  → submits on-chain transaction
  → chain confirms → encumbrances released, balances updated
```

### 7.2 Settlement Authority

Only 7R7 bots hold settlement authority. No human, Curator, or daemon can submit settlement transactions. This prevents:
- Double-spend (only the bot that issued the key can settle it)
- Balance manipulation (bots verify against CNS logs)
- Replay attacks (each settlement batch is nonce-protected)

### 7.3 On-Chain Token

The rJoule maps to an on-chain token (anticipated: ERC-20 or similar). The token contract:
- Mints rJ at genesis allocation
- Burns rJ on settlement (consumed energy is destroyed, not transferred)
- Transfers rJ between wallets (replicant-to-replicant)
- Locks rJ for encumbrance (key allocations)

---

## 8. Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Gas heuristic / cap | ✅ Implemented | `ReplSettings.gas_heuristic`, `gas_cap` |
| CNS energy budget manager | ✅ Implemented | `hkask-cns::EnergyBudgetManager` |
| Wallet derivation (HD) | ✅ Implemented | `hkask-wallet` with WebID→HD derivation |
| rJoule ↔ gas conversion | ✅ Implemented | `wallet::tests::gas_to_rjoules_conversion` |
| Encumbrance system | ✅ Implemented | `hkask-wallet::WalletManager::encumber/release_encumbrance/consume`; atomic `consume_encumbrance` in `WalletStore`; `encumbrances` table |
| API key issuance (6-gate) | ✅ Implemented | `POST /api/keys/request` with gates 1,4,5,6 active; `POST /api/keys/{id}/fund`; `DELETE /api/keys/{id}`; `ApiKeyCapability` extended with `scope`/`purpose`/`rate_limit` |
| API key metering (CNS spans) | ✅ Implemented | `hkask-cns::api_metering` — `ApiMeter` (in-memory rate limiter), `ApiRequestSpan`, `ApiMeteringAlert` (5 alert types), `endpoint_weight` table |
| 7R7 bot key management | ✅ Implemented | `DelegationResource::Key` variant in capability system; `key:issue`, `key:revoke`, `key:fund` parseable |
| Hedera deposit/withdrawal | ✅ Implemented | `HederaPort` — HTS USDC via mirror node REST + gRPC (rustls). Deposit monitoring with sender extraction, protobuf `CryptoTransferTransactionBody` withdrawal, CNS spans on all HTTP/gRPC errors. Testnet integration test (`#[ignore]`). Configurable via `HEDERA_CONSENSUS_NODE_URL`. |
| Hinkal privacy layer | ✅ Implemented | `HinkalPort` — session management (24h TTL), shielded deposit monitoring (`GET /balance`), shielded withdrawal (`POST /withdraw`), shield orchestration (`POST /deposit`), circuit breaker with cooldown. CNS spans on all HTTP/parse/rejection paths. Settles on Hedera. |
| Price feed system | ✅ Implemented | Multi-source: `EodhdPriceFeed` (primary), `CoinGeckoPriceFeed` (fallback), `StaticPriceFeed` (dev/test). `CompositePriceFeed` with caching and stale fallback. User-configurable via `PriceFeedConfig` in `WalletConfig`. `resolve_price_feed()` factory. |
| WalletService extraction | ✅ Implemented | `WalletService::build()` encapsulates chain port assembly, price feed resolution, and CNS wiring. `context.rs` wallet block reduced from ~280 lines to ~10 lines. |
| Shield orchestration | ✅ Implemented | `WalletManager::shield_assets()` — build → sign → submit → record `TransactionType::Shield` (zero rJoule delta). |
| CNS chain error spans | ✅ Implemented | Both chain ports emit `cns.wallet.chain_error` on RPC/gRPC/HTTP failures. `HederaPort` on mirror node + gRPC errors, `HinkalPort` on all API call failures. |
| On-chain settlement | ⏸️ Deferred | Fails essentialist G1 (deletion test): system works without it for single-node deployments. Spec preserved for future multi-node/token economics phase. |
| CNS abuse history query (gate 2) | ⏸️ Deferred | Stubbed in `approve_key_request`; awaits CNS alert query API exposure via service layer. |
| Registry scope validation (gate 3) | ⏸️ Deferred | Stubbed in `approve_key_request`; awaits MCP tool registry endpoint enumeration. |

---

## 9. Energy Accounting Audit

> **Incorporated from:** `docs/status/energy_accounting_hardening_audit.md`

### Call Chain (7 operations, 5 layers)

```
GovernedTool → CyberneticsLoop (pass-through) → EnergyBudgetManager (dispatch)
  ├── EnergyBudget (in-memory)
  └── WalletBackedBudget → WalletManager → WalletStore (SQLite, atomic CAS)
```

### Key Findings

| Finding | Severity | Status |
|---------|----------|--------|
| `CyberneticsLoop` pass-through layer (6 methods, 0 behavior) | Medium | Candidate for removal |
| `EnergyBudget` is in-memory only — no tamper-evidence for free-tier agents | Low | Acceptable risk |
| `WalletStore` SQL constraints are the sole tamper-evident anchor | Info | Documented |
| Bypassing `WalletManager::consume()` loses CNS span emission | Medium | Observability gap |
| EMA gas calibration introduces time-of-day non-determinism | High | Needs fixed calibration schedule |
| Gas↔rJoule conversion uses `Ordering::Relaxed` — no happens-before | High | Use Acquire/Release |
| Float `f64` alert threshold — platform-dependent | Medium | Replace with Rational |

### Recommendations

1. **Remove `CyberneticsLoop` pass-through** — migrate callers to use `Arc<EnergyBudgetManager>` directly, eliminating 6 public surface methods and one delegation hop.
2. **Add CNS span on every `settle()`/`consume()`** — emit `cns.energy.budget_state` for post-hoc audit trail.
3. **Fix gas calibration determinism** — use fixed schedule, saturating fixed-point arithmetic, `Ordering::Acquire/Release`.
4. **Reproducibility test** — run identical workload twice, verify identical energy accounting outcomes.

---

## 10. References

- PRINCIPLES.md §P12 — Replicant Host Mandate (API key request flow, approval criteria, metering)
- canonical CNS span registry: `crates/hkask-types/src/cns.rs` (`CnsSpan`) — CNS spans and variety counters
- loop-architecture.md — EnergyBudget subsumption of RateLimiting
- AGENTS.md — `/repl` settings for gas_heuristic, gas_cap
- hkask-wallet — HD wallet derivation and balance operations
- hkask-cns — EnergyBudgetManager and span hierarchy
