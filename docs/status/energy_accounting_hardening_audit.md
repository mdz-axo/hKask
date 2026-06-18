---
title: "Energy Accounting Hardening Audit"
audience: [architects, auditors]
last_updated: 2026-06-18
version: "0.28.0"
status: "Active"
mds_categories: [trust, lifecycle]
---

# Energy Accounting Hardening Audit

**Purpose:** Audit the energy accounting call chain for tamper-evidence, layer duplication, and determinism vulnerabilities. Maps the 7 core semantic operations through every layer they appear in.

**Related:** [`PRINCIPLES.md §P1-11`](../architecture/core/PRINCIPLES.md#p11--trust-foundation), [`EnergyBudget`](../../crates/hkask-cns/src/energy.rs), [`WalletBackedBudget`](../../crates/hkask-cns/src/wallet_budget.rs), [`EnergyBudgetManager`](../../crates/hkask-cns/src/energy_budget_management.rs), [`CyberneticsLoop`](../../crates/hkask-cns/src/cybernetics_loop.rs), [`WalletManager`](../../crates/hkask-wallet/src/manager/), [`WalletStore`](../../crates/hkask-storage/src/wallet_store.rs), [`WalletGasCalibrator`](../../crates/hkask-cns/src/wallet_gas_calibrator.rs)

---

## 1. Current Architecture

Seven semantic operations define the energy accounting contract:

| # | Operation | Purpose |
|---|-----------|---------|
| 1 | `can_proceed` | Check whether an agent has sufficient budget for a proposed operation |
| 2 | `reserve` | Hold gas/rJoules for an in-flight operation (hold-settle pattern) |
| 3 | `settle` | Deduct actual cost after operation completes; refund difference |
| 4 | `consume` | Deduct cost immediately (non-reserved path, known-cost operations) |
| 5 | `encumber` | Allocate rJoules to an API key from a wallet |
| 6 | `release_encumbrance` | Return unspent rJoules from an API key back to the wallet |
| 7 | `can_afford` | Check raw wallet balance (no encumbrance membrane) |

These 7 operations are duplicated across 5 layers with 23 discrete public accounting surfaces:

```
GovernedTool (membrane)
    │
    ▼
CyberneticsLoop (pass-through)
    │  can_proceed(), reserve_gas(), settle_gas()
    ▼
EnergyBudgetManager (dispatch)
    │  can_proceed(), reserve_gas(), settle_gas()
    │
    ├──► EnergyBudget (in-memory)
    │      can_proceed(), reserve(), settle(), consume()
    │
    └──► WalletBackedBudget (wallet-anchored)
           can_proceed(), reserve(), settle()
             │
             ▼
           WalletManager (orchestration)
             can_afford(), reserve_rjoules(), settle_rjoules()
             encumber(), release_encumbrance(), consume()
               │
               ▼
             WalletStore (SQLite persistence)
               consume_deposit_reference(), encumber_rjoules()
               release_encumbrance(), consume_encumbrance()
```

---

## 2. Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│  GovernedTool (crates/hkask-cns/src/governed_tool.rs)            │
│  │  Step 2: loop6.can_proceed(agent, estimated_cost)             │
│  │  Step 2: loop6.reserve_gas(agent, estimated_cost)             │
│  │  Step 5: loop6.settle_gas(agent, estimated, actual)           │
│  ▼                                                               │
├─────────────────────────────────────────────────────────────────┤
│  CyberneticsLoop (crates/hkask-cns/src/cybernetics_loop.rs)      │  ◄── PASS-THROUGH
│  │  cybernetics_loop.rs:156: self.energy_budget_manager.can_proceed()
│  │  cybernetics_loop.rs:166: self.energy_budget_manager.reserve_gas()
│  │  cybernetics_loop.rs:175: self.energy_budget_manager.settle_gas()
│  ▼                                                               │
├─────────────────────────────────────────────────────────────────┤
│  EnergyBudgetManager (crates/hkask-cns/src/energy_budget_management.rs)
│  │  Checks wallet budgets first, falls back to gas budgets       │
│  │  wallet_budget.can_proceed(gas) → EnergyBudget.can_proceed()   │
│  │  wallet_budget.reserve(gas)    → EnergyBudget.reserve()        │
│  │  wallet_budget.settle(r, a)    → EnergyBudget.settle()         │
│  ▼                                                               │
├──────────────────────┬──────────────────────────────────────────┤
│  EnergyBudget        │  WalletBackedBudget                       │
│  (in-memory)         │  (wallet-anchored)                        │
│  energy.rs:323       │  wallet_budget.rs:95                      │
│  • can_proceed()     │  • can_proceed(gas) → gas_to_rjoules(gas) │
│  • reserve()   :356  │  • reserve(gas)     :151                  │
│  • settle()    :391  │  • settle(r,a)      :168                  │
│  • consume()   :427  │      │                                    │
│  NO store backing    │      ▼                                    │
│                      │  WalletManager                            │
│                      │  (manager/budget.rs, manager/encumbrance.rs)
│                      │  • can_afford(wallet, cost_rj)    :59     │
│                      │  • reserve_rjoules(wallet, amt)   :64     │
│                      │  • settle_rjoules(wallet, r, a)   :75     │
│                      │  • encumber(wallet, key, amt)     :6      │
│                      │  • release_encumbrance(key)       :26     │
│                      │  • consume(key, gas_rj)           :39     │
│                      │      │                                    │
│                      │      ▼                                    │
│                      │  WalletStore (SQLite, WAL mode)           │
│                      │  • consume_deposit_reference(ref):669     │
│                      │  • encumber_rjoules(w, k, amt)   :721     │
│                      │  • release_encumbrance(key)      :778     │
│                      │  • consume_encumbrance(key, cost):826     │
│                      │  Atomic CAS guards (no double-spend)      │
└──────────────────────┴──────────────────────────────────────────┘
```

---

## 3. Duplicate Analysis

Each operation appears in multiple layers. The table maps every implementation to its layer and file location:

| Operation | Layer | File | Line | Tamper-Evident? |
|-----------|-------|------|------|-----------------|
| `can_proceed` | `EnergyBudget` | `energy.rs` | 323 | ❌ In-memory, no store |
| `can_proceed` | `WalletBackedBudget` | `wallet_budget.rs` | 95 | ✅ Checks encumbrance/WalletManager |
| `can_proceed` | `EnergyBudgetManager` | `energy_budget_management.rs` | 90 | ✅ Dispatches to energy or wallet budget |
| `can_proceed` | `CyberneticsLoop` | `cybernetics_loop.rs` | 156 | ⚠️ Pass-through to EnergyBudgetManager |
| `reserve` | `EnergyBudget` | `energy.rs` | 356 | ❌ In-memory, no store |
| `reserve` | `WalletBackedBudget` | `wallet_budget.rs` | 151 | ❌ Optimistic (no debit) |
| `reserve_gas` | `EnergyBudgetManager` | `energy_budget_management.rs` | 114 | ✅ Dispatches to energy or wallet budget |
| `reserve_gas` | `CyberneticsLoop` | `cybernetics_loop.rs` | 166 | ⚠️ Pass-through to EnergyBudgetManager |
| `reserve_rjoules` | `WalletManager` | `manager/budget.rs` | 64 | ❌ Checks can_afford only, no actual debit |
| `settle` | `EnergyBudget` | `energy.rs` | 391 | ❌ In-memory, no store |
| `settle` | `WalletBackedBudget` | `wallet_budget.rs` | 168 | ✅ Consumes encumbrance or debits wallet |
| `settle_gas` | `EnergyBudgetManager` | `energy_budget_management.rs` | 136 | ✅ Dispatches to energy or wallet budget |
| `settle_gas` | `CyberneticsLoop` | `cybernetics_loop.rs` | 175 | ⚠️ Pass-through to EnergyBudgetManager |
| `settle_rjoules` | `WalletManager` | `manager/budget.rs` | 75 | ✅ Delegates to WalletStore.debit_rjoules |
| `consume` | `EnergyBudget` | `energy.rs` | 427 | ❌ In-memory, no store |
| `consume` | `WalletManager` | `manager/encumbrance.rs` | 39 | ✅ Delegates to WalletStore.consume_encumbrance |
| `consume_encumbrance` | `WalletStore` | `wallet_store.rs` | 826 | ✅ Atomic SQL UPDATE with check constraint |
| `consume_deposit_reference` | `WalletStore` | `wallet_store.rs` | 669 | ✅ Atomic CAS (spent=0 → spent=1) |
| `encumber` | `WalletManager` | `manager/encumbrance.rs` | 6 | ✅ Delegates to WalletStore + emits CNS span |
| `encumber_rjoules` | `WalletStore` | `wallet_store.rs` | 721 | ✅ Atomic balance decrease + insert |
| `release_encumbrance` | `WalletManager` | `manager/encumbrance.rs` | 26 | ✅ Status guard (active only) + emits CNS span |
| `release_encumbrance` | `WalletStore` | `wallet_store.rs` | 778 | ✅ Idempotent: status guard (active only) |
| `can_afford` | `WalletManager` | `manager/budget.rs` | 59 | ✅ Reads WalletStore balance |

`CyberneticsLoop` entries marked ⚠️ are structural pass-throughs that add no behavior. The `GovernedTool` calls `CyberneticsLoop::can_proceed()` which immediately delegates to `EnergyBudgetManager::can_proceed()`. These 6 surface methods (3 operations × 2 pass-through methods each: `can_proceed`, `reserve_gas`, `settle_gas`) are candidates for removal.

---

## 4. Tamper-Evidence Analysis

### 4.1 WalletBackedBudget → WalletManager → WalletStore Chain

Each layer in the wallet-backed path independently verifies:

| Layer | Verification |
|-------|-------------|
| `WalletBackedBudget::can_proceed()` | Calls `WalletManager::get_encumbrance()` and checks `enc.remaining_rj() >= cost_rj`. Verifies spending limit against `check_key_health()`. |
| `WalletManager::consume()` | Calls `WalletStore::consume_encumbrance()`. No independent pre-check — relies on store atomicity. |
| `WalletStore::consume_encumbrance()` | Atomic SQL: `UPDATE encumbrances SET consumed_rj = consumed_rj + ?1 WHERE key_id = ?2 AND status = 'active' AND (amount_rj - consumed_rj) >= ?1`. If rows_affected==0, returns `InsufficientEncumbrance`. Full consumption transitions status to `consumed`. |

**Verdict:** The WalletStore is the single tamper-evident anchor. `WalletManager` and `WalletBackedBudget` add CNS span emission and gas↔rJoule conversion, respectively, but their balance checks are advisory — only the `WalletStore` SQL constraint provides atomic enforcement.

**Gap:** If `WalletManager::consume()` is bypassed and `WalletStore::consume_encumbrance()` is called directly, no CNS span is emitted. This creates an observability blind spot.

### 4.2 EnergyBudget (in-memory)

`EnergyBudget` operates entirely in memory with no persistent store backing. Its state is held in two `EnergyCost` fields:
- `remaining: EnergyCost` — current balance
- `reserved: EnergyCost` — hold-settle reservation

There is no tamper-evidence mechanism for in-memory budgets. A process restart, memory corruption, or out-of-band mutation would not be detected. The budget is reset on restart.

**Verdict:** ❌ Not tamper-evident. Intended for non-wallet-backed agents (soft limit). Acceptable risk under current architecture where memory budgets are used for free-tier/development agents.

---

## 5. Determinism Concerns

### 5.1 EMA-Based Gas Calibration

The `WalletGasCalibrator` (`wallet_gas_calibrator.rs:103`) uses an Exponential Moving Average to compute the `gas_per_rjoule` rate from aggregate settled events. This rate is written to `WalletManager` via `set_gas_per_rjoule()` (`manager/budget.rs:25`), which stores it as an `AtomicU64`.

The EMA introduces non-determinism:
- **Time-of-day variance**: Calibration at different times yields different rates depending on which settled events are in the window.
- **Race condition**: If calibration runs between `can_proceed()` and `settle()`, the rate used for the reservation check may differ from the rate used for the actual debit.
- **Warmup period**: Initial calibration uses `DEFAULT_WALLET_INITIAL_LOOKBACK` (wallet_gas_calibrator.rs:30) but may yield different rates depending on whether historical events exist.

### 5.2 Float Math in Gas→rJoule Conversion

The `WalletManager::gas_to_rjoules()` function (`manager/budget.rs:7-14`):

```rust
pub fn gas_to_rjoules(&self, gas: u64) -> RJoule {
    let rate = self.gas_per_rjoule.load(Ordering::Relaxed);
    let rj = gas / rate;
    RJoule::new(if rj == 0 { 1 } else { rj })
}
```

Integer division here introduces floor semantics: `gas / rate` truncates toward zero. A zero-cost protection clamps to 1 rJoule. This means:
- Small gas values (< rate) all cost 1 rJoule (loss of precision)
- The reverse conversion (`rjoules_to_gas`, line 17: `rj.as_u64() * rate`) may not round-trip

The `Ordering::Relaxed` atomic load means no happens-before relationship with `set_gas_per_rjoule()` — the conversion rate can change between reservation and settlement.

### 5.3 EnergyBudget Float Threshold

`EnergyBudget::with_alert_threshold()` (`energy.rs:295`) accepts an `f64` threshold. Float comparison for alert thresholds introduces platform-dependent behavior (x86 vs ARM may differ at the LSB).

---

## 6. Recommendations

### (a) Remove CyberneticsLoop Pass-Through Layer

**Severity:** Medium | **Effort:** Low

`CyberneticsLoop`'s energy accounting methods are pure pass-throughs to `EnergyBudgetManager`:

| CyberneticsLoop Method | Line | Delegates To |
|------------------------|------|-------------|
| `can_proceed()` | 156 | `self.energy_budget_manager.can_proceed(agent, gas).await` |
| `reserve_gas()` | 166 | `self.energy_budget_manager.reserve_gas(agent, gas).await` |
| `settle_gas()` | 175 | `self.energy_budget_manager.settle_gas(agent, reserved, actual).await` |
| `register_energy_budget()` | 142 | `self.energy_budget_manager.register_energy_budget(agent, budget).await` |
| `register_wallet_budget()` | 150 | `self.energy_budget_manager.register_wallet_budget(agent, budget).await` |
| `acquire_budget()` | 187 | `self.energy_budget_manager.acquire_budget(agent).await` |

Callers (`GovernedTool`, `GovernedInference`, `WalletService`) should take a direct reference to `EnergyBudgetManager` (or hold an `Arc<EnergyBudgetManager>`) instead of going through `CyberneticsLoop`. This removes 6 public surface methods and eliminates one delegation hop per energy operation.

The refactoring follows the strangler fig pattern: add `EnergyBudgetManager` accessor to `CyberneticsLoop`, migrate callers one at a time, then delete the pass-through methods.

### (b) Add Tamper-Evidence Checks at EnergyBudget Level

**Severity:** High | **Effort:** Medium

`EnergyBudget` has no store backing. Add optional CNS span verification:

1. After every `settle()` or `consume()`, emit a `cns.energy.budget_state` span with `{remaining, reserved, cap}`
2. On `can_proceed()`, optionally verify against the last known CNS span (cross-reference check)
3. For hard-limit budgets, persist a checksum of `(cap, remaining, reserved)` in the CNS event store

This does not make `EnergyBudget` fully tamper-evident (it remains in-memory), but it provides an audit trail for post-hoc verification.

### (c) Audit Float Determinism in Gas Calibration Path

**Severity:** High | **Effort:** Medium-High

Audit and fix non-determinism sources:

1. **EMA calibration:** Add a `last_calibrated_at` guard that prevents calibration from running during active reservations. Consider using a fixed calibration schedule instead of side-effect-triggered recalibration.
2. **Gas↔rJoule conversion:** Use saturating fixed-point arithmetic with a minimum precision of 1e-9 rJoules. Replace `Ordering::Relaxed` with `Ordering::Acquire/Release` for the `gas_per_rjoule` atomic to establish happens-before between rate updates and reads.
3. **Alert threshold:** Replace `f64` with `Rational` (numerator/denominator `u64` pair) for alert threshold comparisons to ensure deterministic cross-platform behavior.
4. **Reproducibility test:** Add a test that runs the same workload twice with identical seed conditions and verifies identical energy accounting outcomes.
