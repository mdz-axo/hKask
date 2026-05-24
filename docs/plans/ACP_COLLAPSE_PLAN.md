---
title: "Phase 1C: Collapse Dual ACP Systems"
audience: [developers, architects]
last_updated: 2026-05-24
togaf_phase: "E"
version: "1.0.0"
status: "Active"
domain: "Application"
---

# Phase 1C: Collapse Dual ACP Systems — Implementation Plan
**Priority:** P0 (Security Critical)  
**Estimated Effort:** 6 hours across 2 sessions

---

## Executive Summary

This plan addresses **Root Cause 3 (RC3): Dual ACP Systems** — a P0 architectural defect where two incompatible ACP implementations coexist with zero code sharing:

- **`AcpRuntimeAdapter`** (59 lines, stub) — Used by `PodManager`, ignores capabilities, never stores agents
- **`AcpRuntime`** (889 lines, full implementation) — Exported but orphaned, never wired into pod lifecycle

**Goal:** Unify on `AcpRuntime` via a well-defined `AcpPort` trait, delete the stub, add transport abstraction, and expose a Russell registration endpoint.

**Constraints:**
- Phase 1A (Eliminate Hardcoded Secrets) should complete first
- No cross-machine ACP (loopback-only per F3 decision)
- Maintain hexagonal architecture (ports/adapters)
- All changes must preserve existing test coverage

---

## Current State Analysis

### The Interface Mismatch

```rust
// Current port trait (pod.rs:583-598) — SYNCHRONOUS
pub trait ACPRuntimePort {
    fn register_agent(
        &self,
        webid: WebID,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, String>;  // Returns String error
}

// Real runtime (acp.rs:348-409) — ASYNC, different signature
impl AcpRuntime {
    pub async fn register_agent(
        &self,
        webid: WebID,
        agent_type: String,        // Extra parameter
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, String> { ... }
}
```

**Problems:**
1. Trait is sync, implementation is async
2. Trait lacks `agent_type` parameter
3. Both return `Result<T, String>` instead of typed `AcpError`
4. Trait has only 1 method (no `send_message`, `list_capabilities`, `unregister`)

### Dependency Map

```
PodManager (pod.rs:736)
  └── acp_runtime: AcpRuntimeAdapter (CONCRETE, not trait object)
        └── implements ACPRuntimePort
        └── ignores capabilities
        └── never stores agents

AcpRuntime (acp.rs:256)
  └── Full agent registry
  └── A2A messaging
  └── OCAP delegation
  └── Audit logging
  └── Exported from lib.rs:48
  └── NOT USED by PodManager
```

### Files Affected

| File | Current Role | Changes Required |
|------|--------------|------------------|
| `crates/hkask-agents/src/adapters/acp_runtime.rs` | Stub adapter | **DELETE** |
| `crates/hkask-agents/src/adapters/mod.rs` | Module index | Remove `acp_runtime` module |
| `crates/hkask-agents/src/pod.rs` | Port trait + PodManager | Redesign port, wire to `AcpRuntime` |
| `crates/hkask-agents/src/acp.rs` | Real runtime | Implement new `AcpPort` trait |
| `crates/hkask-agents/src/lib.rs` | Crate exports | Update re-exports and docs |
| `crates/hkask-agents/src/ports/` | Port definitions | Add `acp.rs` (new port trait) |
| `crates/hkask-api/src/lib.rs` | API state | Update `with_defaults()` |
| `crates/hkask-api/src/routes.rs` | HTTP routes | Add `/api/v1/acp/register` |
| `Cargo.toml` (workspace) | Dependencies | Remove `acp-runtime = "0.1"` |

---

## Session A: Core Collapse (2.5 hours)

**Objective:** Fix the P0 architectural defect by unifying on `AcpRuntime` via a proper port trait.

**Tasks:** 1.19 → 1.20 → 1.21 → 1.15

**Deliverable:** `PodManager` uses `AcpRuntime` through `AcpPort` trait; stub deleted.

---

### Task 1.19: Define `AcpPort` Trait

**File:** `crates/hkask-agents/src/ports/acp.rs` (NEW)

**Rationale:** Replace the thin `ACPRuntimePort` (1 sync method, String errors) with a rich async port trait matching `AcpRuntime`'s actual interface.

#### Implementation

```rust
//! ACP Port — Agent Communication Protocol hexagonal port
//!
//! Defines the interface for agent registration, A2A messaging,
//! and capability management.

use async_trait::async_trait;
use hkask_types::{CapabilityToken, WebID};

use crate::acp::{A2AMessage, AcpError};

/// ACP Port — Agent registration and A2A communication
///
/// # Hexagonal Architecture
///
/// This port is implemented by `AcpRuntime` (in-process) and can be
/// adapted for remote ACP servers via transport adapters.
#[async_trait]
pub trait AcpPort: Send + Sync {
    /// Register an agent with the ACP runtime
    ///
    /// # Arguments
    /// * `webid` — Agent's WebID
    /// * `agent_type` — "Bot" or "Replicant"
    /// * `capabilities` — Explicit capability list (no wildcards)
    ///
    /// # Returns
    /// * `Ok(CapabilityToken)` — Primary capability token
    /// * `Err(AcpError)` — Registration failure
    async fn register_agent(
        &self,
        webid: WebID,
        agent_type: &str,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, AcpError>;

    /// Unregister an agent and revoke its capabilities
    async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError>;

    /// Send an A2A message
    ///
    /// # Arguments
    /// * `msg` — A2A message (TemplateDispatch, TemplateResponse, MemoryArtifact)
    ///
    /// # Returns
    /// * `Ok(String)` — Correlation ID for tracking
    /// * `Err(AcpError)` — Send failure
    async fn send_message(&self, msg: A2AMessage) -> Result<String, AcpError>;

    /// List capabilities for a registered agent
    async fn list_capabilities(&self, webid: &WebID) -> Result<Vec<String>, AcpError>;

    /// Check if an agent is registered
    async fn is_registered(&self, webid: &WebID) -> bool;
}
```

