# Plan — Wallet, rJoule Payments & Multi-Chain Architecture

**Session date:** 2026-06-12
**Project:** hKask v0.27.0
**Status:** Phase 1 complete ✅ — Phase 2 in progress

---

## 0. Epistemic Frame (Pragmatic Semantics)

Every statement in this plan is classified on two axes per the pragmatic-semantics discipline:

| Axis | Values |
|------|--------|
| **Ontological mode** | **IS** (descriptive — what exists or is measured) vs **OUGHT** (prescriptive — what should be, a rule or requirement) |
| **Epistemic mode** | **Declarative** (direct measurement/self-evident) vs **Probabilistic** (statistical inference) vs **Subjunctive** (what-if projection) |

Statements below are tagged `[IS-DECL]`, `[OUGHT-DECL]`, `[IS-PROB]`, `[IS-SUBJ]` etc. per the cross-axis classification. The constraint hierarchy (Prohibition > Guardrail > Guideline > Evidence > Hypothesis) governs which can be relaxed and which cannot.

**Provenance of facts in this plan:**
- Chain performance metrics: Directly Stated from Chainspect.app (live data, 2026-06-12) — `[IS-DECL]`
- Hinkal capabilities: Directly Stated from Hinkal whitepaper + documentation — `[IS-DECL]`
- hKask internal architecture: Directly Stated from codebase inspection — `[IS-DECL]`
- Design decisions: Prescriptive, derived from Magna Carta principles — `[OUGHT-DECL]`
- Implementation estimates: Subjunctive projections based on similar past work — `[IS-SUBJ]`

---

## 1. Session Context

This session designed the architecture for connecting hKask with cryptocurrency/stablecoin payment systems. The design introduces:

- **rJoule** (lowercase "r", capital "J") — a stable value unit bridging external payments to internal gas. 1 rJoule = 1000 gas (configurable). 1 USDC = 1000 rJoules (configurable). `[OUGHT-DECL]`
- **Multi-chain support** from day one: Solana (SPL USDC) + Hedera (HTS USDC), abstracted behind a `ChainPort` trait. `[OUGHT-DECL]`
- **Hinkal privacy layer** — optional shielded deposits/withdrawals via Hinkal's zkSNARK-based Shared Privacy Protocol, with a deposit-reference scheme for private attribution. `[OUGHT-DECL]`
- **API key "printing"** — Ed25519-signed capability tokens issued by the wallet, carrying embedded spending limits and privacy mode. `[OUGHT-DECL]`
- **Gas↔rJoule bridge** — `WalletBackedBudget` variant of `EnergyBudget` that converts gas costs to rJoule debits at the CNS level. `[OUGHT-DECL]`

The architecture is fully Magna Carta compliant (P1–P4), headless, and leverages existing infrastructure (keystore, CNS, OCAP, gas system) rather than building parallel systems.

---

## 2. Rust Expertise Review — Type Design & Ownership Architecture

### 2.1 Core Invariant (Type-Driven Design)

**The invariant:** Every rJoule debit must be traceable to a verified on-chain deposit or a wallet-issued API key with a valid spending limit. No rJoule can be created or destroyed without a corresponding on-chain event or capability token. `[OUGHT-DECL]`

This invariant drives the type design. The types must make it impossible to:
- Credit rJoules without a verified deposit event
- Spend rJoules without a valid API key capability
- Exceed an API key's spending limit
- Mix transparent and shielded accounting

### 2.2 Type Design — Making Invalid States Unrepresentable

#### `RJoule` — Value Unit Newtype

```rust
/// Replicated Joule — a stable value unit for hKask payments.
/// 1 rJoule ≈ 0.001 USDC (configurable via WalletConfig.rj_per_usdc).
/// Internal gas: 1 rJoule = configurable gas units (default: 1000 gas).
///
/// # Invariant
/// RJoule values are always non-negative. Construction via `new()` validates.
/// Arithmetic operations saturate at 0 (no negative balances) and u64::MAX.
///
/// # Provenance
/// Every RJoule in the system originates from a verified on-chain deposit
/// (ChainPort::monitor_deposits) or a shielded deposit (PrivacyPort::monitor_shielded_transfers).
/// No RJoule is ever created from thin air.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RJoule(u64);
```

**Rust-expertise check:**
- ✅ Newtype with validation at construction — `new()` returns `Option<Self>` (zero is valid, but construction from raw u64 requires explicit intent)
- ✅ `Copy` is appropriate — `RJoule` is a trivial scalar, no resources managed
- ✅ `PartialOrd` + `Ord` — enables balance comparisons and "can afford" checks
- ✅ No `Default` — there is no meaningful default rJoule amount
- ✅ `Display` impl shows "rJ" suffix for user-facing output
- ❌ Anti-pattern avoided: not a bare `u64` — the newtype prevents accidental mixing with gas units or raw integers

#### `ChainId` — Enum, Not String

```rust
/// Supported blockchain networks for deposits and withdrawals.
///
/// # Extensibility
/// Adding a new chain requires: a new variant here, a ChainPort implementation,
/// a treasury key derivation context, and a storage migration for the new chain.
/// This is intentional — adding a chain is an architectural commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainId {
    Solana,
    Hedera,
}
```

**Rust-expertise check:**
- ✅ Enum over String — `"solana"` typos caught at compile time
- ✅ `Copy` — lightweight discriminant-only type
- ✅ Exhaustive matching — compiler finds every use site when a chain is added
- ❌ Anti-pattern avoided: not `String`-typed — no `"solnaa"` typos possible

#### `PrivacyMode` — Enum, Not Bool

```rust
/// Deposit and API key privacy mode.
///
/// # Semantic distinction from bool
/// `PrivacyMode::Transparent` and `PrivacyMode::Shielded` carry meaning.
/// A bare `bool` (`is_private: true`) would be "boolean blindness" —
/// the reader must decode what `true` means at every use site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrivacyMode {
    /// Direct on-chain deposit/withdrawal — visible to public explorers
    Transparent,
    /// Via Hinkal Shielded Pool — wallet addresses and amounts not visible on-chain
    Shielded,
}
```

**Rust-expertise check:**
- ✅ Enum over bool — eliminates "boolean blindness" anti-pattern
- ✅ `Copy` — lightweight discriminant
- ✅ Exhaustive matching — compiler enforces handling both modes at every decision point

#### `ApiKeyCapability` — Signed Capability Token

```rust
/// An API key is an Ed25519-signed capability token, not an opaque bearer string.
///
/// # OCAP alignment (P4)
/// The capability carries its own attenuation: spending_limit_rj, expiry, privacy_mode.
/// The Ed25519 signature proves it was issued by a specific wallet.
/// Verification: derive public key from the presented private key, look up capability,
/// check signature against wallet's public key, check limits.
///
/// # Invariant
/// spent_rj <= spending_limit_rj at all times. The WalletBackedBudget enforces this
/// before every tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCapability {
    pub wallet_id: WalletId,
    pub key_id: ApiKeyId,
    pub public_key: Ed25519PublicKey,
    pub spending_limit_rj: RJoule,
    pub spent_rj: RJoule,
    pub expiry: Option<DateTime<Utc>>,
    pub issued_at: DateTime<Utc>,
    pub privacy_mode: PrivacyMode,
    pub preferred_chain: Option<ChainId>,
}
```

**Rust-expertise check:**
- ✅ `RJoule` newtype for limits — can't accidentally set limit in gas units
- ✅ `Option<DateTime<Utc>>` for expiry — `None` means "no expiry," explicit
- ✅ `PrivacyMode` on the capability — privacy is a property of the key, not the wallet
- ✅ `Ed25519PublicKey` — type-safe, not a `String` or `[u8; 32]`
- ❌ Anti-pattern avoided: not a JWT string that must be parsed at every use site

#### `WalletId` and `ApiKeyId` — Phantom-Type IDs

```rust
// Following the existing hkask-types pattern (see id.rs):
pub enum WalletKind {}
impl private::Sealed for WalletKind {}
impl IdKind for WalletKind {}
pub type WalletId = Id<WalletKind>;

pub enum ApiKeyKind {}
impl private::Sealed for ApiKeyKind {}
impl IdKind for ApiKeyKind {}
pub type ApiKeyId = Id<ApiKeyKind>;
```

**Rust-expertise check:**
- ✅ Follows existing `Id<T>` pattern — `WalletId` and `ApiKeyId` are different types, compiler prevents swapping them
- ✅ PhantomData — zero runtime overhead
- ✅ `Copy` + UUID-based — lightweight, globally unique

### 2.3 Ownership Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  OWNERSHIP DAG                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  AgentService (sole owner)                              │
│  ├── WalletManager (owned)                              │
│  │   ├── Vec<ChainPort> (owned, one per chain)          │
│  │   │   ├── SolanaPort (owned)                         │
│  │   │   └── HederaPort (owned)                         │
│  │   ├── Option<PrivacyPort> (owned, HinkalPort)        │
│  │   ├── ApiKeyIssuer (owned)                           │
│  │   └── Arc<WalletStore> (shared with CNS)             │
│  │                                                       │
│  ├── CyberneticsLoop (owned)                            │
│  │   └── Arc<WalletStore> (shared, read-only)           │
│  │                                                       │
│  └── ApiState / ReplState                               │
│      └── &WalletService (borrowed from AgentService)    │
│                                                         │
│  Key decisions:                                         │
│  - WalletManager is sole-owned by AgentService          │
│  - WalletStore is Arc<> because CNS needs read access   │
│    for algedonic alerts (balance monitoring)            │
│  - ChainPorts are owned by WalletManager — they live    │
│    and die with the wallet subsystem                    │
│  - API surfaces borrow WalletService, never own it      │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Rust-expertise check:**
- ✅ Sole ownership as default — `WalletManager` owns its ports
- ✅ `Arc<WalletStore>` is justified — CNS needs concurrent read access for algedonic monitoring. This is a deliberate relaxation, not a default.
- ✅ Surfaces borrow (`&WalletService`), never own — follows hexagonal dependency direction
- ❌ Anti-pattern avoided: no `Rc<RefCell<>>` — the ownership DAG is clear

