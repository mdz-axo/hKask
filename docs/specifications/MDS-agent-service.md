---
title: "MDS — AgentService Specification"
audience: [architects, developers, agents]
last_updated: 2026-06-09
version: "0.27.1"
status: "Active"
domain: "Composition"
mds_categories: [domain, composition, trust, lifecycle]
---

# MDS — AgentService Specification

**Purpose:** Specification for the condensed service layer architecture. `AgentService` (formerly `ServiceContext`) is the canonical service layer that owns all shared infrastructure.

**Supersedes:** `hKask-architecture-master.md` §Service Layer (v0.27.0)

**Related:** [`PRINCIPLES.md`](PRINCIPLES.md), [`MDS.md`](MDS.md), [`condensed-erd.md`](condensed-erd.md)

---

## 1. Domain Spec — AgentService

### 1.1 Bounded Context

`AgentService` is the **single source of truth** for all shared infrastructure in hKask. Both CLI and API surfaces compose an `AgentService` instance and add only presentation-specific state.

**Boundary:** In-process only. MCP servers do NOT depend on `AgentService` (P1 Prohibition — out-of-process isolation).

### 1.2 Domain Grouping (7 Categories)

All 27 fields are grouped into 7 domain categories by cohesion:

| Category | Fields | Purpose | Accessor Pattern |
|----------|--------|---------|-----------------|
| **Memory** | `episodic_storage`, `semantic_storage` | Private + public memory adapters | `agent_service.memory().episodic()` |
| **CNS** | `cns_runtime`, `cns`, `cybernetics_loop`, `loop_system` | Cybernetic regulation | `agent_service.cns().runtime()` |
| **Governance** | `mcp_runtime`, `mcp_dispatcher`, `capability_checker`, `governed_tool` | OCAP enforcement | `agent_service.governance().dispatcher()` |
| **Storage** | `registry`, `goal_repo`, `standing_session_store`, `sovereignty_boundary_store`, `spec_store`, `agent_registry_store`, `user_store` | Persistent storage | `agent_service.storage().registry()` |
| **Coordination** | `escalation_queue`, `curation_inbox_tx`, `session_manager`, `pod_manager`, `acp_runtime` | Multi-agent coordination | `agent_service.coordination().session_manager()` |
| **Identity** | `system_webid`, `event_sink` | System identity + audit trail | `agent_service.identity().webid()` |
| **Config** | `config` | Deployment configuration | `agent_service.config()` |

### 1.3 hLexicon Allocation

| Term | Domain | Definition |
|------|--------|-----------|
| `AgentService` | WordAct | Canonical service layer owning all shared infrastructure |
| `MemoryAdapter` | FlowDef | Episodic + semantic storage ports |
| `CNSAdapter` | FlowDef | Cybernetic runtime + regulation loops |
| `GovernanceAdapter` | FlowDef | OCAP-gated tool dispatch |
| `StorageAdapter` | FlowDef | Persistent domain stores |
| `CoordinationAdapter` | FlowDef | Multi-agent session + pod management |
| `IdentityAdapter` | FlowDef | System WebID + ν-event audit |

### 1.4 Focusing Assumptions

| ID | Statement | Rationale |
|----|-----------|-----------|
| FA-AS1 | All 27 fields are used (verified by grep audit) | No fields can be deleted without breaking functionality |
| FA-AS2 | CLI and API use different subsets of fields | Grouping allows surfaces to access only what they need |
| FA-AS3 | Domain crates use `AgentService` fields directly | Refactoring will update domain crates to use accessors |

---

## 2. Composition Spec — AgentService Interface

### 2.1 Public API (7 Accessor Methods)

```rust
impl AgentService {
    /// Build AgentService from ServiceConfig
    pub async fn build(config: ServiceConfig) -> Result<Self, ServiceError>;
    
    /// Memory domain access
    pub fn memory(&self) -> &MemoryAdapters;
    
    /// CNS domain access
    pub fn cns(&self) -> &CnsAdapters;
    
    /// Governance domain access
    pub fn governance(&self) -> &GovernanceAdapters;
    
    /// Storage domain access
    pub fn storage(&self) -> &StorageAdapters;
    
    /// Coordination domain access
    pub fn coordination(&self) -> &CoordinationAdapters;
    
    /// Identity domain access
    pub fn identity(&self) -> &IdentityAdapters;
    
    /// Configuration access
    pub fn config(&self) -> &ServiceConfig;
}
```