**Update module index:**

```rust
// crates/hkask-agents/src/ports/mod.rs
pub mod acp;
pub mod sovereignty;

pub use acp::AcpPort;
pub use sovereignty::SovereigntyPort;
```

**Acceptance Criteria:**
- [ ] `AcpPort` trait compiles
- [ ] Trait is object-safe (can use `dyn AcpPort`)
- [ ] All methods are async
- [ ] Error type is `AcpError` (not `String`)

**Estimated Time:** 15 minutes

---

### Task 1.20: Implement `AcpPort` for `AcpRuntime`

**File:** `crates/hkask-agents/src/acp.rs`

**Rationale:** Bridge the existing `AcpRuntime` to the new port trait. Requires converting internal error handling from `String` to `AcpError`.

#### Step 1: Extend `AcpError` with Missing Variants

```rust
// Add to AcpError enum (acp.rs:72-99)
#[derive(Debug, Error)]
pub enum AcpError {
    // Existing variants...
    #[error("Agent {0:?} already registered")]
    AgentAlreadyRegistered(WebID),

    #[error("Agent {0:?} not found")]
    AgentNotFound(WebID),

    // ... (keep existing variants)

    // NEW: Conversion from String for legacy code
    #[error("{0}")]
    LegacyError(String),
}

impl From<String> for AcpError {
    fn from(s: String) -> Self {
        AcpError::LegacyError(s)
    }
}
```

#### Step 2: Convert `AcpRuntime` Methods to Return `AcpError`

**Changes required in `acp.rs`:**

| Method | Line | Current Return | New Return |
|--------|------|----------------|------------|
| `register_agent()` | 348 | `Result<CapabilityToken, String>` | `Result<CapabilityToken, AcpError>` |
| `unregister_agent()` | 412 | `Result<(), String>` | `Result<(), AcpError>` |
| `send_message()` | 445 | `Result<String, String>` | `Result<String, AcpError>` |

**Example conversion for `register_agent()`:**

```rust
pub async fn register_agent(
    &self,
    webid: WebID,
    agent_type: String,
    capabilities: Vec<String>,
) -> Result<CapabilityToken, AcpError> {  // Changed from String
    let mut agents = self.agents.write().await;

    if agents.contains_key(&webid) {
        return Err(AcpError::AgentAlreadyRegistered(webid));  // Typed error
    }

    // Validate capabilities - reject wildcards
    for cap in &capabilities {
        if cap == "*" {
            return Err(AcpError::WildcardCapabilityNotAllowed);
        }
    }

    // ... rest of implementation ...

    let (resource, action) = parse_capability(&primary_capability)?;  // Uses ? operator

    let token = self
        .root_authority
        .create_root_token(resource, primary_capability.clone(), action, webid)
        .await?;  // Propagates AcpError

    // ... store agent and tokens ...

    Ok(token)
}
```

**Repeat for `unregister_agent()` and `send_message()`.**

#### Step 3: Implement `AcpPort` Trait

```rust
// Add to acp.rs after AcpRuntime impl block
use crate::ports::AcpPort;
use async_trait::async_trait;

#[async_trait]
impl AcpPort for AcpRuntime {
    async fn register_agent(
        &self,
        webid: WebID,
        agent_type: &str,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, AcpError> {
        // Delegate to existing method
        self.register_agent(webid, agent_type.to_string(), capabilities).await
    }

    async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError> {
        self.unregister_agent(webid).await
    }

    async fn send_message(&self, msg: A2AMessage) -> Result<String, AcpError> {
        self.send_message(msg).await
    }

    async fn list_capabilities(&self, webid: &WebID) -> Result<Vec<String>, AcpError> {
        let agent = self.get_agent(webid).await
            .ok_or_else(|| AcpError::AgentNotFound(*webid))?;
        Ok(agent.capabilities)
    }

    async fn is_registered(&self, webid: &WebID) -> bool {
        self.is_registered(webid).await
    }
}
```

**Acceptance Criteria:**
- [ ] `AcpRuntime` implements `AcpPort`
- [ ] All `AcpRuntime` methods return `Result<T, AcpError>`
- [ ] Existing `AcpRuntime` tests pass (if any)
- [ ] `cargo check -p hkask-agents` succeeds

**Estimated Time:** 30 minutes

---

### Task 1.21: Wire `PodManager` to `AcpRuntime` via Port

**Files:** `crates/hkask-agents/src/pod.rs`, `crates/hkask-api/src/lib.rs`, `crates/hkask-agents/src/lib.rs`

**Rationale:** Replace concrete `AcpRuntimeAdapter` with trait object `Arc<dyn AcpPort>`, making `PodManager` depend on the port abstraction.

#### Step 1: Update `AgentPod::register()` to Async

```rust
// pod.rs:376-409
pub async fn register(  // Add async
    &mut self,
    acp: &dyn AcpPort,  // Change from &dyn ACPRuntimePort
    cns: &dyn CNSSpanPort,
) -> AgentPodResult<()> {
    if self.state != PodLifecycleState::Populated {
        return Err(AgentPodError::InvalidStateTransition(
            self.state,
            PodLifecycleState::Registered,
        ));
    }

    let capabilities: Vec<String> = self.persona.capabilities.clone();
    let agent_type = self.agent_type.to_string();
    
    let token = acp
        .register_agent(self.webid, &agent_type, capabilities)  // Add agent_type
        .await  // Await the async call
        .map_err(|e| AgentPodError::ACPRegistrationError(e.to_string()))?;

    self.capability_token = token;
    self.state = PodLifecycleState::Registered;

    cns.emit_event(
        "cns.agent_pod.registered",
        "registered",
        &serde_json::json!({
            "pod_id": self.id.to_string(),
            "webid": self.webid.to_string(),
            "agent_type": self.agent_type.to_string(),
        }),
        1.0,
    );

    info!("Agent pod {} registered with ACP", self.id);
    Ok(())
}
```

