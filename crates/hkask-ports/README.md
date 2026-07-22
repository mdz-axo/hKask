# hkask-ports — Hexagonal Port Traits

Hexagonal architecture port traits for infrastructure abstractions. Enables domain crates to depend on interfaces rather than concrete implementations, respecting the Authority DAG dependency direction.

## Public Modules

| Module | Purpose |
|--------|---------|
| `regulation` | Regulation observer, storage, backpressure, circuit breaker ports |
| `inference_port` | `InferencePort` — LLM inference abstraction |
| `inference_types` | `InferenceResult`, `StructuredToolCall`, `TokenProbability` |
| `embedding_port` | `EmbeddingPort` — vector embedding store/search |
| `consent_port` | Consent storage and query port |
| `escalation` | Escalation queue port for Regulation algedonic signals |
| `federation` | `FederationDispatch` — inter-replica federation operations |
| `registry_port` | Agent registry port |
| `git_cas` | Git content-addressed storage port |
| `flowdef_validation` | FlowDef manifest validation |
| `tool` | `ToolPort` — MCP tool invocation port |
| `registry` | `Skill`, `RegistryIndex`, `SkillZone` types |

## Key Types

| Type | Description |
|------|-------------|
| `InferencePort` | LLM inference abstraction (generate + stream) |
| `EmbeddingPort` | Vector embedding store and similarity search |
| `LedgerObserver` | Regulation event observer trait |
| `LedgerStoragePort` | Regulation persistence (replay, algedonic query) |
| `CircuitBreakerPort` | Circuit breaker state machine |
| `FederationDispatch` | Inter-replica federation lifecycle |
| `ToolPort` / `ToolInfo` | MCP tool invocation and metadata (includes `ToolTaint` label for FIDES IFC) |
| `BackpressureSignal` | Regulation backpressure communication |
| `RegistryPort` | Agent registration and lookup |

## Dependencies

- `hkask-types` — `WebID`, `InfrastructureError`, `NuEvent`
- `thiserror`, `serde`, `tokio`, `async-trait`
