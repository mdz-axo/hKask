---
title: "hkask-cns — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "e17e69e2"
---

# hkask-cns — API Reference

Cybernetic Nervous System — homeostatic self-regulation for the hKask agent platform. Performs variety sensing, algedonic alerting, energy budget management, OCAP governance, and sovereignty enforcement per Ashby's Law of Requisite Variety.

## Public Modules

| Module | Description |
|---|---|
| `acp_span` | ACP (Agent Client Protocol) span types |
| `api_metering` | API key metering: rate limits, CNS spans, alerts. Types: `ApiMeter`, `ApiMeteringAlert`, `ApiRequestSpan`, `EndpointWeight`, `RateLimitStatus`, `endpoint_weight()` |
| `calibrated_energy_estimator` | Self-regulating per-server gas estimator. Types: `CalibratedEnergyEstimator`, constants `DEFAULT_CALIBRATION_INTERVAL`, `DEFAULT_INITIAL_LOOKBACK` |
| `circuit_breaker` | Regulation circuit breaker: `CircuitBreaker` (implements `CircuitBreakerPort`) |
| `classify_span` | Span classification: `ClassifySpan` |
| `composite_energy_estimator` | Composite routing: inference → token-based, others → table. Type: `CompositeEnergyEstimator` |
| `contract_events` | Contract lifecycle events: `emit_contract_proposed()`, `emit_contract_accepted()`, `emit_contract_rejected()`, `emit_contract_violated()` |
| `contract_span` | Contract span type: `ContractSpan` |
| `cybernetics_loop` | Core regulation loop (Loop 6): `CyberneticsLoop` |
| `dynamic_gas_table` | Dynamic gas cost table: `DynamicGasTable` |
| `energy` | Energy budget types: `GasBudget`, `GasCost`, `GasDelta`, `GasError`, `AgentGasStatus`, `DEFAULT_GAS_ALERT_THRESHOLD` |
| `energy_budget_management` | Budget registration/reservation/settlement: `GasBudgetManager` |
| `gas_report` | Gas reporting: `AgentGasReport`, `AgentGasSummary`, `GasReport`, `GasTotals`, `ToolGasBreakdown` |
| `governed_inference` | Inference call membrane: `GovernedInference` (implements `InferencePort`) |
| `governed_tool` | Tool invocation membrane: `GovernedTool` (implements `ToolPort`), `EnergyEstimator` trait |
| `infra_span` | Infrastructure span: `InfraSpan` |
| `qa_span` | QA span: `QaSpan` |
| `runtime` | CNS runtime: `CnsRuntime`, `NoopEventSink` |
| `seam_span` | Seam span: `SeamSpan` |
| `seam_types` | Seam inventory types: `SeamInventory`, `SeamCoverage` |
| `seam_watcher` | Public seam watcher: `SeamWatcher`, `SeamDrift`, `SeamSummary` |
| `set_points` | Homeostatic set-points: `SetPoints`, `SetPointsConfig`, `InferenceThrottleMode`, `load_set_points()`, plus 7 named constants |
| `slo_manager` | SLO evaluation: `SloManager`, `SloDataProvider`, `SloDataPoint`, `SloManagerError` |
| `slo_span` | SLO span: `SloSpan` |
| `slo_types` | SLO types: `SloDefinition`, `SloEvaluation`, `SloSeverity`, `seed_slos()` |
| `storage_guard` | Moved to `hkask-storage-guard` crate. See that crate for `StorageGuardLoop`, `StorageGuardConfig`, `walk_dir()`. |
| `types` | Loop core types: re-exports from `hkask_types::loops` |
| `wallet_budget` | Wallet-backed energy budgets: `WalletBackedBudget` |
| `wallet_energy_estimator` | Wallet-aware energy estimation: `WalletEnergyEstimator` |
| `wallet_gas_calibrator` | Wallet gas calibration: `WalletGasCalibrator`, `DEFAULT_WALLET_CALIBRATION_INTERVAL`, `DEFAULT_WALLET_INITIAL_LOOKBACK` |
| `wallet_manager` | Wallet manager |
| `well` | Well management |

## Key Public Types

### `CnsRuntime`