#### Step 2: Update `PodManager` Struct

```rust
// pod.rs:731-741
use std::sync::Arc;
use crate::ports::AcpPort;

pub struct PodManager {
    pods: Arc<RwLock<HashMap<PodID, AgentPod>>>,
    #[allow(dead_code)]
    keystore: Keychain,
    git_cas: GitCasAdapter,
    acp_runtime: Arc<dyn AcpPort>,  // Changed from AcpRuntimeAdapter
    cns_emitter: CnsEmitterAdapter,
    mcp_runtime: McpRuntimeAdapter,
    memory_storage: Arc<Mutex<MemoryStorageAdapter>>,
    security_context: SecurityContext,
}
```

#### Step 3: Update `PodManager::new()`

```rust
// pod.rs:757-774
pub fn new(
    git_cas: GitCasAdapter,
    acp_runtime: Arc<dyn AcpPort>,  // Changed parameter type
    cns_emitter: CnsEmitterAdapter,
    mcp_runtime: McpRuntimeAdapter,
    memory_storage: MemoryStorageAdapter,
) -> Self {
    Self {
        pods: Arc::new(RwLock::new(HashMap::new())),
        keystore: Keychain::default(),
        git_cas,
        acp_runtime,
        cns_emitter,
        mcp_runtime,
        memory_storage: Arc::new(Mutex::new(memory_storage)),
        security_context: SecurityContext::default(),
    }
}
```

#### Step 4: Update `activate_pod()` to Await Register

```rust
// pod.rs:988-1009
pub async fn activate_pod(&self, pod_id: &PodID) -> AgentPodResult<()> {
    let mut pods = self.pods.write().await;
    let pod = pods
        .get_mut(pod_id)
        .ok_or_else(|| AgentPodError::ACPRegistrationError("Pod not found".to_string()))?;

    // Register with ACP runtime if not already registered
    if pod.state() == PodLifecycleState::Populated {
        pod.register(&*self.acp_runtime, &self.cns_emitter).await?;  // Add .await
    }

    // Activate the pod with MCP runtime
    pod.activate(&self.mcp_runtime, &self.cns_emitter)?;

    info!(
        target: "hkask.pod",
        pod_id = %pod_id,
        "Pod activated"
    );

    Ok(())
}
```

#### Step 5: Update `PodManagerBuilder`

```rust
// pod.rs:818-917
pub struct PodManagerBuilder {
    git_cas: Option<GitCasAdapter>,
    acp_runtime: Option<Arc<dyn AcpPort>>,  // Changed type
    cns_emitter: Option<CnsEmitterAdapter>,
    mcp_runtime: Option<McpRuntimeAdapter>,
    memory_storage: Option<MemoryStorageAdapter>,
    security_context: Option<SecurityContext>,
}

impl PodManagerBuilder {
    pub fn acp_runtime(mut self, adapter: Arc<dyn AcpPort>) -> Self {  // Changed parameter
        self.acp_runtime = Some(adapter);
        self
    }

    /// Use default AcpRuntime (convenience method)
    pub fn with_default_acp(self) -> Self {
        use crate::acp::AcpRuntime;
        self.acp_runtime(Arc::new(AcpRuntime::default()))
    }

    pub fn build(self) -> PodManager {
        PodManager::new(
            self.git_cas
                .unwrap_or_else(|| GitCasAdapter::from_path(PathBuf::from("./registry/templates"))),
            self.acp_runtime.unwrap_or_else(|| {  // Default to AcpRuntime
                use crate::acp::AcpRuntime;
                Arc::new(AcpRuntime::default())
            }),
            self.cns_emitter
                .unwrap_or_else(|| CnsEmitterAdapter::new(WebID::new())),
            self.mcp_runtime.unwrap_or_default(),
            self.memory_storage
                .unwrap_or_else(|| MemoryStorageAdapter::in_memory().unwrap()),
        )
    }
}
```

#### Step 6: Create `MockAcpPort` for Tests

```rust
// pod.rs (in #[cfg(test)] module or separate test_helpers.rs)
#[cfg(test)]
pub struct MockAcpPort;

#[cfg(test)]
#[async_trait]
impl AcpPort for MockAcpPort {
    async fn register_agent(
        &self,
        webid: WebID,
        _agent_type: &str,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, AcpError> {
        // Create a dummy token for testing
        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            capabilities.first().cloned().unwrap_or_else(|| "test".to_string()),
            CapabilityAction::Execute,
            WebID::new(),
            webid,
            b"test-secret",
        );
        Ok(token)
    }

    async fn unregister_agent(&self, _webid: &WebID) -> Result<(), AcpError> {
        Ok(())
    }

    async fn send_message(&self, _msg: A2AMessage) -> Result<String, AcpError> {
        Ok("mock-correlation-id".to_string())
    }

    async fn list_capabilities(&self, _webid: &WebID) -> Result<Vec<String>, AcpError> {
        Ok(vec!["test:capability".to_string()])
    }

    async fn is_registered(&self, _webid: &WebID) -> bool {
        true
    }
}
```

#### Step 7: Update `PodManager::new_mock()`

