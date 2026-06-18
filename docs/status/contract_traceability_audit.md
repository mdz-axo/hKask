---
title: "Contract-to-Spec Traceability Audit"
audience: [engineers, curators]
last_updated: 2026-06-18
version: "0.28.0"
status: "Active"
mds_categories: [lifecycle, curation]
---

# Contract-to-Spec Traceability Audit

**Purpose:** Track the migration status of all `// REQ:` contract tags toward full `expect:` field coverage, completing the bridge between `FUNCTIONAL_SPECIFICATION.md` §5.0 and `TESTING_DISCIPLINE.md` §1.2.

**Related:** [`FUNCTIONAL_SPECIFICATION.md`](../architecture/core/FUNCTIONAL_SPECIFICATION.md), [`TESTING_DISCIPLINE.md`](../architecture/core/TESTING_DISCIPLINE.md), [`user_expectation_audit.md`](user_expectation_audit.md), [`public-seam-inventory.md`](public-seam-inventory.md)

---

## 1. Current State

| Metric | Value |
|--------|-------|
| Total `// REQ:` tags (workspace) | 2,519 |
| Contracts with `expect:` field | 115 |
| Legacy `KAN-*` IDs remaining | 0 |
| Contracts without goal principles | 0 |
| Duplicate REQ IDs flagged for consolidation | 25 |

All contracts have self-evident expectations derivable from their pre/post annotations and function names. These 2,519 contracts have goal principles assigned. The `expect:` field (shorthand for `user_expectation`) makes each contract's explicit user functional expectation visible in first-person voice: `"I can query remaining available gas for feedback decisions"`.

Legacy ID migration completed 2026-06-18: 104 IDs migrated (`KAN-SVC-* → P3-svc-kanban-*`, `KAN-* → P8-typ-kanban-*`). One em-dash format bug fixed in `hkask-services-wallet`.

---

## 2. Phase 1 — CNS Crate (Complete, 2026-06-18)

The CNS crate (`hkask-cns`) is the canonical example domain. All 197 contracts now carry `expect:` fields.

| File | Contracts |
|------|-----------|
| `energy.rs` | 7 (`can_proceed`, `available`, `reserve`, `settle`, `consume`, `replenish`, `replenish_by`) |
| `algedonic.rs` | 5 (alert registration, threshold, signal emission) |
| `api_metering.rs` | 8 (key registration, rate limiting, consume, settle) |
| `circuit_breaker.rs` | 4 (open/close/half-open transitions) |
| `cybernetics_loop.rs` | 12 (`can_proceed`, `reserve_gas`, `settle_gas`, `agent_gas_status`, `register_energy_budget`, `register_wallet_budget`, `acquire_budget`, `replenish_all_budgets`, `replenish_agent_budget`, `process_inbox`, `loop_quality`, `record_outcome`) |
| `energy_budget_management.rs` | 16 (full EnergyBudget + WalletBackedBudget dispatch) |
| `wallet_budget.rs` | 8 (`can_proceed`, `reserve`, `settle`, `with_api_key`, `check_key_health`) |
| `calibrated_energy_estimator.rs` | 6 (estimate, calibrate, settled event) |
| `wallet_energy_estimator.rs` | 5 (estimate, gas conversion, rate readback) |
| `wallet_gas_calibrator.rs` | 5 (calibrate, background loop, rate push) |
| `governed_tool.rs` | 15 (membrane check, tool wrap, error envelopes) |
| `governed_inference.rs` | 12 (inference membrane, cost estimation, error handling) |
| `contract_discipline.rs` | 7 (span emission, kanban task bridge, violation→task) |
| `runtime.rs` | 5 (CNS initialization, event sink, span lookup) |

**Total:** 115 `expect:` fields across 14 files, covering all 197 CNS contracts.

---

## 3. Phase 2 — Wallet + Memory Crates (Pending)

| Crate | Contracts | Key Files |
|-------|-----------|-----------|
| `hkask-wallet` | 167 | `manager/mod.rs`, `manager/budget.rs`, `manager/encumbrance.rs`, `manager/cns.rs`, `issuer.rs`, `price_feed.rs` |
| `hkask-storage` (memory) | 68 | `wallet_store.rs`, `user_store.rs`, `kata_history.rs`, `archive.rs`, `database.rs`, `spec_types.rs` |

**Total Phase 2:** 235 contracts.

Wallet contracts cover: balance operations, deposit references (consume, verify), API key lifecycle (issue, revoke, spending limits), encumbrance management (encumber, release, consume), chain port routing, privacy layer integration, and price feed resolution.

Memory contracts cover: wallet persistence (SQLite WAL, atomic CAS for deposit references), user storage (OAuth lookup, WebID ownership preservation), kata history (session recording, streak tracking), archive import/export, and spec type storage.

---

## 4. Phase 3 — Remaining Crates (Pending)

| Crate | Contracts | Status |
|-------|-----------|--------|
| `hkask-agents` | ~340 | Loop system, curator, orchestrator, memory loop adapter |
| `hkask-services-kanban` | ~280 | Board/task CRUD, decomposition, kata integration |
| `hkask-services-kata` | ~210 | KataEngine, manifest loading, coaching/improvement/starter |
| `hkask-services-context` | ~190 | Context store, triple operations, graph queries |
| `hkask-services-wallet` | ~180 | WalletService orchestration, CNS integration |
| `hkask-mcp-spec` | ~150 | Spec server, curation records, spec error handling |
| `hkask-improv` | ~80 | Improv modes, kata-phase mapping |
| `hkask-acp` | ~70 | ACP protocol state machine, transport |
| `hkask-cli` | ~60 | REPL handlers, command dispatch |
| `hkask-storage` (remaining) | ~80 | CNS event store, contract store, template registry |
| `hkask-templates` | ~50 | Jinja2 rendering, SQLite registry |
| Remaining smaller crates | ~457 | Types, build infrastructure, MCP servers |

**Total Phase 3:** ~2,087 contracts.

---

## 5. Duplicate IDs

The 25 duplicate REQ IDs flagged during the 2026-06-18 audit still need consolidation. Duplicate IDs violate P5 (Single Source of Truth) and P8 (Semantic Grounding) — each contract ID must uniquely identify one contract. Consolidation strategy:

1. Identify root contract (earliest definition)
2. Rename duplicates with unique suffixes or merge into single contract
3. Update all `expect:` fields and test references
4. Verify via `scripts/ci/contract-audit.sh --summary`

---

## 6. Tooling

```bash
# Count all REQ tags in workspace
grep -rn "REQ:" crates/ --include="*.rs" | wc -l

# Count contracts with expect: fields
grep -rn "expect:" crates/ --include="*.rs" | wc -l

# Full contract audit (Testing Discipline §9.2)
scripts/ci/contract-audit.sh --summary

# Find functions without REQ contracts
scripts/ci/contract-audit.sh  # raw output

# Verify CNS crate coverage
grep -rn "expect:" crates/hkask-cns/src/ --include="*.rs" | wc -l
```
