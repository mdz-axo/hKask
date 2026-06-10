# AgentService Refactoring — Implementation Plan

**Version:** 0.27.1  
**Date:** 2026-06-10  
**Status:** In Progress (strangler-fig)

---

## Overview

**Goal:** Condense `ServiceContext` → `AgentService` with private fields and 8 group methods (strangler-fig migration)

**Approach:** Strangler-fig migration — not a big bang. Old and new access paths coexist during transition; all surfaces remain functional at every intermediate step.

**Note:** The original plan called for "big bang migration (8 phases, all in one PR)" — this was revised in favor of strangler-fig per FA-AS3.

**Specification:** [`MDS-agent-service.md`](MDS-agent-service.md)

---

## TDD Tracer Bullets (Priority Order)

### P0 (Security) — REQ-MDS-T1

**Test:** Direct field access fails to compile

```rust
// TEST FILE: hkask-services/tests/encapsulation.rs
// REQ: REQ-MDS-T1 — Direct field access fails to compile

#[test]
#[compile_fail]
fn cannot_access_fields_directly() {
    // This should NOT compile - fields are private
    let ctx = AgentService::build(config).await.unwrap();
    let _ = ctx.registry; // ERROR: private field
}
```

**Implementation:** Make all 26 fields private in `AgentService` (originally planned 27; sovereignty_boundary_store removed from private set — see FA-AS2)

**Verification:** Test fails to compile (as expected)

---

### P1 (Correctness) — REQ-MDS-C1

**Test:** All 8 group methods exist and return correct tuple types

```rust
// TEST FILE: hkask-services/tests/accessor_methods.rs
// REQ: REQ-MDS-C1 — All 8 group methods exist

#[tokio::test]
async fn agent_service_has_memory_accessor() {
    let ctx = build_test_agent_service().await;
    let memory = ctx.memory();
    assert!(memory.episodic().is_some());
    assert!(memory.semantic().is_some());
}

#[tokio::test]
async fn agent_service_has_cns_accessor() {
    let ctx = build_test_agent_service().await;
    let cns = ctx.cns();
    assert!(cns.runtime().is_some());
    assert!(cns.service().is_some());
}

// ... repeat for all 8 group methods
```

**Implementation:** Add 8 group methods returning tuples of references (no adapter structs)

**Verification:** All tests pass

---

### P1 (Correctness) — REQ-MDS-D1

**Test:** AgentService::build() assembles all 26 fields

```rust
// TEST FILE: hkask-services/tests/build.rs
// REQ: REQ-MDS-D1 — AgentService::build() assembles all 26 fields

#[tokio::test]
async fn build_assembles_all_domains() {
    let config = test_config();
    let ctx = AgentService::build(config).await.unwrap();
    
    // Verify all 8 group methods are populated
    assert!(ctx.memory().episodic().is_some());
    assert!(ctx.cns().runtime().is_some());
    assert!(ctx.governance().dispatcher().is_some());
    assert!(ctx.storage().registry().is_some());
    assert!(ctx.coordination().session_manager().is_some());
    assert!(ctx.identity().webid().is_some());
    assert!(ctx.config().is_some());
}
```

**Implementation:** `AgentService::build()` method (renamed from `ServiceContext::build()`)

**Verification:** Test passes

---

### P2 (Lifecycle) — REQ-MDS-L1

**Test:** Bootstrap completes in <5 seconds

```rust
// TEST FILE: hkask-services/tests/lifecycle.rs
// REQ: REQ-MDS-L1 — Bootstrap completes in <5 seconds

#[tokio::test]
async fn build_completes_within_time_budget() {
    let config = test_config();
    let start = std::time::Instant::now();
    let _ctx = AgentService::build(config).await.unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed < std::time::Duration::from_secs(5));
}
```

**Implementation:** Optimized build sequence (no changes from current)

**Verification:** Test passes

---

## Implementation Phases (Strangler-Fig)

### Phase 1: Rename ServiceContext → AgentService

**Files to Update:**
- `hkask-services/src/context.rs` — struct name
- `hkask-services/src/lib.rs` — exports
- `hkask-cli/**/*.rs` — all call sites
- `hkask-api/**/*.rs` — all call sites
- `hkask-agents/**/*.rs` — all call sites
- `hkask-cns/**/*.rs` — all call sites
- `hkask-mcp/**/*.rs` — all call sites