```rust
// pod.rs:777-788
pub fn new_mock() -> Self {
    Self {
        pods: Arc::new(RwLock::new(HashMap::new())),
        keystore: Keychain::default(),
        git_cas: GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-mock")),
        acp_runtime: Arc::new(MockAcpPort),  // Use mock
        cns_emitter: CnsEmitterAdapter::new(WebID::new()),
        mcp_runtime: McpRuntimeAdapter::new(),
        memory_storage: Arc::new(Mutex::new(MemoryStorageAdapter::in_memory().unwrap())),
        security_context: SecurityContext::default(),
    }
}
```

#### Step 8: Delete `ACPRuntimePort` Trait

```rust
// DELETE pod.rs:583-598 (the old trait definition)
// pub trait ACPRuntimePort { ... }  // REMOVE ENTIRELY
```

#### Step 9: Update `hkask-api`

```rust
// crates/hkask-api/src/lib.rs:107-118
use hkask_agents::acp::AcpRuntime;
use std::sync::Arc;

pub fn with_defaults(
    registry: SqliteRegistry,
    mcp_runtime: hkask_mcp::runtime::McpRuntime,
    capability_secret: &[u8],
    system_webid: WebID,
) -> Self {
    let git_cas = GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-templates"));
    let acp_runtime = Arc::new(AcpRuntime::default());  // Use real runtime
    let observer_webid = WebID::new();
    let cns_emitter_adapter = CnsEmitterAdapter::new(observer_webid);
    let mcp_runtime_adapter = McpRuntimeAdapter::new();
    let memory_storage = MemoryStorageAdapter::in_memory().unwrap();
    let pod_manager = PodManager::new(
        git_cas,
        acp_runtime,  // Pass Arc<dyn AcpPort>
        cns_emitter_adapter,
        mcp_runtime_adapter,
        memory_storage,
    );
    Self::new(
        registry,
        mcp_runtime,
        pod_manager,
        capability_secret,
        system_webid,
        None,
    )
}
```

#### Step 10: Update Crate Exports

```rust
// crates/hkask-agents/src/lib.rs
pub use acp::{A2AMessage, AcpAgent, AcpError, AcpRuntime, TemplateDispatchHandler};
pub use pod::{
    AgentPersona, AgentPod, AgentPodError, AgentPodResult, AgentType, CNSSpanPort, GitCASPort,
    MCPRuntimePort, MemoryStoragePort, PodID, PodLifecycleState, PodManager, PodStatus,
    TemplateCrate,
};
pub use ports::AcpPort;  // Export the new port

// Update doc example (lines 12-33)
//! ```rust,no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use hkask_agents::pod::PodManager;
//! use hkask_agents::acp::AcpRuntime;
//! use hkask_agents::adapters::git_cas::GitCasAdapter;
//! use std::sync::Arc;
//! use std::path::PathBuf;
//!
//! let git_cas = GitCasAdapter::from_path(PathBuf::from("/tmp/hkask-templates"));
//! let acp_runtime = Arc::new(AcpRuntime::default());
//! let cns_emitter = CnsEmitterAdapter::new(WebID::new());
//! let mcp_runtime = McpRuntimeAdapter::new();
//! let memory_storage = MemoryStorageAdapter::in_memory()?;
//!
//! let manager = PodManager::new(git_cas, acp_runtime, cns_emitter, mcp_runtime, memory_storage);
//! # Ok(())
//! # }
//! ```
```

**Acceptance Criteria:**
- [ ] `PodManager` uses `Arc<dyn AcpPort>` instead of `AcpRuntimeAdapter`
- [ ] `AgentPod::register()` is async
- [ ] `PodManager::activate_pod()` compiles and awaits register
- [ ] `ACPRuntimePort` trait deleted
- [ ] `MockAcpPort` exists for tests
- [ ] `hkask-api` compiles with new `PodManager` signature
- [ ] `cargo check -p hkask-agents` succeeds
- [ ] `cargo check -p hkask-api` succeeds

**Estimated Time:** 60 minutes

---

### Task 1.15: Delete `AcpRuntimeAdapter`

**Files:** `crates/hkask-agents/src/adapters/acp_runtime.rs`, `crates/hkask-agents/src/adapters/mod.rs`, `Cargo.toml`

**Rationale:** With `AcpRuntime` wired in via `AcpPort`, the stub adapter is dead code.

#### Step 1: Delete the Stub File

```bash
rm crates/hkask-agents/src/adapters/acp_runtime.rs
```

#### Step 2: Update Module Index

```rust
// crates/hkask-agents/src/adapters/mod.rs
//! Adapter implementations for hexagonal ports

pub mod cns_emitter;
pub mod git_cas;
pub mod keystore_port;
pub mod mcp_runtime;
pub mod memory_storage;

// DELETE: pub mod acp_runtime;
// DELETE: pub use acp_runtime::AcpRuntimeAdapter;

