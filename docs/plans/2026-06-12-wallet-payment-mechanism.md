---
title: "Plan — Wallet Subsystem Payment Mechanism Integration"
audience: [architects, developers]
last_updated: 2026-06-12
version: "0.27.0"
status: "Draft"
domain: "Application"
mds_categories: [domain, composition, trust, lifecycle]
---

# Plan — Connecting the Wallet Subsystem as a Payment Mechanism for hKask Services

**Date:** 2026-06-12
**Project:** hKask v0.27.0
**Status:** Foundation built (Phases 1–8 + chain ports + audit complete). Integration plan.

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

## 1. What We Have Built — Foundation Assessment `[IS-DECL]`

The wallet subsystem is complete across 8 crates with 137 passing tests. Here is what exists and what each piece enables for a payment mechanism:

### 1.1 The Payment Ledger (Storage)

```
wallet_balances     — one row per wallet, current rJoule balance
wallet_transactions — append-only ledger of every credit/debit
api_keys            — Ed25519 public keys with spending limits, expiry, spent_rj
deposit_addresses   — derived addresses per wallet per chain
deposit_references  — one-time shielded deposit references (anti-replay)
```

**What this enables:** Every rJoule is traceable from on-chain deposit → internal credit → tool spend → balance. The MUST-10 property test verifies `sum(ledger deltas) == current_balance`. This is the accounting foundation — without it, billing is fiction.

### 1.2 The Value Unit (Types)

```
RJoule              — internal accounting unit (1 rJ ≈ 0.001 USDC, configurable)
ChainId             — Solana, Hedera
PrivacyMode         — Transparent, Shielded
WalletConfig        — rj_per_usdc, gas_per_rjoule, enabled_chains
```

**What this enables:** Gas (dimensionless computational cost) converts to rJoules at a configurable rate. USDC deposits convert to rJoules at a configurable rate. The conversion is bidirectional — rJoules can be withdrawn as USDC. This is the economic bridge between external value and internal computation.

### 1.3 The Key Infrastructure (Keystore + Signing)

```
resolve_treasury_key(chain)  — per-chain Ed25519 key via HKDF from master passphrase
resolve_wallet_seed()        — 32-byte seed for deposit references + API key signing
sign_withdrawal(chain, bytes) — per-operation key loading, zeroized on drop
sign_capability(cap)          — Ed25519 signature over canonical JSON
```

**What this enables:** Treasury keys exist only at signing time. API keys are Ed25519 keypairs — the private key IS the API key, returned once, never stored. This is the security foundation — without it, payments are trust-me-bro.

### 1.4 The Chain Ports (Solana + Hedera)

```
SolanaPort  — 6/6 ChainPort methods: derive, monitor, build, submit, confirm, price
              Raw JSON-RPC via reqwest (rustls). No openssl. Full functionality.
HederaPort  — 4/6 ChainPort methods: derive, monitor, confirm, price
              Mirror node REST API. Write path documented (needs hiero-sdk-proto + tonic).
HinkalPort  — Stub. JS-only SDK. Spec says accept pre-minted Access Tokens.
```

**What this enables:** Solana deposits can be detected and credited today. Hedera deposits can be detected and credited today. Solana withdrawals can be executed today. This is the on-chain bridge — without it, rJoules are a closed loop with no external value entry/exit.

### 1.5 The CNS Membrane (GovernedTool + EnergyBudgetManager)

```
WalletBackedBudget     — gas→rJoule conversion, wallet balance check, key limit check
EnergyBudgetManager    — dual-map: wallet budgets checked before gas budgets
GovernedTool::invoke() — OCAP → budget check → reserve → execute → settle
Algedonic alerts       — WalletBalanceRatio, WalletKeyHealth → Curator escalation
```

**What this enables:** When an agent has a wallet-backed budget, every tool invocation converts gas to rJoules, checks the wallet balance AND the API key spending limit, reserves before execution, and debits after. This is the enforcement mechanism — without it, billing is advisory.

### 1.6 The Surfaces (CLI + API)

