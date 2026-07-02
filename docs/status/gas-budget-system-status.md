---
title: "Gas Budget System — Implementation Status & Gap Analysis"
audience: [architects, CNS developers, curator]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Trust"
mds_categories: [trust, lifecycle, domain]
---

# Gas Budget System — Status & Gaps

Post-rename and post-implementation audit of the gas budget system. Names reflect the `Energy*` → `Gas*` rename (2026-07-01). Prior plan doc was `energy-accounting-requirements-assessment.md` — superseded by this document.

## 1. What Was Built (v0.31.0)

### 1.1 Rename: `Energy*` → `Gas*`

Types renamed to disambiguate dimensionless gas budgets from rJoule-denominated wallet budgets:

| Was | Now | Rationale |
|-----|-----|-----------|
| `EnergyBudget` | `GasBudget` | Dimensionless gas units — not real energy |
| `EnergyCost` | `GasCost` | Same |
| `EnergyBudgetManager` | `GasBudgetManager` | Manages gas budgets specifically |
| `EnergyDelta` | `GasDelta` | Rate-of-change tracking |
| `EnergyError` | `GasError` | Error type |
| `AgentEnergyStatus` | `AgentGasStatus` | Read-only budget snapshot |

**Preserved** (Curation-level concepts use "Energy" intentionally — they span both gas and wallet budgets):
- `CuratorDirective::OverrideEnergyBudget`
- `ActionType::AdjustEnergyBudget` / `ActionType::OverrideEnergyBudget`
- `ToolPortError::EnergyBudgetExceeded` (external API surface)

### 1.2 E02: Budget Persistence — Partial

**Implemented:**
- `GasBudgetManager::save_all(path)` — serializes budgets to JSON file
- `GasBudgetManager::load_all(path)` — deserializes from JSON file
- `CyberneticsLoop::with_budget_persistence(path)` — builder
- `CyberneticsLoop::load_budgets()` — public method to restore saved budgets
- Automatic save after each `replenish_all_budgets()` cycle in `act()`

**Not implemented (gaps):**
- `load_budgets()` is never called — dead code. No startup hook, no integration test.
- JSON file format has no version marker — fragile across type changes.
- No concurrent-write protection (two instances sharing a file = last write wins).
- `Result<(), String>` error type — unidiomatic.

### 1.3 E04: Budget Exhaustion Detection — Partial

**Implemented:**
- `GasBudgetManager::exhausted_agents()` — returns agents at zero with `hard_limit == true`
- `tracing::warn!` at `cns.cybernetics` target in `act()` after replenishment

**Not implemented (gaps):**
- The `tracing::warn!` is a log line — not connected to the Curator's algedonic pathway (`alerts_tx` → `CurationInput::Alert`). The cybernetic feedback loop is **open**.
- `exhausted_agents()` is a single-use convenience — could be inlined.

### 1.4 Also Built (not in prior plan)

- `GasBudgetManager::all_agent_statuses()` — returns all agent budget snapshots
- `GasBudgetManager::gas_ratios()` — returns (remaining, cap) pairs (superset of `all_agent_statuses()` — duplication; see G1 below)

## 2. Gap Inventory (prioritized, decomposed)

Each gap is decomposed into the smallest actionable step. Steps are atomic — each is one file change or one test addition.

### G1: Delete `gas_ratios()` — redundant with `all_agent_statuses()`

**Force:** Guideline | **Effort:** 5 min | **Risk:** None

| Step | Action | File |
|------|--------|------|
| 1.1 | Replace caller in `cybernetics_loop.rs:415` with `all_agent_statuses()` | `cybernetics_loop.rs` |
| 1.2 | Delete `gas_ratios()` method from `GasBudgetManager` | `energy_budget_management.rs` |
| 1.3 | Run `cargo test -p hkask-cns` — verify 99 tests pass | — |

### G2: Delete `exhausted_agents()` — inline at call site

**Force:** Guideline | **Effort:** 5 min | **Risk:** None

| Step | Action | File |
|------|--------|------|
| 2.1 | Inline the filter: `all_agent_statuses().into_iter().filter(\|(_, s)\| s.remaining.0 == 0 && s.hard_limit)` in `act()` | `cybernetics_loop.rs` |
| 2.2 | Delete `exhausted_agents()` method | `energy_budget_management.rs` |
| 2.3 | Run `cargo test -p hkask-cns` — verify passes | — |

### G3: Close the escalation feedback loop

**Force:** Guardrail | **Effort:** 15 min | **Risk:** Low — uses existing algedonic pathway

