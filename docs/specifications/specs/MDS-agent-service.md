---
title: "MDS — AgentService Specification"
audience: [architects, developers, agents]
last_updated: 2026-06-18
version: "0.31.0"
status: "Active"
domain: "Composition"
mds_categories: [domain, composition, trust, lifecycle]
---

# MDS — AgentService Specification

**Purpose:** Specification for the condensed service layer architecture. `AgentService` is the canonical service layer that owns all shared infrastructure.

**Supersedes:** `hKask-architecture-master.md` §Service Layer (v0.27.0)

**Related:** [`PRINCIPLES.md`](../../architecture/core/PRINCIPLES.md), [`MDS.md`](../../architecture/core/MDS.md)

---

## 1. Domain Spec — AgentService

### 1.1 Bounded Context

`AgentService` is the **single source of truth** for all shared infrastructure in hKask. Both CLI and API surfaces compose an `AgentService` instance and add only presentation-specific state.

**Boundary:** In-process only. MCP servers do NOT depend on `AgentService` (P1 Prohibition — out-of-process isolation).

### 1.2 Individual Named Accessor Methods

All 28 fields are **private** and exposed through **individual named accessor methods** — one method per field or small domain-coherent pair. This replaces the earlier 8-group-method tuple pattern (v0.27.0 strangler-fig migration, now complete).

| Method | Returns | Category |
|--------|---------|----------|
| `config()` | `&ServiceConfig` | Configuration |
| `wallet()` | `Option<&Arc<WalletService>>` | Payments |
| `wallet_store()` | `Option<&Arc<WalletStore>>` | Payments |
| `memory()` | `(&Arc<dyn EpisodicStoragePort>, &Arc<dyn SemanticStoragePort>)` | Memory |
| `registry()` | `&Arc<tokio::sync::Mutex<SqliteRegistry>>` | Storage |
| `goal_repo()` | `&Arc<SqliteGoalRepository>` | Storage |
| `cns_runtime()` | `&Arc<RwLock<CnsRuntime>>` | CNS |
| `cybernetics_loop()` | `&Arc<RwLock<CyberneticsLoop>>` | CNS |
| `loop_system()` | `&Arc<LoopSystem>` | CNS |
| `event_sink()` | `&Arc<dyn NuEventSink>` | CNS |
| `seam_watcher()` | `&Arc<RwLock<Option<SeamWatcher>>>` | CNS (R7.3) |
| `capability_checker()` | `&Arc<CapabilityChecker>` | Governance |
| `mcp_dispatcher()` | `&Arc<McpDispatcher>` | Governance |
| `escalation_queue()` | `&Arc<EscalationQueue>` | Governance |
| `inference_port()` | `Option<Arc<dyn InferencePort>>` | Coordination |
| `mcp_runtime()` | `&Arc<McpRuntime>` | Coordination |
| `active_pods()` | `&Arc<ActivePods>` | Coordination |
| `identity()` | `(&WebID, &Arc<hkask_agents::A2ARuntime>)` | Identity |
| `sovereignty()` | `SovereigntyService` (wraps `consent_manager`) | Sovereignty |
| `curation_inbox_tx()` | `&Option<mpsc::UnboundedSender<CurationInput>>` | Internal |
| `sovereignty_boundary_store()` | `&SovereigntyBoundaryStore` | Sovereignty |
| `spec_store()` | `&SqliteSpecStore` | Surface-specific |
| `agent_registry_store()` | `&hkask_storage::AgentRegistryStore` | Surface-specific |
| `user_store()` | `&Arc<std::sync::Mutex<UserStore>>` | Surface-specific |
| `daemon_handler()` | `&Arc<ServiceDaemonHandler>` | Daemon |
| `matrix_transport()` | `Option<&Arc<tokio::sync::Mutex<MatrixTransport>>>` | Communication |

**Design rationale:** Individual accessors replaced the 8-group-method tuple pattern because:
- Callers typically need one field, not an entire domain group — tuple destructuring forced unnecessary binding of unused fields
- Individual methods are self-documenting: `svc.active_pods()` is clearer than `let (_, _, pm, _) = svc.coordination()`
- Two small tuples remain where the pair is always used together: `memory()` (episodic + semantic always co-accessed) and `identity()` (WebID + A2A runtime always co-accessed)

### 1.3 Vocabulary Allocation

| Term | Domain | Definition |
|------|--------|-----------|
| `AgentService` | WordAct | Canonical service layer owning all shared infrastructure |
| `NamedAccessor` | FlowDef | Single-field accessor method returning a reference to one private field |

### 1.4 Focusing Assumptions

| ID | Statement | Rationale |
|----|-----------|-----------|
| FA-AS1 | All fields are used (verified by call-site audit) | No fields can be deleted without breaking functionality |
| FA-AS2 | Individual named accessors — one per field or small coherent pair | Callers typically need one field; tuple destructuring forced unnecessary binding |
| FA-AS3 | Strangler-fig migration from group methods to individual accessors is complete | All surfaces use individual accessors; old group methods deleted |

