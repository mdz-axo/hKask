---
title: "Energy Accounting Requirements Assessment"
audience: [architects, CNS developers, curator]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Trust"
mds_categories: [trust, lifecycle, domain]
---

# Energy Accounting — Requirements Assessment for v0.32+

Assessment of the current energy/gas accounting system against its functional specification and the hardening recommendations from the 2026-06-18 audit. Informs the plan to build the next iteration.

## 1. Current State

### 1.1 Architecture

The energy accounting system spans three layers:

| Layer | Component | Location |
|-------|-----------|----------|
| **Regulation (L6)** | `CyberneticsLoop` + `EnergyBudgetManager` | `crates/hkask-cns/src/cybernetics_loop.rs`, `energy_budget_management.rs` |
| **Enforcement (L5)** | `GovernedTool` — tool invocation membrane | `crates/hkask-cns/src/governed_tool.rs` |
| **Self-throttle (L1)** | `InferenceLoop` — self-regulating gas counter | `crates/hkask-agents/src/inference_loop.rs` |

### 1.2 Core Data Model

```
EnergyBudget {
    cap: u64              // Maximum gas allocation (dimensionless)
    remaining: u64        // Current balance
    replenish_rate: u64   // Units per replenishment cycle
    alert_threshold: f64  // Ratio below which alerts fire
    hard_limit: bool      // False = soft limit (can exceed budget)
    reserved: u64         // Gas held for in-flight operations
    priority: f64         // Weight for priority-scaled replenishment
}
```

### 1.3 Operations Implemented

| Operation | Caller | Description |
|-----------|--------|-------------|
| `register_energy_budget` | Bootstrap | Creates a budget for an agent |
| `can_proceed` | GovernedTool | Pre-flight check (wallet → gas fallback) |
| `reserve_gas` | GovernedTool | Hold-settle pattern: reserve estimate before call |
| `settle_gas` | GovernedTool | Adjust to actual cost, refund difference |
| `replenish_all_budgets` | CyberneticsLoop | Restore capacity on regulation cycle |
| `apply_override_energy_budget` | Curation | Metacognitive override (skips replenishment) |
| `apply_clear_override` | Curation | Resume normal replenishment |
| `replenish_agent_budget` | Curation | Directed replenishment by directive |

### 1.4 Integration Points

- **CNS span emission**: `cns.cybernetics.energy.budget` events on reserve/settle/replenish
- **Wallet integration**: `WalletBackedBudget` — rJoule-denominated wallets checked before gas budgets
- **Curation override**: `CuratorDirective::OverrideEnergyBudget` + `ClearOverride`
- **Dampener**: Metacognitive overrides dampened at longer window (prevents budget oscillation)

## 2. Audit Findings (2026-06-18)

The energy accounting hardening audit mapped 7 semantic operations across 23 surfaces:

| # | Operation | Surfaces | Current State |
|---|-----------|----------|---------------|
| 1 | Budget registration | 4 | Implemented |
| 2 | Pre-flight check (`can_proceed`) | 3 | Implemented |
| 3 | Reserve → execute → settle | 5 | Implemented |
| 4 | Replenishment | 3 | Implemented |
| 5 | Curation override | 3 | Implemented |
| 6 | Wallet-backed budgeting | 2 | Implemented (partial) |
| 7 | Budget query/status | 3 | **Not implemented** |

Three hardening recommendations were identified:

### R1: Remove CyberneticsLoop pass-through
**Status**: Open. `EnergyBudgetManager` is already extracted per Fowler H8, but `CyberneticsLoop` still contains pass-through methods that delegate directly to `EnergyBudgetManager` without adding regulation logic. These should be removed so callers use `EnergyBudgetManager` directly.

### R2: Add EnergyBudget tamper-evidence
**Status**: Open. The `EnergyBudget` struct is stored as `Arc<RwLock<HashMap<...>>>` but has no integrity verification. An agent (or bug) mutating its own budget would not be detected. Recommendation: hash the budget state on every mutation and verify on read.

