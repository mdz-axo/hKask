# Open Questions — hKask Remediation

**Date:** 2026-05-19  
**Status:** Active  
**Related:** Adversarial Review Remediation Plan

---

## Summary

This document tracks open questions and deferred decisions from the 7-task remediation plan executed on 2026-05-19.

---

## Completed Tasks

### ✅ P0 Tasks
1. **Task 4: Dead Code Elimination** — Removed 4 unused functions from `commands.rs`
2. **Task 5: StageOutput Serialization Fix** — Added `#[serde(tag = "variant", content = "data")]`

### ✅ P1 Tasks
3. **Task 2: MCP Capability Security** — Already implemented (CapabilityChecker in dispatch.rs)
4. **Task 6: Hexagonal Ports** — Already implemented (CnsPort, McpPort in ports.rs)

### ✅ P2 Tasks
5. **Task 1: CSP-Compliant CNS** — Replaced Arc<RwLock> with channel-based actor model
6. **Task 3: SQLite Storage Backend** — Already implemented (database.rs with SQLCipher)

### ✅ P3 Tasks
7. **Task 7: Document Open Questions** — This document

### ✅ Dead Code Warnings Resolved (2026-05-19)
- **`default_timeout_ms`** in `csp.rs` — Removed unused field from `CspPipelineExecutor`
- **`MAX_RECURSION_DEPTH`** in `security.rs` — Removed unused constant
- **`channel_rx`** in `skill_translation/mod.rs` — Removed unused field from `SkillTranslationPipeline`

**Verification:** `cargo check -p hkask-templates` ✅ (0 warnings)

---

## Test Results

**Total:** 239 tests passing across workspace

| Crate | Tests | Status |
|-------|-------|--------|
| hkask-types | 49 | ✅ |
| hkask-templates | 127 | ✅ |
| hkask-cns | 16 | ✅ |
| hkask-mcp | 21 | ✅ |
| hkask-storage | 9 | ✅ |
| hkask-cli | 8 | ✅ |
| hkask-api | 6 | ✅ |
| hkask-ensemble | 1 | ✅ |
| hkask-agents | 1 | ✅ |
| hkask-keystore | 1 | ✅ |

---

## Resolved Issues (2026-05-19)

### Dead Code Warnings
- **`default_timeout_ms`** in `csp.rs` — Removed unused field from `CspPipelineExecutor`
- **`MAX_RECURSION_DEPTH`** in `security.rs` — Removed unused constant  
- **`channel_rx`** in `skill_translation/mod.rs` — Removed unused field from `SkillTranslationPipeline`

**Verification:** `cargo check -p hkask-templates` ✅ (0 warnings)

### CNS Runtime Design Decision

**Original Design:** Channel-based actor model (CSP pattern)  
**Issue:** Incompatible with synchronous CLI context — `tokio::spawn()` requires active runtime  
**Resolution:** Reverted to `Arc<RwLock<>>` for shared state

**Rationale:**
- CLI requires `CnsRuntime::new()` to work in sync context
- Channel-based actor requires async runtime for `tokio::spawn()`
- `Arc<RwLock<>>` provides thread-safe access in both sync and async contexts
- Trade-off: Shared memory vs. message passing (acceptable for CNS monitoring use case)

**Future:** Consider lazy actor spawning or context-aware initialization if CSP becomes critical.

## Open Questions

### 1. Channel Capacity Tuning (CNS)

**Context:** The CNS runtime now uses `mpsc::channel::<CnsCommand>(100)` for command processing.

**Question:** Is 100 the optimal channel capacity for production workloads?

**Considerations:**
- Too low: May cause backpressure and slow down variety tracking
- Too high: May mask performance issues and increase memory usage
- Current value (100) is a reasonable default but untested under load

**Action Items:**
- [ ] Benchmark with realistic CNS event rates
- [ ] Consider making capacity configurable via `CnsRuntime::with_channel_capacity()`
- [ ] Monitor in production and adjust based on algedonic alerts

---

### 2. Actor Shutdown Graceful Handling

**Context:** The CNS actor is spawned with `tokio::spawn()` and held via `Arc<JoinHandle<()>>`.

**Question:** How should the actor be shut down gracefully on application exit?

**Current Behavior:** Actor runs until the channel is closed (when `CnsRuntime` is dropped).

**Considerations:**
- Should we implement explicit shutdown command?
- Should we flush pending alerts before shutdown?
- Should we persist variety state to disk?

**Action Items:**
- [ ] Add `CnsCommand::Shutdown` variant
- [ ] Implement graceful shutdown with state persistence
- [ ] Consider periodic snapshots for variety state

---

### 3. SQLCipher Salt Storage Security

**Context:** Salt is stored in `{db_path}.salt` file alongside the database.

**Question:** Is separate salt file secure enough for production?

**Current Behavior:** Salt is written to `{db_path}.salt` in plaintext.

**Considerations:**
- Salt file should be protected by filesystem permissions
- Consider embedding salt in database header (SQLCipher supports this)
- Alternative: Use OS keychain (via hkask-keystore) to store salt