pub use cns_emitter::CnsEmitterAdapter;
pub use git_cas::{GitCasAdapter, MockGitCas};
pub use keystore_port::{KeystorePort, Secret};
pub use mcp_runtime::McpRuntimeAdapter;
pub use memory_storage::MemoryStorageAdapter;
```

#### Step 3: Remove Dead Dependency

```toml
# Cargo.toml (workspace root, line 63)
[workspace.dependencies]
# DELETE: acp-runtime = "0.1"
```

```toml
# crates/hkask-agents/Cargo.toml (line 14)
[dependencies]
# DELETE: acp-runtime.workspace = true
```

#### Step 4: Update User Guide

```bash
# Search for references
grep -r "AcpRuntimeAdapter" docs/user-guides/
```

Update `docs/user-guides/AGENT-POD-CREATION-GUIDE.md` lines 808, 818 to use `AcpRuntime` instead.

**Acceptance Criteria:**
- [ ] `adapters/acp_runtime.rs` deleted
- [ ] `adapters/mod.rs` no longer exports `AcpRuntimeAdapter`
- [ ] `acp-runtime` removed from workspace `Cargo.toml`
- [ ] `acp-runtime` removed from `hkask-agents/Cargo.toml`
- [ ] User guide updated
- [ ] `cargo build -p hkask-agents` succeeds
- [ ] `cargo build -p hkask-api` succeeds
- [ ] `cargo test -p hkask-agents` passes

**Estimated Time:** 15 minutes

---

## Session B: Transport + Russell (3.5 hours)

**Objective:** Add transport abstraction for ACP and expose Russell registration endpoint.

**Tasks:** 1.16 → 1.17 → 1.18 → 1.22

**Deliverable:** `AcpTransport` trait with stdio and loopback HTTP implementations; Russell can register via API.

---

### Task 1.16: Define `AcpTransport` Trait

**File:** `crates/hkask-agents/src/ports/acp_transport.rs` (NEW)

**Rationale:** Enable ACP communication over different transports (stdio, HTTP) while maintaining security boundaries (loopback-only per F3).

#### Implementation

```rust
//! ACP Transport — Wire protocol abstraction for ACP communication
//!
//! Defines the interface for sending/receiving ACP messages over
//! different transports (stdio, HTTP loopback).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::acp::{A2AMessage, AcpError};

/// ACP wire message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpWireMessage {
    /// Message ID for correlation
    pub id: String,
    /// Message payload
    pub payload: A2AMessage,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
}

/// ACP wire response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpWireResponse {
    /// Correlation ID matching the request
    pub id: String,
    /// Success status
    pub success: bool,
    /// Result data (if successful)
    pub result: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// ACP Transport — Send and receive ACP messages
///
/// # Security
///
/// Implementations must enforce security boundaries:
/// - `StdioTransport`: Process isolation, no network exposure
/// - `LoopbackHttpTransport`: Refuses non-loopback addresses
#[async_trait]
pub trait AcpTransport: Send + Sync {
    /// Send an ACP message
    async fn send(&self, msg: &AcpWireMessage) -> Result<AcpWireResponse, AcpError>;

    /// Receive an ACP message (blocking)
    async fn receive(&self) -> Result<AcpWireMessage, AcpError>;

    /// Check if transport is connected
    fn is_connected(&self) -> bool;
}
```

**Update module index:**

```rust
// crates/hkask-agents/src/ports/mod.rs
pub mod acp;
pub mod acp_transport;
pub mod sovereignty;

pub use acp::AcpPort;
pub use acp_transport::{AcpTransport, AcpWireMessage, AcpWireResponse};
pub use sovereignty::SovereigntyPort;
```

**Extend `AcpError`:**

```rust
// Add to AcpError enum (acp.rs)
#[error("Transport error: {0}")]
TransportError(String),

#[error("Non-loopback address refused: {0}")]
NonLoopbackRefused(std::net::IpAddr),

#[error("Connection refused: {0}")]
ConnectionRefused(String),

#[error("Transport disconnected")]
Disconnected,
```

**Acceptance Criteria:**
- [ ] `AcpTransport` trait compiles
- [ ] `AcpWireMessage` and `AcpWireResponse` types defined
- [ ] Trait is object-safe
- [ ] `AcpError` extended with transport variants

**Estimated Time:** 20 minutes

---

### Task 1.17: Implement `StdioTransport`

**File:** `crates/hkask-agents/src/adapters/stdio_transport.rs` (NEW)

**Rationale:** JSON-RPC over stdio for process-isolated ACP communication. Matches Russell's `russell-acp-server` pattern.

#### Implementation

```rust
//! Stdio Transport — JSON-RPC over stdin/stdout
//!
//! Provides process-isolated ACP communication with no network exposure.
//! Used for parent-child process communication (e.g., Russell pods).

use async_trait::async_trait;
use serde_json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::acp::AcpError;
use crate::ports::{AcpTransport, AcpWireMessage, AcpWireResponse};

/// Stdio transport for ACP communication
///
/// # Security
///
/// No network exposure. Communication is limited to parent-child processes.
pub struct StdioTransport {
    /// Buffered reader for stdin
    reader: BufReader<tokio::io::Stdin>,
    /// Writer for stdout
    writer: tokio::io::Stdout,
    /// Connection status
    connected: bool,
}

impl StdioTransport {
    /// Create new stdio transport
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
            connected: true,
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AcpTransport for StdioTransport {
    async fn send(&self, msg: &AcpWireMessage) -> Result<AcpWireResponse, AcpError> {
        let mut writer = self.writer.clone();
        
        // Serialize message to JSON
        let json = serde_json::to_string(msg)
            .map_err(|e| AcpError::TransportError(format!("Serialization failed: {}", e)))?;
        
        // Write newline-delimited JSON
        writer
            .write_all(json.as_bytes())
            .await
            .map_err(|e| AcpError::TransportError(format!("Write failed: {}", e)))?;
        
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| AcpError::TransportError(format!("Write failed: {}", e)))?;
        
        writer
            .flush()
            .await
            .map_err(|e| AcpError::TransportError(format!("Flush failed: {}", e)))?;
        
        // For stdio, we don't wait for response in send()
        // Response comes via receive() on the other end
        Ok(AcpWireResponse {
            id: msg.id.clone(),
            success: true,
            result: None,
            error: None,
        })
    }

