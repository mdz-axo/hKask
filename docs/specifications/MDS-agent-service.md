---
title: "MDS — AgentService Specification"
audience: [architects, developers, agents]
last_updated: 2026-06-10
version: "0.27.2"
status: "Active"
domain: "Composition"
mds_categories: [domain, composition, trust, lifecycle]
---

# MDS — AgentService Specification

**Purpose:** Specification for the condensed service layer architecture. `AgentService` (formerly `ServiceContext`) is the canonical service layer that owns all shared infrastructure.

**Supersedes:** `hKask-architecture-master.md` §Service Layer (v0.27.0)

**Related:** [`PRINCIPLES.md`](PRINCIPLES.md), [`MDS.md`](MDS.md)

---

## 1. Domain Spec — AgentService

### 1.1 Bounded Context

`AgentService` is the **single source of truth** for all shared infrastructure in hKask. Both CLI and API surfaces compose an `AgentService` instance and add only presentation-specific state.

**Boundary:** In-process only. MCP servers do NOT depend on `AgentService` (P1 Prohibition — out-of-process isolation).

### 1.2 Domain Grouping (7 Group Methods)

All 27 fields are grouped into 7 group methods returning tuples of references — no adapter structs, no new types:

| Method | Fields Returned | Accessor Pattern |
|--------|---------|-----------------|
| **memory()** | `episodic_storage`, `semantic_storage` | `let (ep, sem) = svc.memory()` |
| **cns()** | `cns_runtime`, `cybernetics_loop`, `loop_system`, `event_sink` | `let (rt, cy, ls, ev) = svc.cns()` |
| **governance()** | `capability_checker`, `mcp_dispatcher`, `escalation_queue` | `let (cc, dp, eq) = svc.governance()` |
| **storage()** | `registry`, `goal_repo`, `spec_store`, `standing_session_store`, `user_store`, `agent_registry_store`, `git_cas_port` | `let (reg, gl, sp, ss, us, ar, gc) = svc.storage()` |
| **coordination()** | `inference_port`, `mcp_runtime`, `pod_manager`, `session_manager` | `let (inf, mcp, pm, sm) = svc.coordination()` |
| **identity()** | `system_webid`, `acp_runtime` | `let (wid, acp) = svc.identity()` |
| **config()** | `config` | `let cfg = svc.config()` |

### 1.3 hLexicon Allocation

| Term | Domain | Definition |
|------|--------|-----------|
| `AgentService` | WordAct | Canonical service layer owning all shared infrastructure |
| `GroupMethod` | FlowDef | Tuple of references to related fields, accessed via destructuring |

### 1.4 Focusing Assumptions

| ID | Statement | Rationale |
|----|-----------|-----------|
| FA-AS1 | All fields are used (verified by call-site audit) | No fields can be deleted without breaking functionality |
| FA-AS2 | Group methods return tuples — no adapter structs needed | Essentialist G3: zero-field structs add no behavior |
| FA-AS3 | Old individual accessors coexist during strangler-fig migration | Surfaces transition one domain at a time |

---

## 2. Composition Spec — AgentService Interface

### 2.1 Public API (7 Group Methods + 1 config)

```rust
impl AgentService {
    pub async fn build(config: ServiceConfig) -> Result<Self, ServiceError>;

    pub fn memory(&self) -> (&Arc<EpisodicStoragePort>, &Arc<SemanticStoragePort>);
    pub fn cns(&self) -> (&Arc<RwLock<CnsRuntime>>, &Arc<RwLock<CyberneticsLoop>>, &Arc<LoopSystem>, &Arc<dyn NuEventSink>);
    pub fn governance(&self) -> (&Arc<CapabilityChecker>, &Arc<McpDispatcher>, &Arc<EscalationQueue>);
    pub fn storage(&self) -> (/* 7 store references */);
    pub fn coordination(&self) -> (&Option<Arc<InferencePort>>, &Arc<McpRuntime>, &Arc<PodManager>, &Arc<RwLock<SessionManager>>);
    pub fn identity(&self) -> (&WebID, &Arc<AcpRuntime>);
    pub fn config(&self) -> &ServiceConfig;
}
```

### 2.2 Group Method Field Mapping

| Method | Fields (in tuple order) | Private Fields (not exposed) |
|--------|------------------------|------------------------------|
| `memory()` | episodic_storage, semantic_storage | — |
| `cns()` | cns_runtime, cybernetics_loop, loop_system, event_sink | — |
| `governance()` | capability_checker, mcp_dispatcher, escalation_queue | consent_manager, sovereignty_boundary_store (P1) |
| `storage()` | registry, goal_repo, spec_store, standing_session_store, user_store, agent_registry_store, git_cas_port | — |
| `coordination()` | inference_port, mcp_runtime, pod_manager, session_manager | curation_inbox_tx (internal channel) |
| `identity()` | system_webid, acp_runtime | — |
| `config()` | config | — |

**Sovereignty fix (P1):** `consent_manager` and `sovereignty_boundary_store` are excluded from `governance()`. Callers access sovereignty checks through mediated service methods, never raw stores.

### 2.3 Interface Equivalence

| Interface | CLI Uses | API Uses | Equivalent? |
|-----------|----------|---------|-------------|
| `memory()` | ✅ (chat, REPL) | ✅ (episodic, semantic routes) | ✅ Yes |
| `cns()` | ✅ (CNS commands, REPL) | ✅ (CNS routes) | ✅ Yes |
| `governance()` | ✅ (MCP commands) | ✅ (MCP routes) | ✅ Yes |
| `storage()` | ✅ (goals, specs, agents) | ✅ (templates, bundles, goals) | ✅ Yes |
| `coordination()` | ✅ (ensemble, pods) | ✅ (ensemble, pods, curator) | ✅ Yes |
| `identity()` | ❌ | ❌ | ✅ N/A (internal use) |
| `config()` | ✅ (all commands) | ✅ (all routes) | ✅ Yes |