### 2.4 Error Design

```rust
/// Wallet-specific error domain.
///
/// # Design principles (rust-expertise §7)
/// - Typed errors for library code (thiserror)
/// - Each variant carries context, not just a name
/// - Never discard errors silently
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("insufficient rJoule balance: have {have}, need {need}")]
    InsufficientBalance { have: RJoule, need: RJoule },

    #[error("API key {key_id} spending limit exceeded: {spent} / {limit}")]
    SpendingLimitExceeded { key_id: ApiKeyId, spent: RJoule, limit: RJoule },

    #[error("API key {key_id} expired at {expiry}")]
    KeyExpired { key_id: ApiKeyId, expiry: DateTime<Utc> },

    #[error("API key {key_id} has been revoked")]
    KeyRevoked { key_id: ApiKeyId },

    #[error("chain {chain:?} is not enabled for this wallet")]
    ChainNotEnabled { chain: ChainId },

    #[error("privacy layer unavailable for chain {chain:?}")]
    PrivacyUnavailable { chain: ChainId },

    #[error("deposit reference {reference} not found or expired")]
    DepositReferenceInvalid { reference: String },

    #[error("chain error ({chain:?}): {message}")]
    ChainError { chain: ChainId, message: String },

    #[error("privacy layer error: {message}")]
    PrivacyError { message: String },
}
```

**Rust-expertise check:**
- ✅ `thiserror` for library error type
- ✅ Each variant carries context (`have`/`need`, `spent`/`limit`, `chain`, `reference`)
- ✅ `Display` impls are human-readable
- ❌ Anti-pattern avoided: no `Error::Other(String)` — every error path is typed

---

## 3. Coding Guidelines Review — Surgical Change Audit

### 3.1 Think Before Coding — Assumptions Surfaced

| Assumption | Status | Risk |
|-----------|--------|------|
| Solana RPC will be available and responsive for deposit monitoring | `[IS-SUBJ]` — depends on RPC provider (Helius, Triton, or public) | MEDIUM — mitigated by retry + fallback RPC endpoints |
| Hedera mirror node will be available for HTS transaction monitoring | `[IS-SUBJ]` — Hedera public mirror nodes are reliable but rate-limited | LOW — mirror nodes are well-established |
| Hinkal will deploy on Solana within the implementation timeline | `[IS-SUBJ]` — Hinkal's roadmap says "Solana planned" but no date | HIGH — `PrivacyPort` trait abstracts this; transparent path works regardless |
| Users will have Solana/Hedera wallets and understand crypto deposits | `[IS-PROB]` — based on crypto adoption trends | MEDIUM — mitigated by clear CLI help text and deposit-address command |
| USDC will remain the dominant stablecoin on both chains | `[IS-PROB]` — USDC has $115M on Hedera, billions on Solana | LOW — USDC is well-established on both |

### 3.2 Simplicity First — What We Are NOT Building

Per coding-guidelines principle 2, these are explicitly excluded:

- ❌ No on-chain rJoule token (ERC-20, SPL, HTS) — rJoule is an internal accounting unit, not a tradeable token
- ❌ No DEX integration, swapping, or yield — hKask is not a DeFi platform
- ❌ No multi-sig, DAO governance, or voting for treasury — treasury is single-key, derived from master passphrase
- ❌ No invoice generation, billing statements, or PDF receipts — headless constraint
- ❌ No subscription tiers, pricing plans, or "premium" features — all features available to all users; spending limits are user-set, not system-imposed
- ❌ No fiat on-ramp (Stripe, PayPal) in Phase 1 — crypto-native only; fiat on-ramp is a future consideration
- ❌ No Lightning Network support in Phase 1 — Solana + Hedera cover the micropayment use case
- ❌ No KYC/AML integration — Hinkal's zkMe attestation is optional and user-side; hKask does not collect identity data

### 3.3 Surgical Changes — What Each Phase Touches

| Phase | Crate | Files Touched | New Files | Lines Changed (est.) |
|-------|-------|--------------|-----------|---------------------|
| 1 | `hkask-types` | `event.rs` (+spans), `lib.rs` (+modules) | `wallet.rs` (~200 lines) | ~30 changed, ~200 new |
| 2 | `hkask-storage` | `lib.rs` (+module) | `wallet_store.rs` (~300 lines), migration SQL | ~10 changed, ~300 new |
| 3 | `hkask-keystore` | `keychain.rs` (+2 resolve fns) | none | ~40 changed |
| 4 | `hkask-wallet` | none (new crate) | `lib.rs`, `chain.rs`, `solana.rs`, `hedera.rs`, `privacy.rs`, `hinkal.rs`, `manager.rs`, `issuer.rs`, `deposit_ref.rs` (~1500 lines) | ~1500 new |
| 5 | `hkask-cns` | `energy.rs` (+variant), `energy_budget_management.rs` (+estimator), `cybernetics_loop.rs` (+wallet spans) | `wallet_budget.rs` (~200 lines) | ~100 changed, ~200 new |
| 6 | `hkask-services` | `lib.rs` (+WalletService field), `agent_service.rs` (+build wiring) | `wallet_service.rs` (~150 lines) | ~50 changed, ~150 new |
| 7 | `hkask-cli` | `cli/actions.rs` (+WalletAction), `commands/mod.rs` (+re-export) | `commands/wallet.rs` (~300 lines) | ~20 changed, ~300 new |
| 8 | `hkask-api` | `routes/mod.rs` (+wallet routes) | `routes/wallet.rs` (~200 lines), `middleware/api_key_auth.rs` (~100 lines) | ~20 changed, ~300 new |

**Total estimated:** ~200 lines changed across existing crates, ~2950 new lines across new crate + new files. `[IS-SUBJ]` — based on similar past work (replicant server mode was ~1500 lines).

### 3.4 Goal-Driven Execution — Success Criteria Per Phase

Each phase has verifiable success criteria. No phase is "done" until its criteria pass.

---

## 4. Cybernetic Analysis (Pragmatic Cybernetics)

### 4.1 VSM Mapping — Where the Wallet Fits

The wallet subsystem maps into hKask's Viable System Model:

| VSM System | Wallet Component | Function |
|------------|-----------------|----------|
| **S1 (Operations)** | `ChainPort` implementations, `HinkalPort` | Primary activity: monitor deposits, submit withdrawals, shield/unshield |
| **S2 (Coordination)** | `WalletBackedBudget.reserve()` / `settle()` | Anti-oscillation: reserve-before-spend prevents double-spending; settlement refunds unused gas |
| **S3 (Control)** | `cns.wallet.balance` span + algedonic threshold | "Is the wallet healthy?" Balance < 10% of 30-day avg → Curator alert |
| **S3\* (Audit)** | `kask wallet balance`, `kask wallet history` | Sporadic direct probe — user checks balance, bypassing cached CNS state |
| **S4 (Intelligence)** | Curator Agent + `cns.wallet.*` spans | "What does this spending pattern mean? Is the treasury running low?" |
| **S5 (Policy)** | Magna Carta P1–P4, `ApiKeyCapability` attenuation | Identity: wallet keys derived from master passphrase. Constraints: spending limits, expiry, privacy mode. Refusal: exhausted keys reject operations. |

**Viability assessment:** `[IS-SUBJ]`
- S1 is viable: both chain ports have independent monitoring loops
- S2 is viable: reserve-settle pattern is proven in existing gas system
- S3 is viable: CNS spans + algedonic alerts provide regulation
- S3* is viable: CLI commands provide direct audit path
- S4 is viable: Curator receives wallet spans through existing CNS→Curator pathway
- S5 is viable: Magna Carta principles are embedded in capability token design

### 4.2 Feedback Loop Analysis

The wallet introduces three new feedback loops:

#### Loop 1: Deposit → Credit → Balance

```
User deposits USDC → ChainPort detects → WalletManager credits rJoules
    → WalletStore updates balance → CNS emits cns.wallet.balance
    → Curator observes balance change
```

| Property | Analysis |
|----------|----------|
| **Polarity** | Positive (balance increases) — bounded by deposit amount. Not runaway. |
| **Delay** | Solana: ~12.8s finality + poll interval (30s). Hedera: ~0s finality + poll interval (30s). `[IS-DECL]` |
| **Gain** | 1:1 — each USDC credits exactly `rj_per_usdc` rJoules. No amplification. |
| **Closure** | Closed — deposit triggers credit, credit updates balance, balance is observable. |
| **Fidelity** | High — on-chain transaction confirmation is ground truth. Risk: RPC failure → missed deposit. Mitigated by retry + multiple RPC endpoints. |

#### Loop 2: Spend → Debit → Exhaustion

```
API key used → GovernedTool estimates gas → WalletEnergyEstimator converts to rJoules
    → WalletBackedBudget reserves → tool executes → settles (debits rJoules)
    → WalletStore updates spent_rj on ApiKeyCapability
    → If spent_rj >= spending_limit_rj: CNS emits cns.wallet.key_exhausted → Curator alert
```

