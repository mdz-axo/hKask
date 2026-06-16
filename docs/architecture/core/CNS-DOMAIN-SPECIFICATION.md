---
title: "CNS Domain Specification"
audience: [architects, developers, agents]
last_updated: 2026-06-16
version: "0.27.0"
status: "Active"
domain: "cybernetics"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# CNS Domain Specification

**Purpose:** A formal specification of the 6 cybernetic sub-domains implementing hKask's autonomous nervous system in `hkask-cns`. Each sub-domain maps to an authoritative principle from [`PRINCIPLES.md`](PRINCIPLES.md) and a concrete Rust module. All 44 contracts are implemented and tested.

**MDS Reference:** [`MDS.md`](MDS.md)

**Codebase Reference:** `crates/hkask-cns/src/` — 128 `pub fn` across 7 modules, 126 REQ-tagged contracts (98.4% coverage), all 200+ tests pass.

---

## 1. Sub-Domain Architecture

The CNS is structured into 6 sub-domains, each implementing a specific cybernetic function:

```
hkask-cns/src/
├── algedonic.rs           ← 4 contracts  — P9: Alert creation, escalation, severity classification
├── runtime.rs             ← 24 contracts — P9: Variety monitoring, outcome tracking, energy budgets
├── governed_tool.rs       ← 3 contracts  — P4: Tool membrane, OCAP gate, consumption channels
├── governed_inference.rs  ← 2 contracts  — P4: Inference membrane, agent attribution
├── circuit_breaker.rs     ← 3 contracts  — P4: Failure gating, state transitions
├── api_metering.rs       ← 8 contracts  — P9: Rate limiting, span creation, alert classification
└── energy.rs             ← 16 contracts — P9/P8: Gas budget types, delta, reserve, settle, repl
```

Each sub-domain is implemented in a single Rust file, following deep-module discipline (Ousterhout). The public surface is ≤ 7 items per module; internal plumbing is `pub(crate)`.

---

## 2. Contract Inventory

### 2.1 Algedonic Domain (`algedonic.rs`)

**Motivating principle:** P9 (Homeostatic Self-Regulation) — algedonic (pain/pleasure) feedback for cybernetic control. When variety deficit exceeds threshold, alerts are escalated to the Curator/human.

**Source:** `crates/hkask-cns/src/algedonic.rs` (261 lines, 6 tests)

| FR | Requirement | Contract |
|----|-----------|----------|
| `FR-ALGEDONIC-001` | Alert creation — binary threshold classification (deficit vs threshold) | `P9-cns-algedonic-alert-new` |
| `FR-ALGEDONIC-002` | Escalation check — escalate when severity == Critical | `P9-cns-algedonic-alert-should-escalate` |
| `FR-ALGEDONIC-003` | Critical severity check — severity == Critical | `P9-cns-algedonic-alert-is-critical` |
| `FR-ALGEDONIC-004` | Warning severity check — severity == Warning | `P9-cns-algedonic-alert-is-warning` |

**Note:** `DEFAULT_EXPECTED_VARIETY` and `DEFAULT_THRESHOLD` are P7 (configurable parameters emerged from real usage), not P9.

### 2.2 Runtime Domain (`runtime.rs`)

**Motivating principle:** P9 (Homeostatic Self-Regulation) — the CNS runtime is the single entry point for all observability and regulation operations.

**Source:** `crates/hkask-cns/src/runtime.rs` (723 lines, 6 tests)

| FR | Requirement | Contract |
|----|-----------|----------|
| `FR-RUNTIME-001` | Variety monitor creation — empty counters, ready to track | `P9-cns-runtime-variety-monitor-new` |
| `FR-RUNTIME-002` | Variety for domain — query distinct state count per domain | `P9-cns-runtime-variety-for-domain` |
| `FR-RUNTIME-003` | Variety monitor domains — enumerate all tracked domains | `P9-cns-runtime-variety-monitor-domains` |
| `FR-RUNTIME-004` | Runtime creation — configure threshold, initialize state | `P9-cns-runtime-with-threshold` |
| `FR-RUNTIME-005` | Health query — current CNS health snapshot | `P9-cns-runtime-health` |
| `FR-RUNTIME-006` | Alert retrieval — get all alerts | `P9-cns-runtime-alerts` |
| `FR-RUNTIME-007` | Default threshold — get configured threshold value | `P9-cns-runtime-default-threshold` |
| `FR-RUNTIME-008` | Critical alerts — get critical-severity alerts only | `P9-cns-runtime-critical-alerts` |
| `FR-RUNTIME-009` | Variety query — get variety counts across all domains | `P9-cns-runtime-variety` |
| `FR-RUNTIME-010` | Domain-specific variety — get variety for a specific domain | `P9-cns-runtime-variety-for-domain` |
| `FR-RUNTIME-011` | Blocking variety — sync access for non-async contexts | `P3-cns-runtime-blocking-variety-for-domain` |
| `FR-RUNTIME-012` | Outcome recording — track tool success/failure per domain | `P9-cns-runtime-record-outcome` |
| `FR-RUNTIME-013` | Outcome check — check success rate against thresholds | `P9-cns-runtime-check-outcome` |
| `FR-RUNTIME-014` | Outcome success rate — get success rate for a domain | `P9-cns-runtime-outcome-success-rate` |
| `FR-RUNTIME-015` | Variety increment — increment counter and check thresholds | `P9-cns-runtime-increment-variety` |
| `FR-RUNTIME-016` | Variety check — check variety health for a domain | `P9-cns-runtime-check-variety` |
| `FR-RUNTIME-017` | Threshold calibration — adjust expected variety per domain | `P7-cns-runtime-calibrate-threshold` |
| `FR-RUNTIME-018` | Blocking calibration — sync threshold calibration for bootstrap | `P3-cns-runtime-calibrate-threshold-blocking` |
| `FR-RUNTIME-019` | Observer subscription — subscribe CnsObserver to events | `P12-cns-runtime-subscribe` |
| `FR-RUNTIME-020` | Async subscription — subscribe observer from async context | `P12-cns-runtime-subscribe-async` |
| `FR-RUNTIME-021` | Backpressure emission — emit backpressure signal to subscribers | `P9-cns-runtime-emit-backpressure` |
| `FR-RUNTIME-022` | Energy budget registration — register budget for an agent | `P9-cns-runtime-register-energy-budget` |
| `FR-RUNTIME-023` | Agent budget replenishment — replenish an agent's budget | `P9-cns-runtime-replenish-agent-budget` |
| `FR-RUNTIME-024` | Agent gas status — get read-only budget snapshot | `P9-cns-runtime-agent-gas-status` |