| Step | Action | File |
|------|--------|------|
| 3.1 | In `act()`, after detecting exhausted agents, construct `CurationInput::Alert` with `RuntimeAlert` | `cybernetics_loop.rs` |
| 3.2 | Send via `alerts_tx` if connected, fall back to `event_sink` if not (match existing pattern at line ~638) | `cybernetics_loop.rs` |
| 3.3 | Add test: `exhausted_agent_sends_alert_to_channel` — verify alert reaches `mpsc::UnboundedReceiver` | `cybernetics_loop.rs` tests |
| 3.4 | Run `cargo test -p hkask-cns` — verify passes | — |

### G4: Wire `load_budgets()` into startup

**Force:** Guardrail | **Effort:** 10 min | **Risk:** Low — JSON parse of known schema

| Step | Action | File |
|------|--------|------|
| 4.1 | Call `load_budgets()` after construction in `LoopSystem::tick()` or `AgentService::build()` — whichever constructs the CyberneticsLoop | `loop_system.rs` or caller |
| 4.2 | Add CNS span or tracing::info! on successful load (count loaded) | caller |
| 4.3 | Add test: `load_budgets_restores_persisted_state` — save, reload, verify budgets match | `cybernetics_loop.rs` tests |
| 4.4 | Run `cargo test -p hkask-cns` — verify passes | — |

### G5: Fix `Result<(), String>` → proper error type

**Force:** Guideline | **Effort:** 10 min | **Risk:** None — internal API

| Step | Action | File |
|------|--------|------|
| 5.1 | Add `GasPersistenceError` variant to `GasError` enum | `energy.rs` |
| 5.2 | Update `save_all` and `load_all` signatures to `Result<_, GasError>` | `energy_budget_management.rs` |
| 5.3 | Update `load_budgets()` to use `GasError` | `cybernetics_loop.rs` |
| 5.4 | Run `cargo test -p hkask-cns` — verify passes | — |

### G6: Fix plan doc references (stale names)

**Force:** Prohibition | **Effort:** This document replaces the old one

| Step | Action |
|------|--------|
| 6.1 | This document supersedes `energy-accounting-requirements-assessment.md` |
| 6.2 | Update `corpus_inventory.yaml` entry for the old plan → point to this doc |
| 6.3 | Update `TODO.md` P1-11 to reference this doc |

### G7: Persistence robustness — async I/O + version marker

**Force:** Guideline | **Effort:** 30 min | **Risk:** Medium — touches I/O path

| Step | Action | File |
|------|--------|------|
| 7.1 | Replace `std::fs::write` with `tokio::fs::write` in `save_all` | `energy_budget_management.rs` |
| 7.2 | Add `version: 1` field to serialized JSON envelope | `energy_budget_management.rs` |
| 7.3 | On load, check version field; reject unknown versions | `energy_budget_management.rs` |
| 7.4 | Add test: `reject_unknown_persistence_version` | `energy_budget_management.rs` tests |

### G8: Variety — add consumption velocity signal

**Force:** Guideline | **Effort:** 30 min | **Risk:** Low — new CNS span

| Step | Action | File |
|------|--------|------|
| 8.1 | Add `previous_remaining: HashMap<WebID, u64>` to `GasBudgetManager` | `energy_budget_management.rs` |
| 8.2 | In `replenish_all_budgets`, compute delta from previous → emit `tracing::info!` with consumption rate | `energy_budget_management.rs` |
| 8.3 | Update `previous_remaining` after delta computation | `energy_budget_management.rs` |

## 3. Remaining Deferred Items

These were pruned by the essentialist review. Listed for traceability; not planned.

| ID | Item | Why deferred |
|----|------|-------------|
| E03 | Tamper-evidence hash | Security theater — same module can recompute hash |
| E05 | Audit log | Valuable but not essential for correctness |
| E06 | Dynamic rJoule conversion | Wallet budgeting is partial; come back after wallet solidification |
| E07 | Remove pass-through methods | Code cleanup — separate task |
| E08 | Fixed-point arithmetic | Edge-case in distributed deployments only |
| E09–E11 | Anomaly/pool/dynamic pricing | P2 — revisit after core is solid |

## 4. Implementation Order

```
G1 (delete gas_ratios) ─┐
G2 (inline exhausted)   ├── 10 min, zero risk
G5 (fix error type)    ─┘

G3 (close escalation loop) ─┐
G4 (wire load_budgets)     ─┤── 25 min, closes two cybernetic loops
                            ─┘

G6 (update plan doc)     ─── 5 min, prohibition fix

G7 (async I/O + version) ─┐
G8 (consumption velocity) ─┘── 60 min, hardening pass
```

Total effort: ~100 min to eliminate all Guardrails and Prohibitions.

## 5. Verification

```bash
# After all gaps closed:
cargo build --workspace                    # 0 errors, 0 warnings
cargo test -p hkask-cns                    # 99+ tests pass
cargo test -p hkask-cns --lib cybernetics  # New escalation tests pass
```

---

*Supersedes `docs/status/energy-accounting-requirements-assessment.md`. Last updated 2026-07-01.*