    async fn receive(&self) -> Result<AcpWireMessage, AcpError> {
        let mut reader = self.reader.clone();
        let mut line = String::new();
        
        // Read newline-delimited JSON
        let bytes_read = reader
            .read_line(&mut line)
            .await
            .map_err(|e| AcpError::TransportError(format!("Read failed: {}", e)))?;
        
        if bytes_read == 0 {
            return Err(AcpError::Disconnected);
        }
        
        // Deserialize message
        let msg: AcpWireMessage = serde_json::from_str(&line)
            .map_err(|e| AcpError::TransportError(format!("Deserialization failed: {}", e)))?;
        
        Ok(msg)
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
```

**Update adapters module:**

```rust
// crates/hkask-agents/src/adapters/mod.rs
pub mod cns_emitter;
pub mod git_cas;
pub mod keystore_port;
pub mod mcp_runtime;
pub mod memory_storage;
pub mod stdio_transport;  // NEW

pub use cns_emitter::CnsEmitterAdapter;
pub use git_cas::{GitCasAdapter, MockGitCas};
pub use keystore_port::{KeystorePort, Secret};
pub use mcp_runtime::McpRuntimeAdapter;
pub use memory_storage::MemoryStorageAdapter;
pub use stdio_transport::StdioTransport;  // NEW
```

**Acceptance Criteria:**
- [ ] `StdioTransport` compiles
- [ ] Implements `AcpTransport` trait
- [ ] Handles newline-delimited JSON
- [ ] Returns `AcpError::Disconnected` on EOF
- [ ] Test: round-trip send/receive with mock stdin/stdout

**Estimated Time:** 45 minutes

---

### Task 1.18: Implement `LoopbackHttpTransport`

**File:** `crates/hkask-agents/src/adapters/loopback_http_transport.rs` (NEW)

**Rationale:** HTTP transport for systemd-managed pods that outlive their parent process. Enforces loopback-only binding for security.

#### Implementation

```rust
//! Loopback HTTP Transport — HTTP on 127.0.0.1/::1 only
//!
//! Provides HTTP-based ACP communication restricted to loopback addresses.
//! Used for systemd-managed agent pods.

use async_trait::async_trait;
use reqwest::Client;
use std::net::{IpAddr, SocketAddr};

use crate::acp::AcpError;
use crate::ports::{AcpTransport, AcpWireMessage, AcpWireResponse};

/// Loopback HTTP transport for ACP communication
///
/// # Security
///
/// Constructor **refuses** non-loopback addresses structurally.
/// This is a security boundary, not a limitation.
pub struct LoopbackHttpTransport {
    /// Target endpoint (must be loopback)
    endpoint: SocketAddr,
    /// HTTP client
    client: Client,
    /// Connection status
    connected: bool,
}

impl LoopbackHttpTransport {
    /// Create new loopback HTTP transport
    ///
    /// # Security
    ///
    /// Returns `AcpError::NonLoopbackRefused` if endpoint is not loopback.
    ///
    /// # Arguments
    /// * `endpoint` — Target socket address (must be 127.0.0.1 or ::1)
    pub fn new(endpoint: SocketAddr) -> Result<Self, AcpError> {
        // Enforce loopback-only binding
        if !endpoint.ip().is_loopback() {
            return Err(AcpError::NonLoopbackRefused(endpoint.ip()));
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AcpError::TransportError(format!("HTTP client creation failed: {}", e)))?;

        Ok(Self {
            endpoint,
            client,
            connected: true,
        })
    }

    /// Get the endpoint URL
    fn endpoint_url(&self) -> String {
        format!("http://{}", self.endpoint)
    }
}

#[async_trait]
impl AcpTransport for LoopbackHttpTransport {
    async fn send(&self, msg: &AcpWireMessage) -> Result<AcpWireResponse, AcpError> {
        let url = format!("{}/acp/message", self.endpoint_url());
        
        let response = self
            .client
            .post(&url)
            .json(msg)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    AcpError::ConnectionRefused(format!("{}: {}", self.endpoint, e))
                } else {
                    AcpError::TransportError(format!("HTTP request failed: {}", e))
                }
            })?;

        if !response.status().is_success() {
            return Err(AcpError::TransportError(format!(
                "HTTP {} from {}",
                response.status(),
                self.endpoint
            )));
        }

        let wire_response: AcpWireResponse = response
            .json()
            .await
            .map_err(|e| AcpError::TransportError(format!("Response deserialization failed: {}", e)))?;

        Ok(wire_response)
    }

    async fn receive(&self) -> Result<AcpWireMessage, AcpError> {
        // For HTTP client mode, receive is not applicable
        // Server-side implementation would use axum/warp
        Err(AcpError::TransportError(
            "LoopbackHttpTransport client does not support receive()".to_string(),
        ))
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
```

**Update adapters module:**

```rust
// crates/hkask-agents/src/adapters/mod.rs
pub mod cns_emitter;
pub mod git_cas;
pub mod keystore_port;
pub mod loopback_http_transport;  // NEW
pub mod mcp_runtime;
pub mod memory_storage;
pub mod stdio_transport;

pub use cns_emitter::CnsEmitterAdapter;
pub use git_cas::{GitCasAdapter, MockGitCas};
pub use keystore_port::{KeystorePort, Secret};
pub use loopback_http_transport::LoopbackHttpTransport;  // NEW
pub use mcp_runtime::McpRuntimeAdapter;
pub use memory_storage::MemoryStorageAdapter;
pub use stdio_transport::StdioTransport;
```

**Acceptance Criteria:**
- [ ] `LoopbackHttpTransport` compiles
- [ ] Constructor rejects `192.168.1.1` with `AcpError::NonLoopbackRefused`
- [ ] Constructor accepts `127.0.0.1:8080`
- [ ] Implements `AcpTransport` trait
- [ ] Test: `LoopbackHttpTransport::new("192.168.1.1:8080".parse().unwrap())` returns error
- [ ] Test: `LoopbackHttpTransport::new("127.0.0.1:8080".parse().unwrap())` succeeds

**Estimated Time:** 60 minutes

---

### Task 1.22: Add Russell ACP Registration Endpoint

**File:** `crates/hkask-api/src/routes.rs`

**Rationale:** Expose ACP registration for external agents (Russell) via HTTP API.

#### Implementation

```rust
// Add to routes.rs

use hkask_agents::ports::AcpPort;

/// ACP registration request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcpRegisterRequest {
    /// Agent WebID
    pub webid: String,
    /// Agent type ("Bot" or "Replicant")
    pub agent_type: String,
    /// Capabilities to grant
    pub capabilities: Vec<String>,
}

