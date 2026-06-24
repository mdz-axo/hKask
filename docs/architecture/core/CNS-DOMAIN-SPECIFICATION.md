---
title: "CNS Domain Specification"
audience: [architects, developers, agents]
last_updated: 2026-06-18
version: "0.30.0"
status: "Active"
domain: "cybernetics"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# CNS Domain Specification

**Purpose:** A formal specification of the 8 cybernetic sub-domains implementing hKask's autonomous nervous system in `hkask-cns`. Each sub-domain maps to an authoritative principle from [`PRINCIPLES.md`](PRINCIPLES.md) and a concrete Rust module. All 197 contracts carry `expect:` fields encoding user functional expectations per [`FUNCTIONAL_SPECIFICATION.md` §5](FUNCTIONAL_SPECIFICATION.md).

**MDS Reference:** [`MDS.md`](MDS.md)

**Codebase Reference:** `crates/hkask-cns/src/` — 152 `pub fn`, 197 REQ-tagged contracts (129.6% coverage), 115 `expect:` fields encoding user expectations. All tests pass.

---

## 1. Sub-Domain Architecture

The CNS is structured into 8 sub-domains, each implementing a specific cybernetic function:

```
hkask-cns/src/
├── algedonic.rs                  ← 4 contracts  — P9: Alert creation, escalation, severity classification
├── runtime.rs                    ← 24 contracts — P9: Variety monitoring, outcome tracking, energy budgets
├── governed_tool.rs              ← 3 contracts  — P4: Tool membrane, OCAP gate, consumption channels
├── governed_inference.rs         ← 2 contracts  — P4: Inference membrane, agent attribution
├── circuit_breaker.rs          ← 3 contracts  — P4: Failure gating, state transitions
├── api_metering.rs            ← 8 contracts  — P9: Rate limiting, span creation, alert classification
├── energy.rs                  ← 16 contracts — P9/P8: Gas budget types, delta, reserve, settle, repl
├── dynamic_gas_table.rs       ← P9: Per-server gas cost calibration from CNS settled events
├── gas_report.rs             ← P9: Aggregate gas queries across agents and time windows
├── calibrated_energy_estimator.rs ← P9: Self-regulating per-server gas cost estimator
├── composite_energy_estimator.rs  ← P9: Routes inference to token estimator, others to table
├── wallet_budget.rs          ← P9/P4: Wallet-backed energy budget (rJoule debits)
├── wallet_energy_estimator.rs ← P9: Wallet-aware composite estimator with gas→rJoule rate
└── wallet_gas_calibrator.rs  ← P9: Runtime calibration of wallet gas→rJoule conversion
```

Each sub-domain is implemented in a single Rust file (or a tight cluster of files), following deep-module discipline (Ousterhout). The public surface is ≤ 7 items per module; internal plumbing is `pub(crate)`.

---

## 2. Contract Inventory

### 2.1 Algedonic Domain (`algedonic.rs`)

**Goal principle:** P9 (Homeostatic Self-Regulation) — algedonic (pain/pleasure) feedback for cybernetic control. When variety deficit exceeds threshold, alerts are escalated to the Curator/human.

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

### 2.7 Gas Cost Calibration Domain (`dynamic_gas_table.rs`, `gas_report.rs`, `calibrated_energy_estimator.rs`)

**Motivating principle:** P9 (Homeostatic Self-Regulation) — closes the Good Regulator feedback loop for per-server gas costs. Observes `cns.gas.settled` events, compares actual vs estimated gas per server, and adjusts hardcoded costs via exponential moving average (EMA).

**Source:**
- `crates/hkask-cns/src/dynamic_gas_table.rs` (198 lines, 8 tests)
- `crates/hkask-cns/src/gas_report.rs` (566 lines, 6 tests)
- `crates/hkask-cns/src/calibrated_energy_estimator.rs` (230 lines, 5 tests)