**Verification:** `cargo check --workspace` passes

---

### Phase 2: Make All Fields Private

**Files to Update:**
- `hkask-services/src/context.rs` — remove `pub` from all 26 fields

**Verification:** Compile-fail test (REQ-MDS-T1) fails to compile as expected

---

### Phase 3: Add 7 Domain Adapter Structs ⛔ (NOT DONE)

**Status:** Replaced by tuple-based approach per FA-AS2. No adapter structs were created — group methods return tuples of references directly via destructuring.

<details>
<summary>Original Plan (abandoned)</summary>

**Files to Create:**
- `hkask-services/src/adapters/mod.rs` — module exports
- `hkask-services/src/adapters/memory.rs` — MemoryAdapters
- `hkask-services/src/adapters/cns.rs` — CnsAdapters
- `hkask-services/src/adapters/governance.rs` — GovernanceAdapters
- `hkask-services/src/adapters/storage.rs` — StorageAdapters
- `hkask-services/src/adapters/coordination.rs` — CoordinationAdapters
- `hkask-services/src/adapters/identity.rs` — IdentityAdapters

**Verification:** `cargo check -p hkask-services` passes

</details>

---

### Phase 4: Add Accessor Methods to AgentService

**Files to Update:**
- `hkask-services/src/context.rs` — impl 8 group methods

**Verification:** REQ-MDS-C1 tests pass

---

### Phase 5: Update CLI Call Sites

**Files to Update:**
- `hkask-cli/src/commands/**/*.rs` — all command handlers
- `hkask-cli/src/repl/**/*.rs` — REPL state + handlers

**Pattern:**
```rust
// Before:
ctx.episodic_storage.store_episodic(...)

// After:
ctx.memory().episodic().store_episodic(...)
```

**Verification:** `cargo test -p hkask-cli` passes

---

### Phase 6: Update API Call Sites

**Files to Update:**
- `hkask-api/src/routes/**/*.rs` — all route handlers
- `hkask-api/src/lib.rs` — ApiState construction

**Pattern:**
```rust
// Before:
state.service_context.registry.lock().await

// After:
state.agent_service.storage().registry().lock().await
```

**Verification:** `cargo test -p hkask-api` passes

---

### Phase 7: Update Domain Crate Call Sites

**Files to Update:**
- `hkask-agents/src/**/*.rs` — all agent/pod/curator code
- `hkask-cns/src/**/*.rs` — all CNS code
- `hkask-mcp/src/**/*.rs` — all MCP code

**Pattern:**
```rust
// Before:
ctx.mcp_runtime.invoke_tool(...)

// After:
ctx.governance().runtime().invoke_tool(...)
```

**Verification:** `cargo test --workspace` passes

---

### Phase 8: Final Verification

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

**Verification:** All checks pass, no warnings

---

## Test Files to Create

| File | REQ Tag | Purpose | Status |
|------|---------|---------|--------|
| `hkask-services/tests/encapsulation.rs` | REQ-MDS-T1 | Compile-fail: direct field access | ✅ Exists |
| `hkask-services/tests/accessor_methods.rs` | REQ-MDS-C1 | All 8 accessors exist | ❌ Does not exist |
| `hkask-services/tests/build.rs` | REQ-MDS-D1 | Build assembles all domains | ❌ Does not exist |
| `hkask-services/tests/lifecycle.rs` | REQ-MDS-L1 | Bootstrap <5 seconds | ❌ Does not exist |

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| **Breaking changes to MCP servers** | None — MCP servers don't depend on AgentService |
| **Breaking changes to agent pods** | Update pod manager code in Phase 7 |
| **Compile errors in domain crates** | Fix in Phase 7, test with `cargo test --workspace` |
| **Long compile times** | Run phases incrementally, test per phase |
| **Rollback needed** | Single commit per phase, easy to revert |

---

## Success Criteria

- ✅ All 26 fields are private (no direct access)
- ✅ 8 group methods exist and work correctly
- ✅ All CLI call sites updated
- ✅ All API call sites updated
- ✅ All domain crate call sites updated
- ✅ `cargo test --workspace` passes
- ✅ `cargo clippy --workspace -- -D warnings` passes
- ✅ Compile-fail test ensures encapsulation is enforced

---

*ℏKask — A Minimal Viable Container for Agents — v0.27.1 (strangler-fig)*