### 2.2 Domain Adapter Structs

#### MemoryAdapters

```rust
pub struct MemoryAdapters {
    episodic: Arc<dyn EpisodicStoragePort>,
    semantic: Arc<dyn SemanticStoragePort>,
}

impl MemoryAdapters {
    pub fn episodic(&self) -> &dyn EpisodicStoragePort;
    pub fn semantic(&self) -> &dyn SemanticStoragePort;
}
```

#### CnsAdapters

```rust
pub struct CnsAdapters {
    runtime: Arc<RwLock<CnsRuntime>>,
    service: CnsService,
    cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,
    loop_system: Arc<LoopSystem>,
}

impl CnsAdapters {
    pub fn runtime(&self) -> &Arc<RwLock<CnsRuntime>>;
    pub fn service(&self) -> &CnsService;
    pub fn cybernetics_loop(&self) -> &Arc<RwLock<CyberneticsLoop>>;
    pub fn loop_system(&self) -> &Arc<LoopSystem>;
}
```

#### GovernanceAdapters

```rust
pub struct GovernanceAdapters {
    mcp_runtime: Arc<McpRuntime>,
    mcp_dispatcher: Arc<McpDispatcher>,
    capability_checker: Arc<CapabilityChecker>,
    governed_tool: Arc<GovernedTool>,
}

impl GovernanceAdapters {
    pub fn runtime(&self) -> &Arc<McpRuntime>;
    pub fn dispatcher(&self) -> &Arc<McpDispatcher>;
    pub fn capability_checker(&self) -> &Arc<CapabilityChecker>;
    pub fn governed_tool(&self) -> &Arc<GovernedTool>;
}
```

#### StorageAdapters

```rust
pub struct StorageAdapters {
    registry: Arc<tokio::sync::Mutex<SqliteRegistry>>,
    goal_repo: Arc<SqliteGoalRepository>,
    standing_session_store: Arc<StandingSessionStore>,
    sovereignty_boundary_store: SovereigntyBoundaryStore,
    spec_store: SqliteSpecStore,
    agent_registry_store: AgentRegistryStore,
    user_store: Arc<std::sync::Mutex<UserStore>>,
}

impl StorageAdapters {
    pub fn registry(&self) -> &Arc<tokio::sync::Mutex<SqliteRegistry>>;
    pub fn goal_repo(&self) -> &Arc<SqliteGoalRepository>;
    pub fn standing_session_store(&self) -> &Arc<StandingSessionStore>;
    pub fn sovereignty_boundary_store(&self) -> &SovereigntyBoundaryStore;
    pub fn spec_store(&self) -> &SqliteSpecStore;
    pub fn agent_registry_store(&self) -> &AgentRegistryStore;
    pub fn user_store(&self) -> &Arc<std::sync::Mutex<UserStore>>;
}
```

#### CoordinationAdapters

```rust
pub struct CoordinationAdapters {
    escalation_queue: Arc<EscalationQueue>,
    curation_inbox_tx: Option<tokio::sync::mpsc::UnboundedSender<CurationInput>>,
    session_manager: Arc<RwLock<SessionManager>>,
    pod_manager: Arc<PodManager>,
    acp_runtime: Arc<AcpRuntime>,
}

impl CoordinationAdapters {
    pub fn escalation_queue(&self) -> &Arc<EscalationQueue>;
    pub fn curation_inbox_tx(&self) -> &Option<tokio::sync::mpsc::UnboundedSender<CurationInput>>;
    pub fn session_manager(&self) -> &Arc<RwLock<SessionManager>>;
    pub fn pod_manager(&self) -> &Arc<PodManager>;
    pub fn acp_runtime(&self) -> &Arc<AcpRuntime>;
}
```

#### IdentityAdapters