```
kask wallet balance              GET  /api/wallet/balance
kask wallet deposit-address      GET  /api/wallet/deposit-address
kask wallet deposit-reference    POST /api/wallet/deposit-reference
kask wallet history              GET  /api/wallet/transactions
kask wallet key create           POST /api/wallet/keys
kask wallet key list             GET  /api/wallet/keys
kask wallet key revoke           DELETE /api/wallet/keys/{id}
kask wallet withdraw             POST /api/wallet/withdraw

ApiKeyAuthService — Bearer token → Ed25519 private key → public key lookup → verify expiry/limit
```

**What this enables:** Users can check balances, get deposit addresses, create API keys, and withdraw via CLI or REST API. API keys authenticate requests. This is the user interface — without it, the payment system is invisible.

---

## 2. Gap Analysis — What Stands Between Foundation and Working Payment System

### 2.1 Critical Gaps `[IS-DECL]`

| # | Gap | Impact | Effort |
|---|-----|--------|--------|
| G1 | **WalletService not wired into AgentService::build()** | API server has no wallet. `kask serve` starts without wallet capability. | Small — add `WalletService` construction to `AgentService::build()` or `ApiState::from_service_context()` |
| G2 | **Deposit monitor not started** | Deposits are never detected. rJoules are never credited. The payment loop is open. | Small — spawn `wallet_manager.start_deposit_monitor()` as a background tokio task |
| G3 | **No wallet budget auto-registration** | Agents don't get wallet-backed budgets automatically. CNS doesn't enforce payment. | Small — register `WalletBackedBudget` when an agent pod is created with a wallet |
| G4 | **No user→wallet association** | `WalletId::default()` is used everywhere. Multi-user support requires wallet-per-user mapping. | Medium — add `user_id → wallet_id` mapping in UserStore or a new table |
| G5 | **No deposit address persistence across restarts** | Deposit addresses are derived but the derivation index isn't persisted. Restart → new address. | Small — persist derivation index in `deposit_addresses` table (already stored) |
| G6 | **API key auth middleware not applied to wallet routes** | Wallet endpoints are unprotected. Anyone can query balances or create keys. | Small — add `.layer(api_key_auth_middleware)` to wallet router |

### 2.2 Design Gaps `[IS-SUBJ]`

| # | Gap | Design Question |
|---|-----|----------------|
| D1 | **No billing model** | How much does a tool invocation cost in rJoules? Who sets the price? |
| D2 | **No invoice/receipt mechanism** | How does a user see what they spent and on what? |
| D3 | **No prepaid vs postpaid model** | Does the user prepay (deposit first) or postpay (invoice after)? Current model is prepaid. |
| D4 | **No free tier / trial** | How do new users try hKask before depositing? Gas budgets provide this for non-wallet agents. |
| D5 | **No payment failure UX** | What happens when a wallet runs out of rJoules mid-session? Current: hard reject. |
| D6 | **No multi-wallet architecture** | `WalletId::default()` everywhere. When do we need per-user wallets? |

### 2.3 Deferred Infrastructure `[IS-DECL]`

| # | Item | Status |
|---|------|--------|
| H1 | Hedera withdrawal (write path) | Documented gap — needs `hiero-sdk-proto` + `tonic` (rustls) |
| H2 | Hinkal privacy | Stub — JS-only SDK, pre-minted Access Tokens per spec |
| H3 | Price feeds (SOL/USD, HBAR/USD) | Placeholder constants |
| H4 | 30-day moving average for alerts | Simplified to nominal 1M rJ capacity |

---

## 3. Design Proposal — The Payment Loop

### 3.1 End-to-End Flow `[OUGHT-DECL]`

