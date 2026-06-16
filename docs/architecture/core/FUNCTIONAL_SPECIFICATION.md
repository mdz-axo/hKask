# hKask Functional Specification

**Version:** v0.27.0
**Created:** 2026-06-16
**Status:** Active — anchor for the rSolidity contract vocabulary
**Last Updated:** 2026-06-16

> This document maps the complete system to its motivating principles, enumerates functional requirements per domain, and links each requirement to the contracts that implement it. It serves as the specification anchor from which the rSolidity contract vocabulary will be derived.

---

## 1. Domain Breakdown

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
| 9 | Cybernetics Loop | `loop` | hkask-cns | 1 | P9 (Homeostatic Self-Regulation) |
| 10 | Wallet | `wallet` | hkask-wallet | 23 | P9 (Homeostatic Self-Regulation) |
| 11 | Storage | `storage` | hkask-storage | 12 | P3 (Generative Space) |
| 12 | Memory | `memory` | hkask-memory | 52 | P3 (Generative Space) |
| 13 | Inference Engine | `inference` | hkask-inference | 15 | P9 + P4 (Homeostatic + Boundary) |
| 14 | Template Engine | `templates` | hkask-templates | 10 | P3 (Generative Space) |
| 15 | MCP Servers | `mcp` | mcp-servers/ | 18 | P5 (Essentialism) |
| 16 | Service Layer | `services` | hkask-services | 14 | P5 + P7 (Essentialism + Evolution) |
| 17 | Agent Runtime | `agents` | hkask-agents | 30 | P1 (User Sovereignty) |
| 18 | Communication | `comm` | hkask-comm | 6 | P1 (User Sovereignty) |
| 19 | Keystore | `keystore` | hkask-keystore | 5 | P1 (User Sovereignty) |
| 20 | Type System | `types` | hkask-types | 40 | P8 (Semantic Grounding) |
| 21 | API Surface | `api` | hkask-api | 25 | P1 + P4 (Sovereignty + Boundaries) |
| 22 | CLI Surface | `cli` | kask | 12 | P3 (Generative Space) |

### Domain Ownership Rules

Each contract carries a **motivating principle** in its ID prefix and **constraining principles** in its body annotations.

1. **P9 (Homeostatic Self-Regulation)** owns all CNS regulation-loop contracts: energy, algedonic, runtime, circuit breaker, API metering, energy estimation
2. **P4 (Clear Boundaries)** owns all membrane/boundary contracts: governed_tool, governed_inference
3. **P8 (Semantic Grounding)** owns all type-level identity contracts: `EnergyCost`, `EnergyDelta` newtypes
4. **P12 (Subscriber Consent)** owns all subscriber/consent contracts: `subscribe`, `subscribe_async`
5. **P3 (Generative Space)** owns all sync/blocking variants and content-domain contracts: blocking accessors, storage, memory, CLI
6. **P7 (Evolutionary Architecture)** owns all configurable-from-real-usage contracts: threshold calibration, replenish rate tuning

A contract may have **one motivating principle** and **multiple constraining principles**. The motivating principle determines the ID prefix (`P{N}`). Constraining principles appear as `[P{N}]` annotations in the contract body.

---

## 2. Functional Requirements by Domain

### 2.1 Energy Budgeting (`energy`)

**Motivating Principle:** P9 (Homeostatic Self-Regulation) — gas budget enforcement prevents runaway agents
**Constraining Principle:** P8 (Semantic Grounding) — type-level identity for energy cost types
**Crate:** `hkask-cns` | **Source:** `src/energy.rs`

#### Production Contracts (16)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-E1 | `P8-cns-energy-cost-from-raw` | `EnergyCost::from_raw(u64) -> Self` | [P8] Motivating: Semantic Grounding — type-level identity preservation; [P5] Constraining: Essentialism |
| FR-E2 | `P8-cns-energy-cost-as-raw` | `EnergyCost::as_raw() -> u64` | [P8] Motivating: Semantic Grounding — symmetric type-level identity; [P5] Constraining: Essentialism |
| FR-E3 | `P8-cns-energy-delta-from-raw` | `EnergyDelta::from_raw(f64) -> Self` | [P8] Motivating: Semantic Grounding — type-level identity for f64 newtype; [P5] Constraining: Essentialism |
| FR-E4 | `P8-cns-energy-delta-as-raw` | `EnergyDelta::as_raw() -> f64` | [P8] Motivating: Semantic Grounding — symmetric type-level identity; [P5] Constraining: Essentialism |
| FR-E5 | `P9-cns-energy-delta-descending` | `EnergyDelta::is_descending() -> bool` | [P9] Motivating: Homeostatic Self-Regulation — lazy universe compliance detection; [P8] Constraining: Semantic Grounding |
| FR-E6 | `P9-cns-energy-delta-ascending` | `EnergyDelta::is_ascending() -> bool` | [P9] Motivating: Homeostatic Self-Regulation — anti-lazy detection triggers alert; [P8] Constraining: Semantic Grounding |
| FR-E7 | `P9-cns-energy-budget-new` | `EnergyBudget::new(cap) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — budget creation enables regulation; [P4] Constraining: Clear Boundaries — cap enforces OCAP boundary |
| FR-E8 | `P9-cns-energy-budget-unlimited` | `EnergyBudget::unlimited() -> Self` | [P9] Motivating: Homeostatic Self-Regulation — observability without throttling; [P4] Constraining: Clear Boundaries |
| FR-E9 | `P9-cns-energy-budget-with-replenish-rate` | `EnergyBudget::with_replenish_rate(rate) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — configurable replenishment knob; [P7] Constraining: Evolutionary Architecture — emerged from real usage |
| FR-E10 | `P9-cns-energy-budget-with-alert-threshold` | `EnergyBudget::with_alert_threshold(threshold) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — configurable alert threshold; [P7] Constraining: Evolutionary Architecture |
| FR-E11 | `P9-cns-energy-budget-with-hard-limit` | `EnergyBudget::with_hard_limit(hard) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — boundary enforcement toggle; [P4] Constraining: Clear Boundaries |
| FR-E12 | `P9-cns-energy-budget-can-proceed` | `EnergyBudget::can_proceed(gas) -> bool` | [P9] Motivating: Homeostatic Self-Regulation — check-before-execute gateway; [P4] Constraining: Clear Boundaries |
| FR-E13 | `P9-cns-energy-budget-available` | `EnergyBudget::available() -> EnergyCost` | [P9] Motivating: Homeostatic Self-Regulation — visible state for feedback loops; [P4] Constraining: Clear Boundaries |
| FR-E14 | `P9-cns-energy-budget-reserve` | `EnergyBudget::reserve(gas) -> Result` | [P9] Motivating: Homeostatic Self-Regulation — hold-settle pattern; [P4] Constraining: Clear Boundaries |
| FR-E15 | `P9-cns-energy-budget-settle` | `EnergyBudget::settle(reserved, actual) -> Result` | [P9] Motivating: Homeostatic Self-Regulation — completes hold-settle cycle; [P4] Constraining: Clear Boundaries |
| FR-E16 | `P9-cns-energy-budget-consume` | `EnergyBudget::consume(gas) -> Result` | [P9] Motivating: Homeostatic Self-Regulation — immediate deduction path; [P4] Constraining: Clear Boundaries |
| FR-E17 | `P9-cns-energy-budget-replenish` | `EnergyBudget::replenish()` | [P9] Motivating: Homeostatic Self-Regulation — regulation cycle; [P4] Constraining: Clear Boundaries |
| FR-E18 | `P9-cns-energy-budget-replenish-by` | `EnergyBudget::replenish_by(amount)` | [P9] Motivating: Homeostatic Self-Regulation — targeted curation replenishment; [P4] Constraining: Clear Boundaries |
| FR-E19 | `P9-cns-energy-budget-replenish-by-weighted` | `EnergyBudget::replenish_by_weighted(amount, prio) -> EnergyCost` | [P9] Motivating: Homeostatic Self-Regulation — priority-weighted replenishment; [P4] + [P7] Constraining |

#### Test Contracts (4)

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-E-T1 | `P9-cns-energy-budget-invariant-test` | budget_never_exceeds_cap — property test: remaining + reserved ≤ cap |
| FR-E-T2 | `P9-cns-energy-budget-available-test` | available_never_negative — property test: available ≥ 0 |
| FR-E-T3 | `P9-cns-energy-budget-replenish-test` | replenish_never_exceeds_cap — property test: remaining ≤ cap after replenish |
| FR-E-T4 | (included above) | `EnergyCost` newtype contract test |


### 2.2 Algedonic Signalling (`algedonic`)

**Motivating Principle:** P9 (Homeostatic Self-Regulation) — algedonic feedback loop for variety deficit escalation
**Constraining Principle:** P4 (Clear Boundaries) — cap enforcement through binary classification
**Crate:** `hkask-cns` | **Source:** `src/algedonic.rs`

#### Production Contracts (4)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-A1 | `P9-cns-algedonic-alert-new` | `RuntimeAlert::new(domain, deficit, threshold) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — alert construction for feedback; [P4] Constraining: Clear Boundaries |
| FR-A2 | `P9-cns-algedonic-alert-should-escalate` | `RuntimeAlert::should_escalate() -> bool` | [P9] Motivating: Homeostatic Self-Regulation — escalation feedback loop; [P4] Constraining: Clear Boundaries |
| FR-A3 | `P9-cns-algedonic-alert-is-critical` | `RuntimeAlert::is_critical() -> bool` | [P9] Motivating: Homeostatic Self-Regulation — critical threshold detection; [P4] Constraining: Clear Boundaries |
| FR-A4 | `P9-cns-algedonic-alert-is-warning` | `RuntimeAlert::is_warning() -> bool` | [P9] Motivating: Homeostatic Self-Regulation — warning threshold detection; [P4] Constraining: Clear Boundaries |

#### Test Contracts (5)

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-A-T1 | `P9-cns-algedonic-binary-threshold-test` | binary_threshold_classifies_critical_and_warning |
| FR-A-T2 | `P9-cns-algedonic-accumulation-test` | algedonic_manager_accumulates_alerts_across_domains |
| FR-A-T3 | `P9-cns-outcome-classify-test` | check_outcome_classifies_success_rate_correctly |
| FR-A-T4 | `P9-cns-outcome-message-test` | check_outcome_alert_message_includes_domain_and_rate |
| FR-A-T5 | `P9-cns-outcome-prefix-test` | check_outcome_domain_prefixed_with_outcome |


### 2.3 Runtime Observability (`runtime`)

**Motivating Principle:** P9 (Homeostatic Self-Regulation) — single entry point for CNS observability and regulation
**Constraining Principles:** P3 (Generative Space — sync variants), P7 (Evolutionary Architecture — calibrate), P12 (Affirmative Consent — subscribe)
**Crate:** `hkask-cns` | **Source:** `src/runtime.rs`

