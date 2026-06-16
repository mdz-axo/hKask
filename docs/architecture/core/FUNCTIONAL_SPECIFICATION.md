# Functional Specification — hKask System

**Version:** v0.27.0  
**Last updated:** 2026-06-16  
**Status:** In progress (CNS crate realignment active)  
**Author:** hKask project (agent-authored, principle-anchored)

## Purpose

This document enumerates every functional requirement (FR) in the hKask system, organized by domain rather than crate. Each FR is linked to the contracts that implement it, and each domain is mapped to its motivating principle. This serves as:

1. **The canonical FR inventory** — the single source of truth for what the system must do.
2. **The contract vocabulary** — each FR maps to `P{N}-{domain}-{operation}` IDs used in code.
3. **The rSolidity anchor** — if the system were reimplemented in rSolidity, these FRs ARE the spec.
4. **The verification target** — `scripts/gen-req-inventory.sh` confirms every FR has a corresponding contract.

## Domain Breakdown — 22 Domains

| # | Domain | Tag | Crate | Principle | Contract Count |
|---|--------|-----|-------|-----------|----------------|
| 1 | **Energy (Gas)** | `cns-energy` | `hkask-cns` | P9 (Homeostasis) | 20 |
| 2 | **Algedonic** | `cns-algedonic` | `hkask-cns` | P9 (Homeostasis) | 4 |
| 3 | **Runtime** | `cns-runtime` | `hkask-cns` | P9 (Homeostasis) | 24 |
| 4 | **Governed Tool** | `cns-governed-tool` | `hkask-cns` | P4 (Boundaries) | 3 |
| 5 | **Governed Inference** | `cns-governed-inference` | `hkask-cns` | P4 (Boundaries) | 2 |
| 6 | **Circuit Breaker** | `cns-circuit` | `hkask-cns` | P4 (Boundaries) | 3 |
| 7 | **API Metering** | `cns-api-meter` | `hkask-cns` | P9 (Homeostasis) | 8 |
| 8 | **Composite Estimator** | `cns-composite-est` | `hkask-cns` | P9 (Homeostasis) | 1 |
| 9 | **Wallet Estimator** | `cns-wallet-est` | `hkask-cns` | P9 (Homeostasis) | 1 |
| 10 | **Wallet** | `wallet` | `hkask-wallet` | P2 (Sovereignty) | 23 |
| 11 | **Storage** | `storage` | `hkask-storage` | P1 (Sovereignty) | (TBD) |
| 12 | **Memory** | `memory` | `hkask-memory` | P8 (Grounding) | (TBD) |
| 13 | **Inference** | `inference` | `hkask-inference` | P8 (Grounding) | (TBD) |
| 14 | **Templates** | `templates` | `hkask-templates` | P7 (Evolution) | (TBD) |
| 15 | **MCP** | `mcp` | `hkask-mcp-*` | P3 (Generative) | (TBD) |
| 16 | **Services** | `services` | `hkask-services` | P5 (Essentialism) | (TBD) |
| 17 | **Agents** | `agents` | `hkask-agents` | P12 (Consent) | 30 |
| 18 | **Communication** | `communication` | `hkask-communication` | P12 (Consent) | (TBD) |
| 19 | **Keystore** | `keystore` | `hkask-keystore` | P2 (Sovereignty) | (TBD) |
| 20 | **Types** | `types` | `hkask-types` | P8 (Grounding) | (TBD) |
| 21 | **API** | `api` | `hkask-api` | P3 (Generative) | (TBD) |
| 22 | **CLI** | `cli` | `hkask-cli` | P3 (Generative) | (TBD) |

## Principle ↔ Domain Mapping

```
P1 (User Sovereignty)  → Storage (data ownership), Keystore (key custody)
P2 (User Sovereignty)  → Wallet (wallet operations, consent records)
P3 (Generative Space)  → MCP, API, CLI (action preserves ability to act)
P4 (Clear Boundaries)   → Governed Tool, Governed Inference, Circuit Breaker, Energy (cap enforcement)
P5 (Essentialism)        → Services (deep modules), Energy (minimal constructors)
P7 (Evolutionary Arch)  → Templates, Energy (configurable params from real usage)
P8 (Semantic Grounding)→ Types, Memory, Inference (type-level identity)
P9 (Homeostatic Reg)    → Energy, Algedonic, Runtime, API Metering, Composite Estimator, Wallet Estimator
P12 (Affirmative Consent)→ Agents, Communication, Runtime (subscriber registration)
```

---

## Section 1: CNS Domains (Realigned)

### 1.1 Energy/Gas Domain (`cns-energy`)

**Motivating principle:** P9 (Homeostatic Self-Regulation) — gas budget enforcement is the primary cybernetic regulation mechanism.

| FR | Requirement | Contracts |
|----|-----------|-----------|
| `FR-ENERGY-001` | Gas type identity — prevent confusion with other u64 quantities | `P8-cns-energy-cost-from-raw`, `P8-cns-energy-cost-as-raw` |
| `FR-ENERGY-002` | Delta type identity — measure energy change between states | `P8-cns-energy-delta-from-raw`, `P8-cns-energy-delta-as-raw` |
| `FR-ENERGY-003` | Lazy universe detection — system moved toward lower energy | `P9-cns-energy-delta-descending` |
| `FR-ENERGY-004` | Anti-lazy detection — system moved toward higher energy | `P9-cns-energy-delta-ascending` |
| `FR-ENERGY-005` | Budget creation — allocate gas cap, set defaults | `P9-cns-energy-budget-new` |
| `FR-ENERGY-006` | Unlimited budget — observability without throttling | `P9-cns-energy-budget-unlimited` |
| `FR-ENERGY-007` | Replenishment rate — configurable replenishment knob | `P9-cns-energy-budget-with-replenish-rate` |
| `FR-ENERGY-008` | Alert threshold — configurable alert threshold | `P9-cns-energy-budget-with-alert-threshold` |
| `FR-ENERGY-009` | Hard limit toggle — boundary enforcement control | `P9-cns-energy-budget-with-hard-limit` |
| `FR-ENERGY-010` | Pre-flight check — can operation proceed? | `P9-cns-energy-budget-can-proceed` |
| `FR-ENERGY-011` | Available calculation — remaining minus reserved | `P9-cns-energy-budget-available` |
| `FR-ENERGY-012` | Hold-settle reserve — gas reservation for in-flight ops | `P9-cns-energy-budget-reserve` |
| `FR-ENERGY-013` | Hold-settle settlement — deduct actual from reserved | `P9-cns-energy-budget-settle` |
| `FR-ENERGY-014` | Immediate consume — non-reserved gas deduction | `P9-cns-energy-budget-consume` |
| `FR-ENERGY-015` | Regulated replenishment — timed budget replenishment | `P9-cns-energy-budget-replenish` |
| `FR-ENERGY-016` | Directed replenishment — targeted replenishment by amount | `P9-cns-energy-budget-replenish-by` |
| `FR-ENERGY-017` | Priority-weighted replenishment — priority-scaled replenishment | `P9-cns-energy-budget-replenish-by-weighted` |
| `FR-ENERGY-018` | Budget cap invariant — remaining + reserved ≤ cap | `P9-cns-energy-budget-invariant` (plus tests) |
| `FR-ENERGY-019` | Available never negative — available ≥ 0 | `P9-cns-energy-budget-available` (test) |
| `FR-ENERGY-020` | Replenish never exceeds cap — replenish bounded by cap | `P9-cns-energy-budget-replenish` (test) |

###