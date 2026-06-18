---
title: "User Expectation Field Audit"
audience: [engineers, agents]
last_updated: 2026-06-18
version: "0.28.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle, curation]
---

# User Expectation Field Audit

**Purpose:** Audit the adoption of the `user_expectation` field introduced in `FUNCTIONAL_SPECIFICATION.md` §5.2 (2026-06-18). This field makes each contract's explicit user functional expectation visible in the contract annotation itself — bridging the functional specification and the testing discipline.

**Status:** 0 of 2,519 REQ tags carry `user_expectation` (0.0% adoption). All existing contracts have self-evident expectations derivable from their pre/post annotations and function names. This audit identifies the migration priority.

---

## 1. Current State

| Metric | Value |
|--------|-------|
| Total REQ tags (workspace) | 2,519 |
| Contracts with `expect:` field | 25 (all in hkask-cns) |
| Adoption rate | 1.0% |
| Legacy IDs migrated 2026-06-18 | 104 (KAN-SVC-* → P3-svc-kanban-*, KAN-* → P8-typ-kanban-*) |
| Em-dash format bugs fixed | 1 (hkask-services-wallet) |
| CNS files with `expect:` field | energy.rs, algedonic.rs, api_metering.rs, circuit_breaker.rs, cybernetics_loop.rs |
| CNS files without `expect:` field | runtime.rs, governed_tool.rs, governed_inference.rs, composite_energy_estimator.rs, wallet_energy_estimator.rs, dynamic_gas_table.rs, gas_report.rs, calibrated_energy_estimator.rs, wallet_budget.rs, wallet_gas_calibrator.rs |

The `expect:` field (shorthand for `user_expectation`) uses first-person voice: `"I can check whether an agent has enough gas before executing"`. This matches the `user_expectation` pattern defined in `FUNCTIONAL_SPECIFICATION.md` §5.2.

## 2. Migration Priority by Domain

### P0 — CNS Regulation (hkask-cns, 197 contracts)

The CNS crate is the canonical example domain. Its contracts already have clear user expectations:

| Contract | User Expectation (Derivable) |
|----------|------------------------------|
| `P9-cns-energy-budget-can-proceed` | "I can check whether an agent has enough gas before executing" |
| `P9-cns-energy-budget-consume` | "I can deduct gas from an agent's budget" |
| `P9-cns-energy-budget-replenish` | "Agent budgets refill automatically over time" |
| `P9-cns-algedonic-alert-should-escalate` | "I am notified when variety deficit exceeds critical threshold" |
| `P9-cns-runtime-health` | "I can check the system's overall health at any time" |
| `P9-cns-gov-tool-new` | "Every tool invocation is governed and tracked" |
| `P9-cns-gov-inf-new` | "Every inference call is governed and tracked" |
| `P9-cns-circuit-allow-request` | "The system self-protects by cutting off failing services" |
| `P9-cns-api-meter-check-and-record` | "API rate limits protect the system from overload" |

### P1 — Wallet (hkask-wallet, 167 contracts)

| Contract | User Expectation (Derivable) |
|----------|------------------------------|
| `P9-wallet-mgr-can-afford` | "I can check if my wallet has enough rJoules before spending" |
| `P9-wallet-mgr-reserve` | "I can reserve rJoules to prevent double-spending" |
| `P9-wallet-mgr-settle` | "I only pay for what was actually used, not the reservation" |
| `P9-wallet-issuer-create-key` | "I can create scoped API keys with spending limits" |
| `P9-wallet-issuer-revoke-key` | "I can revoke API keys and get my unused balance back" |

### P2 — Memory (hkask-memory, 68 contracts)

| Contract | User Expectation (Derivable) |
|----------|------------------------------|
| `P3-mem-episodic-store` | "My private experiences are stored and not shared" |
| `P3-mem-semantic-store` | "Shared knowledge is stored for everyone" |
| `P3-mem-consolidation-bridge-consolidate` | "My private experiences can be promoted to shared knowledge" |
| `P3-mem-salience-compute-batch` | "Important content gets more storage budget" |

### P3 — All Other Domains

Remaining contracts across hkask-storage (168), hkask-agents (167), hkask-inference (90), hkask-templates (80), hkask-api (103), hkask-services (102), hkask-adapter (80), and smaller crates are lower priority but follow the same pattern.

## 3. Auto-Derivability Analysis

| Category | Count | Percentage | Example |
|----------|-------|-----------|---------|
| **Self-evident from function name** | ~1,800 | 71.5% | `can_proceed` → "I can check before executing" |
| **Requires domain context** | ~500 | 19.8% | `new()` → "I can create a new governed tool wrapper" |
| **Requires specification cross-reference** | ~219 | 8.7% | Type-level contracts, invariants, infrastructure |

## 4. Implementation Plan

1. **Phase 1 — CNS crate (197 contracts).** Demonstrate the pattern on the canonical crate. Each contract gets a one-sentence `user_expectation` field in the `/// REQ:` doc comment.
2. **Phase 2 — Wallet + Memory (235 contracts).** Extend to the next most user-visible domains.
3. **Phase 3 — Remaining crates (2,087 contracts).** Bulk migration via automated tooling (`scripts/tooling/add-user-expectations.sh`) with Curator review.

## 5. Verification

```bash
# Count user_expectation adoption
grep -rn "user_expectation" crates/ --include="*.rs" | wc -l

# Target: 2,519 (100% adoption)
```

---

*Audit generated 2026-06-18. Zero `user_expectation` fields in code. Migration is a gradual deepening — start with CNS, extend outward.*