#### P9 Production Contracts (18)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-R1 | `P9-cns-runtime-variety-monitor-new` | `VarietyMonitor::new() -> Self` | [P9] Motivating: Homeostatic Self-Regulation — monitor enables feedback loops; [P5] Constraining: Essentialism |
| FR-R2 | `P9-cns-runtime-variety-for-domain` | `VarietyMonitor::variety_for_domain(domain) -> u64` | [P9] Motivating: Homeostatic Self-Regulation — variety measurement drives loop closure; [P8] Constraining: Semantic Grounding |
| FR-R3 | `P9-cns-runtime-variety-monitor-domains` | `VarietyMonitor::domains() -> Vec<&str>` | [P9] Motivating: Homeostatic Self-Regulation — domain enumeration enables loop feedback; [P8] Constraining: Semantic Grounding |
| FR-R4 | `P9-cns-runtime-with-threshold` | `CnsRuntime::with_threshold(threshold) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — runtime creation enables regulation; [P7] Constraining: Evolutionary Architecture |
| FR-R5 | `P9-cns-runtime-health` | `CnsRuntime::health() -> CnsHealth` | [P9] Motivating: Homeostatic Self-Regulation — health query drives loop decisions; [P8] Constraining: Semantic Grounding |
| FR-R6 | `P9-cns-runtime-alerts` | `CnsRuntime::alerts() -> Vec<RuntimeAlert>` | [P9] Motivating: Homeostatic Self-Regulation — alert retrieval enables loop response; [P8] Constraining: Semantic Grounding |
| FR-R7 | `P9-cns-runtime-default-threshold` | `CnsRuntime::default_threshold() -> u64` | [P9] Motivating: Homeostatic Self-Regulation — threshold config enables loop tuning; [P7] Constraining: Evolutionary Architecture |
| FR-R8 | `P9-cns-runtime-critical-alerts` | `CnsRuntime::critical_alerts() -> Vec<RuntimeAlert>` | [P9] Motivating: Homeostatic Self-Regulation — critical alert filtering enables prioritised response; [P8] Constraining: Semantic Grounding |
| FR-R9 | `P9-cns-runtime-variety` | `CnsRuntime::variety() -> HashMap<SpanNamespace, u64>` | [P9] Motivating: Homeostatic Self-Regulation — variety measurement drives loop closure; [P8] Constraining: Semantic Grounding |
| FR-R10 | `P9-cns-runtime-variety-for-domain` | `CnsRuntime::variety_for_domain(domain) -> u64` | [P9] Motivating: Homeostatic Self-Regulation — domain-specific variety; [P8] Constraining: Semantic Grounding |
| FR-R11 | `P9-cns-runtime-record-outcome` | `CnsRuntime::record_outcome(domain, success, err) -> ()` | [P9] Motivating: Homeostatic Self-Regulation — outcome tracking enables quality-based regulation; [P4] Constraining: Clear Boundaries |
| FR-R12 | `P9-cns-runtime-check-outcome` | `CnsRuntime::check_outcome(domain) -> Option<RuntimeAlert>` | [P9] Motivating: Homeostatic Self-Regulation — outcome check drives loop decisions; [P4] Constraining: Clear Boundaries |
| FR-R13 | `P9-cns-runtime-outcome-success-rate` | `CnsRuntime::outcome_success_rate(domain) -> Option<f64>` | [P9] Motivating: Homeostatic Self-Regulation — success rate is a feedback metric; [P8] Constraining: Semantic Grounding |
| FR-R14 | `P9-cns-runtime-increment-variety` | `CnsRuntime::increment_variety(domain, state_name)` | [P9] Motivating: Homeostatic Self-Regulation — variety counter drives loop closure; [P4] Constraining: Clear Boundaries |
| FR-R15 | `P9-cns-runtime-check-variety` | `CnsRuntime::check_variety(domain) -> Option<RuntimeAlert>` | [P9] Motivating: Homeostatic Self-Regulation — variety check drives loop closure; [P4] Constraining: Clear Boundaries |
| FR-R16 | `P9-cns-runtime-register-energy-budget` | `CnsRuntime::register_energy_budget(agent, budget)` | [P9] Motivating: Homeostatic Self-Regulation — budget registration enables energy tracking; [P4] Constraining: Clear Boundaries |
| FR-R17 | `P9-cns-runtime-replenish-agent-budget` | `CnsRuntime::replenish_agent_budget(agent, amount) -> EnergyCost` | [P9] Motivating: Homeostatic Self-Regulation — budget replenishment drives energy loop; [P4] Constraining: Clear Boundaries |
| FR-R18 | `P9-cns-runtime-agent-gas-status` | `CnsRuntime::agent_gas_status(agent) -> Option<AgentEnergyStatus>` | [P9] Motivating: Homeostatic Self-Regulation — gas status query drives energy loop; [P8] Constraining: Semantic Grounding |

#### P3 Blocking Variants (1)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-R19 | `P3-cns-runtime-blocking-variety-for-domain` | `CnsRuntime::blocking_variety_for_domain(domain) -> u64` | [P3] Motivating: Generative Space — sync access preserves generative capability; [P7] Constraining: Evolutionary Architecture — blocking variant emerged from real usage; [P4] Constraining: Clear Boundaries — must not be called from async context |


#### P7 Calibrate & P3 Blocking Variants (2)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-R20 | `P7-cns-runtime-calibrate-threshold` | `CnsRuntime::calibrate_threshold(domain, new_threshold)` | [P7] Motivating: Evolutionary Architecture — threshold parameter emerged from real usage; [P4] Constraining: Clear Boundaries |
| FR-R21 | `P3-cns-runtime-calibrate-threshold-blocking` | `CnsRuntime::calibrate_threshold_blocking(domain, new_threshold)` | [P3] Motivating: Generative Space — sync access preserves generative capability; [P7] Constraining: Evolutionary Architecture — blocking variant emerged from real usage; [P4] Constraining: Clear Boundaries |

#### P12 Subscriber Contracts (3)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-R22 | `P12-cns-runtime-subscribe` | `CnsRuntime::subscribe(observer: Arc<dyn CnsObserver>)` | [P12] Motivating: Affirmative Consent — observer registration requires explicit subscription; [P2] Constraining: User Sovereignty |
| FR-R23 | `P12-cns-runtime-subscribe-async` | `CnsRuntime::subscribe_async(observer: Arc<dyn CnsObserver>)` | [P12] Motivating: Affirmative Consent — observer registration requires explicit subscription; [P2] Constraining: User Sovereignty |
| FR-R24 | `P9-cns-runtime-emit-backpressure` | `CnsRuntime::emit_backpressure(signal: BackpressureSignal)` | [P9] Motivating: Homeostatic Self-Regulation — backpressure signal closes the regulation loop; [P4] Constraining: Clear Boundaries |

#### Test Contracts (6)

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-R-T1 | `P9-cns-runtime-variety-monitor-test-001` | variety_monitor_tracks_distinct_states |
| FR-R-T2 | `P9-cns-runtime-variety-deficit-test-002` | variety_tracker_deficit_calculation |
| FR-R-T3 | `P9-cns-runtime-variety-isolation-test-003` | variety_monitor_multi_domain_isolation |
| FR-R-T4 | `P9-cns-runtime-outcome-rate-test-004` | outcome_tracker_success_rate_calculation |
| FR-R-T5 | `P9-cns-runtime-outcome-breakdown-test-005` | outcome_tracker_error_kind_breakdown |
| FR-R-T6 | `P9-cns-runtime-outcome-window-test-006` | outcome_tracker_window_reset |


### 2.4 Tool Governance (`gov-tool`)

**Motivating Principles:** P9 (Homeostatic Self-Regulation) + P4 (Clear Boundaries — OCAP enforcement)
**Constraining Principle:** P12 (Affirmative Consent — agent identity is the consent anchor)
**Crate:** `hkask-cns` | **Source:** `src/governed_tool.rs`

#### Production Contracts (3)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-GT1 | `P9-cns-gov-tool-new` | `GovernedTool::new(inner, cybernetics, sink, est, agent) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — tool governance enables feedback loops; [P4] Constraining: Clear Boundaries — cybernetics binding enforces OCAP boundary |
| FR-GT2 | `P9-cns-gov-tool-consumption-channel` | `GovernedTool::with_tool_consumption_channel(tx) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — consumption channel closes cybernetic feedback loop; [P4] Constraining: Clear Boundaries — channel ownership tracks consumer identity |
| FR-GT3 | `P12-cns-gov-tool-with-agent` | `GovernedTool::with_agent(agent) -> Self` | [P12] Motivating: Affirmative Consent — agent identity is the consent anchor; [P4] Constraining: Clear Boundaries — OCAP gate enforces boundary per invocation |

#### Test Contracts (4)

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-GT-T1 | `P9-cns-gov-tool-legacy-exact-match-test` | legacy_exact_match_grants_correct_tool — OCAP Path 1 |
| FR-GT-T2 | `P9-cns-gov-tool-legacy-denies-test` | legacy_exact_match_denies_wrong_tool — OCAP Path 1 denial |
| FR-GT-T3 | `P9-cns-gov-tool-domain-capability-test` | domain_capability_matches_mcp_tool_domain — OCAP Path 2 |
| FR-GT-T4 | `P9-cns-gov-tool-domain-denies-test` | domain_capability_denies_different_domain — OCAP Path 2 denial |


### 2.5 Inference Governance (`gov-inf`)

**Motivating Principles:** P9 (Homeostatic Self-Regulation) + P4 (Clear Boundaries — membrane for inference)
**Constraining Principle:** P12 (Affirmative Consent — agent identity is required for attribution)
**Crate:** `hkask-cns` | **Source:** `src/governed_inference.rs`

#### Production Contracts (2)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-GI1 | `P9-cns-gov-inf-new` | `GovernedInference::new(inner, cybernetics, sink, agent) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — inference governance enables cybernetic control; [P4] Constraining: Clear Boundaries — membrane wraps inner InferencePort at OCAP boundary; [P12] Constraining: Affirmative Consent |
| FR-GI2 | `P12-cns-gov-inf-with-agent` | `GovernedInference::with_agent(agent) -> Self` | [P12] Motivating: Affirmative Consent — agent identity is the consent anchor; [P4] Constraining: Clear Boundaries — OCAP gate enforces boundary per inference call |

#### Test Contracts (2)

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-GI-T1 | `P9-cns-gov-inf-est-cost-max-tokens` | estimate_inference_cost_uses_max_tokens — cost estimation uses max_tokens|
| FR-GI-T2 | `P9-cns-gov-inf-est-cost-floors-at-one` | estimate_inference_cost_floors_at_one — cost estimation floors at 1 |


### 2.6 Circuit Breaker (`circuit`)

**Motivating Principle:** P9 (Homeostatic Self-Regulation) — CNS regulation loop enforces homeostasis over external service calls
**Constraining Principle:** P4 (Clear Boundaries) — circuit state transitions are boundary conditions
**Crate:** `hkask-cns` | **Source:** `src/circuit_breaker.rs`