| FR | Requirement | Contract |
|----|-----------|----------|
| `FR-GAS-CALIB-001` | Dynamic table — per-server gas cost calibration from observations | `GAS-CALIB-001` |
| `FR-GAS-CALIB-002` | Single observation — initializes EMA per server | `GAS-CALIB-002` |
| `FR-GAS-CALIB-003` | Calibrated table — replaces hardcoded `TableEnergyEstimator` costs | `GAS-CALIB-003` |
| `FR-GAS-CALIB-004` | Calibrated estimator — runtime calibration loop wired to production estimator | `GAS-CALIB-004` |

**New CNS spans:** `cns.gas.calibrated` — emitted when `CalibratedEnergyEstimator` adjusts per-server costs.

### 2.8 Wallet-Backed Energy Domain (`wallet_budget.rs`, `wallet_energy_estimator.rs`, `wallet_gas_calibrator.rs`)

**Motivating principle:** P9 (Homeostatic Self-Regulation) + P4 (Clear Boundaries) — wallet-backed energy budgets convert gas costs to rJoules and debit a real wallet. The `gas_per_rjoule` conversion rate is calibrated at runtime from aggregate settled gas events.

**Source:**
- `crates/hkask-cns/src/wallet_budget.rs` (213 lines, 8 tests)
- `crates/hkask-cns/src/wallet_energy_estimator.rs` (114 lines, 5 tests)
- `crates/hkask-cns/src/wallet_gas_calibrator.rs` (185 lines, 5 tests)

| FR | Requirement | Contract |
|----|-----------|----------|
| `FR-WALLET-CALIB-001` | Wallet budget — gas-to-rJoule conversion and encumbrance debit | `cns-wallet-budget-001` … `cns-wallet-budget-008` |
| `FR-WALLET-CALIB-002` | Wallet estimator — EMA calibration of gas→rJoule rate | `P9-cns-wallet-est-calibrate` |
| `FR-WALLET-CALIB-003` | Wallet calibrator — runtime calibration loop pushing rate to `WalletManager` | `GAS-CALIB-005` |

**New CNS spans:** `cns.wallet.conversion.calibrated` — emitted when `WalletGasCalibrator` adjusts the wallet gas→rJoule rate.

---

## 3. Verification

```bash
# Verify CNS crate compiles and tests pass:
cargo check -p hkask-cns
cargo test -p hkask-cns

# CNS span health:
kask cns health
```

---

## Memory Verb Contracts

> **Incorporated from:** `docs/architecture/core/CNS_MEMORY_VERB_CONTRACTS.md`

13 CNS `MemoryEncode` NuEvents emitted by autonomous memory operations:

| # | Verb | Source | Trigger |
|---|------|--------|---------|
| 1 | `episodic_stored` | `episodic.rs` | `store()` succeeds |
| 2 | `semantic_stored` | `semantic.rs` | `store()` succeeds |
| 3 | `consolidated` | `semantic.rs` | `store_consolidated()` succeeds |
| 4 | `episodic_consolidated` | `episodic_loop.rs` | Consolidation bridge, outcome > 0 |
| 5 | `episodic_consolidation_failed` | `episodic_loop.rs` | Bridge returns Err |
| 6 | `episodic_budget_exceeded_no_bridge` | `episodic_loop.rs` | Budget exceeded, no bridge |
| 7 | `episodic_calibrate` | `episodic_loop.rs` | Non-budget calibration action |
| 8 | `episodic_regulate` | `episodic_loop.rs` | Unhandled regulatory action |
| 9 | `confidence_decayed` | `episodic.rs` | Wozniak-Gorzelanczyk forgetting curve R(t)=exp(-t/S) applied at recall |
| 10 | `semantic_condensed` | `semantic_loop.rs` | Old triples condensed |
| 11 | `semantic_budget_enforced` | `semantic_loop.rs` | Low-confidence review deletes |
| 12 | `consolidation_completed` | `consolidation.rs` | Bridge finishes |
| 13 | `consolidation_service_completed` | `consolidation_service.rs` | Service finishes all 3 phases |

### Signal Metrics

The Episodic Loop (2a) emits `cns.memory.life` carrying memory life S in days
(Wozniak-Gorzelanczyk, 1995). Default 180 days. Configurable via
`HKASK_MEMORY_LIFE_DAYS` env var or `ServiceConfig.memory_life_days`.

---

*ℏKask - A Minimal Viable Container for Agents — v0.28.0*