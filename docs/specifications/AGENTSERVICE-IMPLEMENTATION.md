# AgentService Refactoring — Implementation Plan

**Version:** 0.27.1  
**Date:** 2026-06-09  
**Status:** Ready for TDD Implementation

---

## Overview

**Goal:** Condense `ServiceContext` (27 public fields) → `AgentService` (7 domain adapters with private fields)

**Approach:** Big bang migration (8 phases, all in one PR)

**Specification:** [`docs/specifications/MDS-agent-service.md`](docs/specifications/MDS-agent-service.md)

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

**Implementation:** Make all 27 fields private in `AgentService`

**Verification:** Test fails to compile (as expected)

---

### P1 (Correctness) — REQ-MDS-C1

**Test:** All 7 accessor methods exist and return correct types

```rust
// TEST FILE: hkask-services/tests/accessor_methods.rs
// REQ: REQ-MDS-C1 — All 7 accessor methods exist

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

// ... repeat for all 7 accessors
```

**Implementation:** Add 7 domain adapter structs + accessor methods

**Verification:** All tests pass

---

### P1 (Correctness) — REQ-MDS-D1

**Test:** AgentService::build() assembles all 27 fields

```rust
// TEST FILE: hkask-services/tests/build.rs
// REQ: REQ-MDS-D1 — AgentService::build() assembles all 27 fields

#[tokio::test]
async fn build_assembles_all_domains() {
    let config = test_config();
    let ctx = AgentService::build(config).await.unwrap();
    
    // Verify all 7 domain adapters are populated
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

## Implementation Phases (Big Bang)

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
- `hkask-services/src/context.rs` — remove `pub` from all 27 fields

**Verification:** Compile-fail test (REQ-MDS-T1) fails to compile as expected

---

### Phase 3: Add 7 Domain Adapter Structs

**Files to Create:**
- `hkask-services/src/adapters/mod.rs` — module exports
- `hkask-services/src/adapters/memory.rs` — MemoryAdapters
- `hkask-services/src/adapters/cns.rs` — CnsAdapters
- `hkask-services/src/adapters/governance.rs` — GovernanceAdapters
- `hkask-services/src/adapters/storage.rs` — StorageAdapters
- `hkask-services/src/adapters/coordination.rs` — CoordinationAdapters
- `hkask-services/src/adapters/identity.rs` — IdentityAdapters

**Verification:** `cargo check -p hkask-services` passes

---

### Phase 4: Add Accessor Methods to AgentService

**Files to Update:**
- `hkask-services/src/context.rs` — impl 7 accessor methods

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

| File | REQ Tag | Purpose |
|------|---------|---------|
| `hkask-services/tests/encapsulation.rs` | REQ-MDS-T1 | Compile-fail: direct field access |
| `hkask-services/tests/accessor_methods.rs` | REQ-MDS-C1 | All 7 accessors exist |
| `hkask-services/tests/build.rs` | REQ-MDS-D1 | Build assembles all domains |
| `hkask-services/tests/lifecycle.rs` | REQ-MDS-L1 | Bootstrap <5 seconds |

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

- ✅ All 27 fields are private (no direct access)
- ✅ 7 domain adapter structs exist
- ✅ 7 accessor methods exist and work correctly
- ✅ All CLI call sites updated
- ✅ All API call sites updated
- ✅ All domain crate call sites updated
- ✅ `cargo test --workspace` passes
- ✅ `cargo clippy --workspace -- -D warnings` passes
- ✅ Compile-fail test ensures encapsulation is enforced

---

*ℏKask — A Minimal Viable Container for Agents — v0.27.1*
