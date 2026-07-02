---
title: "Gas Budget System — Status"
audience: [architects, CNS developers, curator]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Trust"
mds_categories: [trust, lifecycle, domain]
---

# Gas Budget System — Status

## Implemented (v0.31.0)

### Core Gas System
- **Rename**: `Energy*` → `Gas*` across all crates. Curation concepts preserved (`OverrideEnergyBudget`, `AdjustEnergyBudget`).
- **Persistence**: Budgets saved as JSON `{version: 1, budgets: {...}}` via `tokio::fs`. Loaded at REPL startup.
- **Escalation**: Exhausted agents emit `CurationInput::Alert` via algedonic pathway (`alerts_tx`). Wallet exhaustion also escalates.
- **Stale reservations**: `last_reservation` timestamp on `GasBudget`. Auto-released after 5 min in `replenish_all_budgets`.
- **Consumption velocity**: `previous_remaining` per agent tracks gas burned across ticks.

### Well System
- `WellConfig`, `Well`, `WellManager` in `well.rs`
- Default Well auto-created on first replicant start
- Auto-replenish on each regulation cycle
- Exhaustion → algedonic alert with dampening (no re-alert fatigue)
- **Auto-draw**: agents below 25% budget draw from default Well into wallet

### Wallet System
- SQLite-backed `WalletStore` (`agent_wallets` table) + `WalletManager` in `wallet_manager.rs`
- Wallet auto-created on replicant start with initial balance
- Integrated into spend path: `can_proceed` → `reserve_gas` → `settle_gas` check wallet first
- Priority chain: WalletManager → WalletBackedBudget → GasBudget

### CNS Spans (10 new)
`ReplicantRegistered`, `WellCreated`, `WellReplenished`, `WellDraw`, `WellExhausted`, `WalletCreated`, `WalletDraw`, `WalletSpend`, `WalletExhausted`, `CuratorEfficiencyExceeded`

## Deferred / Future
- Two wallet systems coexist: `WalletBackedBudget` (rJoule/Hedera) and `WalletManager` (gas/SQLite). Gas wallets bridge to rJoule later.
- `acquire_budget()` dead code — zero callers.
- Pass-through methods on `CyberneticsLoop` — 9 methods of pure indirection.
- Hedera HTS token for rJoule.
- Well authorization per-agent.

## Verification
```bash
cargo build --workspace     # 0 errors, 0 warnings
cargo test -p hkask-cns     # 109 passed, 0 failed
```
