---
title: "Hexagonal Port Inventory — hKask v0.21.0"
version: "0.21.0"
status: "Active"
last_updated: "2026-05-24"
---

# Hexagonal Port Inventory — hKask v0.21.0

Comprehensive inventory of all hexagonal ports, adapters, and architectural patterns in hKask v0.21.0.

## Table of Contents

1. [Hexagonal Architecture Principles](#hexagonal-architecture-principles)
2. [Port Classification](#port-classification)
3. [Core Ports](#core-ports)
4. [Capability Ports](#capability-ports)
5. [Storage Ports](#storage-ports)
6. [Transport Ports](#transport-ports)
7. [Port Composition Patterns](#port-composition-patterns)
8. [Dependency Flow](#dependency-flow)
9. [Testing Patterns](#testing-patterns)

---

## Hexagonal Architecture Principles

hKask follows Alastair Cockburn's hexagonal architecture (ports and adapters) with these principles:

### 1. Dependency Rule

**All dependencies point inward.** Domain logic depends on ports; adapters depend on ports. Ports never depend on adapters.

```
┌─────────────────────────────────────────┐
│              Adapters                   │
│  ┌──────────┐  ┌──────────┐            │
│  │ AcpRuntime│  │McpDispatcher│         │
│  └────┬─────┘  └────┬─────┘            │
│       │             │                   │
│  ┌────▼─────────────▼────┐             │
│  │        Ports          │             │
│  │  ┌────────┐ ┌──────┐ │             │
│  │  │AcpPort │ │McpPort│ │             │
│  │  └────────┘ └──────┘ │             │
│  └───────────────────────┘             │
│              │                          │
│  ┌───────────▼───────────┐             │
│  │    Domain Logic       │             │
│  │  (AgentPod, ACP, etc) │             │
│  └───────────────────────┘             │
└─────────────────────────────────────────┘
```

### 2. Async Purity (T10)

All ports use `#[async_trait]` for async methods. No `block_in_place` or `block_on` in library code.

```rust
#[async_trait::async_trait]
pub trait McpPort: Send + Sync {
    async fn discover_tools(&self) -> Vec<String>;
    async fn invoke(&self, tool_name: &str, input: Value) -> Result<Value>;
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
}
```

### 3. Single Capability Primitive (T08)

All access control uses `CapabilityToken` with caveats. No parallel capability systems.

### 4. No Stubs in Production (T09, T18)

All adapters implement real functionality. `PlaceholderGitCAS` deleted; `MockGitCas` moved to test crate.

### 5. Typed Errors (T15)

No `unwrap()` on hot paths. All fallible operations return `Result<T, Error>`.

---

## Port Classification

Ports are classified by their role in the system:

| Classification | Purpose | Examples |
|---------------|---------|----------|
| **Driving Port** | Inbound requests from external systems | `AcpPort`, `McpPort` |
| **Driven Port** | Outbound calls to external systems | `InferencePort`, `MemoryPort` |
| **Capability Port** | Access control and authorization | `CapabilityValidator`, `CapabilityQueryPort` |
| **Storage Port** | Persistence operations | `GoalStoragePort`, `MemoryStoragePort` |
| **Transport Port** | Communication protocols | `McpTransport`, `AcpTransport` |

---

## Core Ports

### hkask-agents

#### AcpPort

**Location:** `crates/hkask-agents/src/ports/acp.rs`

**Purpose:** Agent Communication Protocol — agent registration, A2A messaging, capability management.

**Interface:**
```rust
#[async_trait]
pub trait AcpPort: Send + Sync {
    async fn register_agent(
        &self,
        webid: WebID,
        agent_type: &str,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, AcpError>;

    async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError>;

    async fn send_message(&self, msg: A2AMessage) -> Result<String, AcpError>;

    async fn list_capabilities(&self, webid: &WebID) -> Result<Vec<String>, AcpError>;

    async fn is_registered(&self, webid: &WebID) -> bool;
}
```

**Adapters:**
- `AcpRuntime` — In-process implementation with rate limiting, audit logging, capability verification
- `RussellAcpAdapter` — Bidirectional bridge to Russell ACP server over stdio JSON-RPC

**Security:**
- Rejects wildcard capabilities (`"*"`)
- Rate limiting per agent (default: 100 msg/min)
- Audit logging for all A2A messages
- Capability verification on message routing

---

#### GitCASPort

**Location:** `crates/hkask-agents/src/pod.rs`

**Purpose:** Git content-addressed storage for template crate loading.

**Interface:**
```rust
pub trait GitCASPort {
    fn load_template_crate(&self, crate_name: &str) -> Result<TemplateCrate, GitError>;
    fn resolve_sha(&self, crate_name: &str) -> Result<String, GitError>;
}
```

**Adapters:**
- `GitCasAdapter` — Real implementation using `gix` crate for Git operations
- `MockGitCas` — Test double (moved to `hkask-testing` crate)

**Security:**
- Path traversal prevention
- SHA verification for template provenance

---

#### MCPRuntimePort

**Location:** `crates/hkask-agents/src/pod.rs`

**Purpose:** MCP tool invocation with capability enforcement.

**Interface:**
```rust
pub trait MCPRuntimePort {
    fn grant_tool_access(&self, token: CapabilityToken) -> Result<(), McpError>;
    fn invoke_tool(
        &self,
        tool_name: &str,
        input: Value,
        token: &CapabilityToken,
    ) -> Result<Value, McpError>;
}
```

**Adapters:**
- `McpRuntimeAdapter` — Delegates to `hkask-mcp::McpDispatcher`

**Security:**
- Capability verification before tool invocation
- Rate limiting per tool

---

#### MemoryStoragePort

**Location:** `crates/hkask-agents/src/pod.rs`

**Purpose:** Artifact persistence for episodic and semantic memory.

**Interface:**
```rust
pub trait MemoryStoragePort {
    fn store_artifact(
        &self,
        producer_webid: WebID,
        artifact_type: &str,
        content: Value,
        visibility: &str,
        token: &CapabilityToken,
    ) -> Result<String, MemoryError>;

    fn recall(&self, query: &str, token: &CapabilityToken) -> Result<Vec<Value>, MemoryError>;
}
```

**Adapters:**
- `MemoryStorageAdapter` — SQLite-backed implementation using `hkask-storage`

**Security:**
- Visibility enforcement (private, shared, public)
- Capability verification for write operations

---

#### CnsEmit (re-exported from hkask-cns)

**Location:** `crates/hkask-cns/src/spans.rs`

**Purpose:** CNS span emission for observability.

**Interface:**
```rust
pub trait CnsEmit: Send + Sync {
    fn emit_event(&self, span: &str, phase: &str, observation: &Value, confidence: f64);
}
```

**Adapters:**
- `CnsEmitterAdapter` — Wraps `hkask-cns::CnsRuntime`
- `CnsRuntime` — Direct implementation

**Security:**
- All capability mutations emit spans (T13)

---

#### KeystorePort

**Location:** `crates/hkask-agents/src/adapters/keystore_port.rs`

**Purpose:** Secret management and retrieval.

**Interface:**
```rust
pub trait KeystorePort: Send + Sync {
    fn retrieve(&self, key: &str) -> Result<Secret, KeystoreError>;
    fn store(&self, key: &str, secret: &[u8]) -> Result<(), KeystoreError>;
}
```

**Adapters:**
- `KeychainAdapter` — OS keychain integration (macOS Keychain, Linux Secret Service, Windows Credential Manager)

**Security:**
- Secrets wrapped in `Zeroizing<Vec<u8>>`
- No byte copying on Clone (uses `Arc`)

---

#### SovereigntyPort

**Location:** `crates/hkask-agents/src/ports/sovereignty.rs`

**Purpose:** User sovereignty enforcement at pod level.

**Interface:**
```rust
pub trait SovereigntyPort {
    fn check(
        &self,
        data_category: DataCategory,
        operation: SovereigntyOperation,
        requester: &WebID,
    ) -> SovereigntyCheckResult;

    fn can_access(&self, data_category: DataCategory, requester: &WebID) -> bool;
    fn mark_acquisition_attempt(&mut self, details: &Value);
    fn update_vc_investment(&mut self, vc_investment: f32);
    fn is_compromised(&self) -> bool;
    fn grant_consent(&mut self);
    fn revoke_consent(&mut self);
    fn owner_webid(&self) -> WebID;
}
```

**Adapters:**
- `ConsentManager` — Tracks consent state and sovereignty boundaries

**Security:**
- Enforces user sovereignty over data
- Tracks acquisition attempts for monitoring

---

### hkask-templates

#### InferencePort

**Location:** `crates/hkask-templates/src/inference_port.rs`

**Purpose:** LLM inference with high-temperature generation.

**Interface:**
```rust
#[async_trait]
pub trait InferencePort: Send + Sync {
    async fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError>;

    async fn generate_n(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        n: usize,
    ) -> Result<Vec<InferenceResult>, InferenceError>;
}
```

**Adapters:**
- `OkapiInference` — Okapi API integration with retry, circuit breaker, rate limiting

**Security:**
- Shared HTTP client for connection pooling (T16)
- Concurrent `generate_n` using `futures_util::join_all`

---

#### McpPort

**Location:** `crates/hkask-templates/src/ports.rs`

**Purpose:** MCP tool dispatch for template execution.

**Interface:**
```rust
#[async_trait::async_trait]
pub trait McpPort: Send + Sync {
    async fn discover_tools(&self) -> Vec<String>;
    async fn invoke(&self, tool_name: &str, input: Value) -> Result<Value>;
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
}
```

**Adapters:**
- `McpDispatcher` — Dispatches to `hkask-mcp::McpRuntime` with capability verification

**Security:**
- Capability verification before tool invocation
- Rate limiting per tool

---

#### CnsPort (re-exported from hkask-cns)

**Location:** `crates/hkask-templates/src/ports.rs`

**Purpose:** CNS observability for template execution.

**Interface:** Same as `CnsEmit` above.

**Adapters:**
- `CnsRuntime` — Direct implementation

---

#### MemoryPort

**Location:** `crates/hkask-templates/src/ports.rs`

**Purpose:** Semantic and episodic recall for template context.

**Interface:**
```rust
pub trait MemoryPort: Send + Sync {
    fn query_semantic(&self, entity: &str) -> Result<Vec<MemoryFragment>>;
    fn query_episodic(&self, entity: &str, perspective: &str) -> Result<Vec<MemoryFragment>>;
    fn get_session_history(&self, session_id: &str, max_messages: usize) -> Result<Vec<String>>;
}
```

**Adapters:**
- `MemoryStorageAdapter` — SQLite-backed implementation

---

#### ManifestExecutor

**Location:** `crates/hkask-templates/src/ports.rs`

**Purpose:** YAML-based workflow execution.

**Interface:**
```rust
#[async_trait::async_trait]
pub trait ManifestExecutor: Send + Sync {
    fn load(&self, path: &Path) -> Result<ProcessManifest>;
    async fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value>;
}
```

**Adapters:**
- `ManifestExecutorImpl` — Full implementation with energy accounting, CSP enforcement
- `SimpleExecutor` — Minimal implementation for testing

---

## Capability Ports

### hkask-templates

#### CapabilityValidator

**Location:** `crates/hkask-templates/src/capability_validator.rs`

**Purpose:** OCAP validation for template execution.

**Interface:**
```rust
pub trait CapabilityValidator {
    fn validate_capability(
        &self,
        token: &CapabilityToken,
        template_id: &str,
    ) -> Result<(), TemplateError>;
}
```

**Adapters:**
- `CapabilityAwareValidator` — Verifies `template:render` capability for specific template

---

### hkask-ensemble

#### CapabilityQueryPort

**Location:** `crates/hkask-ensemble/src/ocap_enforcement.rs`

**Purpose:** Capability queries for OCAP enforcement.

**Interface:**
```rust
#[async_trait::async_trait]
pub trait CapabilityQueryPort: Send + Sync {
    async fn has_capability(&self, webid: WebID, operation: OkapiOperation) -> bool;
    async fn get_capabilities(&self, webid: WebID) -> Option<Vec<CapabilityToken>>;
}
```

**Adapters:**
- `WebIDCapabilityRegistry` — Registry of WebID-to-capability mappings

---

## Storage Ports

### hkask-storage

#### GoalStoragePort

**Location:** `crates/hkask-storage/src/goals.rs`

**Purpose:** Goal persistence.

**Interface:**
```rust
pub trait GoalStoragePort: Send + Sync {
    fn store_goal(&self, goal: &Goal) -> Result<String, StorageError>;
    fn retrieve_goal(&self, id: &str) -> Result<Goal, StorageError>;
    fn list_goals(&self, owner: &WebID) -> Result<Vec<Goal>, StorageError>;
}
```

**Adapters:**
- `GoalStore` — SQLite-backed implementation

---

### hkask-memory

#### GoalMemoryPort

**Location:** `crates/hkask-memory/src/goal_memory.rs`

**Purpose:** Goal memory with semantic search.

**Interface:**
```rust
pub trait GoalMemoryPort: Send + Sync {
    fn remember_goal(&self, goal: &Goal) -> Result<(), MemoryError>;
    fn recall_goals(&self, query: &str, limit: usize) -> Result<Vec<Goal>, MemoryError>;
}
```

**Adapters:**
- `GoalMemory` — sqlite-vec backed implementation with embedding search

---

## Transport Ports

### hkask-mcp

#### McpTransport

**Location:** `crates/hkask-mcp/src/transport.rs`

**Purpose:** MCP server communication protocols.

**Interface:**
```rust
#[async_trait]
pub trait McpTransport: Send + Sync + fmt::Debug {
    async fn call(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, String>;

    fn is_connected(&self) -> bool;
}
```

**Adapters:**

| Adapter | Protocol | Use Case |
|---------|----------|----------|
| `InProcessMcpTransport` | In-process function calls | Co-located servers; no network overhead |
| `StdioMcpTransport` | JSON-RPC over stdin/stdout | Child process servers |
| `HttpMcpTransport` | HTTPS with JSON-RPC | Remote servers |

**Security:**
- `InProcessMcpTransport`: No network; handlers registered in-process
- `StdioMcpTransport`: No network; child process isolation
- `HttpMcpTransport`: HTTPS with capability token authentication

---

### hkask-agents

#### AcpTransport

**Location:** `crates/hkask-agents/src/ports/acp_transport.rs`

**Purpose:** ACP wire protocol for inter-agent communication.

**Interface:**
```rust
#[async_trait]
pub trait AcpTransport: Send + Sync {
    async fn send(&self, msg: &AcpWireMessage) -> Result<AcpWireResponse, AcpError>;
    async fn receive(&self) -> Result<AcpWireMessage, AcpError>;
    fn is_connected(&self) -> bool;
}
```

**Adapters:**
- `LoopbackHttpTransport` — HTTP over loopback only (rejects non-loopback addresses)
- `StdioTransport` — JSON-RPC over stdin/stdout for child processes

**Security:**
- Loopback-only binding prevents network exposure
- Capability tokens required for all messages

---

## Port Composition Patterns

### Pattern 1: Adapter Chaining

Adapters can wrap other adapters to add cross-cutting concerns:

```rust
// McpDispatcher wraps McpRuntime to add capability verification
pub struct McpDispatcher {
    runtime: McpRuntime,
    capability_checker: Arc<CapabilityChecker>,
    rate_limiter: RateLimiter,
}

impl McpPort for McpDispatcher {
    async fn invoke(&self, tool_name: &str, input: Value) -> Result<Value> {
        // 1. Verify capability
        // 2. Check rate limit
        // 3. Delegate to runtime
        self.runtime.call_tool(server_id, tool_name, input).await
    }
}
```

### Pattern 2: Port Aggregation

Multiple ports can be aggregated into a single facade:

```rust
pub struct PodManager {
    git_cas: GitCasAdapter,
    acp_runtime: Arc<dyn AcpPort + Send + Sync>,
    cns_emitter: CnsEmitterAdapter,
    mcp_runtime: McpRuntimeAdapter,
    memory_storage: Arc<Mutex<MemoryStorageAdapter>>,
}
```

### Pattern 3: Optional CNS Integration

CNS emitter is optional to avoid breaking tests:

```rust
pub struct AcpRuntime {
    // ...
    cns_emitter: Option<Arc<dyn hkask_cns::CnsEmit + Send + Sync>>,
}

impl AcpRuntime {
    pub fn with_cns_emitter(mut self, emitter: Arc<dyn hkask_cns::CnsEmit + Send + Sync>) -> Self {
        self.cns_emitter = Some(emitter);
        self
    }
}
```

---

## Dependency Flow

### Crate Dependencies

```
hkask-types (foundation)
    ↑
    ├── hkask-cns (observability)
    ├── hkask-keystore (secrets)
    ├── hkask-storage (persistence)
    │       ↑
    │       └── hkask-memory (semantic search)
    │
    ├── hkask-templates (execution)
    │       ↑
    │       └── hkask-agents (lifecycle)
    │               ↑
    │               └── hkask-mcp (tools)
    │
    └── hkask-ensemble (multi-agent)
```

### Port Dependencies

**Rule:** Ports depend only on types; adapters depend on ports and external systems.

```
hkask-types::CapabilityToken (type)
    ↑
hkask-agents::ports::AcpPort (port)
    ↑
hkask-agents::acp::AcpRuntime (adapter)
    ↑
hkask-agents::adapters::RussellAcpAdapter (adapter wrapping adapter)
```

---

## Testing Patterns

### Pattern 1: Mock Adapters

Test doubles implement ports for isolated testing:

```rust
// hkask-testing/src/test_harnesses/mocks.rs
pub struct MockMcpPort {
    tools: Arc<RwLock<HashMap<String, bool>>>,
}

#[async_trait::async_trait]
impl McpPort for MockMcpPort {
    async fn discover_tools(&self) -> Vec<String> {
        let tools = self.tools.read().unwrap();
        tools.iter().filter(|&(_, &enabled)| enabled).map(|(name, _)| name.clone()).collect()
    }
    // ...
}
```

### Pattern 2: In-Memory Storage

Use in-memory SQLite for fast tests:

```rust
let memory_storage = MemoryStorageAdapter::in_memory()
    .expect("In-memory storage initialization should never fail");
```

### Pattern 3: Builder Pattern

Use builders to construct test fixtures:

```rust
let pod_manager = PodManagerBuilder::new()
    .git_cas(MockGitCas::new())
    .acp_runtime(Arc::new(AcpRuntime::default()))
    .with_in_memory_storage()
    .build();
```

---

## Design Principles Summary

1. **Single capability primitive**: `CapabilityToken` with caveats (T08)
2. **Async purity**: All ports use `#[async_trait]` (T10)
3. **No stubs in production**: All adapters implement real functionality (T09, T18)
4. **Typed errors**: No `unwrap()` on hot paths (T15)
5. **Deterministic identity**: WebIDs derived from persona content (T06)
6. **Dependency rule**: All dependencies point inward
7. **Port aggregation**: Multiple ports composed into facades
8. **Optional CNS**: CNS emitter is optional to avoid breaking tests

---

## See Also

- `docs/plans/ADV-REVIEW-F2.md` — Adversarial review and remediation plan
- `docs/plans/IMPLEMENTATION-PLAN-F2.md` — Detailed implementation tasks
- `docs/architecture/security-architecture.md` — Security architecture details
- `docs/architecture/hKask-architecture-master.md` — System architecture