```
┌─────────────────────────────────────────────────────────────────┐
│                        PAYMENT LOOP                             │
│                                                                 │
│  1. USER DEPOSITS                                              │
│     User sends USDC to hKask deposit address (Solana/Hedera)   │
│     └─→ Deposit monitor detects on-chain transfer              │
│     └─→ rJoules credited to user's wallet                      │
│     └─→ CNS span: cns.wallet.deposit + cns.wallet.balance      │
│                                                                 │
│  2. USER CREATES API KEY                                       │
│     kask wallet key create --limit 5000                        │
│     └─→ Ed25519 keypair generated                              │
│     └─→ Private key returned ONCE (64 hex chars)               │
│     └─→ Public key stored in api_keys table                    │
│     └─→ CNS span: cns.wallet.key_issued                        │
│                                                                 │
│  3. AGENT REGISTERS WALLET BUDGET                              │
│     Agent pod created with wallet backing                      │
│     └─→ WalletBackedBudget registered in EnergyBudgetManager   │
│     └─→ CNS algedonic: WalletBalanceRatio sensing active       │
│                                                                 │
│  4. AGENT INVOKES TOOLS (THE MEMBRANE)                         │
│     Agent calls tool via GovernedTool                          │
│     └─→ OCAP check: valid capability token?                   │
│     └─→ Budget check: wallet can afford gas→rJoule cost?      │
│     └─→ Key limit check: would_spend ≤ spending_limit?         │
│     └─→ Reserve rJoules (optimistic)                           │
│     └─→ Execute tool                                           │
│     └─→ Settle rJoules (actual cost, refund if over-estimated) │
│     └─→ CNS span: cns.tool.invoked + cns.tool.completed        │
│                                                                 │
│  5. CNS MONITORS WALLET HEALTH                                 │
│     CyberneticsLoop::sense() checks wallet balance ratios      │
│     └─→ Balance < 10% → WalletBalanceLow → Curator             │
│     └─→ Balance = 0  → WalletBalanceCritical → Curator+Human   │
│     └─→ Key exhausted → WalletKeyHealth → Curator              │
│                                                                 │
│  6. USER WITHDRAWS                                             │
│     kask wallet withdraw 5000 --to <primary_wallet>            │
│     └─→ rJoules → USDC conversion                              │
│     └─→ Treasury signs withdrawal transaction                 │
│     └─→ On-chain transfer executed                             │
│     └─→ CNS span: cns.wallet.withdrawal                        │
│                                                                 │
│  7. USER AUDITS                                                │
│     kask wallet history                                        │
│     GET /api/wallet/transactions                               │
│     └─→ Append-only ledger: every credit, debit, spend, refund │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Pricing Model `[IS-SUBJ]`

The current gas system already has per-tool costs via `TableEnergyEstimator` and `InferenceEnergyEstimator`. The wallet layer adds rJoule conversion:

```
gas_cost = estimator.estimate_cost(server, tool, args)
rj_cost   = wallet_manager.gas_to_rjoules(gas_cost)
usd_cost  = rj_cost / rj_per_usdc
```

With defaults (`gas_per_rjoule = 1000`, `rj_per_usdc = 1000`):
- 1 USDC = 1,000 rJ
- 1 rJ = 1,000 gas
- 1 USDC = 1,000,000 gas

**Example costs at default rates:**
| Operation | Gas (est.) | rJoules | USDC |
|-----------|-----------|---------|------|
| Simple tool call (memory recall) | 100 | 1 rJ | ~$0.001 |
| Inference (small model, short prompt) | 5,000 | 5 rJ | ~$0.005 |
| Inference (large model, long context) | 50,000 | 50 rJ | ~$0.05 |
| Web search + extraction | 2,000 | 2 rJ | ~$0.002 |

**Key property:** The estimator is pluggable. Operators set per-tool costs. The conversion rates are configurable. The pricing model is operator-defined, not hardcoded.

### 3.3 User Identity → Wallet Mapping `[IS-SUBJ]`

Current state: `WalletId::default()` everywhere. This works for a single-user/single-operator model but not for multi-user SaaS.

**Proposed approach (simplest first):**

```
Phase A (current):  WalletId::default() — single operator wallet
Phase B (multi-user): Add wallet_id column to user_store or a user_wallets table
                      Map replicant → wallet_id
                      CLI/API use authenticated user's wallet_id instead of default()