| Property | Analysis |
|----------|----------|
| **Polarity** | Negative (balance decreases) — stabilizing. Exhaustion triggers rejection. |
| **Delay** | ~0ms — reserve/check is synchronous before tool execution. `[IS-DECL]` |
| **Gain** | Configurable via `gas_per_rjoule`. Default 1000 gas/rJ means 0.001 rJ per gas unit — fine-grained. |
| **Closure** | Closed — spend → debit → balance update → limit check → reject on exhaustion. |
| **Fidelity** | High — every tool invocation passes through GovernedTool (existing membrane). No bypass possible. |

#### Loop 3: Treasury Balance → Solvency

```
Deposits increase treasury → Withdrawals decrease treasury
    → CNS emits cns.wallet.treasury per chain
    → If treasury < min_reserve: algedonic alert → Human escalation
```

| Property | Analysis |
|----------|----------|
| **Polarity** | Negative — treasury depletes with withdrawals, replenishes with deposits. |
| **Delay** | On-chain confirmation time + poll interval. |
| **Gain** | 1:1 — each withdrawal decreases treasury by exactly the amount sent. |
| **Closure** | Closed — treasury balance is observable on-chain and via CNS. |
| **Fidelity** | High — on-chain balance is ground truth. Risk: hKask's view of treasury diverges from on-chain reality. Mitigated by periodic full reconciliation. |

### 4.3 Variety Engineering — Ashby's Law Check

**System variety (disturbances the wallet must handle):** `[IS-SUBJ]`
1. Deposit on Solana (SPL USDC transfer)
2. Deposit on Hedera (HTS USDC transfer)
3. Shielded deposit via Hinkal (zkSNARK proof + stealth address)
4. Transparent withdrawal to user address
5. Shielded withdrawal via Hinkal
6. API key exhaustion (spending limit reached)
7. API key expiry
8. API key revocation
9. Treasury insolvency (more withdrawals than deposits)
10. Chain RPC failure (deposit missed)
11. Hinkal relayer failure (shielded transfer missed)
12. Deposit reference collision or replay
13. Gas↔rJoule conversion rate change
14. USDC depeg event (rJoule value drifts from intended $0.001)

**Regulator variety (CNS spans + algedonic alerts):** `[IS-SUBJ]`
1. `cns.wallet.deposit` — covers #1, #2
2. `cns.wallet.deposit_shielded` — covers #3
3. `cns.wallet.withdrawal` — covers #4, #5
4. `cns.wallet.key_exhausted` — covers #6
5. `cns.wallet.key_revoked` — covers #8
6. `cns.wallet.treasury` — covers #9
7. `cns.wallet.conversion` — covers #13
8. `cns.wallet.balance` — covers #14 (balance anomaly detection)

**Gap analysis:** `[IS-DECL]`
- #7 (key expiry): Not covered by a dedicated CNS span. **Gap.** Add `cns.wallet.key_expired` span.
- #10 (RPC failure): Not covered by a dedicated CNS span. **Gap.** Add `cns.wallet.chain_error` span.
- #11 (Hinkal failure): Not covered. **Gap.** Add `cns.wallet.privacy_error` span.
- #12 (deposit reference replay): Not covered. **Mitigation:** References are one-time (burned on use) + time-bounded (expiry). The type system prevents replay at the storage layer.

**Verdict:** Regulator variety (8 spans + 3 gaps to fill = 11) < System variety (14). **Variety deficit of 3.** `[IS-DECL]`