#### Production Contracts (3)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-CB1 | `P9-cns-circuit-default-for-inference` | `CircuitBreaker::default_for_inference(name) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — CNS regulation loop enforces boundary; [P4] Constraining: Clear Boundaries — default thresholds establish failure boundary |
| FR-CB2 | `P9-cns-circuit-allow-request` | `CircuitBreaker::allow_request() -> bool` | [P9] Motivating: Homeostatic Self-Regulation — check-before-execute gateway; [P4] Constraining: Clear Boundaries — state-driven gating enforces the boundary |
| FR-CB3 | `P9-cns-circuit-record-success` | `CircuitBreaker::record_success()` | [P9] Motivating: Homeostatic Self-Regulation — success count drives loop closure; [P4] Constraining: Clear Boundaries — threshold-based state transition enforces boundary |


### 2.7 API Metering (`api`)

**Motivating Principle:** P9 (Homeostatic Self-Regulation) — per-key rate limiting, gas tracking, and CNS spans
**Constraining Principles:** P7 (Evolutionary Architecture — hardcoded endpoint weight table, configurable later), P4 (Clear Boundaries — rate limit thresholds are boundary conditions)
**Crate:** `hkask-cns` | **Source:** `src/api_metering.rs`

#### Production Contracts (8)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-AM1 | `P9-cns-api-meter-endpoint-weight` | `endpoint_weight(path) -> EndpointWeight` | [P9] Motivating: Homeostatic Self-Regulation — per-request rate limiting for API stability; [P7] Constraining: Evolutionary Architecture — hardcoded table to be configurable later |
| FR-AM2 | `P9-cns-api-meter-rate-limit-status` | `RateLimitStatus::as_str() -> &'static str` | [P9] Motivating: Homeostatic Self-Regulation — rate limit status feedback for CNS; [P8] Constraining: Semantic Grounding — string representation must be stable across versions |
| FR-AM3 | `P9-cns-api-meter-new` | `ApiMeter::new() -> Self` | [P9] Motivating: Homeostatic Self-Regulation — empty meter ready for per-key tracking; [P5] Constraining: Essentialism — minimal constructor with empty buckets map |
| FR-AM4 | `P9-cns-api-meter-check-and-record` | `ApiMeter::check_and_record(key_id, max_rpm, max_tokens, tokens) -> RateLimitStatus` | [P9] Motivating: Homeostatic Self-Regulation — rate limit enforcement is the CNS check; [P4] Constraining: Clear Boundaries — rate limit thresholds are boundary conditions |
| FR-AM5 | `P9-cns-api-meter-current-rpm` | `ApiMeter::current_rpm(key_id) -> u32` | [P9] Motivating: Homeostatic Self-Regulation — current rate is the cybernetic state; [P8] Constraining: Semantic Grounding — RPM count must be stable and accurate |
| FR-AM6 | `P9-cns-api-meter-span-new` | `ApiRequestSpan::new(key_id, endpoint, matched, gas, enc, status) -> Self` | [P9] Motivating: Homeostatic Self-Regulation — span creation is the CNS observation layer; [P8] Constraining: Semantic Grounding — span fields must be traceable to source |
| FR-AM7 | `P9-cns-api-meter-alert-type` | `ApiMeteringAlert::alert_type() -> &'static str` | [P9] Motivating: Homeostatic Self-Regulation — alert type is the CNS classification; [P8] Constraining: Semantic Grounding — alert type labels must be stable across versions |
| FR-AM8 | `P9-cns-api-meter-alert-severity` | `ApiMeteringAlert::severity() -> &'static str` | [P9] Motivating: Homeostatic Self-Regulation — severity is the algedonic signal; [P8] Constraining: Semantic Grounding — severity labels must be stable across versions |

#### Test Contracts (8)

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-AM-T1 | `P9-cns-api-meter-endpoint-weight` | endpoint_weight_embed_corpus_is_heavy |
| FR-AM-T2 | `P9-cns-api-meter-endpoint-weight` | endpoint_weight_default_is_one |
| FR-AM-T3 | `P9-cns-api-meter-check-and-record` | rate_limit_bucket_prunes_old_requests |
| FR-AM-T4 | `P9-cns-api-meter-check-and-record` | rate_limit_bucket_enforces_rpm |
| FR-AM-T5 | `P9-cns-api-meter-check-and-record` | token_tracking_resets_on_new_day |
| FR-AM-T6 | `P9-cns-api-meter-check-and-record` | api_meter_enforces_limits |
| FR-AM-T7 | `P9-cns-api-meter-span-new` | api_request_span_serialization |
| FR-AM-T8 | `P9-cns-api-meter-alert-severity` | alert_severity_levels |


### 2.8 Energy Estimation (`est`)

**Motivating Principle:** P9 (Homeostatic Self-Regulation) — composite estimator routes inference and table estimation
**Crate:** `hkask-cns` | **Source:** `src/composite_energy_estimator.rs`, `src/wallet_energy_estimator.rs`

#### Production Contracts (2)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-EE1 | `P9-cns-est-composite-new` | `CompositeEnergyEstimator::new() -> Self` | [P9] Motivating: Homeostatic Self-Regulation — composite estimator enables feedback loops; [P5] Constraining: Essentialism — minimal constructor, empty estimators |
| FR-EE2 | `P9-cns-wallet-est-calibrate` | `WalletEnergyEstimator::calibrate(observed_ratio) -> bool` | [P9] Motivating: Homeostatic Self-Regulation — Good Regulator feedback loop closure; [P4] Constraining: Clear Boundaries — threshold tolerance enforces boundary; [P7] Constraining: Evolutionary Architecture — EMA parameters emerged from real usage |

#### Test Contracts (5)

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-EE-T1 | `P9-cns-est-wallet-001` | calibrate_first_observation_initializes_EMA |
| FR-EE-T2 | `P9-cns-est-wallet-002` | calibrate_within_tolerance_no_adjustment |
| FR-EE-T3 | `P9-cns-est-wallet-003` | calibrate_EMA_smooths_observations |
| FR-EE-T4 | `P9-cns-est-wallet-004` | calibrate_clamps_extreme_ratios |
| FR-EE-T5 | `P9-cns-est-wallet-005` | calibrate_floors_gas_per_rjoule_at_one |

---

## 3. Non-CNS Domain Stubs

These domains are documented here for completeness but are not part of the CNS contract realignment. Their contracts will be realigned in subsequent work packages.

### 3.1 Wallet (`hkask-wallet`)

**Motivating Principle:** P9 (Homeostatic Self-Regulation) — rJoule balance, encumbrance, and fee estimation form the wallet's energy regulation loop
**Constraining Principles:** P1 (User Sovereignty), P2 (Affirmative Consent), P4 (Clear Boundaries), P8 (Semantic Grounding)
**Crate:** `hkask-wallet`
**Sources:** `src/manager.rs`, `src/issuer.rs`, `src/signing.rs`, `src/hinkal.rs`, `src/price_feed.rs`, `src/hedera.rs`, `src/solana.rs`, `tests/hinkal_adapter.rs`

#### Production Contracts (27)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-W1 | `P9-wallet-mgr-struct` | `WalletManager` struct | [P9] Motivating: Homeostatic Self-Regulation — wallet is the energy regulation anchor; [P1] Constraining: User Sovereignty — wallet_seed is user-owned and zeroized |
| FR-W2 | `P9-wallet-mgr-build` | `WalletManager::build(...)` | [P9] Motivating: Homeostatic Self-Regulation — wallet construction; [P1] Constraining: User Sovereignty — wallet_seed resolved and zeroized |
| FR-W3 | `P9-wallet-mgr-balance` | `WalletManager::get_balance(wallet_id)` | [P9] Motivating: Homeostatic Self-Regulation — balance is the cybernetic state; [P8] Constraining: Semantic Grounding — gas/USDC equivalents derive deterministically |
| FR-W4 | `P9-wallet-mgr-api-key-get` | `WalletManager::get_api_key(key_id)` | [P9] Motivating: Homeostatic Self-Regulation — API key health state for feedback loops; [P4] Constraining: Clear Boundaries — revoked keys are excluded |
| FR-W5 | `P9-wallet-mgr-chain-error-span` | `WalletManager::emit_chain_error_for_actor` | [P9] Motivating: Homeostatic Self-Regulation — chain errors feed the CNS sense loop; [P12] Constraining: Subscriber Consent — actor identity is recorded |
| FR-W6 | `P9-wallet-mgr-can-afford` | `WalletManager::can_afford(wallet_id, cost_rj)` | [P9] Motivating: Homeostatic Self-Regulation — optimistic hold-settle prevents overspend; [P4] Constraining: Clear Boundaries — cannot reserve beyond balance |
| FR-W7 | `P9-wallet-mgr-reserve` | `WalletManager::reserve_rjoules(wallet_id, amount)` | [P9] Motivating: Homeostatic Self-Regulation — optimistic hold-settle prevents overspend; [P4] Constraining: Clear Boundaries — cannot reserve beyond balance |
| FR-W8 | `P9-wallet-mgr-settle` | `WalletManager::settle_rjoules(wallet_id, reserved, actual)` | [P9] Motivating: Homeostatic Self-Regulation — optimistic hold-settle prevents overspend; [P4] Constraining: Clear Boundaries — cannot reserve beyond balance |
| FR-W9 | `P9-wallet-mgr-encumber` | `WalletManager::encumber(wallet_id, key_id, amount)` | [P9] Motivating: Homeostatic Self-Regulation — encumbrance locks energy for API keys; [P4] Constraining: Clear Boundaries — only the entitled key can consume; [P8] Constraining: Semantic Grounding — atomic consume/release preserves balance |
| FR-W10 | `P9-wallet-mgr-release-encumbrance` | `WalletManager::release_encumbrance(key_id)` | [P9] Motivating: Homeostatic Self-Regulation — encumbrance locks energy for API keys; [P4] Constraining: Clear Boundaries — only the entitled key can consume; [P8] Constraining: Semantic Grounding — atomic consume/release preserves balance |
| FR-W11 | `P9-wallet-mgr-consume` | `WalletManager::consume(key_id, gas_rj)` | [P9] Motivating: Homeostatic Self-Regulation — encumbrance locks energy for API keys; [P4] Constraining: Clear Boundaries — only the entitled key can consume; [P8] Constraining: Semantic Grounding — atomic consume/release preserves balance |
| FR-W12 | `P9-wallet-mgr-get-encumbrance` | `WalletManager::get_encumbrance(key_id)` | [P9] Motivating: Homeostatic Self-Regulation — encumbrance locks energy for API keys; [P4] Constraining: Clear Boundaries — only the entitled key can consume; [P8] Constraining: Semantic Grounding — atomic consume/release preserves balance |
| FR-W13 | `P9-wallet-mgr-fee-estimate` | `WalletManager::estimate_withdrawal_fee` | [P9] Motivating: Homeostatic Self-Regulation — fee estimate enables cost-aware withdrawal; [P8] Constraining: Semantic Grounding — derived from live/native USD rate |
| FR-W14 | `P9-wallet-mgr-key-alert-span` | `WalletManager::emit_key_alert` | [P9] Motivating: Homeostatic Self-Regulation — algedonic feedback closure for API key lifecycle; [P12] Constraining: Subscriber Consent — emits span only if sink subscribed |
| FR-W15 | `P9-wallet-mgr-deposit-ref-nonce` | `WalletManager::generate_deposit_reference` HKDF context | [P9] Motivating: Homeostatic Self-Regulation — deposit attribution supports energy inflow; [P4] Constraining: Clear Boundaries — nonce binds reference to specific invocation |
| FR-W16 | `P9-wallet-issuer-struct` | `ApiKeyIssuer` struct | [P9] Motivating: Homeostatic Self-Regulation — API keys scope and limit agent energy access; [P2] Constraining: Affirmative Consent — keys are explicitly scoped, revocable, and user-issued; [P4] Constraining: Clear Boundaries — spending limits and expiry enforce capability boundaries; [P1] Constraining: User Sovereignty — private keys are returned once and never stored |
| FR-W17 | `P9-wallet-issuer-new` | `ApiKeyIssuer::new(store)` | [P9] Motivating: Homeostatic Self-Regulation — API keys scope and limit agent energy access; [P1] Constraining: User Sovereignty — wallet_seed resolved and zeroized |
| FR-W18 | `P9-wallet-issuer-create-key` | `ApiKeyIssuer::create_key(...)` | [P9] Motivating: Homeostatic Self-Regulation — API keys scope and limit agent energy access; [P2] Constraining: Affirmative Consent — keys are explicitly scoped, revocable, and user-issued; [P4] Constraining: Clear Boundaries — spending limits and expiry enforce capability boundaries; [P1] Constraining: User Sovereignty — private keys are returned once and never stored |
| FR-W19 | `P9-wallet-issuer-revoke-key` | `ApiKeyIssuer::revoke_key(key_id)` | [P9] Motivating: Homeostatic Self-Regulation — API keys scope and limit agent energy access; [P2] Constraining: Affirmative Consent — revocable capabilities; [P1] Constraining: User Sovereignty — unspent balance returned |
| FR-W20 | `P9-wallet-issuer-list-keys` | `ApiKeyIssuer::list_keys(wallet_id)` | [P9] Motivating: Homeostatic Self-Regulation — API key inventory for feedback loops; [P4] Constraining: Clear Boundaries — only active keys returned |
| FR-W21 | `P9-wallet-issuer-zeroize-seed` | `ApiKeyIssuer::create_key` key generation | [P1] Constraining: User Sovereignty — Ed25519 seed wrapped in Zeroizing for automatic zeroize on drop |
| FR-W22 | `P9-wallet-sign-withdrawal` | `sign_withdrawal(chain, tx_bytes)` | [P9] Motivating: Homeostatic Self-Regulation — signing authorizes energy outflow; [P1] Constraining: User Sovereignty — treasury key derived from user master key; [P4] Constraining: Clear Boundaries — key material never leaves this module |
| FR-W23 | `P9-wallet-sign-hinkal-message` | `sign_message(message)` | [P9] Motivating: Homeostatic Self-Regulation — Hinkal session signing authorizes privacy-layer flow; [P4] Constraining: Clear Boundaries — message is opaque bytes; signature proves treasury origin |
| FR-W24 | `P9-wallet-sign-capability` | `sign_capability(capability)` | [P9] Motivating: Homeostatic Self-Regulation — signing authorizes API key capability; [P1] Constraining: User Sovereignty — treasury key derived from user master key; [P4] Constraining: Clear Boundaries — key material never leaves this module |
| FR-W25 | `P2-wallet-signing-debug-redact` | `LoadedKey` Debug impl | [P2] Constraining: Affirmative Consent — key material redacted from debug output |
| FR-W26 | `P2-wallet-signing-key-boundary` | `LoadedKey` never leaves `signing.rs` | [P2] Constraining: Affirmative Consent — no un-zeroized key material crosses module boundary |
| FR-W27 | `P9-wallet-hinkal-port-new` | `HinkalPort::new` | [P9] Motivating: Homeostatic Self-Regulation — privacy port is part of the energy loop; [P4] Constraining: Clear Boundaries — HTTPS-only and non-empty treasury pubkey |

