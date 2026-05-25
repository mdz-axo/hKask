# Hexagonal Port Inventory

Minimal inventory of all hexagonal ports in hKask v0.21.0.

## Core Ports

### hkask-agents

| Port | Location | Purpose | Adapters |
|------|----------|---------|----------|
| `AcpPort` | `ports/acp.rs` | Agent Communication Protocol | `AcpRuntime`, `RussellAcpAdapter` |
| `GitCASPort` | `pod.rs` | Git content-addressed storage | `GitCasAdapter`, `MockGitCas` |
| `MCPRuntimePort` | `pod.rs` | MCP tool invocation | `McpRuntimeAdapter` |
| `MemoryStoragePort` | `pod.rs` | Artifact persistence | `MemoryStorageAdapter` |
| `CnsEmit` | (from hkask-cns) | CNS span emission | `CnsEmitterAdapter` |
| `KeystorePort` | `adapters/keystore_port.rs` | Secret management | `KeychainAdapter` |
| `SovereigntyPort` | `ports/sovereignty.rs` | Consent management | `ConsentManager` |

### hkask-templates

| Port | Location | Purpose | Adapters |
|------|----------|---------|----------|
| `InferencePort` | `inference_port.rs` | LLM inference | `OkapiInference` |
| `McpPort` | `ports.rs` | MCP tool dispatch | `McpDispatcher` |
| `CnsPort` | `ports.rs` | CNS observability | `CnsRuntime` |
| `MemoryPort` | `ports.rs` | Semantic/episodic recall | `MemoryStorageAdapter` |
| `ManifestExecutor` | `ports.rs` | Manifest execution | `ManifestExecutorImpl`, `SimpleExecutor` |

### hkask-mcp

| Port | Location | Purpose | Adapters |
|------|----------|---------|----------|
| `McpTransport` | `transport.rs` | MCP server communication | `InProcessMcpTransport`, `StdioMcpTransport`, `HttpMcpTransport` |

### hkask-types

| Port | Location | Purpose | Adapters |
|------|----------|---------|----------|
| `CnsEmit` | `cns.rs` | CNS span emission | `CnsRuntime` |

## Capability Ports

### hkask-templates

| Port | Location | Purpose | Adapters |
|------|----------|---------|----------|
| `CapabilityValidator` | `capability_validator.rs` | OCAP validation | `CapabilityAwareValidator` |

### hkask-ensemble

| Port | Location | Purpose | Adapters |
|------|----------|---------|----------|
| `CapabilityQueryPort` | `ocap_enforcement.rs` | Capability queries | `WebIDCapabilityRegistry` |

## Storage Ports

### hkask-storage

| Port | Location | Purpose | Adapters |
|------|----------|---------|----------|
| `GoalStoragePort` | `goals.rs` | Goal persistence | `GoalStore` |

### hkask-memory

| Port | Location | Purpose | Adapters |
|------|----------|---------|----------|
| `GoalMemoryPort` | `goal_memory.rs` | Goal memory | `GoalMemory` |

## Design Principles

1. **Single capability primitive**: `CapabilityToken` with caveats (T08)
2. **Async purity**: All ports use `#[async_trait]` (T10)
3. **No stubs in production**: All adapters implement real functionality (T09, T18)
4. **Typed errors**: No `unwrap()` on hot paths (T15)
5. **Deterministic identity**: WebIDs derived from persona content (T06)

## See Also

- `docs/plans/ADV-REVIEW-F2.md` — Adversarial review and remediation plan
- `docs/plans/IMPLEMENTATION-PLAN-F2.md` — Detailed implementation tasks