---

## 2. Composition Spec — AgentService Interface

### 2.1 Public API (Individual Named Accessors)

```rust
impl AgentService {
    pub async fn build(config: ServiceConfig) -> Result<Self, ServiceError>;
    pub fn build_per_agent_memory(db: Database) -> PerAgentMemory;

    // Configuration
    pub fn config(&self) -> &ServiceConfig;

    // Payments
    pub fn wallet(&self) -> Option<&Arc<WalletService>>;
    pub fn wallet_store(&self) -> Option<&Arc<WalletStore>>;

    // Memory
    pub fn memory(&self) -> (&Arc<dyn EpisodicStoragePort>, &Arc<dyn SemanticStoragePort>);

    // Storage
    pub fn registry(&self) -> &Arc<tokio::sync::Mutex<SqliteRegistry>>;
    pub fn goal_repo(&self) -> &Arc<SqliteGoalRepository>;

    // CNS
    pub fn cns_runtime(&self) -> &Arc<RwLock<CnsRuntime>>;
    pub fn cybernetics_loop(&self) -> &Arc<RwLock<CyberneticsLoop>>;
    pub fn loop_system(&self) -> &Arc<LoopSystem>;
    pub fn event_sink(&self) -> &Arc<dyn NuEventSink>;
    pub fn seam_watcher(&self) -> &Arc<RwLock<Option<SeamWatcher>>>;

    // Governance
    pub fn capability_checker(&self) -> &Arc<CapabilityChecker>;
    pub fn mcp_dispatcher(&self) -> &Arc<McpDispatcher>;
    pub fn escalation_queue(&self) -> &Arc<EscalationQueue>;

    // Coordination
    pub fn inference_port(&self) -> Option<Arc<dyn InferencePort>>;
    pub fn mcp_runtime(&self) -> &Arc<McpRuntime>;
    pub fn active_pods(&self) -> &Arc<ActivePods>;

    // Identity
    pub fn identity(&self) -> (&WebID, &Arc<hkask_agents::A2ARuntime>);

    // Sovereignty
    pub fn sovereignty(&self) -> SovereigntyService;

    // Internal / surface-specific
    pub(crate) fn a2a_runtime(&self) -> &Arc<hkask_agents::A2ARuntime>;
    pub fn curation_inbox_tx(&self) -> &Option<mpsc::UnboundedSender<CurationInput>>;
    pub fn sovereignty_boundary_store(&self) -> &SovereigntyBoundaryStore;
    pub fn spec_store(&self) -> &SqliteSpecStore;
    pub fn agent_registry_store(&self) -> &hkask_storage::AgentRegistryStore;
    pub fn user_store(&self) -> &Arc<std::sync::Mutex<UserStore>>;
    pub fn daemon_handler(&self) -> &Arc<ServiceDaemonHandler>;
    pub fn matrix_transport(&self) -> Option<&Arc<tokio::sync::Mutex<MatrixTransport>>>;
}
```

### 2.2 Field Inventory (28 Private Fields)

| Field | Type | Category |
|-------|------|----------|
| `registry` | `Arc<tokio::sync::Mutex<SqliteRegistry>>` | Storage |
| `mcp_runtime` | `Arc<McpRuntime>` | Coordination |
| `mcp_dispatcher` | `Arc<McpDispatcher>` | Governance |
| `cns_runtime` | `Arc<RwLock<CnsRuntime>>` | CNS |
| `cybernetics_loop` | `Arc<RwLock<CyberneticsLoop>>` | CNS |
| `loop_system` | `Arc<LoopSystem>` | CNS |
| `inference_port` | `Option<Arc<dyn InferencePort>>` | Coordination |
| `episodic_storage` | `Arc<dyn EpisodicStoragePort>` | Memory |
| `semantic_storage` | `Arc<dyn SemanticStoragePort>` | Memory |
| `escalation_queue` | `Arc<EscalationQueue>` | Governance |
| `consent_manager` | `Arc<ConsentManager>` | Sovereignty |
| `goal_repo` | `Arc<SqliteGoalRepository>` | Storage |
| `curation_inbox_tx` | `Option<mpsc::UnboundedSender<CurationInput>>` | Internal |
| `active_pods` | `Arc<ActivePods>` | Coordination |
| `capability_checker` | `Arc<CapabilityChecker>` | Governance |
| `system_webid` | `WebID` | Identity |
| `event_sink` | `Arc<dyn NuEventSink>` | CNS |
| `sovereignty_boundary_store` | `SovereigntyBoundaryStore` | Sovereignty |
| `spec_store` | `SqliteSpecStore` | Surface-specific |
| `a2a_runtime` | `Arc<hkask_agents::A2ARuntime>` | Identity |
| `agent_registry_store` | `hkask_storage::AgentRegistryStore` | Surface-specific |
| `user_store` | `Arc<std::sync::Mutex<UserStore>>` | Surface-specific |
| `daemon_handler` | `Arc<ServiceDaemonHandler>` | Daemon |
| `matrix_transport` | `Option<Arc<tokio::sync::Mutex<MatrixTransport>>>` | Communication |
| `seam_watcher` | `Arc<RwLock<Option<SeamWatcher>>>` | CNS (R7.3) |
| `config` | `ServiceConfig` | Configuration |
| `wallet_service` | `Option<Arc<WalletService>>` | Payments |
| `wallet_store` | `Option<Arc<WalletStore>>` | Payments |