#### Test Contracts (36)

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-W-T1 | `P9-wallet-mgr-gas-conversion-test` | gas_to_rjoules_conversion |
| FR-W-T2 | `P9-wallet-mgr-rjoules-to-gas-test` | rjoules_to_gas_conversion |
| FR-W-T3 | `P9-wallet-mgr-fee-estimate-test` | estimate_withdrawal_fee_uses_price_feed |
| FR-W-T4 | `P9-wallet-mgr-can-afford-test` | can_afford_checks_balance |
| FR-W-T5 | `P9-wallet-mgr-reserve-rejects-test` | reserve_rejects_insufficient_balance |
| FR-W-T6 | `P9-wallet-mgr-settle-debits-test` | settle_debits_actual_cost |
| FR-W-T7 | `P9-wallet-mgr-deposit-ref-gen-test` | deposit_reference_generation |
| FR-W-T8 | `P9-wallet-mgr-balance-conservation-pbt` | balance_conservation_under_encumbrance_lifecycle |
| FR-W-T9 | `P9-wallet-mgr-deposit-monitor-idempotent-test` | deposit_monitor_credits_and_is_idempotent |
| FR-W-T10 | `P9-wallet-mgr-multi-chain-deposit-test` | poll_deposits_once_multi_chain |
| FR-W-T11 | `P9-wallet-mgr-payment-lifecycle-test` | end_to_end_payment_lifecycle |
| FR-W-T12 | `P9-wallet-mgr-encumbrance-state-machine-test` | encumbrance_status_state_machine_no_released_to_active |
| FR-W-T13 | `P9-wallet-mgr-withdraw-pipeline-test` | withdraw_full_pipeline_success |
| FR-W-T14 | `P9-wallet-mgr-withdraw-insufficient-test` | withdraw_rejects_insufficient_balance |
| FR-W-T15 | `P9-wallet-mgr-withdraw-unsupported-chain-test` | withdraw_rejects_unsupported_chain |
| FR-W-T16 | `P9-wallet-mgr-shielded-withdraw-privacy-test` | withdraw_shielded_hinkal_uses_privacy_path |
| FR-W-T17 | `P9-wallet-mgr-shielded-deposit-test` | shield_assets_uses_privacy_path |
| FR-W-T18 | `P9-wallet-issuer-create-keypair-test` | create_key_produces_valid_keypair |
| FR-W-T19 | `P9-wallet-issuer-expiry-test` | create_key_with_expiry |
| FR-W-T20 | `P9-wallet-issuer-revoke-unspent-test` | revoke_key_returns_unspent_rjoules |
| FR-W-T21 | `P9-wallet-issuer-list-active-test` | list_keys_returns_active_keys |
| FR-W-T22 | `P9-wallet-sign-withdrawal-signature-test` | sign_withdrawal_produces_signature |
| FR-W-T23 | `P9-wallet-sign-withdrawal-per-chain-test` | sign_withdrawal_differs_per_chain |
| FR-W-T24 | `P9-wallet-sign-capability-hex-test` | sign_capability_produces_hex_signature |
| FR-W-T25 | `P9-wallet-sign-withdrawal-all-chains-test` | sign_withdrawal_all_chains |
| FR-W-T26 | `P9-wallet-sign-withdrawal-empty-test` | sign_withdrawal_empty_tx_bytes |
| FR-W-T27 | `P9-wallet-sign-hinkal-message-signature-test` | sign_message_produces_signature |
| FR-W-T28 | `P9-wallet-sign-capability-tamper-test` | sign_capability_tampered_produces_different_signature |
| FR-W-T29 | `P9-wallet-price-static-rate-test` | static_price_feed_returns_expected_rates |
| FR-W-T30 | `P9-wallet-price-fee-nonzero-test` | fee_estimation_produces_non_zero_fee |
| FR-W-T31 | `P9-wallet-price-fee-floor-test` | fee_estimation_floors_at_one_rj |
| FR-W-T32 | `P9-wallet-price-chain-diff-test` | different_chains_produce_different_fees |
| FR-W-T33 | `P9-wallet-price-eodhd-parse-test` | eodhd_feed_parses_close_field |
| FR-W-T34 | `P9-wallet-price-coingecko-parse-test` | coingecko_feed_parses_usd_field |
| FR-W-T35 | `P9-wallet-price-composite-primary-test` | composite_returns_from_primary_source_on_success |
| FR-W-T36 | `P9-wallet-price-composite-fallback-test` | composite_falls_back_when_primary_fails |

> **Note:** Chain-adapter integration tests for Hedera, Solana, and Hinkal are realigned to `P9-wallet-hedera-*`, `P9-wallet-solana-*`, and `P9-wallet-hinkal-*` test IDs and are enumerated in the contract inventory. They are omitted above for brevity; see `docs/architecture/core/REQ_CONTRACT_INVENTORY.md` for the complete list.

### 3.2 Storage (`hkask-storage`)

**168 contracts** — storage spans multiple principles:
- **P3 (Generative Space)** — CRUD stores: agent registry, embeddings, gallery, goals, triples, wallet store, kata history, escalation, NuEvent store, spec store
- **P1 (User Sovereignty)** — user store, sovereignty boundaries, wallet-store tests
- **P2 (Affirmative Consent)** — consent store
- **P4 (Clear Boundaries)** — lock helpers, path safety, encrypted database, service→storage contract tests
- **P8 (Semantic Grounding)** — spec types, embedding/gallery/triple counts

**Crate:** `hkask-storage` | **Sources:** all `src/*.rs` and `tests/contract/services_storage_contract.rs`

#### Production Contracts (168 unique IDs)

