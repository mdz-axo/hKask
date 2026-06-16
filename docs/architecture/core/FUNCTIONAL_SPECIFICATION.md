# hKask Functional Specification

**Version:** v0.27.0
**Created:** 2026-06-16
**Status:** Active — anchor for the rSolidity contract vocabulary
**Last Updated:** 2026-06-16

> This document maps the complete system to its motivating principles, enumerates functional requirements per domain, and links each requirement to the contracts that implement it. It serves as the specification anchor from which the rSolidity contract vocabulary will be derived.

---

## 1. Domain Breakdown

The system is decomposed into **22 functional domains**, cross-cutting the crate boundaries. Each domain is owned by a single principle (the **motivating principle**) and may carry additional constraining principles.

### Domain Map

| # | Domain | Short Tag | Crate | Contracts | Motivating Principle |
|---|---------|-----------|-------|-----------|----------------------|
| 1 | Energy Budgeting | `energy` | hkask-cns | 20 | P9 (Homeostatic Self-Regulation) |
| 2 | Algedonic Signalling | `algedonic` | hkask-cns | 4 | P9 (Homeostatic Self-Regulation) |
| 3 | Runtime Observability | `runtime` | hkask-cns | 24 | P9 (Homeostatic Self-Regulation) |
| 4 | Tool Governance | `gov-tool` | hkask-cns | 3 | P4 (Clear Boundaries) |
| 5 | Inference Governance | `gov-inf` | hkask-cns | 2 | P4 (Clear Boundaries) |
| 6 | Circuit Breaking | `circuit` | hkask-cns | 3 | P9 (Homeostatic Self-Regulation) |
| 7 | API Metering | `api` | hkask-cns | 8 | P9 (Homeostatic Self-Regulation) |
| 8 | Energy Estimation | `est` | hkask-cns | 2 | P9 (Homeostatic Self-Regulation) |
| 9 | Wallet Management | `wallet` | hkask-wallet | 23 | P2 (User Sovereignty) |
| 10 | Storage Operations | `storage` | hkask-storage | 195 | P1 (User Sovereignty) |
| 11 | Memory Management | `memory` | hkask-memory | 52 | P2 (User Sovereignty) |
| 12 | Inference Execution | `inference` | hkask-inference | 86 | P9 (Homeostatic Self-Regulation) |
| 13 | Template Rendering | `templates` | hkask-templates | 52 | P3 (Generative Space) |
| 14 | MCP Framework | `mcp` | hkask-mcp | 41 | P4 (Clear Boundaries) |
| 15 | Service Layer | `services` | hkask-services | 201 | P7 (Evolutionary Architecture) |
| 16 | Agent Runtime | `agents` | hkask-agents | 30 | P12 (Affirmative Consent) |
| 17 | Communication | `comm` | hkask-communication | 25 | P12 (Affirmative Consent) |
| 18 | Keystore Management | `keystore` | hkask-keystore | 28 | P12 (Affirmative Consent) |
| 19 | Type System | `types` | hkask-types | 99 | P8 (Semantic Grounding) |
| 20 | API Surface | `api-surface` | hkask-api | 8 | P4 (Clear Boundaries) |
| 21 | CLI Interface | `cli` | hkask-cli | 2 | P4 (Clear Boundaries) |
| 22 | Test Harness | `test` | hkask-test-harness | 42 | P5 (Essentialism) |

**Total:** 22 domains, ~900 contracts across 15 crates.

### Domain Ownership Rules

- **CNS domains (1–8):** All owned by P9 — the CNS is the cybernetic controller. Every regulation-loop operation begins with P9 as motivating principle.
- **Storage (10):** P1 (User Sovereignty) — data ownership is the root of all consent.
- **Wallet (9):** P2 (User Sovereignty) — the wallet is the user's financial sovereignty anchor.
- **Agents (16):** P12 (Affirmative Consent) — consent records are the consent anchor.
- **Types (19):** P8 (Semantic Grounding) — newtypes and conversions carry meaning-preservation contracts.
- **Services (15):** P7 (Evolutionary Architecture) — configurable parameters emerged from real usage.

---

## 2. CNS Domains — Functional Requirements

### 2.1 Energy Budget (hkask-cns/energy.rs)

The energy budget is the **primary regulation mechanism** — it enforces gas limits on all operations. Cybernetics is the homeostatic controller: it holds the budget state, checks availability, reserves gas, settles actual costs, and replenishes over time.

20 contracts implement the full budget lifecycle:

| ID | Requirement | Contracts |
|----|-----------|-----------|
| `FR-ENERGY-001` | Type-level energy conversions must be identity-preserving | `P8-cns-energy-cost-from-raw`, `P8-cns-energy-cost-as-raw`, `P8-cns-energy-delta-from-raw`, `P8-cns-energy-delta-as-raw` |
| `FR-ENERGY-002` | Delta direction tests must be logically consistent | `P9-cns-energy-delta-descending`, `P9-cns-energy-delta-ascending` |
| `FR-ENERGY-003` | Budget invariant holds for all state transitions | `P9-cns-energy-budget-invariant` |
| `FR-ENERGY-004` | Budget creation requires cap > 0, defaults safe | `P9-cns-energy-budget-new` |
| `FR-ENERGY-005` | Unlimited budget bounds cap at u64::MAX, hard_limit = false | `P9-cns-energy-budget-unlimited` |
| `FR-ENERGY-006` | Budget can be configured with replenish rate | `P9-cns-energy-budget-with-replenish-rate` |
| `FR-ENERGY-007` | Budget can be configured with alert threshold | `P9-cns-energy-budget-with-alert-threshold` |
| `FR-ENERGY-008` | Budget can be configured with hard limit toggle | `P9-cns-energy-budget-with-hard-limit` |
| `FR-ENERGY-009` | Gas check returns true iff gas <= available OR hard_limit is false | `P9-cns-energy-budget-can-proceed` |
| `FR-ENERGY-010` | Available gas calculation is non-negative | `P9-cns-energy-budget-available` |
| `FR-ENERGY-011` | Reserve gas increases reserved, maintains remaining + reserved <= cap | `P9-cns-energy-budget-reserve` |
| `FR-ENERGY-012` | Settle gas decreases reserved, decreases remaining | `P9-cns-energy-budget-settle` |
| `FR-ENERGY-013` | Consume gas decreases remaining directly | `P9-cns-energy-budget-consume` |
| `FR-ENERGY-014` | Replenish increases remaining up to cap | `P9-cns-energy-budget-replenish` |
| `FR-ENERGY-015` | Replenish-by increases remaining by up to amount | `P9-cns-energy-budget-replenish-by` |
| `FR-ENERGY-016` | Weighted replenish scales by priority, returns actual amount | `P9-cns-energy-budget-replenish-by-weighted` |

### 2.2 Algedonic Signalling (hkask-cns/algedonic.rs)

Algedonic signals are the **pain/pleasure** channel of the CNS — they translate health metrics into actionable alerts. 4 contracts implement the Alert → Severity pipeline:

| ID | Requirement | Contracts |
|----|-----------|-----------|
| `FR-ALGEDONIC-001` | Alert creation requires domain and threshold, severity based on deficit | `P9-cns-algedonic-alert-new` |
| `FR-ALGEDONIC-002` | Alert escalation check returns true iff severity is Critical | `P9-cns-algedonic-alert-should-escalate` |
| `FR-ALGEDONIC-003` | Severity classification must be correct for all three levels | `P9-cns-algedonic-alert-is-critical`, `P9-cns-algedonic-alert-is-warning` |
| `FR-ALGEDONIC-004` | Severity is Warning check returns true iff severity == Warning | `P9-cns-algedonic-alert-is-warning` |

### 2.3 Runtime Observability (hkask-cns/runtime.rs)

Runtime is the **CNS observability surface** — it tracks variety, outcomes, and health across domains. 24 contracts implement the full CNS lifecycle:

| ID | Requirement | Contracts |
|----|-----------|-----------|
| `FR-RUNTIME-001` | Variety monitor creation with empty counters | `P9-cns-runtime-variety-monitor-new` |
| `FR-RUNTIME-002` | Variety tracking per domain, returns 0 if domain not tracked | `P9-cns-runtime-variety-for-domain` |
| `FR-RUNTIME-003` | List tracked domains | `P9-cns-runtime-variety-monitor-domains` |
| `FR-RUNTIME-004` | Runtime configuration with threshold | `P9-cns-runtime-with-threshold` |
| `FR-RUNTIME-005` | Health check returns current state | `P9-cns-runtime-health` |
| `FR-RUNTIME-006` | Alert list returns active alerts | `P9-cns-runtime-alerts` |
| `FR-RUNTIME-007` | Default threshold from algedonic manager | `P9-cns-runtime-default-threshold` |
| `FR-RUNTIME-008` | Critical alert filtering | `P9-cns-runtime-critical-alerts` |
| `FR-RUNTIME-009` | Variety retrieval returns namespace->count map | `P9-cns-runtime-variety` |
| `FR-RUNTIME-010` | Variety for domain with non-empty pre | `P9-cns-runtime-variety-for-domain` |
| `FR-RUNTIME-011` | Blocking variety check (P3 — Generative Space) | `P3-cns-runtime-blocking-variety-for-domain` |
| `FR-RUNTIME-012` | Outcome recording after tool/inference operations | `P9-cns-runtime-record-outcome` |
| `FR-RUNTIME-013` | Outcome check triggers alert if below threshold | `P9-cns-runtime-check-outcome` |
| `FR-RUNTIME-014` | Success rate calculation for tracked domains | `P9-cns-runtime-outcome-success-rate` |
| `FR-RUNTIME-015` | Increment variety counter | `P9-cns-runtime-increment-variety` |
| `FR-RUNTIME-016` | Check variety below threshold triggers alert | `P9-cns-runtime-check-variety` |
| `FR-RUNTIME-017` | Calibrate threshold for domain (P7 — configurable) | `P7-cns-runtime-calibrate-threshold` |
| `FR-RUNTIME-018` | Blocking calibraate (P3 — Generattive Space) | `P3-cns-runtim-e-calibrate-threshold-blocking` |
| `FR-RUNTIME-019` | Subscriber regisstration (P12 — Afirmative Consent) | `P12-cns-runtim-e-subscribe` |
| `FR-RUNTIME-020` | Async subscriiber regisstration | `P12-cns-runtme-subscribe-async` |
| `FR-RUNTIME-021` | Backpresssure emission to subscriibers | `P9-cns-runtim-e-emit-backpresssure` |
| `FR-RUNTIME-022` | Energy budget regisstration for agent | `P9-cns-runtime-register-energy-budget` |
| `FR-RUNTIME-023` | Agent budget replenishment | `P9-cns-runtime-replenish-agent-budget` |
| `FR-RUNTIME-024` | Agent gas status query | `P9-cns-runtime-agent-gass-status` |