Single entry point for all CNS operations. Provides variety counting (Ashby's Law) and algedonic alerting (deficit > threshold → escalate). Internally composes `AlgedonicManager`, `VarietyTracker`, and `SloManager`. Uses `NuEventSink` for event emission.

### `CyberneticsLoop`

Closed-loop homeostatic controller (Loop 6). Functional contract: **Sense** (receive `cns.*` spans) → **Compare** (against SetPoints) → **Compute** (produce efferent signal) → **Act** (dispatch regulation).

Key internal components: `Dampener`, `StagnationDetector`, `SensorRegistry`, `SystemSimulator`, `StrategyEvaluator`.

Uses internal channels (`tokio::sync::mpsc`) for signal routing.

### `GovernedTool`

Capability-gated, gas-accounted, observability-emitting membrane around `ToolPort`. Implements `ToolPort` itself.

**Hold-settle pattern:** gas reserved before invocation, settled after with actual cost. If actual cost < reserved, the difference is refunded.

**Invocation pipeline:** Authority (OCAP) → Budget (reserve gas) → Emit span (`cns.tool.invoked`) → Delegate to inner tool → Settle energy cost → Emit outcome span (`cns.tool.completed`).

OCAP verification uses two paths: exact-match on tool name, and capability-domain matching via `capabilities_match()`.

### `GovernedInference`

Energy-budget-gated, observability-emitting membrane around `InferencePort`. Implements `InferencePort` itself. Same hold-settle pattern as `GovernedTool`. Cost estimation: 1 token ≈ 1 gas unit, driven by `LLMParameters.max_tokens`.

### `SetPoints`

Homeostatic set-points for the Cybernetics Loop. All fields have associated `DEFAULT_*` constants.

**Fields:** `gas_min_remaining (f64)`, `variety_max_deficit (f64)`, `error_rate_max (f64)`, `connector_latency_max_secs (f64)`, `communication_backpressure_threshold (QueueDepth)`, `seam_coverage_min (f64)`, `fed_sync_latency_warning_ms (u64)`, `fed_sync_latency_critical_ms (u64)`, `fed_crdt_divergence_warning_factor (f64)`, `fed_link_downtime_warning_secs (u64)`, `fed_link_downtime_critical_secs (u64)`, `fed_max_pause_duration_hours (u64)`, `fed_invitation_rate_warning_per_hour (u64)`, `fed_registry_divergence_warning (u64)`, `dampen_window_secs (u64)`, `metacognitive_window_secs (u64)`, `override_cooldown_secs (u64)`, `outcome_warning_threshold (f64)`, `outcome_critical_threshold (f64)`, `guard_violation_rate_max (f64)`, `max_iterations (u32)`, `stagnation_thresholds (HashMap<String, u32>)`.

**Named Default Constants:** `DEFAULT_ENERGY_MIN_REMAINING_RATIO` (0.2), `DEFAULT_VARIETY_MAX_DEFICIT` (100.0), `DEFAULT_ERROR_RATE_MAX` (0.3), `DEFAULT_CONNECTOR_LATENCY_MAX_SECS` (30.0), `DEFAULT_COMMUNICATION_BACKPRESSURE_THRESHOLD`, `DEFAULT_MAX_ITERATIONS` (100).

### `InferenceThrottleMode`

Enum: `Off` (no automatic throttle), `Autonomous` (direct throttle, pre-authorized), `CuratorMediated { curator_timeout_secs: u64 }` (escalate to Curator with fallback throttle).

### `GasBudget`

Energy budget types namespace. Core types: `GasBudget` (overall budget), `GasCost` (per-operation cost), `GasDelta` (cost difference), `GasError` (budget errors), `AgentGasStatus` (per-agent gas state).

### `SloManager`

Service Level Objective manager. Methods: `new()`, `new_with_seeds()`, `register(slo)`, `evaluate(provider) -> Result<Vec<SloEvaluation>, SloManagerError>`.

### `SloDataProvider`

Trait for ν-event data queries: `fn query(&self, span_namespace: &str, window_seconds: u64) -> Result<SloDataPoint, SloManagerError>`.

### `SeamWatcher`

"Watcher on the wall" for API contract health. Loads public seam inventory at startup (embedded JSON at compile time), registers per-crate coverage as CNS variety dimensions, detects drift via snapshot comparison. Non-fatal by design — silently disabled if no inventory is available.

### `EnergyEstimator`

Trait for gas cost estimation used by `GovernedTool`: `fn estimate(&self, server: &str, tool: &str) -> u64`.

### `CircuitBreaker`

Implements `CircuitBreakerPort`. States: `Closed` (normal), `Open` (blocking), `HalfOpen` (testing).

## Re-exports from Crate Root

`RuntimeAlert`, `ApiMeter`, `ApiMeteringAlert`, `ApiRequestSpan`, `EndpointWeight`, `RateLimitStatus`, `endpoint_weight()`, `CalibratedEnergyEstimator`, `CompositeEnergyEstimator`, `CircuitBreaker`, `AcpSpan`, `ClassifySpan`, `emit_contract_proposed()`, `emit_contract_accepted()`, `emit_contract_rejected()`, `emit_contract_violated()`, `ContractSpan`, `CyberneticsLoop`, `DynamicGasTable`, `GasBudget`, `GasCost`, `GasDelta`, `GasError`, `AgentGasStatus`, `GasBudgetManager`, `AgentGasReport`, `AgentGasSummary`, `GasReport`, `GasTotals`, `ToolGasBreakdown`, `GovernedInference`, `GovernedTool`, `EnergyEstimator`, `QueueDepth`, `CurationThresholdConfig`, `InfraSpan`, `QaSpan`, `CnsRuntime`, `NoopEventSink`, `SeamSpan`, `SeamCoverage`, `SeamInventory`, `SeamDrift`, `SeamSummary`, `SeamWatcher`, `SetPoints`, `SetPointsConfig`, `InferenceThrottleMode`, `load_set_points()`, 7 `DEFAULT_*` constants, `SloManager`, `SloDataProvider`, `SloDataPoint`, `SloManagerError`, `SloSpan`, `SloDefinition`, `SloEvaluation`, `SloSeverity`, `seed_slos()`, `SnapshotLoop`, `SnapshotLoopConfig`, `StorageGuardLoop`, `StorageGuardConfig`, `walk_dir()`, `CurationInput`, `CuratorDirective`, `CuratorHandle`, `ExperienceClassification`, `LoopAction`, `WalletBackedBudget`, `WalletEnergyEstimator`, `WalletGasCalibrator`.