### 2.3 Interface Equivalence

| Accessor | CLI Uses | API Uses | Equivalent? |
|----------|----------|---------|-------------|
| `config()` | ✅ (all commands) | ✅ (all routes) | ✅ Yes |
| `wallet()` | ✅ (wallet commands) | ✅ (wallet routes) | ✅ Yes |
| `memory()` | ✅ (chat, REPL) | ✅ (episodic, semantic routes) | ✅ Yes |
| `registry()` | ✅ (templates, bundles) | ✅ (templates, bundles) | ✅ Yes |
| `goal_repo()` | ✅ (goals) | ✅ (goals) | ✅ Yes |
| `cns_runtime()` | ✅ (CNS commands) | ✅ (CNS routes) | ✅ Yes |
| `loop_system()` | ✅ (loops, serve) | ✅ (start_loops) | ✅ Yes |
| `inference_port()` | ✅ (chat, compose) | ✅ (compose) | ✅ Yes |
| `mcp_runtime()` | ✅ (MCP commands) | ✅ (MCP routes) | ✅ Yes |
| `active_pods()` | ✅ (pods) | ✅ (pods, ACP) | ✅ Yes |
| `sovereignty()` | ✅ (sovereignty) | ✅ (sovereignty) | ✅ Yes |
| `daemon_handler()` | ✅ (daemon) | ❌ | N/A (daemon only) |
| `matrix_transport()` | ✅ (REPL) | ❌ | N/A (REPL only) |

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
| `svc.memory()` | `episodic_memory:read/write` + `semantic_memory:read/write` | Scoped to agent WebID |
| `svc.mcp_dispatcher()` | `tools:execute` | Per-tool capability |
| `svc.active_pods()` | `pods:create/manage` | Agent-scoped |
| `svc.sovereignty()` | `consent:manage` | User-scoped (P1) |

---

## 4. Lifecycle Spec — Bootstrap & Database Pattern

### 4.1 Bootstrap Sequence

```
1. ServiceConfig::from_env() / from_secrets() / in_memory()
2. AgentService::build(config)
   ├── System identity (WebID from agent name)
   ├── Database connection (single shared Arc<Mutex<Connection>>)
   │   ├── in_memory: true  → single in-memory DB shared across all stores
   │   └── in_memory: false → file-backed SQLCipher DB at db_path
   ├── Stores (consent, escalation, goals, sovereignty, specs, users)
   ├── CNS runtime + event sink + seam watcher (R7.3)
   ├── Loop system (Cybernetics, Inference, Episodic, Semantic, Curation, Snapshot, Backup)
   ├── GovernedTool membrane + MCP dispatcher
   ├── Pod manager + capability checker + A2A runtime
   ├── Daemon handler + Unix socket listener (skipped in in_memory mode)
   ├── Matrix transport + 7R7 listener (non-blocking, skipped if Conduit unavailable)
   ├── Registry + agent registry store (ACP state restored from persistent storage)
   ├── Wallet (rJoule payments, deposit monitor, replicant wallet binding)
   └── Memory adapters (episodic + semantic storage ports via MemoryLoopAdapter)
3. Surface wraps AgentService:
   ├── CLI: ReplState { agent_service, prompt_state, ... }
   └── API: ApiState { agent_service: Arc<AgentService>, spec_store, git_cas, wallet_service, ... }
```

### 4.2 In-Memory Database Pattern

When `ServiceConfig::in_memory == true`, `AgentService::build()` creates a **single shared in-memory `Database`** and distributes clones of its `Arc<Mutex<Connection>>` to every store:

```
Database::in_memory()
    → shared_conn: Arc<Mutex<Connection>>
        → primary_conn    → SqliteRegistry, AgentRegistryStore, NuEventStore (CNS events)
        → consent_conn    → ConsentStore → ConsentManager
        → escalation_conn → EscalationQueue
        → goal_conn       → SqliteGoalRepository, NuEventStore (goal telemetry)
        → sovereignty_conn → SovereigntyBoundaryStore
        → spec_conn       → SqliteSpecStore
        → user_conn       → UserStore
        → mem_conn        → TripleStore (×3) + EmbeddingStore → EpisodicMemory, SemanticMemory, MemoryLoopAdapter
        → wallet_conn     → WalletStore
```