| Domain | Principle | Contract Count | Representative IDs |
|--------|-----------|----------------|-------------------|
| Lock helpers | P4 | 3 | `P4-sto-lock-mutex`, `P4-sto-lock-read`, `P4-sto-lock-write` |
| Path safety | P4 | 1 | `P4-sto-path-safe-join` |
| Consent store | P2 | 4 | `P2-sto-consent-schema`, `P2-sto-consent-store`, `P2-sto-consent-get`, `P2-sto-consent-delete` |
| Sovereignty boundaries | P1 | 4 | `P1-sto-sovereignty-schema`, `P1-sto-sovereignty-store`, `P1-sto-sovereignty-get`, `P1-sto-sovereignty-delete` |
| NuEvent store | P3/P9 | 5 | `P3-sto-nu-event-replay`, `P3-sto-nu-event-decay`, `P3-sto-nu-event-cursor-store`, `P3-sto-nu-event-cursor-load`, `P3-sto-nu-event-algedonic-query` |
| Spec store | P3 | 6 | `P3-sto-spec-schema`, `P3-sto-spec-curation-*` |
| Spec types | P8 | 6 | `P8-sto-spec-str-enum-*`, `P8-sto-spec-id-*`, `P8-sto-spec-category-*`, `P8-sto-spec-infer-category` |
| Database | P4 | 7 | `P4-sto-database-open`, `P4-sto-database-in-memory`, `P4-sto-database-conn-arc`, `P4-sto-database-*-unwrap` |
| Kata history | P3 | 7 | `P3-sto-kata-record`, `P3-sto-kata-list-agent`, `P3-sto-kata-count-*`, `P3-sto-kata-last`, `P3-sto-kata-range`, `P3-sto-kata-delete-before` |
| Embeddings | P3 | 8 | `P3-sto-embedding-new`, `P3-sto-embedding-store`, `P3-sto-embedding-get`, `P3-sto-embedding-search`, `P3-sto-embedding-delete`, `P3-sto-embedding-count`, `P3-sto-embedding-prefix` |
| Escalation | P3 | 10 | `P3-sto-escalation-pending`, `P3-sto-escalation-queue-new`, `P3-sto-escalation-add`, `P3-sto-escalation-list-pending`, `P3-sto-escalation-get`, `P3-sto-escalation-resolve`, `P3-sto-escalation-dismiss`, `P3-sto-escalation-stats`, `P3-sto-escalation-summary-new`, `P3-sto-escalation-summary-text` |
| User store | P1 | 13 | `P1-sto-user-schema`, `P1-sto-user-register`, `P1-sto-user-login`, `P1-sto-user-logout`, `P1-sto-user-passphrase-change`, `P1-sto-user-passphrase-expired`, `P1-sto-user-session-get`, `P1-sto-user-session-list`, `P1-sto-user-replicant-get`, `P1-sto-user-human-get`, `P1-sto-user-replicant-list`, `P1-sto-user-wallet-get`, `P1-sto-user-wallet-set` |
| Gallery | P3 | 14 | `P3-sto-gallery-mode-str`, `P3-sto-gallery-schema`, `P3-sto-gallery-create`, `P3-sto-gallery-add-image`, `P3-sto-gallery-get-image`, `P3-sto-gallery-tag-image`, `P3-sto-gallery-get-tags`, `P3-sto-gallery-get`, `P3-sto-gallery-all-tags`, `P3-sto-gallery-face-register`, `P3-sto-gallery-face-list`, `P3-sto-gallery-face-get`, `P3-sto-gallery-face-remove`, `P3-sto-gallery-face-update` |
| Agent registry | P3 | 15 | `P3-sto-agent-registry-schema`, `P3-sto-agent-registry-insert`, `P3-sto-agent-registry-get`, `P3-sto-agent-registry-list`, `P3-sto-agent-registry-list-by-kind`, `P3-sto-agent-registry-remove`, `P3-sto-agent-registry-profile-*`, `P3-sto-agent-registry-contact-*`, `P3-sto-agent-registry-task-*` |
| Goals | P3 | 18 | `P3-sto-goal-repo-new`, `P3-sto-goal-repo-telemetry`, `P3-sto-goal-try-row`, `P3-sto-goal-row-parse`, `P3-sto-goal-create`, `P3-sto-goal-get`, `P3-sto-goal-update-state`, `P3-sto-goal-list`, `P3-sto-goal-criterion-add`, `P3-sto-goal-artifact-add`, `P3-sto-goal-criteria-get`, `P3-sto-goal-artifacts-get`, `P3-sto-goal-subgoal-create`, `P3-sto-goal-subgoal-list`, `P3-sto-goal-delete`, `P3-sto-goal-quarantine`, `P3-sto-goal-repair`, `P3-sto-goal-quarantine-list` |
| Triples | P3 | 22 | `P3-sto-triple-new`, `P3-sto-triple-with-*`, `P3-sto-triple-is-episodic`, `P3-sto-triple-is-semantic`, `P3-sto-triple-insert`, `P3-sto-triple-query-*`, `P3-sto-triple-update`, `P3-sto-triple-get-id`, `P3-sto-triple-low-confidence`, `P3-sto-triple-count-*`, `P3-sto-triple-query-below`, `P3-sto-triple-soft-delete`, `P3-sto-triple-hard-delete`, `P3-sto-triple-delete-prefix` |
| Wallet store | P3 | 25 | `P3-sto-wallet-wal-mode`, `P3-sto-wallet-balance-get`, `P3-sto-wallet-ensure`, `P3-sto-wallet-list-ids`, `P3-sto-wallet-credit`, `P3-sto-wallet-debit`, `P3-sto-wallet-tx-record`, `P3-sto-wallet-tx-list`, `P3-sto-wallet-tx-hash-exists`, `P3-sto-wallet-api-key-*`, `P3-sto-wallet-spent-rj-update`, `P3-sto-wallet-address-*`, `P3-sto-wallet-reference-*`, `P3-sto-wallet-encumber`, `P3-sto-wallet-encumbrance-release`, `P3-sto-wallet-encumbrance-consume`, `P3-sto-wallet-encumbrance-get` |

> **Note:** The original handoff estimated 12 storage contracts; the actual source contains **168 unique contract IDs**. Storage is the largest domain. All have been realigned to `P{N}-sto-*`.

### 3.3 Memory (`hkask-memory`)

**52 production contracts** + **16 test contracts** — P3 (Generative Space)

**Crate:** `hkask-memory` | **Sources:** `src/recall_dedup.rs`, `src/consolidation.rs`, `src/consolidation_service.rs`, `src/episodic.rs`, `src/episodic_loop.rs`, `src/semantic.rs`, `src/semantic_loop.rs`, `src/salience.rs`, `src/ranking.rs`

Memory provides the generative substrate for experience and knowledge: episodic first-person storage, semantic shared storage, consolidation bridges, salience-based budget gating, and cybernetic regulation loops.

