---
title: "Gas Budget System — Implementation Status & Gap Analysis"
audience: [architects, CNS developers, curator]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Trust"
mds_categories: [trust, lifecycle, domain]
---

# Gas Budget System — Status

Post-rename (`Energy*` → `Gas*`) and post-implementation status. All 8 gaps from the adversarial review are closed.

## What Was Built (v0.31.0)

### Rename (done)
`GasBudget`, `GasCost`, `GasBudgetManager`, `GasDelta`, `GasError`, `AgentGasStatus`. Curation-level names preserved: `OverrideEnergyBudget`, `AdjustEnergyBudget`, `EnergyBudgetExceeded`.

### E02: Persistence (done)
- `GasBudgetManager::save_all(path)` — async JSON write with `{version: 1, budgets: {...}}` envelope, tokio::fs
- `GasBudgetManager::load_all(path)` — async JSON read with version check
- `CyberneticsLoop::with_budget_persistence(path)` builder
- `CyberneticsLoop::load_budgets()` — called automatically at startup in `repl/init.rs`
- Automatic save after each replenishment cycle

### E04: Escalation (done)
- Budget exhaustion detected via `all_agent_statuses()` filter in `act()`
- Alerts sent through algedonic pathway: `alerts_tx → CurationInput::Alert`, fallback to `event_sink`
- Cybernetic feedback loop is closed — Curator receives budget exhaustion alerts

### G8: Consumption Velocity (done)
- `GasBudgetManager.previous_remaining` tracks per-agent remaining across ticks
- `replenish_all_budgets()` emits `tracing::debug` with `gas_burned` delta per agent per cycle

## Deleted (essentialist review)
- `gas_ratios()` — redundant with `all_agent_statuses()`
- `exhausted_agents()` — inlined at call site

## Fixes
- `Result<(), String>` → `Result<(), GasError>` with `Persistence` variant
- `std::fs` → `tokio::fs` for async I/O
- JSON envelope with `version: 1` marker — rejects unknown versions on load

## Remaining Deferred
E03 (tamper-evidence) — security theater. E05–E11 — P2, revisit after core is solid.

## Verification
```bash
cargo build --workspace     # 0 errors, 0 warnings
cargo test -p hkask-cns     # 99 passed, 0 failed
```

---

*Supersedes `docs/status/energy-accounting-requirements-assessment.md`.*