**Design intent:** A single shared connection enables cross-store operations — consent records visible to CNS, goals visible to memory, wallet transactions observable by CNS event sink. In production (`in_memory: false`), the same `Arc<Mutex<Connection>>` sharing pattern applies to the file-backed database.

**Test constructor:** `ServiceConfig::in_memory()` creates a config with `in_memory: true`, synthetic secrets (zero-filled ACP/MCP keys), and test agent name. Used by all integration tests.

### 4.3 Per-Agent Memory

`AgentService::build_per_agent_memory(db: Database)` constructs agent-scoped memory infrastructure from a dedicated `Database` connection:

```rust
pub struct PerAgentMemory {
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,
    pub semantic_storage: Arc<dyn SemanticStoragePort>,
    pub consolidation_service: hkask_memory::ConsolidationService,
}
```

All three components share the same underlying DB connection, so consolidation operates on the agent's actual episodic and semantic triples. This is used by the REPL to build agent-scoped memory separate from the shared `AgentService` memory adapted for loops.

### 4.4 Evolution (Complete)

The strangler-fig migration from 8 group methods to individual named accessors is **complete** as of v0.27.0:

| Phase | Action | Status |
|-------|--------|--------|
| **Phase 1** | Add 8 group methods to `AgentService` | ✅ Complete |
| **Phase 2** | Route CNS callers through `cns()` group method | ✅ Complete |
| **Phase 3** | Delete `CnsService` pass-through module | ✅ Complete |
| **Phase 4** | Route remaining callers through group methods per domain | ✅ Complete |
| **Phase 5** | Delete old group methods, replace with individual named accessors | ✅ Complete |

All surfaces now use individual named accessor methods. No group methods remain.

---

## 5. Curation Spec — Coherence

### 5.1 Coherence Metric

**Method:** Jaccard similarity of declared vs. registered accessor methods

```
coherence = |declared ∩ registered| / |declared ∪ registered|
```

**Threshold:** 0.7 (70% overlap)

**Declared Methods:** 26 individual named accessors (see §2.1)

**Registered Methods:** All 26 must be present and return correct types

### 5.2 Curation Decision

**Decision:** ✅ **Accept**

**Rationale:**
- All 28 fields are accounted for via individual named accessor methods
- Each method returns a single field or small coherent pair (memory, identity)
- Private fields + public methods enforce encapsulation (P4 Clear Boundaries)
- Strangler-fig migration complete — no deprecated paths remain

---

## 6. Test Program — REQ Tags

### 6.1 MDS Category → Test Strategy

| MDS Category | Test Strategy | REQ Tags |
|-------------|--------------|----------|
| **Domain** | AgentService construction + field inventory | `// REQ-MDS-D1` |
| **Composition** | Individual accessors return correct types | `// REQ-MDS-C1` |
| **Trust** | Direct field access fails to compile | `// REQ-MDS-T1` |
| **Lifecycle** | Bootstrap sequence completes without error | `// REQ-MDS-L1` |
| **Database** | In-memory DB shared across all stores | `// REQ-MDS-DB1` |

### 6.2 Tracer Bullets (Priority Order)

1. **P0 (Security):** `// REQ-MDS-T1` — Direct field access fails to compile
2. **P1 (Correctness):** `// REQ-MDS-C1` — All 26 accessor methods exist and return correct types
3. **P1 (Correctness):** `// REQ-MDS-D1` — AgentService::build() assembles all fields
4. **P1 (Correctness):** `// REQ-MDS-DB1` — In-memory mode shares a single DB connection across all stores
5. **P2 (Lifecycle):** `// REQ-MDS-L1` — Bootstrap completes in <5 seconds

---

## 7. Open Questions

| ID | Question | Decision Criteria |
|----|----------|------------------|
| F1 | Should accessor methods be public? | **Yes** — surfaces need to access them |
| F2 | Should accessor methods be async? | **No** — fields are Arc'd, no I/O in method body |
| F3 | Should we add builder pattern for AgentService? | **No** — `build()` is sufficient |
| F4 | Should we add `#[non_exhaustive]` to AgentService? | **Yes** — prevents struct literal construction |
| F5 | Should `spec_store`, `agent_registry_store`, `user_store` move to `ApiState`? | **Open** — currently on AgentService with TODO markers; they are surface-specific fields |
| F6 | Should `sovereignty_boundary_store` be removed from public access? | **Open** — currently has a public accessor (strangler-fig artifact); all callers should migrate to `sovereignty()` |

---

*ℏKask — A Minimal Viable Container for Agents — v0.28.0*