### R3: Audit float determinism
**Status**: Open. `alert_threshold` and `priority` are `f64`. Floating-point operations in budget calculations are not deterministic across platforms. Recommendation: switch to fixed-point (e.g., basis points) or use `ordered_float`.

## 3. Gaps Against the Functional Specification

Cross-referencing against `FUNCTIONAL_SPECIFICATION.md` §3 (energy domain):

| Spec Requirement | Status | Gap |
|-----------------|--------|-----|
| Budget registration per-agent | ✅ | — |
| Reserve-settle pattern | ✅ | — |
| Hold-settle refund | ✅ | — |
| Replenishment cycle | ✅ | — |
| Curation override | ✅ | — |
| Wallet-backed budgets | ⚠️ Partial | Wallet integration exists but rJoule conversion rates are not dynamic |
| Energy budget query API | ❌ Missing | No MCP tool or API endpoint to query an agent's remaining budget |
| Budget exhaustion escalation | ❌ Missing | Algedonic alert exists but no automatic escalation path |
| Cross-agent budget sharing | ❌ Missing | Budgets are per-agent; no pool or transfer mechanism |
| Budget persistence across restarts | ❌ Missing | All budgets are in-memory; lost on restart |
| Budget history/audit log | ❌ Missing | No persisted record of budget operations |
| Per-operation cost estimation | ⚠️ Partial | Tool operations have cost estimates; inference token costs are not dynamically priced |
| Budget anomaly detection | ❌ Missing | No detection of sudden budget consumption spikes |

## 4. Requirements for v0.32+ Energy Accounting System

### 4.1 Must Have (P0)

| ID | Requirement | Rationale |
|----|------------|-----------|
| E01 | **Budget query API** — MCP tool `energy_status` returning `AgentEnergyStatus` per agent | Operators and Curation need visibility. Currently only internal `agent_gas_status()` exists. |
| E02 | **Budget persistence** — store budgets in `hkask-storage` (SQLite) so they survive restarts | All budgets lost on crash/restart. Trivial to exploit by restarting. |
| E03 | **Tamper-evidence** — hash budget state on mutation, verify on read (R2) | Prevents silent budget corruption. Simple Merkle-style hash chain per budget. |
| E04 | **Budget exhaustion escalation** — automatic `CuratorDirective` when budget hits zero for critical agents | Inference loop already self-throttles, but no systemic response when a critical agent (e.g., backup bot) exhausts its budget. |

### 4.2 Should Have (P1)

| ID | Requirement | Rationale |
|----|------------|-----------|
| E05 | **Budget history/audit log** — append-only table of all budget operations with CNS span IDs | Required for debugging, billing reconciliation, and security audits. |
| E06 | **Dynamic rJoule conversion** — wallet-backed budgets query live conversion rates | Current conversion is static. Makes wallet budgeting unreliable across provider price changes. |
| E07 | **Remove pass-through methods** (R1) — callers use `EnergyBudgetManager` directly | Simplifies the CyberneticsLoop API surface. Reduces indirection. |
| E08 | **Fixed-point arithmetic** (R3) — replace `f64` with `i64` basis points for deterministic cross-platform behavior | Floating-point non-determinism is a real risk for budget calculations in distributed deployments. |

### 4.3 Nice to Have (P2)

| ID | Requirement | Rationale |
|----|------------|-----------|
| E09 | **Budget anomaly detection** — CNS span for sudden consumption spikes (>3σ from 7-day rolling average) | Early warning for runaway loops or adversarial budget drain. |
| E10 | **Cross-agent budget pool** — shared pool with weighted allocation | Enables "team" budgets for agent pods. Complex — requires consensus on allocation weights. |
| E11 | **Per-operation dynamic pricing** — inference token costs reflect actual provider pricing | Current flat-rate cost model doesn't account for provider price differences. |

## 5. Implementation Plan

### Phase 1: Visibility & Persistence (2–3 sessions)