```rust
pub struct IdentityAdapters {
    system_webid: WebID,
    event_sink: Arc<dyn NuEventSink>,
}

impl IdentityAdapters {
    pub fn webid(&self) -> &WebID;
    pub fn event_sink(&self) -> &Arc<dyn NuEventSink>;
}
```

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
| `memory().episodic()` | `episodic_memory:read/write` | Scoped to agent WebID |
| `memory().semantic()` | `semantic_memory:read/write` | Public data only |
| `governance().dispatcher()` | `tools:execute` | Per-tool capability |
| `coordination().pod_manager()` | `pods:create/manage` | Agent-scoped |

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

### 4.2 Evolution (Big Bang Migration)

| Phase | Action | Verification |
|-------|--------|-------------|
| **Phase 1** | Rename `ServiceContext` → `AgentService` | `cargo check` passes |
| **Phase 2** | Make all fields private | Direct access fails to compile |
| **Phase 3** | Add 7 domain adapter structs | Type-check passes |
| **Phase 4** | Add accessor methods | All call sites updated |
| **Phase 5** | Update CLI call sites | `cargo test -p hkask-cli` passes |
| **Phase 6** | Update API call sites | `cargo test -p hkask-api` passes |
| **Phase 7** | Update domain crate call sites | `cargo test --workspace` passes |
| **Phase 8** | Delete old field access patterns | `cargo clippy -- -D warnings` passes |

### 4.3 Deprecation Policy

**No deprecation.** Big bang migration:

1. Old code (`ctx.episodic_storage`) is deleted when new code (`ctx.memory().episodic()`) is merged
2. No `#[deprecated]` attributes (P6/P7 violation)
3. Single commit per phase for easy rollback

---

## 5. Curation Spec — Coherence

### 5.1 Coherence Metric

**Method:** Jaccard similarity of declared vs. registered domain adapters

```
coherence = |declared ∩ registered| / |declared ∪ registered|
```

**Threshold:** 0.7 (70% overlap)

**Declared Adapters:** Memory, CNS, Governance, Storage, Coordination, Identity, Config (7 total)

**Registered Adapters:** (After implementation) All 7 must be present

### 5.2 Curation Decision

**Decision:** ✅ **Accept**

**Rationale:** 
- All 27 fields are accounted for in 7 domain categories
- Each category has clear cohesion (fields are related by domain)
- Accessor pattern enforces encapsulation (private fields, public methods)
- Migration path is clear (big bang, 8 phases)

---

## 6. Test Program — REQ Tags

### 6.1 MDS Category → Test Strategy

| MDS Category | Test Strategy | REQ Tags |
|-------------|--------------|----------|
| **Domain** | AgentService construction + field grouping | `// REQ-MDS-D1` |
| **Composition** | Accessor methods return correct adapters | `// REQ-MDS-C1` |
| **Trust** | Direct field access fails to compile | `// REQ-MDS-T1` |
| **Lifecycle** | Bootstrap sequence completes without error | `// REQ-MDS-L1` |

### 6.2 Tracer Bullets (Priority Order)

1. **P0 (Security):** `// REQ-MDS-T1` — Direct field access fails to compile
2. **P1 (Correctness):** `// REQ-MDS-C1` — All 7 accessor methods exist and return correct types
3. **P1 (Correctness):** `// REQ-MDS-D1` — AgentService::build() assembles all 27 fields
4. **P2 (Lifecycle):** `// REQ-MDS-L1` — Bootstrap completes in <5 seconds

---

## 7. Open Questions

| ID | Question | Decision Criteria |
|----|----------|------------------|
| F1 | Should domain adapters be pub(crate) or pub? | **pub** — surfaces need to access them |
| F2 | Should accessor methods be async? | **No** — adapters are already Arc'd |
| F3 | Should we add builder pattern for AgentService? | **No** — `build()` is sufficient |
| F4 | Should we add `#[non_exhaustive]` to AgentService? | **Yes** — prevents struct literal construction |
| F5 | Should we add compile-fail tests for field access? | **Yes** — ensures encapsulation is enforced |

---

*ℏKask — A Minimal Viable Container for Agents — v0.27.1*