#### Production Contracts (52 unique IDs)

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-M001 | `P3-mem-consolidation-bridge-new` | `new()` | [P3] Motivating: Generative Space — bridges episodic experience into shared semantic memory; [P4] Constraining: Clear Boundaries — links stores without bypassing their membranes |
| FR-M002 | `P3-mem-consolidation-bridge-consolidate` | `consolidate()` | [P3] Motivating: Generative Space — promotes sovereign episodic triples to shared knowledge; [P1] Constraining: User Sovereignty — strips perspective only under Curator authority; [P4] Constraining: Clear Boundaries — requires ConsolidationToken from expected curator |
| FR-M003 | `P3-mem-consolidation-candidate-count` | `consolidation_candidate_count()` | [P3] Motivating: Generative Space — surfaces how much episodic content is ready for promotion; [P9] Constraining: Homeostatic Self-Regulation — count-only query avoids loading full store |
| FR-M004 | `P3-mem-consolidation-service-new` | `new()` | [P3] Motivating: Generative Space — user-facing entry point for memory consolidation and cleanup; [P4] Constraining: Clear Boundaries — requires Curator-issued ConsolidationToken |
| FR-M005 | `P3-mem-consolidation-service-consolidate` | `consolidate()` | [P3] Motivating: Generative Space — combines episodic promotion with semantic cleanup; [P9] Constraining: Homeostatic Self-Regulation — enforces confidence floor and max triple limits; [P4] Constraining: Clear Boundaries — delegates to token-gated bridge |
| FR-M006 | `P3-mem-consolidation-service-candidate-count` | `consolidation_candidate_count()` | [P3] Motivating: Generative Space — reports how many episodic triples can be promoted; [P9] Constraining: Homeostatic Self-Regulation — count-only, graceful degradation on error |
| FR-M007 | `P3-mem-consolidation-service-low-confidence-count` | `semantic_low_confidence_count()` | [P3] Motivating: Generative Space — reports low-confidence semantic triples for cleanup; [P9] Constraining: Homeostatic Self-Regulation — threshold-driven pruning signal |
| FR-M008 | `P3-mem-consolidation-service-triple-count` | `semantic_triple_count()` | [P3] Motivating: Generative Space — reports total semantic memory size; [P9] Constraining: Homeostatic Self-Regulation — count used for budget monitoring |
| FR-M009 | `P3-mem-episodic-memory-new` | `new()` | [P3] Motivating: Generative Space — creates a sovereign first-person experience store; [P9] Constraining: Homeostatic Self-Regulation — default decay and budget are regulation defaults |
| FR-M010 | `P3-mem-episodic-store` | `store()` | [P3] Motivating: Generative Space — stores a first-person experience triple; [P1] Constraining: User Sovereignty — rejects Public visibility (episodic is sovereign); [P4] Constraining: Clear Boundaries — requires perspective owner |
| FR-M011 | `P3-mem-episodic-query-deduped` | `query_for_deduped()` | [P3] Motivating: Generative Space — recalls deduplicated episodic triples for an entity; [P9] Constraining: Homeostatic Self-Regulation — applies confidence decay and temporal attention at recall |
| FR-M012 | `P3-mem-episodic-storage-usage` | `storage_usage()` | [P3] Motivating: Generative Space — reports episodic storage usage per perspective; [P9] Constraining: Homeostatic Self-Regulation — COUNT query avoids loading full store |
| FR-M013 | `P3-mem-episodic-storage-budget` | `storage_budget()` | [P3] Motivating: Generative Space — exposes the episodic storage set-point; [P9] Constraining: Homeostatic Self-Regulation — budget bounds per-agent experience growth |
| FR-M014 | `P3-mem-episodic-candidate-count` | `consolidation_candidate_count()` | [P3] Motivating: Generative Space — reports how many episodic triples are eligible for consolidation; [P9] Constraining: Homeostatic Self-Regulation — uses decayed confidence for prioritization |
| FR-M015 | `P3-mem-episodic-loop-new` | `new()` | [P3] Motivating: Generative Space — wraps episodic memory in a regulated generative loop; [P9] Constraining: Homeostatic Self-Regulation — storage_budget is the cybernetic set-point |
| FR-M016 | `P3-mem-episodic-loop-with-consolidation` | `with_consolidation()` | [P3] Motivating: Generative Space — enables promotion path when episodic budget is exceeded; [P9] Constraining: Homeostatic Self-Regulation — consolidation bridge fires only under token authority |
| FR-M017 | `P3-mem-episodic-loop-storage-budget` | `storage_budget()` | [P3] Motivating: Generative Space — exposes the generative budget set-point for context assembly; [P9] Constraining: Homeostatic Self-Regulation — budget value is immutable after construction |
| FR-M018 | `P3-mem-ranking-rrf-score` | `rrf_score()` | [P3] Motivating: Generative Space — fuses rank positions for context retrieval; [P8] Constraining: Semantic Grounding — reciprocal rank fusion is a standard ranking signal |
| FR-M019 | `P3-mem-ranking-parse-age` | `parse_age_to_days()` | [P3] Motivating: Generative Space — converts human-readable age strings into comparable temporal signals; [P5] Constraining: Essentialism — returns -1.0 for unparseable input, no exceptions |
| FR-M020 | `P3-mem-ranking-normalize-date-bucket` | `normalize_date_bucket()` | [P3] Motivating: Generative Space — buckets parsed age into human-readable recency labels; [P8] Constraining: Semantic Grounding — five fixed buckets preserve stable ordering |
| FR-M021 | `P3-mem-recall-eav-hash` | `eav_hash()` | [P3] Motivating: Generative Space — canonical recall dedup enables reuse of factual content across memory; [P8] Constraining: Semantic Grounding — deterministic BLAKE3 hash over canonical EAV content |
| FR-M022 | `P3-mem-recall-dedup-triples` | `dedup_triples()` | [P3] Motivating: Generative Space — deduplication preserves generative storage budget; [P5] Constraining: Essentialism — first-seen wins, no speculative retention policy |
| FR-M023 | `P3-mem-salience-method-signals` | `compute_method_signals()` | [P3] Motivating: Generative Space — extracts cheap stylometric signals for method-aware retrieval; [P8] Constraining: Semantic Grounding — signals are deterministic heuristics over raw text |
| FR-M024 | `P3-mem-salience-declared-method-matches` | `matches()` | [P3] Motivating: Generative Space — matches passage signals against declared method thresholds; [P8] Constraining: Semantic Grounding — unconfigured thresholds are always satisfied |
| FR-M025 | `P3-mem-salience-tag-entities` | `tag_entities()` | [P3] Motivating: Generative Space — tags passages with declared entities for the salience graph; [P8] Constraining: Semantic Grounding — case-insensitive substring matching |
| FR-M026 | `P3-mem-salience-all-tags` | `all_tags()` | [P3] Motivating: Generative Space — flattens entity categories for graph construction; [P5] Constraining: Essentialism — minimal iterator over existing vectors |
| FR-M027 | `P3-mem-salience-tag-count` | `tag_count()` | [P3] Motivating: Generative Space — counts distinct tags across all categories; [P5] Constraining: Essentialism — simple sum of category lengths |
| FR-M028 | `P3-mem-salience-compute-batch` | `compute_salience_batch()` | [P3] Motivating: Generative Space — scores passage salience to gate triple storage budget; [P9] Constraining: Homeostatic Self-Regulation — graph centrality bounded by neighbor sampling |
| FR-M029 | `P3-mem-salience-budget-resolve` | `resolve()` | [P3] Motivating: Generative Space — resolves passage count into absolute triple budget; [P9] Constraining: Homeostatic Self-Regulation — budget caps generative storage growth |
| FR-M030 | `P3-mem-semantic-memory-new` | `new()` | [P3] Motivating: Generative Space — creates shared semantic knowledge store; [P8] Constraining: Semantic Grounding — unifies triple and embedding stores |
| FR-M031 | `P3-mem-semantic-query-deduped` | `query_deduped()` | [P3] Motivating: Generative Space — recalls deduplicated public semantic triples; [P4] Constraining: Clear Boundaries — filters to Public visibility |
| FR-M032 | `P3-mem-semantic-store` | `store()` | [P3] Motivating: Generative Space — stores shared semantic triple; [P4] Constraining: Clear Boundaries — requires Public visibility and no perspective |
| FR-M033 | `P3-mem-semantic-triple-count` | `triple_count()` | [P3] Motivating: Generative Space — reports total shared knowledge triples; [P9] Constraining: Homeostatic Self-Regulation — count feeds storage budget loop |
| FR-M034 | `P3-mem-semantic-triple-count-entity` | `triple_count_for_entity()` | [P3] Motivating: Generative Space — reports semantic triples per entity; [P9] Constraining: Homeostatic Self-Regulation — per-entity budget monitoring |
| FR-M035 | `P3-mem-semantic-query-attribute` | `query_by_attribute()` | [P3] Motivating: Generative Space — queries shared triples by attribute; [P8] Constraining: Semantic Grounding — attribute-based recall expands context |
| FR-M036 | `P3-mem-semantic-store-embedding` | `store_embedding()` | [P3] Motivating: Generative Space — indexes embedding vector for similarity retrieval; [P8] Constraining: Semantic Grounding — vector indexed by triple entity_ref |
| FR-M037 | `P3-mem-semantic-search-similar` | `search_similar()` | [P3] Motivating: Generative Space — KNN search augments recall beyond exact matches; [P8] Constraining: Semantic Grounding — results ordered by embedding distance |
| FR-M038 | `P3-mem-semantic-embedding-count` | `embedding_count()` | [P3] Motivating: Generative Space — reports indexed embedding count; [P9] Constraining: Homeostatic Self-Regulation — count used for embedding budget monitoring |
| FR-M039 | `P3-mem-semantic-embedding-store` | `embedding_store()` | [P3] Motivating: Generative Space — exposes embedding store for advanced operations; [P5] Constraining: Essentialism — direct accessor avoids duplicate wrappers |
| FR-M040 | `P3-mem-semantic-compute-centroid` | `compute_centroid()` | [P3] Motivating: Generative Space — computes mean style vector for corpus validation; [P8] Constraining: Semantic Grounding — arithmetic mean over matching embeddings |
| FR-M041 | `P3-mem-semantic-purge-prefix` | `purge_by_prefix()` | [P3] Motivating: Generative Space — purges embeddings for idempotent re-ingest; [P5] Constraining: Essentialism — prefix-based deletion, count of successes returned |
| FR-M042 | `P3-mem-semantic-chunk-text` | `chunk_text()` | [P3] Motivating: Generative Space — chunks text into passage-sized units for embedding; [P5] Constraining: Essentialism — paragraph/sentence boundary splitting with min/max words |
| FR-M043 | `P3-mem-semantic-strip-gutenberg` | `strip_gutenberg_headers()` | [P3] Motivating: Generative Space — removes boilerplate for clean corpus ingestion; [P5] Constraining: Essentialism — marker-based trim, no regex |
| FR-M044 | `P3-mem-semantic-delete-triple` | `delete_triple()` | [P3] Motivating: Generative Space — deletes semantic triple for budget enforcement or cleanup; [P9] Constraining: Homeostatic Self-Regulation — used by regulation loops to free space |
| FR-M045 | `P3-mem-semantic-lowest-confidence` | `lowest_confidence_triples()` | [P3] Motivating: Generative Space — identifies lowest-confidence triples for pruning; [P9] Constraining: Homeostatic Self-Regulation — ordered by confidence and age |
| FR-M046 | `P3-mem-semantic-low-confidence-count` | `low_confidence_count()` | [P3] Motivating: Generative Space — counts uncertain semantic triples; [P9] Constraining: Homeostatic Self-Regulation — threshold-driven count |
| FR-M047 | `P3-mem-semantic-low-confidence-triples` | `low_confidence_triples()` | [P3] Motivating: Generative Space — retrieves uncertain semantic triples for review; [P9] Constraining: Homeostatic Self-Regulation — bounded by threshold and limit |
| FR-M048 | `P3-mem-semantic-loop-new` | `new()` | [P3] Motivating: Generative Space — wraps semantic memory in a regulated knowledge loop; [P9] Constraining: Homeostatic Self-Regulation — default budget and low-confidence threshold are set-points |
| FR-M049 | `P3-mem-semantic-loop-with-budget` | `with_budget()` | [P3] Motivating: Generative Space — customizes storage budget per user or agent; [P9] Constraining: Homeostatic Self-Regulation — configurable set-point for memory homeostasis |
| FR-M050 | `P3-mem-semantic-loop-with-budget-threshold` | `with_budget_and_threshold()` | [P3] Motivating: Generative Space — customizes both budget and cleanup threshold; [P7] Constraining: Evolutionary Architecture — thresholds emerge from usage patterns |
| FR-M051 | `P3-mem-semantic-loop-storage-budget` | `storage_budget()` | [P3] Motivating: Generative Space — exposes the semantic storage set-point; [P9] Constraining: Homeostatic Self-Regulation — immutable budget reference for regulation |
| FR-M052 | `P3-mem-semantic-loop-low-confidence-threshold` | `low_confidence_threshold()` | [P3] Motivating: Generative Space — exposes the low-confidence cleanup set-point; [P9] Constraining: Homeostatic Self-Regulation — threshold triggers pruning of uncertain knowledge |

#### Test Contracts (16 unique IDs)

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-MT001 | `P3-mem-salience-hemingway-test` | `method_signals_hemingway_like()` |
| FR-MT002 | `P3-mem-salience-wilde-test` | `method_signals_wilde_like()` |
| FR-MT003 | `P3-mem-salience-declared-method-test` | `declared_method_matches()` |
| FR-MT004 | `P3-mem-salience-zero-empty-test` | `salience_zero_for_empty_tags()` |
| FR-MT005 | `P3-mem-salience-shared-entities-test` | `salience_increases_with_shared_entities()` |
| FR-MT006 | `P3-mem-salience-clustering-zero-test` | `clustering_zero_when_neighbors_disconnected()` |
| FR-MT007 | `P3-mem-salience-bridge-higher-test` | `bridge_scores_higher_than_dense_clique()` |
| FR-MT008 | `P3-mem-salience-methods-graph-test` | `methods_participate_in_graph()` |
| FR-MT009 | `P3-mem-salience-budget-per-page-test` | `budget_per_page_resolve()` |
| FR-MT010 | `P3-mem-salience-budget-absolute-test` | `budget_absolute()` |
| FR-MT011 | `P3-mem-salience-tag-case-insensitive-test` | `entity_tagging_case_insensitive()` |
| FR-MT012 | `P3-mem-salience-dialogue-ratio-test` | `dialogue_ratio_detection()` |
| FR-MT013 | `P3-mem-salience-valid-range-test` | `salience_scores_in_valid_range()` |
| FR-MT014 | `P3-mem-salience-empty-tags-proptest` | `empty_tags_produce_zero_salience()` |
| FR-MT015 | `P3-mem-semantic-centroid-dimensions-test` | `centroid_accumulation_skips_out_of_range_dimensions()` |
| FR-MT016 | `P3-mem-semantic-centroid-short-test` | `centroid_accumulation_handles_short_embedding()` |

> **Note:** The original handoff estimated ~8 memory contracts; the actual source contains **52 production** and **16 test** unique contract IDs. All have been realigned to `P3-mem-*`.

### 3.4 Inference (`hkask-inference`)

**15 contracts** — P9 + P4 (Homeostatic + Boundary)
- `InferencePort` trait — the interface for all inference backends (P4)
- `InferenceEnergyEstimator` — token-based cost estimation (P9)
- Provider adapters: Ollama, Fireworks, DeepInfra, OpenAI (P9)

### 3.5 Templates (`hkask-templates`)

**10 contracts** — P3 (Generative Space)
- `LLMParameters` — structured parameter set for LLM calls (P3)
- `PromptTemplate` — template engine for prompt construction (P3)
- Variable interpolation, partial application, template caching

### 3.6 MCP Servers (`mcp-servers/`)

**18 contracts** — P5 (Essentialism)
- `hkask-mcp-research` — web research agent (P5)
- `hkask-mcp-spec` — specification document server (P5)
- `hkask-mcp-condenser` — context compression agent (P5)
- Tool registration, capability declaration, resource serving

### 3.7 Service Layer (`hkask-services`)

**14 contracts** — P5 + P7 (Essentialism + Evolution)
- `AgentLifecycleService` — agent creation, monitoring, teardown (P5)
- `CnsService` — CNS health, alerts, variety, budget queries (P5)
- `KeystoreService` — key management and signing operations (P5)
- Service registration pattern: all services are discovered, not coupled

### 3.8 Agents (`hkask-agents`)

**30 contracts** — agents span four motivating principles:
- **P1 (User Sovereignty)** — `AgentPod`, `PodManager`, `SovereigntyChecker`
- **P2 (Affirmative Consent)** — `ConsentRecord`, `ConsentManager`
- **P4 (Clear Boundaries)** — ACP runtime, root authority, MCP capability adapters
- **P9 (Homeostatic Self-Regulation)** — Curator, Metacognition, LoopSystem, BotHealth, prompt classification