---

## 3. Trust Spec — OCAP Boundaries

### 3.1 Threat Model

| Adversary | Vector | Mitigation |
|-----------|--------|------------|
| Surface crate accessing fields directly | Direct field access bypasses encapsulation | **Private fields** + accessor methods only |
| Domain crate depending on `AgentService` | Circular dependency | **Prohibition:** Domain crates use ports, not `AgentService` |
| MCP server depending on `AgentService` | Out-of-process boundary violation | **Prohibition:** MCP servers use primitives |

### 3.2 OCAP Boundaries

1. **All fields are private** — no direct access from any crate
2. **Accessor methods are the only interface** — surfaces call methods, not fields
3. **Domain crates do NOT depend on `AgentService`** — they use port traits
4. **MCP servers do NOT depend on `AgentService`** — out-of-process isolation

### 3.3 Capability Attenuation

| Operation | Required Capability | Attenuation |
|-----------|-------------------|-------------|
| `let (ep, _) = svc.memory()` | `episodic_memory:read/write` | Scoped to agent WebID |
| `let (_, sem) = svc.memory()` | `semantic_memory:read/write` | Public data only |
| `let (_, dp, _) = svc.governance()` | `tools:execute` | Per-tool capability |
| `let (_, _, pm, _) = svc.coordination()` | `pods:create/manage` | Agent-scoped |

---

## 4. Lifecycle Spec — Migration Path

### 4.1 Bootstrap Sequence

```
1. ServiceConfig::from_env() / from_secrets()
2. AgentService::build(config)
   ├── Open databases (primary, consent, escalation, goals, etc.)
   ├── Initialize stores (consent, escalation, goals, standing, sovereignty, specs, users)
   ├── Build CNS runtime + event sink
   ├── Build loop system (Cybernetics, Inference, Episodic, Semantic, Curation, Snapshot)
   ├── Build Governance (MCP runtime, dispatcher, governed tool, capability checker)
   ├── Build Coordination (pod manager, ACP runtime, session manager)
   ├── Build Storage (registry, goal repo, etc.)
   └── Build Identity (system WebID, event sink)
3. Surface wraps AgentService:
   ├── CLI: ReplState { agent_service, prompt_state, ... }
   └── API: ApiState { agent_service, standing_sessions, router, ... }
```

### 4.2 Evolution (Strangler-Fig Migration)

| Phase | Action | Verification |
|-------|--------|-------------|
| **Phase 1** | Add 7 group methods to `AgentService` (tuples of refs, zero new types) | `cargo check` passes |
| **Phase 2** | Route CNS callers through `cns()` group method | `cargo test -p hkask-cli -p hkask-api` passes |
| **Phase 3** | Delete `CnsService` pass-through module | `cargo check --workspace` passes |
| **Phase 4** | Route remaining callers through group methods per domain | Surface tests pass |
| **Phase 5** | Delete old individual accessors (strangler-fig DELETE) | `cargo clippy -- -D warnings` passes |

### 4.3 Deprecation Policy

**Strangler fig — no deprecation.** Old accessors coexist during migration and are deleted when all consumers are on the new path. No `#[deprecated]` attributes (P6/P7 violation).

---

## 5. Curation Spec — Coherence

### 5.1 Coherence Metric

**Method:** Jaccard similarity of declared vs. registered domain adapters

```
coherence = |declared ∩ registered| / |declared ∪ registered|
```

**Threshold:** 0.7 (70% overlap)

**Declared Methods:** memory, cns, governance, storage, coordination, identity, config (7 total)

**Registered Methods:** (After implementation) All 7 must be present

### 5.2 Curation Decision

**Decision:** ✅ **Accept**

**Rationale:** 
- All fields are accounted for in 7 group methods
- Each method returns related fields by domain (memory, cns, governance, storage, coordination, identity, config)
- Tuple destructuring enforces encapsulation (private fields, public methods)
- Migration path is strangler-fig (5 phases, functional at every step)

---

## 6. Test Program — REQ Tags

### 6.1 MDS Category → Test Strategy

| MDS Category | Test Strategy | REQ Tags |
|-------------|--------------|----------|
| **Domain** | AgentService construction + field grouping | `// REQ-MDS-D1` |
| **Composition** | Group methods return correct tuple types | `// REQ-MDS-C1` |
| **Trust** | Direct field access fails to compile | `// REQ-MDS-T1` |
| **Lifecycle** | Bootstrap sequence completes without error | `// REQ-MDS-L1` |

### 6.2 Tracer Bullets (Priority Order)

1. **P0 (Security):** `// REQ-MDS-T1` — Direct field access fails to compile
2. **P1 (Correctness):** `// REQ-MDS-C1` — All 7 group methods exist and return correct tuple types
3. **P1 (Correctness):** `// REQ-MDS-D1` — AgentService::build() assembles all fields
4. **P2 (Lifecycle):** `// REQ-MDS-L1` — Bootstrap completes in <5 seconds

---

## 7. Open Questions

| ID | Question | Decision Criteria |
|----|----------|------------------|
| F1 | Should group methods be public? | **Yes** — surfaces need to access them |
| F2 | Should group methods be async? | **No** — fields are Arc'd, no I/O in method body |
| F3 | Should we add builder pattern for AgentService? | **No** — `build()` is sufficient |
| F4 | Should we add `#[non_exhaustive]` to AgentService? | **Yes** — prevents struct literal construction |
| F5 | Should we delete old individual accessors? | **Yes** — strangler-fig DELETE phase (post-migration) |

---

*ℏKask — A Minimal Viable Container for Agents — v0.27.2*