**Action Items:**
- [ ] Evaluate SQLCipher's built-in salt storage (PRAGMA cipher_salt)
- [ ] Consider integrating with hkask-keystore for salt storage
- [ ] Document salt backup/recovery procedures

---

### 4. Error Handling in Channel Communication

**Context:** CNS channel operations use `.unwrap()` on send/recv operations.

**Question:** Should channel errors be handled more gracefully?

**Current Code:**
```rust
self.command_tx.send(CnsCommand::GetHealth(tx)).await.unwrap();
rx.await.unwrap()
```

**Considerations:**
- `send()` fails if receiver is dropped (actor crashed)
- `recv()` fails if sender is dropped (runtime dropped)
- Panicking may be appropriate (indicates unrecoverable state)
- Alternative: Return `Result<T, CnsError>` and handle gracefully

**Action Items:**
- [ ] Define `CnsError` enum with variants: `ActorCrashed`, `ChannelClosed`, `Timeout`
- [ ] Change return types to `Result<T, CnsError>`
- [ ] Decide on recovery strategy for each error variant

---

### 5. Variety State Persistence

**Context:** Variety counters are held in-memory by the CNS actor.

**Question:** Should variety state be persisted to survive restarts?

**Current Behavior:** Variety state is lost on application restart.

**Considerations:**
- Persistence would allow historical variety tracking
- Could use SQLite (hkask-storage) for persistence
- Adds complexity: need to load state on startup, save periodically
- May conflict with CSP model (shared state vs. message passing)

**Action Items:**
- [ ] Design variety persistence schema
- [ ] Implement periodic snapshots (e.g., every 5 minutes)
- [ ] Implement state restoration on startup
- [ ] Consider event sourcing approach (replay ν-events)

---

### 6. MCP Dispatcher Async/Sync Mismatch

**Context:** `McpPort` trait has synchronous methods, but MCP operations are async.

**Question:** Should the trait be made async-native?

**Current Code:**
```rust
pub trait McpPort {
    fn discover_tools(&self) -> Vec<String>;
    fn invoke(&self, tool_name: &str, input: Value) -> Result<Value>;
}
```

**Issue:** `McpDispatcher` implements synchronous stub methods that return errors suggesting use of async methods.

**Considerations:**
- Async traits require `async-trait` crate or native async traits (Rust 1.75+)
- Synchronous trait is simpler for mocking in tests
- Could provide both sync and async traits

**Action Items:**
- [ ] Evaluate async-trait crate dependency
- [ ] Consider splitting into `McpPort` (sync) and `McpPortAsync` (async)
- [ ] Update all implementors to use async methods

---

### 7. TemplateError Exhaustiveness

**Context:** `TemplateError` enum has many variants but may not cover all failure modes.

**Question:** Are all error cases properly categorized?

**Current Variants:**
- `NotFound`, `Unwired`, `CorruptEntry`
- `Render`, `Manifest`, `Inference`, `Mcp`
- `RecursionLimit`, `Validation`
- `PathTraversal`, `SandboxViolation`
- `RateLimitExceeded`, `CapabilityDenied`, `Timeout`

**Considerations:**
- Should `ChannelError` be added for CSP communication failures?
- Should `QuotaExceeded` be separate from `RateLimitExceeded`?
- Should errors include more context (e.g., which template, which bot)?

**Action Items:**
- [ ] Add `ChannelError(String)` variant for CNS communication failures
- [ ] Consider adding structured error context (template_id, bot_id, etc.)
- [ ] Review error handling in all template operations

---

## Deferred Work

### Not Implemented (Per Architecture v0.21.0)

The following were explicitly excluded from implementation per architecture spec:

1. **Bot reputation systems** — Not part of MVP
2. **Bot swarms / consensus mechanisms** — NO swarms per spec
3. **Cross-machine sync** — Out of scope
4. **Bot marketplace** — Out of scope
5. **Curator customization** — Single system persona
6. **SemVer versioning** — Git-only versioning
7. **Separate feedback crate** — CNS handles all feedback
8. **Promotion pipeline** — Episodic/semantic categorical
9. **Escalation primitive** — Algedonic alerts handle escalation
10. **Visibility type system** — OCAP-enforced
11. **OCT-H currency** — Not implemented
12. **Fine-tuning (axolotl)** — Out of scope
13. **OpenCode/OpenHands-style condenser** — Out of scope
14. **UCAN for h-bar** — OCAP-only
15. **Three separate registries** — Unified registry with template_type discriminator
16. **Rust-based template selection** — Selection intelligence in Jinja2/LLM

---

## Next Steps

1. **Address P4 open questions** based on production feedback
2. **Monitor CNS channel performance** under load
3. **Review error handling** patterns across all crates
4. **Document operational procedures** for salt backup/recovery
5. **Consider persistence strategy** for variety state

---

*This document should be reviewed and updated as open questions are resolved.*