**Crate:** `hkask-agents` | **Sources:** `src/consent.rs`, `src/sovereignty.rs`, `src/loop_system.rs`, `src/prompt_analysis.rs`, `src/registry_loader.rs`, `src/acp/**/*.rs`, `src/curator/**/*.rs`, `src/curator_agent/**/*.rs`, `src/pod/**/*.rs`, `src/adapters/**/*.rs`, `src/ports/memory_storage.rs`, `tests/agent_pod_integration.rs`

#### Production Contracts (30 unique IDs)

| Domain | Principle | Contract IDs |
|--------|-----------|-------------|
| Consent | P2 | `P2-agt-consent-record-new`, `P2-agt-consent-record-grant`, `P2-agt-consent-record-revoke`, `P2-agt-consent-record-is-active`, `P2-agt-consent-record-has-category`, `P2-agt-consent-manager-new`, `P2-agt-consent-manager-with-sink`, `P2-agt-consent-manager-grant`, `P2-agt-consent-manager-revoke`, `P2-agt-consent-manager-check`, `P2-agt-consent-manager-granted-categories` |
| Sovereignty | P1 | `P1-agt-sovereignty-checker-new`, `P1-agt-sovereignty-checker-can-access`, `P1-agt-sovereignty-checker-can-perform` |
| Loop System | P9 | `P9-agt-loop-id`, `P9-agt-loop-system-new`, `P9-agt-loop-system-interval`, `P9-agt-loop-system-register`, `P9-agt-loop-system-cancel-token`, `P9-agt-loop-system-run`, `P9-agt-loop-system-tick`, `P9-agt-loop-system-run-ticks`, `P9-agt-loop-system-stop`, `P9-agt-loop-system-count`, `P9-agt-loop-system-ids` |
| Prompt Analysis | P9 | `P9-agt-prompt-classify` |
| Registry | P3 | `P3-agt-registry-loader-new`, `P3-agt-registry-loader-restore`, `P3-agt-registry-loader-load`, `P3-agt-registry-loader-store`, `P3-agt-registry-source-new` |
| ACP | P4 | `P4-agt-acp-audit-new`, `P4-agt-acp-audit-append`, `P4-agt-acp-message-visit`, `P4-agt-acp-message-sender`, `P4-agt-acp-message-id`, `P4-agt-acp-message-type`, `P4-agt-acp-runtime-new`, `P4-agt-acp-secret-derive`, `P4-agt-acp-token-issue`, `P4-agt-acp-agent-unregister`, `P4-agt-acp-agents-restore`, `P4-agt-acp-agents-list`, `P4-agt-acp-root-new`, `P4-agt-acp-root-token-issue` |
| MCP Adapters | P4 | `P4-agt-mcp-capability-adapter-new`, `P4-agt-mcp-full-adapter-new` |
| Memory Adapter | P3 | `P3-agt-memory-adapter-new`, `P3-agt-memory-adapter-in-memory`, `P3-agt-memory-adapter-in-memory-unwrap`, `P3-agt-memory-adapter-encrypted` |
| Memory Ports | P3 | `P3-agt-memory-request-new`, `P3-agt-memory-request-episodic`, `P3-agt-memory-request-semantic`, `P3-agt-memory-confidence-map`, `P3-agt-memory-recall-episodic`, `P3-agt-memory-recall-semantic` |
| Curator | P9 | `P9-agt-curator-persona-check`, `P9-agt-curator-persona-strip`, `P9-agt-curator-loop-new`, `P9-agt-curator-loop-new-with-consolidation`, `P9-agt-curator-loop-inbox`, `P9-agt-curator-loop-context`, `P9-agt-curator-loop-handle`, `P9-agt-curator-loop-restore-cursor`, `P9-agt-curator-context-new`, `P9-agt-curator-context-with-store`, `P9-agt-curator-context-with-acp`, `P9-agt-curator-context-handle`, `P9-agt-curator-context-directive` |
| Curator Agent | P9 | `P9-agt-curator-agent-escalation-check`, `P9-agt-curator-agent-meta-new`, `P9-agt-curator-agent-tick`, `P9-agt-curator-agent-summary`, `P9-agt-curator-agent-direct`, `P9-agt-curator-agent-issue-directive`, `P9-agt-curator-agent-new`, `P9-agt-curator-agent-new-with-config`, `P9-agt-curator-agent-new-with-consolidation`, `P9-agt-curator-agent-curation-loop`, `P9-agt-curator-agent-metacognition-loop`, `P9-agt-curator-agent-context`, `P9-agt-curator-agent-spec-curator`, `P9-agt-curator-agent-spec-new`, `P9-agt-curator-agent-spec-calibrate`, `P9-agt-curator-agent-spec-with-config`, `P9-agt-curator-agent-spec-drift-threshold`, `P9-agt-curator-agent-spec-with-sink`, `P9-agt-curator-agent-spec-channel`, `P9-agt-curator-agent-spec-check` |
| Bot Health | P9 | `P9-agt-bot-health-classify` |
| Pod Lifecycle | P1 | `P1-agt-pod-new`, `P1-agt-pod-register`, `P1-agt-pod-activate`, `P1-agt-pod-deactivate`, `P1-agt-pod-delegate`, `P1-agt-pod-is-active`, `P1-agt-pod-state`, `P1-agt-pod-enter-server-mode`, `P1-agt-pod-enter-chat-mode`, `P1-agt-pod-exit-mode`, `P1-agt-pod-is-server-mode`, `P1-agt-pod-set-voice`, `P1-agt-pod-get-voice`, `P1-agt-pod-voice-description`, `P1-agt-pod-is-chat-mode`, `P1-agt-pod-check-sovereignty` |
| Pod Manager | P1 | `P1-agt-pod-manager-new`, `P1-agt-pod-manager-with-consent`, `P1-agt-pod-manager-activation-hook`, `P1-agt-pod-manager-with-checker`, `P1-agt-pod-manager-with-sink`, `P1-agt-pod-manager-with-governed-tool`, `P1-agt-pod-manager-with-ports`, `P1-agt-pod-manager-inference-port`, `P1-agt-pod-manager-sovereignty-checker`, `P1-agt-pod-manager-default`, `P1-agt-pod-manager-create-pod`, `P1-agt-pod-manager-activate-pod`, `P1-agt-pod-manager-deactivate-pod`, `P1-agt-pod-manager-recall-lifecycle`, `P1-agt-pod-manager-status`, `P1-agt-pod-manager-list-status`, `P1-agt-pod-manager-acp-port`, `P1-agt-pod-manager-find-by-name`, `P1-agt-pod-manager-webid`, `P1-agt-pod-manager-has-role`, `P1-agt-pod-manager-has-capability`, `P1-agt-pod-manager-assign-role`, `P1-agt-pod-manager-set-mode` |
| Pod Types | P4 | `P4-agt-pod-lifecycle-can-transition` |

> **Note:** The inventory generator reports 30 contracts because it groups by unique ID. Production contract occurrences in `hkask-agents` total **174** (including repeated annotations on struct + impl + tests). Each production function retains the same contract ID across its struct comment, impl comment, and test comment.

### 3.2 Storage (`hkask-storage`)
### 3.9 Communication (`hkask-comm`)

**6 contracts** — P1 (User Sovereignty)
- `Channel` — message passing between agents (P1)
- `Broadcast` — pub/sub event distribution (P1)
- Message serialization, delivery guarantees

### 3.10 Keystore (`hkask-keystore`)

**5 contracts** — P1 (User Sovereignty)
- `KeyManagement` — key generation, storage, rotation (P1)
- `SigningKey` — delegation token signing (P1)
- Key derivation, expiry, revocation

### 3.11 Types (`hkask-types`)

**40 contracts** — P8 (Semantic Grounding)
- `CnsSpan` — canonical span registry (P8)
- `WebID` — agent identity type (P8)
- `NuEvent` — event type system (P8)
- Port definitions, error types, serialization

### 3.12 API Surface (`hkask-api`)

**25 contracts** — P1 + P4 (Sovereignty + Boundaries)
- REST endpoints for all service operations (P1)
- MCP protocol handler (P1)
- Authentication, authorization, rate limiting

### 3.13 CLI (`kask`)

**12 contracts** — P3 (Generative Space)
- `kask` binary — the user-facing command entry point (P3)
- Subcommands: `agent`, `cns`, `wallet`, `keystore` (P3)
- Flag parsing, help text, error reporting

### 3.14 Test Harness

**Cross-cutting** — shared across all crates
- `hkask-test-harness` — integration test infrastructure
- Test fixtures, mock implementations, property-based testing

---

## 4. Realignment Status

### 4.1 Contract ID Migration Summary

