---
title: Hexagonal Port Inventory
version: v0.21.0-p4-parity
status: accurate
last_updated: 2026-05-28
audience: [architects, developers]
domain: "Application"
ddmvss_categories: [interface]
---

# Hexagonal Port Inventory — hKask v0.21.0

## Remaining Traits (19 total)

All 19 traits have at least one real implementation and active `dyn` dispatch callers.

### hkask-agents (7)

| Port | Trait | File | Impls |
|---|---|---|---|
| ACP | `AcpPort` | `ports/acp.rs` | `AcpRuntime`, `RussellAcpAdapter` |
| ACP Transport | `AcpTransport` | `ports/acp_transport.rs` | `LoopbackHttpTransport`, `StdioTransport` |
| Git CAS | `GitCASPort` | `ports/git_cas.rs` | `GitCasAdapter`, `MockGitCas` |
| MCP Runtime | `MCPRuntimePort` | `ports/mcp_runtime.rs` | `McpRuntimeAdapter` |
| Memory Storage | `MemoryStoragePort` | `ports/memory_storage.rs` | `MemoryStorageAdapter` |
| Standing Session | `StandingSessionPort` | `ports/standing_session.rs` | `StandingSessionStoreAdapter` |
| Sovereignty | `SovereigntyChecker` | `sovereignty.rs` | Concrete struct (no trait) |
| Metacognition | `MetacognitionStoreAdapter` | `adapters/metacognition_store.rs` | Concrete struct (no trait) |

### hkask-templates (3)

| Port | Trait | File | Impls |
|---|---|---|---|
| Inference | `InferencePort` | `inference_port.rs` | `OkapiInference` |
| MCP Dispatch | `McpPort` | `ports.rs` | `McpDispatcher` (in hkask-mcp) |
| Registry | `RegistryIndex` | `ports.rs` | `Registry`, `SqliteRegistry` |

### hkask-ensemble (3)

| Port | Trait | File | Impls |
|---|---|---|---|
| Inference Client | `InferenceClient` | `ports.rs` | `OkapiImprovClient`, `OkapiHttpClient`, `MockInferenceClient` |
| Metrics Source | `MetricsSource` | `ports.rs` | `OkapiSseAdapter`, `MockMetricsSource` |
| Capability Query | `CapabilityQueryPort` | `ocap_enforcement.rs` | `WebIDCapabilityRegistry` |

### hkask-cns (1)

| Port | Trait | File | Impls |
|---|---|---|---|
| CNS Emit | `CnsEmit` | `spans.rs` | `SpanScope`, `CnsEmitterAdapter` |

### hkask-types (4)

| Port | Trait | File | Impls |
|---|---|---|---|
| Audit Log | `AuditLogPort` | `audit.rs` | `AuditLogStoreAdapter` |
| Nu Event Sink | `NuEventSink` | `event.rs` | `NuEventStore` |
| Spec Store | `SpecStore` | `spec.rs` | `SqliteSpecStore` |
| Spec Observer | `SpecObserver` | `spec.rs` | `CnsSpecObserver` |
| Spec Curator | `SpecCurator` | `spec.rs` | `DefaultSpecCurator` |

### Removed Ports (from prior v0.21.0 inventory)

| Trait | Why Removed |
|---|---|
| `SovereigntyPort` | Inlined to concrete `SovereigntyChecker` |
| `RateLimitPort` | Deleted — rate limiting consolidated into energy budget enforcement (`EnergyBudget.try_consume()`) |
| `KeystorePort` | Dead — `KeychainAdapter` had no callers |
| `OCAPPort` | Dead — `OCAPAdapter` had no callers |
| `ManifestExecutor` | Dead — `ManifestExecutorImpl` is concrete |
| `TemplateRenderer` | Dead — `TemplateRendererImpl` is concrete |
| `MemoryPort` | Dead — `AppMemoryAdapter` is concrete |
| `CspEnforcer` | Dead — `NoopCsp` is concrete |
| `SyncInferencePort` | Dead — manifest executor no longer uses it |
| `SecurityMetricPort` | Dead — zero implementations |
| `SpecSigner` | Dead — zero implementations |
| `CapabilityProviderPort` | Dead — zero implementations |
| `CuratorMetacognitionPort` | Dead — zero implementations |
| `OkapiClientTrait` | Dead — zero callers |
| `InputValidator` | Inlined to `AgentPersonaInput::validate()` |
| `GoalMemoryPort` | Inlined to `GoalMemory::store_semantic()` etc. |
| `GoalRepositoryPort` | Inlined to `SqliteGoalRepository` |
| `MetacognitionPort` | Inlined to `MetacognitionStoreAdapter` |
| `AuditLogStoragePort` | Dead — only an orphan error enum |
| `McpTransport` | Already an enum, never was a trait |
| `CnsQueryPort` | Dead — only domain types remain in `cns_query.rs` |

## Dependency Flow

```
hkask-cli → hkask-agents → hkask-types (ports defined here: AuditLogPort, NuEventSink, SpecStore, etc.)
                        → hkask-cns (CnsEmit)
                        → hkask-templates (InferencePort, McpPort, RegistryIndex)
           hkask-ensemble → hkask-agents (StandingSessionPort)

hkask-mcp → hkask-templates (McpPort impl)
          → hkask-cns (CnsEmit, optional CNS integration)
```

## See Also

- `hKask-architecture-master.md` — authoritative architecture spec
- `interface-and-composition.md` — port composition patterns
- `subsystem-erds.md` — entity relationship diagrams