**Resolution:** Add the three gap spans in Phase 1 (types). This brings regulator variety to 11, which is sufficient — the remaining 3 disturbances (#7 expiry handled by capability check, #12 replay prevented by type system, #14 depeg is an economic externality beyond CNS scope).

### 4.4 Good Regulator Check (Conant-Ashby)

**The regulator's model of the system:** The CNS wallet spans model the wallet as: balance, deposits, withdrawals, conversions, key lifecycle, treasury. `[IS-DECL]`

**Where the model diverges from reality:** `[IS-SUBJ]`
- The CNS model tracks rJoule balance but not the on-chain USDC balance directly. If the conversion rate changes, the model's "USDC equivalent" diverges from reality.
- The CNS model tracks per-key spending but not per-tool spending patterns. A key used exclusively for expensive tools (fal at 100 gas) depletes faster than one used for cheap tools (memory at 1 gas). The model doesn't distinguish.

**Is the model updated when the system changes?** Yes — every deposit, withdrawal, and spend emits a CNS span. The model is event-sourced from ν-events. `[IS-DECL]`

**Does the model include failure modes?** Partially. Deposit detection failure (RPC down) and privacy layer failure (Hinkal relayer down) are the gaps identified above. After adding the three gap spans, the model covers 11 of 14 disturbance paths. `[IS-DECL]`

---

## 5. Detailed Implementation Steps

### Phase 1: Types (`hkask-types`) — EST. 2–3 hours

**Goal:** All new types compile. CNS span registry updated. No behavior changes to existing code.

**Files:**
- **NEW:** `crates/hkask-types/src/wallet.rs` — all wallet types
- **MODIFIED:** `crates/hkask-types/src/event.rs` — add `cns.wallet.*` spans to `CANONICAL_NAMESPACES`
- **MODIFIED:** `crates/hkask-types/src/id.rs` — add `WalletKind`, `ApiKeyKind`, type aliases
- **MODIFIED:** `crates/hkask-types/src/lib.rs` — add `mod wallet`, re-exports

**Step 1.1 — ID types** (surgical: follow existing `IdKind` pattern exactly)
```rust
// In id.rs, after existing kinds:
pub enum WalletKind {}
impl private::Sealed for WalletKind {}
impl IdKind for WalletKind {}
pub type WalletId = Id<WalletKind>;

pub enum ApiKeyKind {}
impl private::Sealed for ApiKeyKind {}
impl IdKind for ApiKeyKind {}
pub type ApiKeyId = Id<ApiKeyKind>;
```

**Step 1.2 — Core value types** (new file: `wallet.rs`)
- `RJoule(u64)` — with `new(v: u64) -> Self` (infallible, zero is valid), `Display` (shows "rJ"), `Add`/`Sub` (saturating), `PartialOrd`+`Ord`
- `ChainId` — enum `{ Solana, Hedera }`, `Display`, `FromStr`
- `PrivacyMode` — enum `{ Transparent, Shielded }`, `Display`, `FromStr`
- `DepositAddress(String, ChainId, PrivacyMode)` — newtype for validated deposit addresses

**Step 1.3 — Configuration** (new file: `wallet.rs`)
- `WalletConfig` — `rj_per_usdc: u64` (default 1000), `gas_per_rjoule: u64` (default 1000), `min_deposit_usdc: Decimal`, `enabled_chains: Vec<ChainId>`, `privacy_enabled: bool`, `hinkal_relayer_url: Option<String>`
- `WalletBalance` — `wallet_id: WalletId`, `rjoules: u64`, `usdc_equivalent: Decimal`, `gas_equivalent: u64`

**Step 1.4 — API key capability** (new file: `wallet.rs`)
- `ApiKeyCapability` — as designed in §2.2 above
- `Ed25519PublicKey` — newtype around `[u8; 32]` (or use existing ed25519-dalek types)
- Note: `Ed25519PublicKey` may already exist in keystore; if so, re-export rather than duplicate

**Step 1.5 — Transactions** (new file: `wallet.rs`)
- `TransactionType` — enum with variants: `Deposit { chain, privacy, tx_hash, amount_usdc }`, `Withdrawal { chain, privacy, tx_hash, amount_usdc }`, `Spend { key_id, tool, gas, rj }`, `Refund { key_id, reason, rj }`
- `WalletTransaction` — `id: u64`, `wallet_id: WalletId`, `tx_type: TransactionType`, `rjoules_delta: i64`, `balance_after: u64`, `timestamp: DateTime<Utc>`

**Step 1.6 — Error type** (new file: `wallet.rs`)
- `WalletError` — as designed in §2.4 above

**Step 1.7 — CNS spans** (modify: `event.rs`)
Add to `CANONICAL_NAMESPACES`:
```rust
"cns.wallet.balance",
"cns.wallet.deposit",
"cns.wallet.deposit_shielded",
"cns.wallet.withdrawal",
"cns.wallet.conversion",
"cns.wallet.key_issued",
"cns.wallet.key_revoked",
"cns.wallet.key_expired",       // ← gap fill from variety analysis
"cns.wallet.key_exhausted",
"cns.wallet.treasury",
"cns.wallet.chain_error",       // ← gap fill from variety analysis
"cns.wallet.privacy.shield",
"cns.wallet.privacy.unshield",
"cns.wallet.privacy_error",     // ← gap fill from variety analysis
```

**Step 1.8 — Re-exports** (modify: `lib.rs`)
```rust
pub mod wallet;
pub use wallet::*;
```

**Verification:**
```bash
cargo check -p hkask-types
# Must compile with zero errors and zero warnings
```

**Success criteria:**
- [x] `RJoule` newtype compiles with `Display`, `Add`, `Sub`, `PartialOrd`
- [x] `ChainId` enum compiles with exhaustive `Display` and `FromStr`
- [x] `PrivacyMode` enum compiles — no boolean blindness
- [x] `ApiKeyCapability` struct compiles with all fields
- [x] `WalletError` enum compiles with `thiserror::Error` derive
- [x] `CANONICAL_NAMESPACES` includes all 14 new `cns.wallet.*` spans
- [x] `WalletId` and `ApiKeyId` are distinct types (compiler rejects swapping them)
- [x] `cargo check -p hkask-types` passes with zero errors/warnings
- [x] `cargo test -p hkask-types` passes (11/11 tests, 7 new wallet tests)

**Deviation from plan:** `Decimal` type not available in workspace. Used `u64` micro-USDC (1 = 0.000001 USDC) for all USDC amount fields. Fields suffixed `_micro` for clarity. This avoids adding a new dependency and keeps money math exact (integer arithmetic).

---

### Phase 2: Storage (`hkask-storage`) — EST. 3–4 hours

**Goal:** Wallet tables exist in SQLite. Migrations run. Integration test passes.

**Files:**
- **NEW:** `crates/hkask-storage/src/wallet_store.rs` — `WalletStore` struct + methods
- **MODIFIED:** `crates/hkask-storage/src/lib.rs` — add module, re-export
- **NEW:** Migration SQL files

**Step 2.1 — Schema design** (surgical: follow existing table patterns in hkask-storage)

```sql
-- wallet_balances: one row per wallet
CREATE TABLE wallet_balances (
    wallet_id TEXT PRIMARY KEY,
    balance_rj INTEGER NOT NULL DEFAULT 0,       -- u64 stored as INTEGER
    usdc_equivalent REAL NOT NULL DEFAULT 0.0,   -- approximate, for display
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- wallet_transactions: append-only ledger
CREATE TABLE wallet_transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_id TEXT NOT NULL REFERENCES wallet_balances(wallet_id),
    tx_type TEXT NOT NULL,                       -- "deposit" | "withdrawal" | "spend" | "refund"
    tx_subtype TEXT,                             -- "transparent" | "shielded"
    chain TEXT,                                  -- "solana" | "hedera" | NULL (for shielded)
    on_chain_tx_hash TEXT,                       -- NULL for spends/refunds
    amount_rj INTEGER NOT NULL,                 -- positive=credit, negative=debit
    balance_after_rj INTEGER NOT NULL,
    key_id TEXT,                                 -- NULL for deposits/withdrawals
    tool_name TEXT,                              -- NULL for non-spend transactions
    gas_units INTEGER,                           -- NULL for non-spend transactions
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_wallet_tx_wallet_id ON wallet_transactions(wallet_id);
CREATE INDEX idx_wallet_tx_created_at ON wallet_transactions(created_at);

-- api_keys: issued capability tokens
CREATE TABLE api_keys (
    key_id TEXT PRIMARY KEY,
    wallet_id TEXT NOT NULL REFERENCES wallet_balances(wallet_id),
    public_key BLOB NOT NULL,                    -- Ed25519 public key (32 bytes)
    spending_limit_rj INTEGER NOT NULL,
    spent_rj INTEGER NOT NULL DEFAULT 0,
    privacy_mode TEXT NOT NULL DEFAULT 'transparent',  -- "transparent" | "shielded"
    preferred_chain TEXT,                       -- "solana" | "hedera" | NULL
    expires_at TEXT,                             -- NULL = no expiry
    issued_at TEXT NOT NULL,
    revoked_at TEXT,                             -- NULL = active
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_api_keys_wallet_id ON api_keys(wallet_id);
CREATE INDEX idx_api_keys_public_key ON api_keys(public_key);

-- deposit_addresses: derived addresses per wallet per chain
CREATE TABLE deposit_addresses (
    wallet_id TEXT NOT NULL,
    chain TEXT NOT NULL,                         -- "solana" | "hedera"
    address TEXT NOT NULL,
    derivation_index INTEGER NOT NULL,
    privacy_mode TEXT NOT NULL DEFAULT 'transparent',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (wallet_id, chain, derivation_index)
);

-- deposit_references: one-time shielded deposit references
CREATE TABLE deposit_references (
    reference TEXT PRIMARY KEY,
    wallet_id TEXT NOT NULL REFERENCES wallet_balances(wallet_id),
    chain TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    spent BOOLEAN NOT NULL DEFAULT 0,            -- burned on use
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_deposit_refs_wallet_id ON deposit_references(wallet_id);
CREATE INDEX idx_deposit_refs_expires ON deposit_references(expires_at);
```

**Step 2.2 — `WalletStore` struct** (new file)

```rust
pub struct WalletStore {
    conn: Arc<Mutex<SqliteConnection>>,  // or ConnectionPool if hkask-storage uses pooling
}

impl WalletStore {
    // ── Balance ──
    pub fn get_balance(&self, wallet_id: WalletId) -> Result<Option<WalletBalance>, StorageError>;
    pub fn credit_rjoules(&self, wallet_id: WalletId, amount: RJoule) -> Result<WalletBalance, StorageError>;
    pub fn debit_rjoules(&self, wallet_id: WalletId, amount: RJoule) -> Result<WalletBalance, StorageError>;

    // ── Transactions ──
    pub fn record_transaction(&self, tx: WalletTransaction) -> Result<(), StorageError>;
    pub fn get_transactions(&self, wallet_id: WalletId, limit: u32, offset: u32) -> Result<Vec<WalletTransaction>, StorageError>;

    // ── API Keys ──
    pub fn store_api_key(&self, capability: &ApiKeyCapability) -> Result<(), StorageError>;
    pub fn get_api_key(&self, key_id: ApiKeyId) -> Result<Option<ApiKeyCapability>, StorageError>;
    pub fn get_api_key_by_public_key(&self, public_key: &[u8]) -> Result<Option<ApiKeyCapability>, StorageError>;
    pub fn list_api_keys(&self, wallet_id: WalletId) -> Result<Vec<ApiKeyCapability>, StorageError>;
    pub fn revoke_api_key(&self, key_id: ApiKeyId) -> Result<(), StorageError>;
    pub fn update_spent_rj(&self, key_id: ApiKeyId, spent: RJoule) -> Result<(), StorageError>;

    // ── Deposit Addresses ──
    pub fn store_deposit_address(&self, wallet_id: WalletId, chain: ChainId, address: &str, index: u64, privacy: PrivacyMode) -> Result<(), StorageError>;
    pub fn get_deposit_addresses(&self, wallet_id: WalletId) -> Result<Vec<DepositAddress>, StorageError>;

    // ── Deposit References ──
    pub fn store_deposit_reference(&self, reference: &str, wallet_id: WalletId, chain: ChainId, expires_at: DateTime<Utc>) -> Result<(), StorageError>;
    pub fn consume_deposit_reference(&self, reference: &str) -> Result<Option<WalletId>, StorageError>;  // returns wallet_id if valid, burns reference
    pub fn purge_expired_references(&self) -> Result<u64, StorageError>;
}
```

**Step 2.3 — Migration** (follow existing hkask-storage migration pattern)
- Add migration version for wallet tables
- Ensure idempotent (IF NOT EXISTS)

**Verification:**
```bash
cargo check -p hkask-storage
cargo test -p hkask-storage -- wallet_store
```

**Success criteria:**
- [ ] All 5 tables created via migration
- [ ] `credit_rjoules` + `debit_rjoules` maintain balance invariant (never negative)
- [ ] `consume_deposit_reference` burns reference (spent=true) and returns correct wallet_id
- [ ] `get_api_key_by_public_key` finds key by Ed25519 public key bytes
- [ ] `update_spent_rj` correctly tracks cumulative spending
- [ ] Integration test: create wallet → credit → debit → verify balance → record transaction → retrieve history

---

### Phase 3: Keystore (`hkask-keystore`) — EST. 1–2 hours

**Goal:** Treasury key derivation per chain. Wallet seed derivation. Ed25519 signing for API keys.

**Files:**
- **MODIFIED:** `crates/hkask-keystore/src/keychain.rs` — add `resolve_treasury_key(chain)` and `resolve_wallet_seed()`
- **MODIFIED:** `crates/hkask-keystore/src/lib.rs` — re-exports

**Step 3.1 — Treasury key derivation** (surgical: follow existing `resolve_*` pattern)

```rust
/// Derive a chain-specific treasury keypair from the master key.
///
/// Uses HKDF-SHA256 with domain-separated context strings.
/// Same master passphrase → same treasury key for a given chain.
///
/// # Context strings
/// - Solana: "hkask:treasury-solana"
/// - Hedera: "hkask:treasury-hedera"
pub fn resolve_treasury_key(chain: ChainId) -> Result<TreasuryKeypair, KeychainError> {
    let context = match chain {
        ChainId::Solana => "hkask:treasury-solana",
        ChainId::Hedera => "hkask:treasury-hedera",
    };
    let master_key = derive_master_key()?;
    let key_bytes = hkdf_expand(&master_key, context.as_bytes())?;
    Ok(TreasuryKeypair::from_bytes(&key_bytes)?)
}
```

**Step 3.2 — Wallet seed derivation**

```rust
/// Derive a wallet-specific seed for HD derivation and deposit reference generation.
///
/// Context: "hkask:wallet-seed"
/// This seed is used for:
/// - Deriving deposit addresses (BIP44-style per chain)
/// - Generating deposit references (HKDF with nonce + expiry)
/// - Signing API key capability tokens (Ed25519)
pub fn resolve_wallet_seed() -> Result<Zeroizing<[u8; 32]>, KeychainError> {
    let master_key = derive_master_key()?;
    hkdf_expand(&master_key, b"hkask:wallet-seed")
}
```

**Step 3.3 — Ed25519 API key signing** (leverage existing `spec_signer.rs` pattern)

```rust
/// Sign an ApiKeyCapability with the wallet's Ed25519 key.
///
/// The signature proves the capability was issued by the wallet holder.
/// Verification: derive public key from wallet seed, verify signature against capability bytes.
pub fn sign_api_key_capability(capability: &ApiKeyCapability) -> Result<Ed25519Signature, KeychainError> {
    let seed = resolve_wallet_seed()?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
    let canonical_bytes = serde_json::to_vec(capability)?;  // canonical JSON per spec_signer pattern
    Ok(signing_key.sign(&canonical_bytes))
}
```

**Verification:**
```bash
cargo check -p hkask-keystore
cargo test -p hkask-keystore -- treasury
```

**Success criteria:**
- [ ] `resolve_treasury_key(Solana)` and `resolve_treasury_key(Hedera)` return different keys
- [ ] Same master passphrase → same treasury key (deterministic)
- [ ] `resolve_wallet_seed()` returns 32 zeroizing bytes
- [ ] `sign_api_key_capability` produces verifiable Ed25519 signature
- [ ] Signature verification succeeds with correct capability, fails with tampered capability

---

### Phase 4: Wallet Crate (`hkask-wallet`) — EST. 8–12 hours

**Goal:** New crate with ChainPort, PrivacyPort, WalletManager, ApiKeyIssuer. Unit tests with mock ports.

**Files (all NEW):**
- `crates/hkask-wallet/Cargo.toml`
- `crates/hkask-wallet/src/lib.rs`
- `crates/hkask-wallet/src/chain.rs` — `ChainPort` trait + `DepositEvent` type
- `crates/hkask-wallet/src/solana.rs` — `SolanaPort` implementation
- `crates/hkask-wallet/src/hedera.rs` — `HederaPort` implementation
- `crates/hkask-wallet/src/privacy.rs` — `PrivacyPort` trait + `ShieldedTransfer` type
- `crates/hkask-wallet/src/hinkal.rs` — `HinkalPort` implementation
- `crates/hkask-wallet/src/manager.rs` — `WalletManager` (orchestration)
- `crates/hkask-wallet/src/issuer.rs` — `ApiKeyIssuer` (key generation + signing)
- `crates/hkask-wallet/src/deposit_ref.rs` — Deposit reference generation + verification
- `crates/hkask-wallet/src/error.rs` — Wallet-specific error types (if not all in hkask-types)
- `crates/hkask-wallet/tests/mock_chain.rs` — Mock ChainPort for testing
- `crates/hkask-wallet/tests/mock_privacy.rs` — Mock PrivacyPort for testing
- `crates/hkask-wallet/tests/manager_tests.rs` — WalletManager integration tests
- `crates/hkask-wallet/tests/issuer_tests.rs` — ApiKeyIssuer tests

**Step 4.1 — Cargo.toml**

```toml
[package]
name = "hkask-wallet"
version.workspace = true
edition.workspace = true

[dependencies]
hkask-types = { path = "../hkask-types" }
hkask-keystore = { path = "../hkask-keystore" }
hkask-storage = { path = "../hkask-storage" }
solana-sdk = { version = "2", optional = true }
solana-client = { version = "2", optional = true }
spl-token = { version = "6", optional = true }
# Hedera SDK — use hedera-sdk-rust or direct HAPI gRPC
# hedera-sdk = { version = "...", optional = true }
ed25519-dalek = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true, features = ["full"] }
chrono = { workspace = true }
async-trait = "0.1"
zeroize = { workspace = true }

[features]
default = ["solana"]
solana = ["dep:solana-sdk", "dep:solana-client", "dep:spl-token"]
hedera = []  # placeholder until SDK is integrated

[dev-dependencies]
hkask-storage = { path = "../hkask-storage", features = ["test-utils"] }
```

**Step 4.2 — `ChainPort` trait** (chain.rs)

```rust
/// Abstract interface for blockchain deposit monitoring and withdrawal.
///
/// # Implementations
/// - SolanaPort: SPL USDC on Solana
/// - HederaPort: HTS USDC on Hedera
///
/// # Design (rust-expertise §6: Composition over Inheritance)
/// ChainPort is a trait for capability, not a base class. Each implementation
/// is a standalone struct with its own RPC connection, key material, and state.
#[async_trait]
pub trait ChainPort: Send + Sync {
    fn chain_id(&self) -> ChainId;

    /// Derive a new deposit address from the treasury seed + index.
    /// Deterministic: same seed + index → same address.
    fn derive_deposit_address(&self, index: u64) -> Result<String, WalletError>;

    /// Poll the blockchain for deposits to the given addresses.
    /// Returns confirmed deposits not yet recorded.
    async fn monitor_deposits(&self, addresses: &HashSet<String>) -> Result<Vec<DepositEvent>, WalletError>;

    /// Submit a withdrawal transaction from the treasury to a user address.
    async fn submit_withdrawal(&self, to: &str, amount_usdc: Decimal) -> Result<TxHash, WalletError>;

    /// Get the number of confirmations for a transaction.
    async fn confirmations(&self, tx_hash: &TxHash) -> Result<u64, WalletError>;

    /// Get the current USD value of the native token (for fee estimation).
    async fn native_token_usd_rate(&self) -> Result<Decimal, WalletError>;
}

pub struct DepositEvent {
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub amount_usdc: Decimal,
    pub confirmations: u64,
    pub block_time: DateTime<Utc>,
}

pub struct TxHash(pub String);
```

**Step 4.3 — `SolanaPort`** (solana.rs)

Key design decisions:
- Use `solana-client` for RPC connection (Helius, Triton, or public endpoint)
- Use `spl-token` for USDC token account parsing
- Treasury keypair loaded from keystore via `resolve_treasury_key(ChainId::Solana)`
- Deposit addresses derived via HD derivation (BIP44: m/44'/501'/0'/0/{index})
- Monitor: poll `get_signatures_for_address` + `get_transaction` for USDC transfers
- Withdrawal: build SPL token transfer instruction, sign with treasury keypair, submit

**Step 4.4 — `HederaPort`** (hedera.rs)

Key design decisions:
- Use Hedera mirror node REST API for transaction monitoring (no SDK dependency initially)
- Or use `hedera-sdk-rust` if available and stable
- Treasury account ID derived from treasury keypair
- HTS USDC token ID: `0.0.456858` (Hedera mainnet USDC token)
- Monitor: poll mirror node `/api/v1/transactions?account.id=<treasury>&token.id=0.0.456858`
- Withdrawal: `CryptoTransferTransaction` with HTS token transfer
- Fixed fee: $0.0001 per HTS transfer (predictable, no gas estimation needed)

**Step 4.5 — `PrivacyPort` trait** (privacy.rs)

```rust
#[async_trait]
pub trait PrivacyPort: Send + Sync {
    /// hKask's shielded address in the privacy pool.
    fn our_shielded_address(&self) -> Result<String, WalletError>;

    /// Generate a shielded deposit address for a user wallet.
    fn shielded_deposit_address(&self, wallet_id: WalletId) -> Result<String, WalletError>;

    /// Monitor the Shielded Pool for incoming transfers to our shielded address.
    async fn monitor_shielded_transfers(&self) -> Result<Vec<ShieldedTransfer>, WalletError>;

    /// Shield assets into the pool (for private withdrawals).
    async fn shield(&self, amount_usdc: Decimal, chain: ChainId) -> Result<TxHash, WalletError>;

    /// Unshield assets from the pool to a public address.
    async fn unshield(&self, to_public: &str, amount_usdc: Decimal) -> Result<TxHash, WalletError>;

    /// Check if the privacy layer is available for a given chain.
    fn available_for_chain(&self, chain: ChainId) -> bool;
}

pub struct ShieldedTransfer {
    pub commitment: String,       // zkSNARK commitment hash
    pub from_shielded: String,    // sender's shielded address
    pub to_shielded: String,      // our shielded address
    pub amount_usdc: Decimal,
    pub memo: Option<String>,     // deposit reference
    pub block_time: DateTime<Utc>,
}
```

**Step 4.6 — `HinkalPort`** (hinkal.rs)

Key design decisions:
- Hinkal is primarily EVM-native today; Solana support is on their roadmap
- Initial implementation: monitor Hinkal's Shielded Pool contract events on supported chains
- If Hinkal provides a relayer API, use that for shielded transfer detection
- Fallback: direct contract event monitoring via RPC
- `available_for_chain(Solana)` returns `false` until Hinkal deploys on Solana
- `available_for_chain(Hedera)` returns `false` (Hinkal not announced for Hedera)
- This is NOT a failure — the `PrivacyPort` trait gracefully degrades to transparent-only

**Step 4.7 — Deposit Reference Scheme** (deposit_ref.rs)

```rust
/// Generate a one-time deposit reference for shielded deposits.
///
/// # Privacy property
/// The reference is derived via HKDF from the wallet seed + nonce + expiry.
/// It appears random on-chain but hKask can verify it belongs to a specific wallet.
///
/// # Anti-replay
/// References are burned on use (consumed in WalletStore).
/// References expire after `validity_duration` (default 24h).
pub fn generate_deposit_reference(
    wallet_seed: &[u8; 32],
    wallet_id: WalletId,
    chain: ChainId,
    validity_duration: Duration,
) -> Result<DepositReference, WalletError> {
    let nonce = generate_nonce();  // random 16 bytes
    let expiry = Utc::now() + validity_duration;
    let context = format!("hkask:deposit-ref:{}:{}:{}", wallet_id, chain, expiry.timestamp());
    let ref_bytes = hkdf_expand(wallet_seed, context.as_bytes())?;
    let reference = hex::encode(&ref_bytes[..16]);  // 32-char hex string
    Ok(DepositReference {
        reference,
        wallet_id,
        chain,
        nonce,
        expires_at: expiry,
    })
}

/// Verify a deposit reference belongs to a wallet.
pub fn verify_deposit_reference(
    wallet_seed: &[u8; 32],
    reference: &DepositReference,
) -> Result<bool, WalletError> {
    let context = format!("hkask:deposit-ref:{}:{}:{}",
        reference.wallet_id, reference.chain, reference.expires_at.timestamp());
    let ref_bytes = hkdf_expand(wallet_seed, context.as_bytes())?;
    let expected = hex::encode(&ref_bytes[..16]);
    Ok(expected == reference.reference)
}
```

**Step 4.8 — `WalletManager`** (manager.rs)

```rust
pub struct WalletManager {
    config: WalletConfig,
    store: Arc<WalletStore>,
    chains: HashMap<ChainId, Box<dyn ChainPort>>,
    privacy: Option<Box<dyn PrivacyPort>>,
    wallet_seed: Zeroizing<[u8; 32]>,
}

impl WalletManager {
    pub fn build(config: WalletConfig, store: Arc<WalletStore>) -> Result<Self, WalletError>;

    // ── Balance ──
    pub fn get_balance(&self, wallet_id: WalletId) -> Result<WalletBalance, WalletError>;

    // ── Deposit monitoring loop ──
    pub async fn start_deposit_monitor(&self) -> Result<(), WalletError>;
    async fn process_deposit(&self, event: DepositEvent) -> Result<(), WalletError>;
    async fn process_shielded_deposit(&self, transfer: ShieldedTransfer) -> Result<(), WalletError>;

    // ── Withdrawal ──
    pub async fn withdraw(
        &self,
        wallet_id: WalletId,
        amount_rj: RJoule,
        to_address: &str,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<TxHash, WalletError>;

    // ── Deposit address ──
    pub fn get_deposit_address(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        privacy: PrivacyMode,
    ) -> Result<DepositAddress, WalletError>;

    // ── Deposit reference ──
    pub fn generate_deposit_reference(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
    ) -> Result<DepositReference, WalletError>;

    // ── Gas ↔ rJoule conversion ──
    pub fn gas_to_rjoules(&self, gas: u64) -> RJoule;
    pub fn rjoules_to_gas(&self, rj: RJoule) -> u64;
    pub fn can_afford(&self, wallet_id: WalletId, cost_rj: RJoule) -> Result<bool, WalletError>;
    pub fn reserve_rjoules(&self, wallet_id: WalletId, amount: RJoule) -> Result<(), WalletError>;
    pub fn settle_rjoules(&self, wallet_id: WalletId, reserved: RJoule, actual: RJoule) -> Result<(), WalletError>;
}
```

**Step 4.9 — `ApiKeyIssuer`** (issuer.rs)

```rust
pub struct ApiKeyIssuer {
    wallet_seed: Zeroizing<[u8; 32]>,
    store: Arc<WalletStore>,
}

impl ApiKeyIssuer {
    /// "Print" a new API key — generate an Ed25519 keypair, create a signed capability token.
    pub fn create_key(
        &self,
        wallet_id: WalletId,
        spending_limit_rj: RJoule,
        expiry_days: Option<u32>,
        privacy_mode: PrivacyMode,
        preferred_chain: Option<ChainId>,
    ) -> Result<ApiKeyMaterial, WalletError>;

    /// Revoke an API key — marks it as revoked, returns unspent rJoules to wallet.
    pub fn revoke_key(&self, key_id: ApiKeyId) -> Result<(), WalletError>;

    /// List active API keys for a wallet.
    pub fn list_keys(&self, wallet_id: WalletId) -> Result<Vec<ApiKeyCapability>, WalletError>;
}

/// The material returned to the user when an API key is "printed."
pub struct ApiKeyMaterial {
    pub key_id: ApiKeyId,
    pub private_key_hex: String,     // Ed25519 private key — THIS IS THE API KEY
    pub capability: ApiKeyCapability, // public metadata about the key
}
```

**Verification:**
```bash
cargo check -p hkask-wallet
cargo test -p hkask-wallet
```

**Success criteria:**
- [ ] `MockChainPort` simulates deposits correctly
- [ ] `MockPrivacyPort` simulates shielded transfers with deposit references
- [ ] `WalletManager::process_deposit` credits rJoules on confirmed deposit
- [ ] `WalletManager::process_shielded_deposit` credits rJoules when deposit reference matches
- [ ] `WalletManager::withdraw` debits rJoules and calls ChainPort::submit_withdrawal
- [ ] `WalletManager::can_afford` returns false when balance insufficient
- [ ] `WalletManager::reserve_rjoules` + `settle_rjoules` correctly handle partial refunds
- [ ] `ApiKeyIssuer::create_key` produces valid Ed25519 keypair + signed capability
- [ ] `ApiKeyIssuer::revoke_key` returns unspent rJoules to wallet balance
- [ ] Deposit reference: generate → verify → consume → verify consumed (replay rejected)
- [ ] `gas_to_rjoules(1000)` returns `RJoule(1)` with default config
- [ ] `rjoules_to_gas(RJoule(1))` returns `1000` with default config

---

### Phase 5: CNS Integration (`hkask-cns`) — EST. 3–4 hours

**Goal:** `WalletBackedBudget` variant. `WalletEnergyEstimator`. CNS spans emitted. Algedonic alerts.

**Files:**
- **NEW:** `crates/hkask-cns/src/wallet_budget.rs` — `WalletBackedBudget`
- **MODIFIED:** `crates/hkask-cns/src/energy.rs` — add `WalletBacked` variant to `EnergyBudget` (or as separate type)
- **MODIFIED:** `crates/hkask-cns/src/energy_budget_management.rs` — add `WalletEnergyEstimator`
- **MODIFIED:** `crates/hkask-cns/src/cybernetics_loop.rs` — add wallet span monitoring + algedonic thresholds
- **MODIFIED:** `crates/hkask-cns/src/lib.rs` — re-exports

**Step 5.1 — `WalletBackedBudget`** (wallet_budget.rs)

```rust
/// An EnergyBudget backed by a wallet's rJoule balance.
///
/// # Difference from standard EnergyBudget
/// Standard budgets use dimensionless gas with periodic replenishment.
/// WalletBackedBudget converts gas to rJoules and debits a wallet.
/// Replenishment happens via on-chain deposits, not periodic cycles.
pub struct WalletBackedBudget {
    pub wallet_id: WalletId,
    pub key_id: Option<ApiKeyId>,        // None = wallet-level, Some = key-level
    pub spending_limit_rj: Option<RJoule>, // per-key limit, if applicable
    pub wallet_manager: Arc<WalletManager>,
    pub gas_per_rjoule: u64,
    pub hard_limit: bool,                // always true for wallet budgets
}
```

**Step 5.2 — `WalletEnergyEstimator`** (energy_budget_management.rs)

```rust
/// Estimates gas cost and converts to rJoules for wallet-backed budgets.
///
/// Wraps the existing CompositeEnergyEstimator and adds rJoule conversion.
pub struct WalletEnergyEstimator {
    inner: CompositeEnergyEstimator,
    gas_per_rjoule: u64,
}

impl EnergyEstimator for WalletEnergyEstimator {
    fn estimate_cost(&self, server: &str, tool: &str, params: &ToolParams) -> EnergyCost {
        let gas_cost = self.inner.estimate_cost(server, tool, params);
        // Conversion happens at reserve/settle time, not estimate time
        gas_cost
    }
}
```

**Step 5.3 — CNS spans in CyberneticsLoop** (cybernetics_loop.rs)

Add wallet-specific sense→compute→act logic:

```rust
// In CyberneticsLoop::sense():
// - Read cns.wallet.balance for each active wallet
// - Read cns.wallet.treasury per chain
// - Compute balance ratios

// In CyberneticsLoop::compute():
// - If wallet balance < 10% of 30-day average → WalletBalanceLow alert
// - If treasury < min_reserve → TreasuryLow alert (escalate to human)
// - If key_exhausted event detected → KeyExhausted alert

// In CyberneticsLoop::act():
// - Emit cns.wallet.balance spans
// - Emit cns.wallet.treasury spans
// - Route algedonic alerts to Curator
```

**Step 5.4 — Algedonic thresholds** (cybernetics_loop.rs)

| Alert | Threshold | Escalation |
|-------|-----------|------------|
| `WalletBalanceLow` | balance < 10% of 30-day moving average | → Curator |
| `WalletBalanceCritical` | balance = 0 | → Curator + Human |
| `TreasuryLow` | treasury < min_reserve (configurable, default 100 USDC) | → Human |
| `KeyExhausted` | key spent_rj >= spending_limit_rj | → Curator |
| `KeyExpired` | key expiry passed | → Curator (informational) |
| `ChainError` | RPC failure count > 3 in 5 minutes | → Curator |
| `PrivacyError` | Hinkal relayer failure count > 3 in 5 minutes | → Curator |

**Verification:**
```bash
cargo check -p hkask-cns
cargo test -p hkask-cns -- wallet
```

**Success criteria:**
- [ ] `WalletBackedBudget` correctly converts gas → rJoules using `gas_per_rjoule`
- [ ] `WalletBackedBudget` rejects operations when wallet balance insufficient
- [ ] `WalletBackedBudget` rejects operations when key spending limit exceeded
- [ ] `WalletEnergyEstimator` wraps existing estimator and adds conversion
- [ ] CNS emits `cns.wallet.balance` span on balance changes
- [ ] CNS emits `cns.wallet.key_exhausted` when key limit reached
- [ ] Algedonic alert fires when wallet balance drops below 10% threshold
- [ ] Algedonic alert fires when treasury drops below min_reserve

---

### Phase 6: Service Layer (`hkask-services`) — EST. 2–3 hours

**Goal:** `WalletService` composed into `AgentService`. Both CLI and API surfaces can access wallet functionality.

**Files:**
- **NEW:** `crates/hkask-services/src/wallet_service.rs` — `WalletService`
- **MODIFIED:** `crates/hkask-services/src/agent_service.rs` — add `wallet_service` field, wire in `build()`
- **MODIFIED:** `crates/hkask-services/src/lib.rs` — re-exports

**Step 6.1 — `WalletService`** (wallet_service.rs)

```rust
/// Shared wallet service for CLI and API surfaces.
///
/// # Design (refactor-service-layer pattern)
/// WalletService is a thin facade over WalletManager + ApiKeyIssuer.
/// It adds no business logic — it exists to provide a consistent interface
/// for both surfaces and to manage Arc<> sharing.
pub struct WalletService {
    manager: Arc<WalletManager>,
    issuer: Arc<ApiKeyIssuer>,
}

impl WalletService {
    pub fn new(manager: Arc<WalletManager>, issuer: Arc<ApiKeyIssuer>) -> Self;

    // ── Pass-through to WalletManager ──
    pub fn get_balance(&self, wallet_id: WalletId) -> Result<WalletBalance, WalletError>;
    pub fn get_deposit_address(&self, wallet_id: WalletId, chain: ChainId, privacy: PrivacyMode) -> Result<DepositAddress, WalletError>;
    pub fn generate_deposit_reference(&self, wallet_id: WalletId, chain: ChainId) -> Result<DepositReference, WalletError>;
    pub fn get_transactions(&self, wallet_id: WalletId, limit: u32, offset: u32) -> Result<Vec<WalletTransaction>, WalletError>;
    pub async fn withdraw(&self, wallet_id: WalletId, amount_rj: RJoule, to: &str, chain: ChainId, privacy: PrivacyMode) -> Result<TxHash, WalletError>;

    // ── Pass-through to ApiKeyIssuer ──
    pub fn create_api_key(&self, wallet_id: WalletId, limit_rj: RJoule, expiry_days: Option<u32>, privacy: PrivacyMode, chain: Option<ChainId>) -> Result<ApiKeyMaterial, WalletError>;
    pub fn list_api_keys(&self, wallet_id: WalletId) -> Result<Vec<ApiKeyCapability>, WalletError>;
    pub fn revoke_api_key(&self, key_id: ApiKeyId) -> Result<(), WalletError>;
}
```

**Step 6.2 — Wire into `AgentService`** (agent_service.rs)

```rust
// In AgentService::build():
let wallet_config = WalletConfig::default();  // or from settings.json
let wallet_store = Arc::new(WalletStore::new(conn.clone())?);
let wallet_manager = Arc::new(WalletManager::build(wallet_config, wallet_store.clone())?);
let api_key_issuer = Arc::new(ApiKeyIssuer::new(wallet_seed, wallet_store.clone())?);
let wallet_service = WalletService::new(wallet_manager.clone(), api_key_issuer.clone());

// Start deposit monitor in background
tokio::spawn(async move {
    wallet_manager.start_deposit_monitor().await;
});

AgentService {
    // ... existing fields ...
    wallet_service,
    wallet_manager,  // for CNS access
}
```

**Verification:**
```bash
cargo check -p hkask-services
cargo test -p hkask-services -- wallet
```

**Success criteria:**
- [ ] `WalletService` composes `WalletManager` + `ApiKeyIssuer`
- [ ] `AgentService::build()` wires wallet subsystem
- [ ] Deposit monitor starts as background tokio task
- [ ] All `WalletService` methods delegate correctly (integration test with mock ports)

---

### Phase 7: CLI (`hkask-cli`) — EST. 3–4 hours

**Goal:** `kask wallet` subcommand group. All wallet operations available from command line.

**Files:**
- **NEW:** `crates/hkask-cli/src/commands/wallet.rs` — wallet command handlers
- **MODIFIED:** `crates/hkask-cli/src/cli/actions.rs` — add `WalletAction` enum
- **MODIFIED:** `crates/hkask-cli/src/commands/mod.rs` — re-export wallet commands

**Step 7.1 — `WalletAction` enum** (actions.rs)

```rust
pub enum WalletAction {
    Balance,
    DepositAddress {
        chain: Option<ChainId>,
        private: bool,
    },
    DepositReference {
        chain: ChainId,
    },
    History {
        limit: Option<u32>,
    },
    KeyCreate {
        limit_rj: u64,
        expiry_days: Option<u32>,
        private: bool,
        chain: Option<ChainId>,
    },
    KeyList,
    KeyRevoke {
        key_id: String,
    },
    Withdraw {
        amount_rj: u64,
        to_address: String,
        chain: Option<ChainId>,
        private: bool,
    },
}
```

**Step 7.2 — Command handlers** (commands/wallet.rs)

```rust
pub async fn handle_wallet_balance(state: &ReplState) -> Result<(), CliError> {
    let wallet_id = state.get_or_create_wallet_id()?;
    let balance = state.wallet_service().get_balance(wallet_id)?;
    println!("Balance: {} rJ  (~{:.2} USDC, ~{} gas)",
        balance.rjoules,
        balance.usdc_equivalent,
        balance.gas_equivalent,
    );
    Ok(())
}

pub async fn handle_wallet_deposit_address(state: &ReplState, chain: Option<ChainId>, private: bool) -> Result<(), CliError> {
    let wallet_id = state.get_or_create_wallet_id()?;
    let chain = chain.unwrap_or(ChainId::Solana);  // default to Solana
    let privacy = if private { PrivacyMode::Shielded } else { PrivacyMode::Transparent };
    let addr = state.wallet_service().get_deposit_address(wallet_id, chain, privacy)?;
    println!("Deposit address ({chain:?}, {privacy:?}): {addr}");
    if private {
        println!("Use `kask wallet deposit-reference --chain {chain:?}` to generate a reference for shielded deposit.");
    }
    Ok(())
}

// ... similar handlers for each WalletAction variant ...
```

**Step 7.3 — CLI argument parsing**

```bash
kask wallet balance
kask wallet deposit-address [--chain solana|hedera] [--private]
kask wallet deposit-reference --chain solana|hedera
kask wallet history [--limit N]
kask wallet key create --limit <RJ> [--expiry DAYS] [--private] [--chain solana|hedera]
kask wallet key list
kask wallet key revoke <KEY_ID>
kask wallet withdraw <RJ> --to <ADDRESS> [--chain solana|hedera] [--private]
```

**Verification:**
```bash
cargo check -p hkask-cli
cargo test -p hkask-cli -- wallet
```

**Success criteria:**
- [ ] `kask wallet balance` displays rJoule balance with USDC + gas equivalents
- [ ] `kask wallet deposit-address` shows chain-specific deposit address
- [ ] `kask wallet deposit-address --private` shows shielded deposit instructions
- [ ] `kask wallet deposit-reference` generates valid one-time reference
- [ ] `kask wallet key create --limit 5000` prints Ed25519 API key
- [ ] `kask wallet key list` shows active keys with spending status
- [ ] `kask wallet key revoke <id>` revokes key and returns unspent rJoules
- [ ] `kask wallet withdraw 5000 --to <addr>` initiates withdrawal
- [ ] `kask wallet history` shows paginated transaction list

---

### Phase 8: API (`hkask-api`) — EST. 3–4 hours

**Goal:** Wallet REST endpoints. API key authentication middleware. Privacy-aware routing.

**Files:**
- **NEW:** `crates/hkask-api/src/routes/wallet.rs` — wallet endpoints
- **NEW:** `crates/hkask-api/src/middleware/api_key_auth.rs` — API key authentication
- **MODIFIED:** `crates/hkask-api/src/routes/mod.rs` — add wallet routes
- **MODIFIED:** `crates/hkask-api/src/lib.rs` — re-exports

**Step 8.1 — Wallet endpoints** (routes/wallet.rs)

| Endpoint | Method | Request Body | Response |
|----------|--------|-------------|----------|
| `/api/wallet/balance` | GET | — | `WalletBalanceResponse` |
| `/api/wallet/deposit-address` | GET | `?chain=solana&private=false` | `DepositAddressResponse` |
| `/api/wallet/deposit-reference` | POST | `{ chain: "solana" }` | `DepositReferenceResponse` |
| `/api/wallet/transactions` | GET | `?limit=50&offset=0` | `TransactionListResponse` |
| `/api/wallet/keys` | POST | `{ limit_rj: 5000, expiry_days: 30, private: false, chain: "solana" }` | `ApiKeyCreatedResponse` |
| `/api/wallet/keys` | GET | — | `ApiKeyListResponse` |
| `/api/wallet/keys/{key_id}` | DELETE | — | `ApiKeyRevokedResponse` |
| `/api/wallet/withdraw` | POST | `{ amount_rj: 5000, to_address: "...", chain: "solana", private: false }` | `WithdrawalResponse` |

**Step 8.2 — API key auth middleware** (middleware/api_key_auth.rs)

```rust
/// Middleware that authenticates requests using hKask-issued API keys.
///
/// # Flow
/// 1. Extract Bearer token from Authorization header
/// 2. Parse as Ed25519 private key (hex-encoded 32 bytes)
/// 3. Derive public key
/// 4. Look up ApiKeyCapability by public key in WalletStore
/// 5. Verify: not revoked, not expired, spending limit not exceeded
/// 6. Attach (wallet_id, key_id, spending_limit_rj) to request extensions
///
/// # OCAP alignment (P4)
/// The API key IS a capability token. The middleware verifies the capability
/// and extracts its attenuation (spending limit). Downstream handlers use
/// the attached wallet context for gas→rJoule billing.
pub struct ApiKeyAuthLayer {
    wallet_store: Arc<WalletStore>,
}

impl ApiKeyAuthLayer {
    pub fn new(wallet_store: Arc<WalletStore>) -> Self;

    async fn authenticate(&self, request: &mut Request) -> Result<WalletContext, AuthError> {
        let header = request.headers().get("Authorization")
            .ok_or(AuthError::MissingAuthorization)?;
        let token = header.to_str()?.strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidAuthorizationFormat)?;
        let private_key = hex::decode(token)?;
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&private_key);
        let public_key = signing_key.verifying_key();
        let capability = self.wallet_store.get_api_key_by_public_key(public_key.as_bytes())?
            .ok_or(AuthError::UnknownApiKey)?;

        if capability.revoked_at.is_some() {
            return Err(AuthError::KeyRevoked);
        }
        if let Some(expiry) = capability.expiry {
            if Utc::now() > expiry {
                return Err(AuthError::KeyExpired);
            }
        }
        if capability.spent_rj >= capability.spending_limit_rj {
            return Err(AuthError::SpendingLimitExceeded);
        }

        Ok(WalletContext {
            wallet_id: capability.wallet_id,
            key_id: capability.key_id,
            spending_limit_rj: capability.spending_limit_rj,
            spent_rj: capability.spent_rj,
            privacy_mode: capability.privacy_mode,
        })
    }
}
```

**Step 8.3 — Privacy-aware routing** (routes/wallet.rs)

When a request is authenticated with a shielded API key (`privacy_mode: Shielded`), the response omits on-chain addresses and transaction hashes from public endpoints. Internal CNS spans still record full data.

**Verification:**
```bash
cargo check -p hkask-api
cargo test -p hkask-api -- wallet
```

**Success criteria:**
- [ ] `GET /api/wallet/balance` returns correct rJoule balance
- [ ] `POST /api/wallet/keys` creates and returns Ed25519 API key
- [ ] `DELETE /api/wallet/keys/{id}` revokes key
- [ ] API key auth middleware rejects expired keys
- [ ] API key auth middleware rejects revoked keys
- [ ] API key auth middleware rejects keys over spending limit
- [ ] API key auth middleware attaches `WalletContext` to request extensions
- [ ] Shielded API keys get privacy-aware responses (no on-chain addresses in output)

---

## 6. Dependency Graph & Parallelization

```
Phase 1 (types) ─────────────────────────────────────────────┐
                                                              │
Phase 2 (storage) ──┐                                         │
                     ├── Phase 4 (wallet crate) ──┬── Phase 5 (CNS)
Phase 3 (keystore) ─┘                            │
                                                 ├── Phase 6 (services)
                                                 │
                                                 └── Phase 7 (CLI) + Phase 8 (API) [parallel]
```

- **Phases 2 + 3 can run in parallel** after Phase 1 completes (storage and keystore are independent)
- **Phase 4 depends on Phases 1, 2, 3** (wallet crate uses types, storage, keystore)
- **Phases 5 + 6 can run in parallel** after Phase 4 (CNS and services are independent consumers of wallet)
- **Phases 7 + 8 can run in parallel** after Phase 6 (CLI and API are independent surfaces)

---

## 7. Key Decisions to Preserve

1. **rJoule is an internal accounting unit, not an on-chain token.** `[OUGHT-DECL]` — rJoule exists only in hKask's SQLite database. It is not tradeable, transferable, or redeemable outside hKask. It is a stable value unit pegged to USDC at a configurable rate. Do not create an ERC-20, SPL, or HTS token for rJoule.

2. **API keys are Ed25519 keypairs, not opaque bearer strings.** `[OUGHT-DECL]` — The private key IS the API key. The public key IS the key's identity. This provides proof-of-possession authentication and aligns with hKask's existing Ed25519 infrastructure (spec_signer). Do not use random hex strings or JWTs.

3. **Privacy is opt-in per deposit and per API key.** `[OUGHT-DECL]` — The user chooses `Transparent` or `Shielded` at deposit time and at key creation time. A single wallet can have both transparent and shielded API keys. This preserves P1 User Sovereignty — the user controls their privacy level.

4. **Deposit references are one-time and time-bounded.** `[OUGHT-DECL]` — Each reference is burned on first use and expires after 24 hours. This prevents replay attacks on shielded deposits. The reference scheme uses HKDF derivation from the wallet seed — trustless, deterministic, verifiable.

5. **Treasury keys are derived per chain from the master passphrase.** `[OUGHT-DECL]` — `HKDF-SHA256(master_key, "hkask:treasury-solana")` and `HKDF-SHA256(master_key, "hkask:treasury-hedera")`. Same passphrase → same treasury keys. No separate key material to manage. This follows the existing keystore pattern (ADR-027).

6. **Gas↔rJoule conversion is configurable but defaults to 1000:1.** `[OUGHT-DECL]` — 1 rJoule = 1000 gas units. This means a typical tool call (25 gas for web search) costs 0.025 rJoules (≈$0.000025 at 1 rJ = 0.001 USDC). The conversion rate is in `WalletConfig` and can be adjusted without changing gas cost tables.

7. **The wallet subsystem is a domain crate, not a service.** `[OUGHT-DECL]` — `hkask-wallet` depends on `hkask-types`, `hkask-keystore`, `hkask-storage`. It does NOT depend on `hkask-services`, `hkask-cns`, `hkask-cli`, or `hkask-api`. Dependency direction: surface → service → domain. The wallet is domain.

8. **ChainPort and PrivacyPort are traits, not abstract base classes.** `[OUGHT-DECL]` — Each chain and privacy implementation is a standalone struct. The traits define capability (what a chain/privacy layer can do), not taxonomy (what it is). Adding a new chain means implementing `ChainPort` — no inheritance hierarchy to navigate.

---

## 8. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Hinkal not deployed on Solana by Phase 4 | `[IS-PROB]` — MEDIUM (roadmap says "planned," no date) | LOW — `PrivacyPort` degrades gracefully; transparent path works | `available_for_chain()` returns false; shielded operations return `PrivacyUnavailable` error with clear message |
| Solana RPC provider rate-limiting or downtime | `[IS-PROB]` — LOW (Helius free tier: 25 req/s; Triton: generous free tier) | MEDIUM — missed deposits delay rJoule crediting | Multiple RPC endpoints with fallback; deposit polling is low-frequency (every 30s) |
| Hedera SDK for Rust immature or unstable | `[IS-PROB]` — MEDIUM (community SDK, not official) | LOW — mirror node REST API is stable and well-documented | Use mirror node REST API as primary interface; SDK is optional |
| USDC depeg event (rJoule value drifts from $0.001) | `[IS-SUBJ]` — LOW (USDC has maintained peg since 2018) | HIGH — rJoule balances would be worth less than intended | CNS `cns.wallet.balance` anomaly detection; `rj_per_usdc` is configurable and can be adjusted |
| User loses API key private key (no recovery) | `[IS-PROB]` — MEDIUM (users lose keys) | MEDIUM — unspent rJoules locked in key | Key revocation returns unspent rJoules to wallet; user can issue new key. Private key is user's responsibility (per P1 sovereignty). |
| Treasury key compromise | `[IS-SUBJ]` — LOW (derived from master passphrase, never exposed) | CRITICAL — all treasury funds at risk | Master passphrase is the root of trust (existing keystore architecture). Treasury holds only operational USDC, not user deposits (users can withdraw at any time per P1). |

---

## 9. Magna Carta Compliance Summary

| Principle | Implementation | Verification |
|-----------|---------------|-------------|
| **P1 — User Sovereignty** | Wallet keys derived from user's master passphrase. Full withdrawal at any time. User chooses chain + privacy per deposit. | `kask wallet withdraw` always available. `kask sovereignty verify` checks wallet withdrawal path. |
| **P2 — Affirmative Consent** | Every deposit requires explicit user action. API key creation authorized by wallet owner. Privacy mode is opt-in. | No automatic deposits. No keys created without user command. |
| **P3 — Generative Space** | All wallet state exposed equally via CLI (`kask wallet`), API (`/api/wallet/*`), and REPL (`/wallet`). Same `WalletService` backing all three. | Settings, balance, keys, transactions all visible from all three surfaces. |
| **P4 — Clear Boundaries (OCAP)** | API keys are Ed25519-signed capability tokens with embedded attenuation (spending limits, expiry, privacy mode). Treasury operations require separate treasury-level OCAP. | `ApiKeyAuthLayer` verifies capability before every request. Treasury key separate from wallet key. |

---

## 10. Verification Commands

```bash
# Per-phase verification
cargo check -p hkask-types                                    # Phase 1
cargo test -p hkask-storage -- wallet_store                   # Phase 2
cargo test -p hkask-keystore -- treasury                      # Phase 3
cargo test -p hkask-wallet                                    # Phase 4
cargo test -p hkask-cns -- wallet                             # Phase 5
cargo test -p hkask-services -- wallet                        # Phase 6
cargo test -p hkask-cli -- wallet                             # Phase 7
cargo test -p hkask-api -- wallet                             # Phase 8

# Full workspace verification (after all phases)
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings

# Magna Carta compliance
kask sovereignty verify

# Constraint verification (headless, no stubs)
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/hkask-wallet/ && echo "VIOLATION" || echo "CLEAN"
```

---

## 11. Recommended Skills for Implementation

| Skill | Phase | Why |
|-------|-------|-----|
| **coding-guidelines** | All phases | Enforce simplicity, surgical changes, goal-driven execution before every edit |
| **rust-expertise** | Phases 1–4 | Type-driven design, ownership architecture, error design for new types and wallet crate |
| **tdd** | Phases 2–8 | RED→GREEN→REFACTOR with `// REQ:` tags from this plan's success criteria |
| **pragmatic-semantics** | All phases | Classify statements, trace provenance, maintain IS/OUGHT distinction in code comments |
| **pragmatic-cybernetics** | Phase 5 | Feedback loop analysis, variety engineering, Good Regulator check for CNS integration |
| **strangler-fig** | All phases | Each phase is independently deployable; system functional at every step |
| **deep-module** | Phase 4 | Ensure `hkask-wallet` has deep modules (small interfaces, large implementations) |
| **magna-carta-verifier** | Phase 8 | Verify P1–P4 compliance of wallet endpoints and API key auth |

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0 — Plan 2026-06-12*