| Domain | Source File | Old Format | New Format | Contracts |
|--------|-----------|-----------|-----------|-----------|
| Energy | `energy.rs` | `cns-*` | `P{N}-cns-energy-*` | 23 |
| Algedonic | `algedonic.rs` | `svc-cns-*` | `P{N}-cns-algedonic-*` | 9 |
| Runtime | `runtime.rs` | `cns-runtime-*` | `P{N}-cns-runtime-*` | 30 |
| Governed Tool | `governed_tool.rs` | (various) | `P{N}-cns-gov-tool-*` | 7 |
| Governed Inference | `governed_inference.rs` | (various) | `P{N}-cns-gov-inf-*` | 4 |
| Circuit Breaker | `circuit_breaker.rs` | (various) | `P{N}-cns-circuit-*` | 3 |
| API Metering | `api_metering.rs` | (various) | `P{N}-cns-api-meter-*` | 16 |
| Energy Estimation | `composite_energy_estimator.rs` | (already aligned) | `P9-cns-est-composite-new` | 1 |
| Wallet Estimation | `wallet_energy_estimator.rs` | `cns-calibrate-*` | `P9-cns-est-wallet-*` | 6 |
| Wallet — Manager | `manager.rs` | `WALLET-*`, `wallet-int-*` | `P9-wallet-mgr-*` | 34 |
| Wallet — Issuer | `issuer.rs` | `WALLET-006`, `P4-issuer` | `P9-wallet-issuer-*` | 10 |
| Wallet — Signing | `signing.rs` | `WALLET-007`, `HINKAL-006`, `P4-signing` | `P9-wallet-sign-*` | 10 |
| Wallet — Hinkal Adapter | `hinkal.rs` | `HINKAL-*` | `P9-wallet-hinkal-*` | 20 |
| Wallet — Price Feed | `price_feed.rs` | `wallet-price-*` | `P9-wallet-price-*` | 16 |
| Wallet — Hedera Tests | `hedera.rs` | `hedera-int-*` | `P9-wallet-hedera-*` | 6 |
| Wallet — Solana Tests | `solana.rs` | `solana-int-*` | `P9-wallet-solana-*` | 5 |
| Agents — Consent | `consent.rs` | `AGT-038`–`AGT-048` | `P2-agt-consent-*` | 11 |
| Agents — Sovereignty | `sovereignty.rs` | `AGT-119`–`AGT-121` | `P1-agt-sovereignty-*` | 3 |
| Agents — Loop System | `loop_system.rs` | `AGT-062`–`AGT-072` | `P9-agt-loop-*` | 11 |
| Agents — Prompt Analysis | `prompt_analysis.rs` | `AGT-087` | `P9-agt-prompt-*` | 1 |
| Agents — Registry | `registry_loader.rs`, `adapters/registry_source.rs` | `AGT-108`, `AGT-115`–`AGT-118` | `P3-agt-registry-*` | 5 |
| Agents — ACP | `acp/**/*.rs` | `AGT-073`–`AGT-086` | `P4-agt-acp-*` | 14 |
| Agents — MCP Adapters | `adapters/mcp_runtime.rs` | `AGT-113`–`AGT-114` | `P4-agt-mcp-*` | 2 |
| Agents — Memory | `adapters/memory_loop_adapter.rs`, `ports/memory_storage.rs` | `AGT-032`–`AGT-037`, `AGT-109`–`AGT-112` | `P3-agt-memory-*` | 10 |
| Agents — Curator | `curator/**/*.rs` | `AGT-049`–`AGT-061` | `P9-agt-curator-*` | 13 |
| Agents — Curator Agent | `curator_agent/**/*.rs` | `AGT-088`–`AGT-107`, `BOT-HEALTH-001` | `P9-agt-curator-agent-*`, `P9-agt-bot-health-*` | 20 |
| Agents — Pod Lifecycle | `pod/mod.rs`, `pod/types.rs` | `AGT-122`–`AGT-137`, `AGT-161` | `P1-agt-pod-*`, `P4-agt-pod-lifecycle-*` | 17 |
| Agents — Pod Manager | `pod/manager.rs` | `AGT-138`–`AGT-160` | `P1-agt-pod-manager-*` | 23 |
| Storage — Lock Helpers | `lock_helpers.rs` | `STO-001`–`STO-003` | `P4-sto-lock-*` | 3 |
| Storage — Path Safety | `security.rs` | `STO-004` | `P4-sto-path-safe-join` | 1 |
| Storage — Consent | `consent_store.rs` | `STO-005`–`STO-008` | `P2-sto-consent-*` | 4 |
| Storage — Sovereignty | `sovereignty.rs` | `STO-009`–`STO-012` | `P1-sto-sovereignty-*` | 4 |
| Storage — NuEvent | `nu_event_store.rs` | `STO-013`–`STO-017` | `P3-sto-nu-event-*` | 5 |
| Storage — Spec Store | `spec_store.rs` | `STO-018`–`STO-023` | `P3-sto-spec-*` | 6 |
| Storage — Spec Types | `spec_types.rs` | `STO-163`–`STO-168`, `MDS-spec-svc-001` | `P8-sto-spec-*` | 6 |
| Storage — Database | `database.rs` | `STO-024`–`STO-030` | `P4-sto-database-*` | 7 |
| Storage — Kata History | `kata_history.rs` | `STO-031`–`STO-037` | `P3-sto-kata-*` | 7 |
| Storage — Embeddings | `embeddings.rs` | `STO-038`–`STO-045` | `P3-sto-embedding-*` | 8 |
| Storage — Escalation | `escalation.rs` | `STO-046`–`STO-055` | `P3-sto-escalation-*` | 10 |
| Storage — User Store | `user_store.rs` | `STO-056`–`STO-068` | `P1-sto-user-*` | 13 |
| Storage — Gallery | `gallery.rs` | `STO-069`–`STO-082`, `media-*` | `P3-sto-gallery-*` | 14 |
| Storage — Agent Registry | `agent_registry.rs` | `STO-083`–`STO-097` | `P3-sto-agent-registry-*` | 15 |
| Storage — Goals | `goals.rs` | `STO-098`–`STO-115` | `P3-sto-goal-*` | 18 |
| Storage — Triples | `triples.rs` | `STO-116`–`STO-137` | `P3-sto-triple-*` | 22 |
| Storage — Wallet Store | `wallet_store.rs` | `STO-138`–`STO-162`, `SHOULD-8`, `MUST-10`, `wallet-*` | `P3-sto-wallet-*`, `P1-sto-wallet-*` | 25 |
| Storage — Contract Tests | `tests/contract/services_storage_contract.rs` | `CTR-002` | `P4-sto-services-contract-test` | 0 (tests only) |
| Memory | `src/**/*.rs` | `MEM-*`, `memory-salience-*`, `semantic-*` | `P3-mem-*` | 68 |

**Total CNS contracts:** 99 (across all 9 source files).
**Total wallet contracts:** 23 production occurrences (11 unique IDs).
**Total agents contracts:** 174 production occurrences (30 unique IDs).
**Total storage contracts:** 247 production occurrences (168 unique IDs).
**Total memory contracts:** 67 production occurrences (52 unique production IDs + 16 test IDs).
**Build status:** `cargo check -p hkask-cns`, `cargo check -p hkask-wallet`, `cargo check -p hkask-agents`, `cargo check -p hkask-storage`, and `cargo check -p hkask-memory` pass clean.

### 4.2 Idempotent Migration

The contract ID migration is **idempotent** — the same source file can be reread at any time and the same contract IDs will be extracted. There is no stateful migration step. The contract IDs exist in the source code, not in a database.

### 4.3 Cross-Crate Dependencies

All hKask crates depend on `hkask-types` for the canonical `CnsSpan` registry, `WebID` identity type, and port definitions. The CNS contracts are **leaf nodes** — they do not depend on any other crates. Realignment does not change any downstream crate's behavior.

---

## 5. Contract ID Format Appendix

### 5.1 Formal Specification

Every contract ID follows the pattern:

```
P{N} - {domain-short} - {operation}
```

Where:
- **P{N}** — The motivating principle (1–12). This determines which principle **owns** the contract and appears in the ID prefix.
- **{domain-short}** — Abbreviated domain name (e.g., `energy`, `algedonic`, `runtime`, `gov-tool`, `gov-inf`, `circuit`, `api`, `est`).
- **{operation}** — Verb phrase describing what the contract does (e.g., `new`, `can-proceed`, `settle`, `calibrate`).

Constraining principles appear in the contract body as `[P{N}] Constraining: ...` annotations. A contract may have:
- **One motivating principle** (the ID prefix)
- **Multiple constraining principles** (body annotations)

### 5.2 Principle Legend

| # | Principle | Role in CNS |
|---|----------|------------|
| P1 | User Sovereignty | User owns their data, decisions, and identity |
| P2 | Affirmative Consent | Every action requires explicit user consent |
| P3 | Generative Space | The system can create, modify, and destroy state |
| P4 | Clear Boundaries | Modules own their domains; boundaries are enforced |
| P5 | Essentialism | Remove everything that does not earn its existence |
| P6 | (Reserved) | Not yet assigned to CNS contracts |
| P7 | Evolutionary Architecture | Parameters emerge from real usage, not speculation |
| P8 | Semantic Grounding | Types carry meaning; newtypes prevent confusion |
| P9 | Homeostatic Self-Regulation | Feedback loops maintain system stability |
| P10 | (Reserved) | Not yet assigned to CNS contracts |
| P11 | (Reserved) | Not yet assigned to CNS contracts |
| P12 | Subscriber Consent | Observers register through explicit subscription |

### 5.3 Validation Rules

1. **Unique contract IDs** — No two contracts share the same ID.
2. **Idempotent** — Reading the same source file twice produces the same IDs.
3. **Stable** — Contract IDs persist across code changes unless the contract's purpose changes.
4. **Derivable** — IDs can be derived from `grep "REQ:" crates/hkask-cns/src/*.rs`.

### 5.4 Notational Conventions

- **Production contracts** are labeled `P{N}-{domain}-{operation}` in the contract body (e.g., `P9-cns-energy-budget-new`, `P9-wallet-mgr-build`).
- **Test contracts** are labeled `P{N}-{domain}-{operation}-test` or have a `-T{N}` suffix.
- **Blocking variants** use the P3 prefix: `P3-cns-{domain}-blocking-{operation}`.
- **Calibrate contracts** use the P7 prefix: `P7-cns-{domain}-calibrate-{operation}`.
- **Subscriber contracts** use the P12 prefix: `P12-cns-{domain}-subscribe-{operation}`.

### 5.5 Future Domains

The following domains are **not yet realigned** and will use their own principle prefixes:
- `hkask-storage` (P3): `P3-storage-*`
- `hkask-memory` (P3): `P3-memory-*`
- `hkask-agents` (P1): `P1-agents-*`
- `hkask-inference` (P9+P4): `P9/P4-inference-*`
- `hkask-templates` (P3): `P3-templates-*`
- `hkask-services` (P5+P7): `P5/P7-services-*`
- `hkask-api` (P1+P4): `P1/P4-api-*`
- `hkask-comm` (P1): `P1-comm-*`
- `hkask-keystore` (P1): `P1-keystore-*`
- `kask` CLI (P3): `P3-cli-*`
- `mcp-servers/` (P5): `P5-mcp-*`

- `hkask-wallet` is **complete** as of this revision: `P9-wallet-*`.
- `hkask-agents` is **complete** as of this revision: `P1-agt-*`, `P2-agt-*`, `P3-agt-*`, `P4-agt-*`, `P9-agt-*`.
- `hkask-storage` is **complete** as of this revision: `P1-sto-*`, `P2-sto-*`, `P3-sto-*`, `P4-sto-*`, `P8-sto-*`.

---

## Appendix A: Document Metadata

| Field | Value |
|-------|-------|
| Version | v0.27.0 |
| Created | 2026-06-16 |
| Status | Active — anchor for the rSolidity contract vocabulary |
| Last Updated | 2026-06-16 |
| Contract Count | 99 CNS + wallet/agents/storage/memory realignment (473 total in manifest) |
| Build Status | `cargo check -p hkask-cns -p hkask-wallet -p hkask-agents -p hkask-storage -p hkask-memory` — PASS |
| rSolidity Status | Design complete — see `RSOLIDITY_VOCABULARY.md` |
| Governance | PRINCIPLES.md §1–§5 |

## Appendix B: Validation Checklist

- [x] All 99 CNS contracts carry principle annotations
- [x] Build passes clean: `cargo check -p hkask-cns`
- [x] All test IDs updated to new format
- [x] Domain map complete (22 domains)
- [x] FR tables complete (all 8 CNS domains)
- [x] Realignment status table complete
- [x] Contract ID format specification complete
- [x] Non-CNS domain contracts (wallet) — realigned to `P9-wallet-*`
- [x] rSolidity contract vocabulary derivation — see `RSOLIDITY_VOCABULARY.md` and `data/rsolidity_contract_manifest.json`

## Appendix C: Key References

- [PRINCIPLES.md](PRINCIPLES.md) — 12 governing principles
- [MDS.md](MDS.md) — Minimum Definition Specification
- [TESTING_DISCIPLINE.md](TESTING_DISCIPLINE.md) — Contract testing discipline
- [RSOLIDITY_VOCABULARY.md](RSOLIDITY_VOCABULARY.md) — rSolidity macro vocabulary design
- [hKask Architecture Master](../hKask-architecture-master.md) — Full architecture reference

---
