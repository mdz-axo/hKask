---
title: Hexagonal Port Inventory
version: "0.27.0"
status: "Active"
last_updated: 2026-06-10
audience: [architects, developers]
domain: "Application"
mds_categories: [composition]
---

# Hexagonal Port Inventory — hKask v0.23.0

> **Note:** Active ports are authoritative in [`MDS.md`](../MDS.md) §7 §2. This reference provides implementation detail, removal history, and phantom type corrections.

## Active Traits

All traits have at least one real implementation and active `dyn` dispatch callers.

### hkask-agents (5)

| Port | Trait | File | Impls |
|---|---|---|---|
| ACP | `AcpPort` | `ports/acp.rs` | `AcpRuntime` |
| Git CAS | `GitCASPort` | `ports/git_cas.rs` | `GitCasAdapter`, `MockGitCas` |
| MCP Runtime | `MCPRuntimePort` | `ports/mcp_runtime.rs` | `McpRuntimeAdapter` |
| Standing Session | `StandingSessionPort` | `ports/standing_session.rs` | `StandingSessionStoreAdapter` |
| Sovereignty | `SovereigntyPort` | `sovereignty.rs` (impl) | `SovereigntyChecker` (implements `SovereigntyPort` from `hkask-types/src/sovereignty.rs`) |

### hkask-inference (1)

| Port | Trait | File | Impls |
|---|---|---|---|
| Inference | `InferencePort` | `inference_router.rs` | `InferenceRouter` |

### hkask-templates (2)

| Port | Trait | File | Impls |
|---|---|---|---|
| MCP Dispatch | `McpPort` | `ports.rs` | `McpDispatcher` (in hkask-mcp) |
| Registry | `RegistryIndex` | `ports.rs` | `Registry`, `SqliteRegistry` |

### hkask-ensemble (3)

| Port | Trait | File | Impls |
|---|---|---|---|
| Inference Client | `InferenceClient` | `ports.rs` | `InferencePortAdapter`, `CircuitBreakerInferenceAdapter`, `MockInferenceClient` |
| Metrics Source | `MetricsSource` | `ports.rs` | `MockMetricsSource` |
| Capability Query | `CapabilityQueryPort` | `ocap_enforcement.rs` | `WebIDCapabilityRegistry` |

### hkask-cns (1)

| Port | Trait | File | Impls |
|---|---|---|---|
| CNS Port | `CnsPort` | `runtime.rs` | `CnsRuntime` |

### hkask-types (2)

| Port | Trait | File | Impls |
|---|---|---|---|
| Audit Log | `AuditLogPort` | `audit.rs` | No adapter yet (in-memory `AuditLog` in `hkask-agents/src/acp/audit.rs`) |
| Nu Event Sink | `NuEventSink` | `event.rs` | `NuEventStore` |

### hkask-storage (2)

| Port | Trait | File | Impls |
|---|---|---|---|
| Spec Store | `SpecStore` | `spec_types.rs` | `SqliteSpecStore` |
| Spec Curator | `SpecCurator` | `spec_types.rs` (trait) | `DefaultSpecCurator` (`hkask-agents/src/curator_agent/spec_curator.rs`) |

### Removed Ports (from prior inventories)

| Trait | Why Removed |
|---|---|
| `SovereigntyPort` | **Not actually removed** — trait still exists at `hkask-types/src/sovereignty.rs:374`; `SovereigntyChecker` implements it |
| `RateLimitPort` | Deleted — rate limiting consolidated into gas budget enforcement (`EnergyBudget` in `hkask-cns/src/energy.rs`) |
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
| `InputValidator` | Inlined to `AgentPersonaInput::validate()` |
| `GoalMemoryPort` | Inlined to `GoalMemory::store_semantic()` etc. |
| `GoalRepositoryPort` | Inlined to `SqliteGoalRepository` |
| `MetacognitionPort` | Inlined to `MetacognitionStoreAdapter` |
| `AuditLogStoragePort` | Dead — only an orphan error enum |
| `CnsQueryPort` | Dead — only domain types remain in `cns_query.rs` |
| `AcpTransport` | Removed — transport layer deferred; `AcpPort` remains |
| `AcpWireMessage` | Removed — transport message types deleted with `AcpTransport` |
| `AcpWireResponse` | Removed — transport message types deleted with `AcpTransport` |
| `LoopbackHttpTransport` | Removed — `AcpTransport` impl deleted |
| `StdioTransport` | Removed — `AcpTransport` impl deleted |
| `SpecObserver` | Removed — trait deleted from `spec.rs`; CNS spec spans not yet needed |

### Non-Trait Types Previously Listed as Ports

| Type | Reality |
|---|---|
| `MemoryStoragePort` / `MemoryStorageAdapter` | **Never existed** — no trait, no impl, no file |
| `MetacognitionStoreAdapter` | **Never existed** — no struct, no adapter file |
| `CnsEmit` / `CnsEmitterAdapter` / `SpanScope` | **Never existed** — actual CNS port is `CnsPort` impl'd by `CnsRuntime` |
| `SecurityGateway` | **Never existed** in `hkask-mcp` — actual OCAP enforcement is `GovernedTool` in `hkask-cns` |
| `McpTransport` | **Never existed** as a trait or enum in current codebase |
| `EnergyBudget` | **Never existed** — actual type is `EnergyBudget` (`hkask-cns/src/energy.rs`) |

## Dependency Flow

```
hkask-cli → hkask-agents → hkask-types (ports defined here: AuditLogPort, NuEventSink)
                        → hkask-cns (CnsPort)
                        → hkask-templates (InferencePort, McpPort, RegistryIndex)
           hkask-ensemble → hkask-agents (StandingSessionPort)

hkask-mcp → hkask-templates (McpPort impl)
          → hkask-cns (CnsPort, optional CNS integration)
```

## See Also

- `hKask-architecture-master.md` — authoritative architecture spec
- `MDS.md §7` — port composition patterns