1. **E01: Budget query MCP tool** — Add `energy_status` tool to `hkask-mcp-curator`. Implement `EnergyBudgetManager::all_agent_statuses()` returning `Vec<(WebID, AgentEnergyStatus)>`. Wire into the curator MCP server.
2. **E02: Budget persistence** — Add `energy_budgets` SQLite table to `hkask-storage`. Implement `EnergyBudgetStore` with `save_all`/`load_all`. Call `load_all` in `CyberneticsLoop::build()`, call `save_all` after each `replenish_all_budgets()` cycle plus on shutdown.
3. **E03: Tamper-evidence** — Add `integrity_hash: [u8; 32]` field to `EnergyBudget`. Compute `SHA-256(cap || remaining || replenish_rate || previous_hash)` on every mutation. Verify on read. Store hash chain in the SQLite row.

### Phase 2: Hardening (1–2 sessions)

4. **E04: Escalation** — In `replenish_all_budgets()`, when a critical agent's budget is zero after replenishment, emit `CnsSpan::CyberneticsEnergyBudgetExhausted` and send `CuratorDirective::ReplenishBudget` through the alerts channel.
5. **E07: Remove pass-through** — Audit all callers of `CyberneticsLoop` methods that delegate to `EnergyBudgetManager`. Update callers to use `EnergyBudgetManager` directly. Remove pass-through methods. Run full test suite.
6. **E08: Fixed-point** — Replace `f64` with a `BasisPoints(i64)` newtype. Update all arithmetic. Ensure `ordered_float` is removed from the dependency graph.

### Phase 3: Operational Excellence (2–3 sessions)

7. **E05: Audit log** — Add `energy_operations` append-only table. Log every reserve/settle/replenish/override with CNS span ID, timestamp, agent, operation type, and amounts.
8. **E06: Dynamic pricing** — Add `rJouleRate` provider configuration. `WalletBackedBudget` queries this on reserve/settle. Cache with TTL.
9. **E09: Anomaly detection** — Rolling window (7 days, hourly buckets) in `CyberneticsLoop::sense()`. When consumption > 3σ above mean, emit `CnsSpan::CyberneticsEnergyAnomaly` and flag in Curator dashboard.

## 6. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Budget persistence race condition | Medium | High | Use SQLite WAL mode + serialized writes per agent |
| Hash chain performance overhead | Low | Low | SHA-256 of 64 bytes is ~100ns; negligible vs. budget operations (ms) |
| Fixed-point overflow on conversion | Low | Medium | Use `i64` basis points (range: ±9e14 rJoules — far exceeds any realistic budget) |
| Dynamic pricing API dependency | Medium | Medium | Cache with 1-hour TTL; fail open (use last known rate) on API error |
| Anomaly detection false positives | High | Low | 3σ threshold is lenient; false positives are alerts, not enforcement actions |

## 7. CNS Span Registry Additions

Proposed new CNS spans for the energy accounting domain:

```rust
// Budget lifecycle
CyberneticsEnergyBudgetExhausted,    // Agent budget hit zero after replenishment
CyberneticsEnergyAnomaly,            // Consumption spike detected
CyberneticsEnergyBudgetPersisted,    // Budget saved to storage
CyberneticsEnergyBudgetLoaded,       // Budget restored from storage
CyberneticsEnergyTamperDetected,     // Integrity hash mismatch

// Operational
CyberneticsEnergyAuditLogWritten,    // Audit log entry persisted
CyberneticsEnergyConversionRateUpdated, // rJoule rate changed
```

## 8. Verification

After Phase 1 completion, verify:

```bash
# Build
cargo build --workspace

# Budget persistence test
cargo test -p hkask-storage -- energy

# MCP tool availability
cargo test -p hkask-mcp-curator -- energy_status

# Tamper-evidence
cargo test -p hkask-cns -- tamper
```

---

*Assessment complete. Ready to begin Phase 1 implementation on the next session.*
