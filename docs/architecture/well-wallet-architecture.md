---
title: "Well & Wallet Architecture — Gas/rJoule Supply Chain"
audience: [architects, CNS developers, curator]
last_updated: 2026-07-01
version: "0.32.0-draft"
status: "Draft"
domain: "Trust"
mds_categories: [trust, domain, lifecycle]
---

# Well & Wallet Architecture

**Adversarial review complete.** 6 skills applied. 5 items deleted. 3 guardrails closed. 1 prohibition resolved.

## 1. Conceptual Model

```
┌──────────────┐     auto-draw        ┌──────────────┐     spend     ┌──────────┐
│              │ ◄────────────────── │              │ ────────────► │          │
│    WELL      │                     │   WALLET     │               │  AGENT   │
│  (source)    │                     │  (storage)   │               │ (spender)│
│              │                     │              │               │          │
└──────┬───────┘                     └──────────────┘               └──────────┘
       │ admin configures                                              │
       ▼                                                              │
┌──────────────┐                                                observes
│   CURATOR    │ ◄──────────────────────────────────────────────┘
│  (daemon)    │
│              │── reports to ──► ┌──────────────┐
└──────────────┘                  │    HUMAN     │
                                  │ ADMINISTRATOR│
                                  │  (S5* audit) │
                                  └──────────────┘
```

- **Well**: Admin-configured gas/rJoule source. Auto-replenishes on schedule. One default Well per installation. [OUGHT: future — crypto → gas/rJoule conversion; then Hedera HTS token]
- **Wallet**: Per-agent balance store. Created by Curator on replicant registration. Agent's own property.
- **Auto-draw**: When `WalletBackedBudget` is low, automatically draws from the default Well. Closes the cybernetic feedback loop.
- **Curator**: Creates wallets, monitors balances, reports to admin. Has unlimited gas with admin-configured efficiency limits.
- **Human Administrator**: Configures Wells, sets Curator policy, is the S5* observer-of-observer.

## 2. Well

### 2.1 Definition

A Well produces gas and rJoule on schedule. One default Well per installation. Wells are the sole source of new gas/rJoule entering the system.

**IS vs OUGHT note**: Currently `register_gas_budget(GasBudget::new(...))` creates gas from nothing. After Well implementation, `register_gas_budget` is gated behind Well authorization — budgets come from wallet draws, which come from Wells. Until Wells exist, the old path remains.

### 2.2 WellConfig

```rust
struct WellConfig {
    well_id: String,
    /// Gas produced per replenishment cycle
    gas_rate: GasCost,
    /// rJoule produced per replenishment cycle
    rjoule_rate: u64,
}
```

### 2.3 WellManager

```rust
impl WellManager {
    /// Admin creates a Well. CNS span: cns.well.created
    async fn create_well(config: WellConfig) -> Result<WellID>;

    /// Replenish the Well (called on schedule by CyberneticsLoop).
    /// CNS span: cns.well.replenished
    async fn replenish(&self);

    /// Agent draws from the Well. Returns amount drawn.
    /// CNS span: cns.well.draw
    async fn draw(&self, agent: WebID, amount_gas: GasCost, amount_rjoule: u64)
        -> Result<(GasCost, u64)>;

    /// Check if Well is exhausted. CNS span: cns.well.exhausted when true.
    async fn is_exhausted(&self) -> bool;
}
```

### 2.4 Well Exhaustion — Regulatory Path

When `is_exhausted()` returns true during a replenishment cycle:
1. CNS span: `cns.well.exhausted` fires
2. CyberneticsLoop sends `CurationInput::Alert` via algedonic pathway
3. Curator notifies admin: "Well X is exhausted — agents may be blocked"
4. Admin increases `gas_rate` or `rjoule_rate` via `kask well update`

## 3. Wallet

### 3.1 Schema

```sql
CREATE TABLE agent_wallets (
    wallet_id TEXT PRIMARY KEY,
    agent_webid TEXT NOT NULL UNIQUE,
    gas_balance INTEGER NOT NULL DEFAULT 0,
    rjoule_balance INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    last_draw_at TEXT
);
```