### 2.3 Governed Tool Domain (`governed_tool.rs`)

**Motivating principle:** P4 (Clear Boundaries) — the membrane through which all tool invocations pass. Enforces OCAP authority verification and energy budgets.

**Source:** `crates/hkask-cns/src/governed_tool.rs` (562 lines, 7 tests)

| FR | Requirement | Contract |
|----|-----------|----------|
| `FR-GOVERNED-TOOL-001` | Membrane creation — wrap inner ToolPort with CNS cybernetics | `P9-cns-gov-tool-new` |
| `FR-GOVERNED-TOOL-002` | Consumption channel — wire tool consumption events to Cybernetics | `P9-cns-gov-tool-consumption-channel` |
| `FR-GOVERNED-TOOL-003` | Agent attribution — set agent WebID for ownership tracking | `P12-cns-gov-tool-with-agent` |

### 2.4 Governed Inference Domain (`governed_inference.rs`)

**Motivating principle:** P4 (Clear Boundaries) — the membrane through which all inference calls pass. Enforces energy budgets and CNS observability.

**Source:** `crates/hkask-cns/src/governed_inference.rs` (294 lines, 4 tests)

| FR | Requirement | Contract |
|----|-----------|----------|
| `FR-GOVERNED-INFERENCE-001` | Membrane creation — wrap inner InferencePort with CNS cybernetics | `P9-cns-gov-inf-new` |
| `FR-GOVERNED-INFERENCE-002` | Agent attribution — set agent WebID for ownership tracking | `P12-cns-gov-inf-with-agent` |

### 2.5 Circuit Breaker Domain (`circuit_breaker.rs`)

**Motivating principle:** P4 (Clear Boundaries) — circuit breaking is a CNS regulation mechanism enforcing homeostatic control over external service calls.

**Source:** `crates/hkask-cns/src/circuit_breaker.rs` (208 lines, 3 contracts)

| FR | Requirement | Contract |
|----|-----------|----------|
| `FR-CIRCUIT-001` | Default circuit — create with inference-appropriate defaults | `P9-cns-circuit-default-for-inference` |
| `FR-CIRCUIT-002` | Request gating — check if request is allowed through breaker | `P9-cns-circuit-allow-request` |
| `FR-CIRCUIT-003` | Success recording — count success, may transition to closed | `P9-cns-circuit-record-success` |

### 2.6 API Metering Domain (`api_metering.rs`)

**Motivating principle:** P9 (Homeostatic Self-Regulation) — per-key rate limiting, gas tracking, and CNS spans.

**Source:** `crates/hkask-cns/src/api_metering.rs` (452 lines, 8 tests)

| FR | Requirement | Contract |
|----|-----------|----------|
| `FR-API-METER-001` | Endpoint weight — return weight multiplier based on path pattern | `P9-cns-api-meter-endpoint-weight` |
| `FR-API-METER-002` | Rate limit status — string representation of alert type | `P9-cns-api-meter-rate-limit-status` |
| `FR-API-METER-003` | Meter creation — empty bucket map, ready for tracking | `P9-cns-api-meter-new` |
| `FR-API-METER-004` | Rate limit check — check and record request against limits | `P9-cns-api-meter-check-and-record` |
| `FR-API-METER-005` | Current RPM — get current requests per minute for a key | `P9-cns-api-meter-current-rpm` |
| `FR-API-METER-006` | Span creation — build API request span from metering data | `P9-cns-api-meter-span-new` |
| `FR-API-METER-007` | Alert type — get alert type label from ApiMeteringAlert | `P9-cns-api-meter-alert-type` |
| `FR-API-METER-008` | Alert severity — get severity string from ApiMeteringAlert | `P9-cns-api-meter-alert-severity` |

---

## 3. Verification

```bash
# Verify all 44 contract IDs match the codebase:
cargo check -p hkask-cns
cargo test -p hkask-cns

# Count contracts:
grep -rn "/// REQ: P" crates/hkask-cns/src/ --include="*.rs" | wc -l

# CNS span health:
kask cns status
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*