```

The `WalletId` type already exists as a phantom-type UUID. The storage schema supports multiple wallets. The gap is purely in the identity→wallet mapping layer.

### 3.4 API Key as Capability Token `[OUGHT-DECL]`

The API key IS an OCAP capability token:

```
Bearer <64-hex-chars>  →  Ed25519 private key
                       →  derive public key
                       →  lookup ApiKeyCapability in WalletStore
                       →  verify: not expired, spending limit not exceeded
                       →  attach WalletContext { wallet_id, key_id, limit, spent }
```

**What this enables:**
- **Delegation:** Operator creates keys for their agents with specific spending limits
- **Attenuation:** Keys can be scoped to specific chains, privacy modes, expiry dates
- **Revocation:** Keys can be revoked, unspent rJoules returned to wallet
- **Audit:** Every key-spend is recorded in the transaction ledger

**Current gap:** The middleware exists but isn't applied to wallet routes yet (G6).

---

## 4. Integration Plan — Bridging the Gaps

### 4.1 Phase 9: Wire Wallet into the Running System `[OUGHT-DECL]`

**Goal:** `kask serve` starts with a working wallet. Deposits are detected. Agents can have wallet-backed budgets.

**Steps:**

| Step | What | Where | Effort |
|------|------|-------|--------|
| 9.1 | Build `WalletService` in `AgentService::build()` or `ApiState::from_service_context()` | `hkask-services/src/context.rs` or `hkask-api/src/lib.rs` | Small |
| 9.2 | Spawn deposit monitor as background tokio task | `ApiState::from_service_context()` or `kask serve` | Small |
| 9.3 | Apply `api_key_auth_middleware` to wallet routes | `hkask-api/src/lib.rs` `create_router()` | Small |
| 9.4 | Add `--wallet` flag to `kask serve` (optional, default off) | `hkask-cli/src/commands/serve.rs` | Small |
| 9.5 | Integration test: deposit → credit → key → spend → debit → withdraw | New test file | Medium |

**Success criteria:**
- [ ] `kask serve --wallet` starts API server with wallet endpoints active
- [ ] Deposit monitor polls Solana RPC every 30s
- [ ] Wallet endpoints require API key authentication
- [ ] Agent with wallet budget is charged rJoules for tool invocations

### 4.2 Phase 10: Multi-User Wallet Support `[IS-SUBJ]`

**Goal:** Each replicant/user has their own wallet. Deposits credit the correct wallet.

**Steps:**

| Step | What | Effort |
|------|------|--------|
| 10.1 | Add `wallet_id` column to `user_store` or create `user_wallets` table | Small |
| 10.2 | `WalletService` methods take `wallet_id` from authenticated user context instead of `default()` | Small |
| 10.3 | CLI `kask wallet` commands resolve wallet from logged-in replicant | Small |
| 10.4 | Migration: existing single-wallet data becomes operator wallet | Small |

### 4.3 Phase 11: Hedera Write Path `[IS-SUBJ]`

**Goal:** Full Hedera withdrawal support via `hiero-sdk-proto` + `tonic` (rustls).

**Steps:**

| Step | What | Effort |
|------|------|--------|
| 11.1 | Add `hiero-sdk-proto` + `tonic` (rustls TLS) as optional deps | Small |
| 11.2 | Implement `build_withdrawal_tx` using protobuf `TransactionBody` | Medium |
| 11.3 | Implement `submit_signed_tx` via gRPC to Hedera consensus node | Medium |
| 11.4 | Test on Hedera testnet | Medium |

### 4.4 Phase 12: Polish & Observability `[IS-SUBJ]`

| Step | What | Effort |
|------|------|--------|
| 12.1 | Price feed integration (CoinGecko API for SOL/USD, HBAR/USD) | Small |
| 12.2 | 30-day moving average for wallet balance alerts | Small |
| 12.3 | Privacy-aware API responses (shielded keys omit on-chain data) | Small |
| 12.4 | Spending receipt/invoice generation from transaction ledger | Medium |

---

## 5. Honest Assessment

### 5.1 What's Solid `[IS-DECL]`

1. **The accounting is real.** Every rJoule is traceable from deposit → credit → spend → balance. The MUST-10 property test verifies the invariant. This is not a mock — it's a double-entry ledger in SQLite.

2. **The security boundary is real.** Treasury keys are per-operation loaded via HKDF, used, zeroized. API keys are Ed25519 keypairs — private keys returned once, never stored. The `LoadedKey` struct has redacted Debug. This is not placeholder security — it's the real pattern used by Turnkey and 1Password.

3. **The CNS enforcement is real.** `WalletBackedBudget` checks both wallet balance AND key spending limit before every tool invocation. The membrane (GovernedTool) routes through `EnergyBudgetManager` which checks wallet budgets first. When rJoules run out, operations are hard-rejected.

4. **The on-chain bridge is real (Solana).** `SolanaPort` derives deposit addresses via PDA, monitors deposits via JSON-RPC, builds SPL token transfer instructions, and submits signed transactions. This works today against Solana mainnet/devnet with a real RPC endpoint and a real treasury key.

5. **The surfaces are real.** CLI and REST API expose every wallet operation. The API key auth middleware validates Ed25519 Bearer tokens against the `api_keys` table. This is not a stub — it's a working auth system.

### 5.2 What's Not Solid (Yet) `[IS-DECL]`

1. **The loop isn't closed.** The deposit monitor isn't started. WalletService isn't wired into the running server. Until Phase 9 is done, the payment system exists as tested code but not as a running service.

2. **It's single-user.** `WalletId::default()` everywhere. Multi-user requires Phase 10.

3. **Hedera is read-only.** Deposits are detected but withdrawals can't execute. Phase 11 fixes this.

4. **Pricing is operator-defined but not operator-configured.** The gas cost table and conversion rates exist as code constants. There's no admin UI for setting prices.

5. **No payment UX for agent failure.** When a wallet runs dry mid-session, the agent gets a hard reject. There's no "please deposit more" message, no grace period, no invoice. The CNS algedonic alert goes to the Curator, not the user.

### 5.3 What's Intentionally Out of Scope `[OUGHT-DECL]`

1. **Fiat on-ramp.** Users deposit USDC from their own wallets. hKask never touches fiat.
2. **KYC/AML.** Headless constraint. Hinkal's zkMe is optional and user-side.
3. **Subscription billing.** The model is prepaid per-invocation, not monthly subscription.
4. **rJoule as on-chain token.** rJoule exists only in SQLite. No ERC-20, SPL, or HTS.
5. **General-purpose wallet features.** User's primary wallet handles key storage, multi-chain, DeFi.

---

## 6. Recommendation

The foundation is **ready for single-operator use** after Phase 9 (wire into running system). A single operator can:

1. Start `kask serve --wallet`
2. Get a Solana deposit address
3. Send USDC to that address
4. See rJoules credited
5. Create API keys for their agents
6. Agents spend rJoules on tool invocations
7. CNS monitors wallet health
8. Withdraw remaining rJoules back to USDC

**Estimated effort to production-ready single-operator:** Phase 9 (~2-3 hours).

**Estimated effort to multi-user SaaS:** Phases 9 + 10 (~4-6 hours).

**Estimated effort to full multi-chain:** Phases 9 + 10 + 11 (~8-12 hours, depending on Hedera gRPC complexity).

---

## 7. Verification Commands

```bash
# Foundation verification (all passing today)
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings

# Solana port verification (requires feature flag)
cargo check -p hkask-wallet --features solana
cargo test -p hkask-wallet

# Hedera port verification (requires feature flag)
cargo check -p hkask-wallet --features hedera

# Constraint verification
grep -r "openssl" crates/hkask-wallet/Cargo.toml && echo "VIOLATION" || echo "CLEAN"
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/hkask-wallet/src/ && echo "VIOLATION" || echo "CLEAN"
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0 — Payment Mechanism Plan 2026-06-12*