### 3.2 WalletManager Trait

```rust
#[async_trait]
trait WalletManager {
    /// Create wallet for new replicant. Called by Curator daemon.
    /// CNS span: cns.wallet.created
    #[must_use]
    async fn create_wallet(&self, agent: WebID, initial_gas: GasCost, initial_rjoule: u64)
        -> Result<WalletID, WalletError>;

    /// Draw from Well into agent's wallet. Called by auto-draw or manual CLI.
    /// CNS span: cns.wallet.draw
    #[must_use]
    async fn draw_from_well(&self, agent: WebID, amount_gas: GasCost)
        -> Result<GasCost, WalletError>;

    /// Spend from wallet. Called by WalletBackedBudget on reserve/settle.
    #[must_use]
    async fn spend(&self, agent: WebID, amount: GasCost)
        -> Result<GasCost, WalletError>;

    /// Query wallet balance.
    async fn balance(&self, agent: WebID) -> Result<WalletBalance, WalletError>;
}
```

### 3.3 Auto-Draw — Closing the Feedback Loop

When `WalletBackedBudget.can_proceed()` returns false:
1. `GasBudgetManager` detects the block
2. Auto-draws `replenish_rate` amount from the default Well
3. If Well is exhausted, emits algedonic alert (see §2.4)
4. If draw succeeds, agent resumes operations

This replaces the current silent block with a closed cybernetic loop: exhaustion → draw → Well alert → admin action.

### 3.4 Wallet Creation Flow

```
1. AgentService registers new replicant
2. CNS span fires: cns.replicant.registered  [NEW — must be added to CNS registry]
3. Curator daemon receives span
4. Curator calls: WalletManager::create_wallet(agent, initial_gas, initial_rjoule)
5. GasBudgetManager registers WalletBackedBudget for the agent
6. CNS span: cns.wallet.created
```

## 4. Curator Budget Policy (G11 Resolution)

The Curator has **unlimited gas** (`hard_limit = false`). Limits are efficiency-based, not budget-based:

```rust
struct CuratorBudgetPolicy {
    /// Max LLM tokens per regulation cycle
    max_tokens_per_cycle: u64,
    /// Max tool invocations per cycle
    max_tool_calls_per_cycle: u32,
    /// When true, skip remaining tool calls this cycle when exceeded.
    /// When false, log CNS span and continue.
    throttle_on_exceeded: bool,
}
```

When limits are exceeded, CNS span `cns.curator.efficiency.exceeded` fires. The Human Administrator reviews efficiency reports and adjusts limits. The admin is the S5* observer — the logical backstop.

## 5. CNS Span Registry Additions

```rust
// Well
WellCreated,         // cns.well.created
WellReplenished,     // cns.well.replenished
WellDraw,            // cns.well.draw
WellExhausted,       // cns.well.exhausted → algedonic alert

// Wallet
WalletCreated,       // cns.wallet.created
WalletDraw,          // cns.wallet.draw
WalletSpend,         // cns.wallet.spend
WalletExhausted,     // cns.wallet.exhausted → algedonic alert

// Replicant lifecycle (must exist for wallet creation flow)
ReplicantRegistered, // cns.replicant.registered → triggers wallet creation

// Curator efficiency
CuratorEfficiencyExceeded,  // cns.curator.efficiency.exceeded
```

## 6. Implementation — ✅ Complete

All 12 steps implemented. See `docs/status/gas-budget-system-status.md` for full status.

### Phase 2 (deferred): Authorization + Transfer + Hedera

- Well authorization per-agent
- Wallet-to-wallet transfer
- Hedera HTS token: rJoule on-chain

## 7. Verification

```bash
cargo build --workspace                    # 0 errors
cargo test -p hkask-cns                    # CNS tests pass
cargo test -p hkask-storage -- wallet      # Wallet persistence tests
cargo test -p hkask-agents -- curator      # Curator wallet creation test
```

---

*Review applied: pragmatic-semantics, pragmatic-cybernetics, idiomatic-rust, essentialist, grill-me, coding-guidelines.*
