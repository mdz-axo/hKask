---
title: "hkask-agents — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "e17e69e2"
---

# hkask-agents — API Reference

Agent Pod Lifecycle and A2A Integration. Provides runtime containers for A2A agents (bots and replicants), lifecycle management, OCAP-based access control, agent-to-agent messaging, and capability verification.

## Public Modules

| Module | Description |
|---|---|
| `a2a` | Agent-to-Agent protocol runtime. Types: `A2AAgent`, `A2AError`, `A2AMessage`, `A2ARuntime`. Sub-modules: `audit` (AuditLog, AuditEntry), `root_authority` (RootAuthority) |
| `adapters` | Adapter implementations bridging ports to concrete backends |
| `consent` | User consent tracking for sovereignty boundaries. Types: `ConsentManager`, `ConsentError` |
| `curator` | Curation machinery. Sub-modules: `context` (CuratorContext), `curation_loop` (CurationLoop). Types: `CuratorSync`, `SemanticIndex` |
| `curator_agent` | Curator persona layer (Loop 5). Sub-module: `metacognition` (MetacognitionLoop). Type: `CuratorAgent` |
| `error` | Agent error types: `CoreError`, `MemoryError` |
| `inference_loop` | Inference loop (Loop 1). Type: `InferenceLoop` |
| `loop_system` | Loop orchestration system. Type: `LoopSystem` |
| `pod` | Agent pod lifecycle management. Types: `AgentPod`, `AgentPersona`, `PodDeployment`, `PodFactory`, `PodKind`, `PodID`, `PodRegistry`, `ActivePods`, `AgentMode` |
| `ports` | Hexagonal port traits. Types: `EpisodicStoragePort`, `SemanticStoragePort`, `RecallRequest`, `RecalledEpisode`, `RecalledSemantic`, `StorageRequest` |
| `registry_loader` | Agent registry loading |
| `sovereignty` | Sovereignty enforcement. Types: `SovereigntyChecker`, `SovereigntyConsent`, `AllowAllConsent`, `DenyAllConsent` |
| `types` | Agent-specific types. Sub-module: `voice` (VoiceDesign). Re-exports: `agent` (AgentDefinition, Charter — canonical location is `hkask_types::agent_registry`) |
| `yaml_parser` | YAML parsing for agent personas |
| `yaml_types` | YAML type definitions |

## Key Public Types

### `CuratorAgent`

The persona layer of Curation (Loop 5). Composes the pure regulatory `CurationLoop` with the persona `MetacognitionLoop`. Singleton invariant — exactly one per hKask system.

**Fields:**
| Field | Type | Description |
|---|---|---|
| `curation_loop` | `Arc<CurationLoop>` | Pure regulatory loop |
| `metacognition` | `Arc<MetacognitionLoop>` | Persona: observe & adapt |
| `context` | `Arc<CuratorContext>` | Capability-disciplined access |
| `link_manager` | `Option<Arc<dyn FederationDispatch>>` | Federation link manager, set via `with_federation()` |

Receives `CuratorDirective`s from the Curation Loop through Communication dispatch and formats human-readable output for `kask chat`. Handles federation directives: `InviteToFederation`, `PauseFederationLink`, `RevokeFederationMember`, `LeaveFederation`, `DissolveFederation`.

### `ConsentManager`

User consent tracking for sovereignty boundaries. Manages explicit user consent for data access: grant, revoke, audit, and check. Persisted via `ConsentPort` (SQLite-backed), survives restarts — enforcing User Sovereignty (P1).

**Error type:** `ConsentError` with variants `Store(InfrastructureError)` and `ConsentNotFound(String)`.

### `A2ARuntime`

Agent-to-Agent message runtime. Handles agent registration, capability-gated message routing, audit logging, and response delivery. Uses `RootAuthority` for signing key management and `AuditLog` for forensic analysis.

**Message flow:** Agent Pod A → A2A Message → hKask Router → Capability Verification → Audit Log Entry → Template Execution → Response to Agent A.

### `A2AError`

A2A protocol error type. Variants include `MalformedCapability(String)` for capability parse failures.

### `A2AAgent`

Represents an A2A agent registered in the runtime.

### `A2AMessage`

A2A protocol message envelope.

### `PodDeployment`

Agent pod deployment configuration. Manages the lifecycle of agent pods from populated → registered → activated → deactivated.

### `PodID`

Type alias: `Id<PodIdKind>`. Canonically defined in `hkask_types::id`. Unique identifier for an agent pod.

### `PodRegistry`

Registry tracking all active pods and their lifecycle states.

### `ActivePods`

Set of currently active pod deployments.

### `AgentMode`

Agent operational mode classification.

### `AgentPersona`

Agent persona definition, loadable from YAML via `AgentPersona::from_yaml(yaml_str)`. Defines agent name, type (bot), and persona description.

### `PodKind`

Enumeration of pod types (e.g., bot pod, replicant pod).

### `PodFactory`

Factory for constructing agent pods from personas and adapters.

### `CurationLoop`

Pure regulatory loop (sense/compute/act) with no persona, no chat, no memory. Used by `CuratorAgent`.

### `InferenceLoop`

Inference loop (Loop 1). Wraps inference logic with loop-level regulation. Domain logic; governance applied externally via `GovernedTool` in hkask-cns.

### `LoopSystem`

Loop orchestration system managing all hKask loops.

### `SovereigntyChecker`

Sovereignty enforcement type. Checks capability tokens against sovereignty boundaries.

### `SovereigntyConsent`

Trait for consent decision-making during sovereignty enforcement. Implementations: `AllowAllConsent`, `DenyAllConsent`.

### `VoiceDesign`

Voice design type for agent personas (canonically in `hkask_agents::types::voice`).

### `CoreError`

Core agent error type.

### `MemoryError`

Memory-level agent error type.

## Port Traits (from `ports` module)

### `EpisodicStoragePort`

Port for episodic memory storage operations.

### `SemanticStoragePort`

Port for semantic memory storage operations.

### Supporting Types

- `RecallRequest` — memory recall request parameters
- `RecalledEpisode` — recalled episodic memory entry
- `RecalledSemantic` — recalled semantic memory entry
- `StorageRequest` — storage operation request

## Re-exports from Crate Root

`AgentDefinition`, `Charter` (both canonical in `hkask_types::agent_registry`), `A2AAgent`, `A2AError`, `A2AMessage`, `A2ARuntime`, `ConsentError`, `ConsentManager`, `CuratorContext`, `CurationLoop`, `CuratorSync`, `SemanticIndex`, `CuratorAgent`, `CoreError`, `MemoryError`, `InferenceLoop`, `LoopSystem`, `ActivePods`, `AgentMode`, `AgentPersona`, `PodDeployment`, `PodFactory`, `PodID`, `PodKind`, `PodRegistry`, `EpisodicStoragePort`, `RecallRequest`, `RecalledEpisode`, `RecalledSemantic`, `SemanticStoragePort`, `StorageRequest`, `AllowAllConsent`, `DenyAllConsent`, `SovereigntyChecker`, `SovereigntyConsent`, `VoiceDesign`.