/// ACP registration response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcpRegisterResponse {
    /// Capability token
    pub token: CapabilityToken,
    /// Registration timestamp
    pub registered_at: i64,
}

/// Register an agent with ACP
#[utoipa::path(
    post,
    path = "/api/v1/acp/register",
    request_body = AcpRegisterRequest,
    responses(
        (status = 200, description = "Agent registered", body = AcpRegisterResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Agent already registered"),
        (status = 429, description = "Rate limit exceeded")
    )
)]
pub async fn acp_register(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<AcpRegisterRequest>,
) -> Result<Json<AcpRegisterResponse>, ApiError> {
    // Rate limit registration
    let rate_key = format!("acp_register:{}", request.webid);
    state
        .rate_limiter
        .acquire(&rate_key, 1.0)
        .await
        .map_err(|_| ApiError::RateLimitExceeded)?;

    // Parse WebID
    let webid = WebID::parse(&request.webid)
        .map_err(|e| ApiError::BadRequest(format!("Invalid WebID: {}", e)))?;

    // Validate agent type
    if !["Bot", "Replicant"].contains(&request.agent_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid agent_type: {}. Must be 'Bot' or 'Replicant'",
            request.agent_type
        )));
    }

    // Validate capabilities
    if request.capabilities.is_empty() {
        return Err(ApiError::BadRequest("At least one capability required".to_string()));
    }

    // Get ACP port from PodManager
    // Note: This requires adding an accessor method to PodManager
    let acp_port = state.pod_manager.acp_runtime();
    
    // Register agent
    let token = acp_port
        .register_agent(webid, &request.agent_type, request.capabilities)
        .await
        .map_err(|e| match e {
            hkask_agents::acp::AcpError::AgentAlreadyRegistered(_) => {
                ApiError::Conflict(format!("Agent {} already registered", request.webid))
            }
            _ => ApiError::InternalServerError(format!("Registration failed: {}", e)),
        })?;

    Ok(Json(AcpRegisterResponse {
        token,
        registered_at: chrono::Utc::now().timestamp(),
    }))
}
```

**Add accessor to `PodManager`:**

```rust
// crates/hkask-agents/src/pod.rs
impl PodManager {
    /// Get ACP runtime port (for external registration)
    pub fn acp_runtime(&self) -> Arc<dyn AcpPort> {
        Arc::clone(&self.acp_runtime)
    }
}
```

**Wire route:**

```rust
// crates/hkask-api/src/routes.rs (in router setup)
pub fn create_router(state: Arc<ApiState>) -> Router {
    Router::new()
        // ... existing routes ...
        .route("/api/v1/acp/register", post(acp_register))  // NEW
        .with_state(state)
}
```

**Acceptance Criteria:**
- [ ] `POST /api/v1/acp/register` endpoint exists
- [ ] Request validation (WebID format, agent_type, capabilities)
- [ ] Rate limiting applied
- [ ] Returns `CapabilityToken` on success
- [ ] Returns 409 if agent already registered
- [ ] Test: Successful registration returns 200
- [ ] Test: Duplicate registration returns 409
- [ ] Test: Invalid WebID returns 400

**Estimated Time:** 45 minutes

---

## Testing Strategy

### Unit Tests

**File:** `crates/hkask-agents/src/acp.rs` (add `#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::AcpPort;

    #[tokio::test]
    async fn test_acp_runtime_register_agent() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        
        let webid = WebID::new();
        let result = runtime
            .register_agent(webid, "Bot", vec!["tool:execute".to_string()])
            .await;
        
        assert!(result.is_ok());
        assert!(runtime.is_registered(&webid).await);
    }

    #[tokio::test]
    async fn test_acp_runtime_rejects_wildcard() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        
        let webid = WebID::new();
        let result = runtime
            .register_agent(webid, "Bot", vec!["*".to_string()])
            .await;
        
        assert!(matches!(result, Err(AcpError::WildcardCapabilityNotAllowed)));
    }

    #[tokio::test]
    async fn test_acp_runtime_duplicate_registration() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        
        let webid = WebID::new();
        runtime
            .register_agent(webid, "Bot", vec!["tool:execute".to_string()])
            .await
            .unwrap();
        
        let result = runtime
            .register_agent(webid, "Bot", vec!["tool:execute".to_string()])
            .await;
        
        assert!(matches!(result, Err(AcpError::AgentAlreadyRegistered(_))));
    }

    #[tokio::test]
    async fn test_acp_port_list_capabilities() {
        let runtime = AcpRuntime::new(b"test-secret", None);
        
        let webid = WebID::new();
        let capabilities = vec!["tool:execute".to_string(), "template:render".to_string()];
        
        runtime
            .register_agent(webid, "Bot", capabilities.clone())
            .await
            .unwrap();
        
        let listed = runtime.list_capabilities(&webid).await.unwrap();
        assert_eq!(listed, capabilities);
    }
}
```

### Integration Tests

**File:** `crates/hkask-agents/src/pod.rs` (add to existing test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::AcpRuntime;

    #[tokio::test]
    async fn test_pod_manager_with_real_acp() {
        let acp_runtime = Arc::new(AcpRuntime::new(b"test-secret", None));
        let pod_manager = PodManagerBuilder::new()
            .acp_runtime(acp_runtime)
            .with_in_memory_storage()
            .build();

        // Create a pod
        let persona = AgentPersona {
            agent: AgentPersonaInput {
                name: "Test Bot".to_string(),
                agent_type: "bot".to_string(),
                version: "1.0.0".to_string(),
                description: "Test bot".to_string(),
                editor: "test".to_string(),
                capabilities: vec!["tool:execute".to_string()],
            },
            charter: CharterInput {
                description: "Test charter".to_string(),
                editor: "test".to_string(),
            },
        };

        let pod_id = pod_manager
            .create_pod("test-template", &persona, None)
            .await
            .unwrap();

        // Activate pod (triggers ACP registration)
        pod_manager.activate_pod(&pod_id).await.unwrap();

        // Verify pod is registered
        let status = pod_manager.get_pod_status(&pod_id).await.unwrap();
        assert_eq!(status.state, "Registered");
    }
}
```

