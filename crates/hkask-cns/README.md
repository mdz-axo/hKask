# hkask-cns — Cybernetic Nervous System

Homeostatic self-regulation engine for the hKask agent platform. CNS enforces Ashby's Law of Requisite Variety through variety sensing, algedonic alerts, energy budgets, OCAP governance, and sovereignty enforcement (Loop 6).

## Public Modules

| Module | Purpose |
|--------|---------|
| `runtime` | `CnsRuntime` — central CNS state machine |
| `cybernetics_loop` | Loop 6 main sense→compute→act cycle |
| `energy` | Gas budgets (`hJoules`), `GasBudget`, `GasCost` |
| `energy_budget_management` | Budget registration, reservation, settlement |
| `governed_inference` | Inference call membrane — CNS-gated LLM access |
| `governed_tool` | Tool invocation membrane — CNS-gated MCP calls |
| `algedonic` | Algedonic signal channel (positive/negative valence) |
| `circuit_breaker` | Regulation circuit breaker |
| `types::loops` | `CurationInput`, `LoopAction`, `CuratorDirective` |
| `seam_watcher` | Seam drift detection and inventory |
| `slo_manager` | SLO evaluation, error budgets, breach escalation |
| `storage_guard` | Autonomous disk space management (Loop 7) |
| `wallet_manager` | Wallet-backed energy budgets |

## Key Types

| Type | Description |
|------|-------------|
| `CnsRuntime` | Central CNS state machine with health, variety, alerts |
| `CyberneticsLoop` | Loop 6 regulation cycle |
| `GovernedInference` | OCAP-gated inference boundary |
| `GovernedTool` | OCAP-gated tool invocation boundary |
| `GasBudget` | Energy budget with hJoule accounting |
| `CircuitBreaker` | Fail-open regulation circuit breaker |
| `SetPoints` | Configurable regulatory thresholds |
| `SloManager` | SLO evaluation with error budget tracking |
| `StorageGuardLoop` | Autonomous disk space reclamation |

## Dependencies

- `hkask-types` — foundation types (WebID, NuEvent, InfrastructureError)
- `hkask-ports` — hexagonal port traits (InferencePort, CircuitBreakerPort)
- `hkask-storage` — persistence (via ports)
- `hkask-capability` — OCAP delegation tokens
- `tokio`, `tracing`, `serde`, `chrono`