### Transport Tests

**File:** `crates/hkask-agents/src/adapters/loopback_http_transport.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loopback_rejects_non_loopback() {
        let addr: SocketAddr = "192.168.1.1:8080".parse().unwrap();
        let result = LoopbackHttpTransport::new(addr);
        
        assert!(matches!(result, Err(AcpError::NonLoopbackRefused(_))));
    }

    #[test]
    fn test_loopback_accepts_127_0_0_1() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let result = LoopbackHttpTransport::new(addr);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_loopback_accepts_ipv6_loopback() {
        let addr: SocketAddr = "[::1]:8080".parse().unwrap();
        let result = LoopbackHttpTransport::new(addr);
        
        assert!(result.is_ok());
    }
}
```

### API Tests

**File:** `crates/hkask-api/src/routes.rs` (add test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_acp_register_success() {
        let state = create_test_state();
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/acp/register")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&AcpRegisterRequest {
                            webid: WebID::new().to_string(),
                            agent_type: "Bot".to_string(),
                            capabilities: vec!["tool:execute".to_string()],
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_acp_register_duplicate() {
        let state = create_test_state();
        let app = create_router(state);

        let webid = WebID::new().to_string();
        let request_body = serde_json::to_string(&AcpRegisterRequest {
            webid: webid.clone(),
            agent_type: "Bot".to_string(),
            capabilities: vec!["tool:execute".to_string()],
        })
        .unwrap();

        // First registration
        let app_clone = app.clone();
        let response = app_clone
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/acp/register")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Duplicate registration
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/acp/register")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
```

---

## Risk Mitigation

### Risk 1: Breaking `hkask-api` Compilation

**Severity:** High  
**Mitigation:** Update `ApiState::with_defaults()` in the same commit as task 1.21. Run `cargo check -p hkask-api` after each change.

### Risk 2: Async Propagation Cascade

**Severity:** Medium  
**Mitigation:** All callers of `AgentPod::register()` are already in async context (`PodManager::activate_pod()` is async). No cascade beyond the immediate call site.

### Risk 3: `AcpRuntime` Error Type Conversion

**Severity:** Medium  
**Mitigation:** Add `From<String> for AcpError` impl to allow gradual migration. Convert all `AcpRuntime` methods to return `AcpError` in task 1.20.

### Risk 4: Transport Trait Premature Abstraction

**Severity:** Low  
**Mitigation:** `AcpRuntime` can implement `AcpPort` without transport initially. Transport is additive — doesn't block Session A.

### Risk 5: External `acp-runtime` Crate Name Collision

**Severity:** Low  
**Mitigation:** Remove from `Cargo.toml` in task 1.15. The crate is unused (zero imports).

---

## Success Criteria

### Session A Completion

- [ ] `AcpPort` trait defined in `ports/acp.rs`
- [ ] `AcpRuntime` implements `AcpPort`
- [ ] `PodManager` uses `Arc<dyn AcpPort>`
- [ ] `AcpRuntimeAdapter` deleted
- [ ] `ACPRuntimePort` trait deleted
- [ ] `acp-runtime` dependency removed
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test -p hkask-agents` passes
- [ ] `cargo test -p hkask-api` passes

### Session B Completion

- [ ] `AcpTransport` trait defined
- [ ] `StdioTransport` implements `AcpTransport`
- [ ] `LoopbackHttpTransport` implements `AcpTransport` and rejects non-loopback
- [ ] `POST /api/v1/acp/register` endpoint exists
- [ ] Russell can register via API
- [ ] All transport tests pass
- [ ] API integration tests pass

### Overall Phase 1C Completion

- [ ] RC3 (Dual ACP Systems) resolved
- [ ] No dead code or unused dependencies
- [ ] Hexagonal architecture restored (ports/adapters)
- [ ] All tests pass
- [ ] Documentation updated
- [ ] `cargo clippy --workspace -- -D warnings` passes

---

## Conclusion

This plan systematically eliminates the P0 dual ACP defect through two focused sessions:

1. **Session A** fixes the core architectural incoherence by unifying on `AcpRuntime` via a proper port trait
2. **Session B** adds transport abstraction and external integration surface

The approach minimizes risk by:
- Preserving existing functionality during refactoring
- Using trait objects to maintain hexagonal architecture
- Adding comprehensive test coverage
- Deferring transport layer until core is stable

**Total estimated effort:** 6 hours  
**Recommended timeline:** Complete Session A first, verify stability, then proceed to Session B.

---

*Phase 1C: Collapse Dual ACP Systems — v1.0.0*  
*As simple as possible, but no simpler.*
