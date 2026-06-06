# Fowler Pattern Audit Remediation — Continuation Prompt

## Session Purpose

Continue the Fowler Pattern Audit remediation on the **hKask** codebase. P1 and P2 items are **complete and verified**. P4.2 and P4.3 are also done. Your job is to continue with P3 items (high impact, higher risk) and remaining P4 items.

---

## What Was Completed

### P1 (Complete — Prior Session)

| ID | Refactoring | Status |
|----|-------------|--------|
| P1.1 | Extract `verify_delegation_token()` + `VerificationOutcome` into `hkask-types/src/capability/verification.rs` | ✅ |
| P1.2 | `EscalationQueue` implements `Store` trait; `ConsentManager.load_from_store()` uses `lock_conn()`; `now_rfc3339()` helper added | ✅ |
| P1.3 | `From<DatabaseError>`, `From<EpisodicMemoryError>`, `From<SemanticMemoryError>` for `MemoryError`; 8 `.map_err()` calls replaced with `?` | ✅ |
| P1.4 | `in_memory_db()` helper in `hkask-storage/src/database.rs` and exported | ✅ |
| P1.5 | `StorageRequest` struct | ✅ (done in P2.4) |
| P1.6 | `DelegationAction::permits_write()/permits_read()`, `DelegationToken::allows_write()/allows_read()`, `require_write_access()`, `require_read_access()` in `hkask-types` | ✅ |
| P1.7 | Dead code removed: `Vault::new()` in keystore, `#[allow(dead_code)] health()` in exa.rs | ✅ |

### P2 (Complete — This Session)

| ID | Refactoring | Files | Status |
|----|-------------|-------|--------|
| P2.1 | Consolidate `AcpState` behind single lock | `hkask-agents/src/acp/mod.rs` | ✅ 5 `Arc<RwLock<...>>` → 1 `Arc<RwLock<AcpState>>` |
| P2.2 | Extract `ApiState::open_db()` builder | `hkask-api/src/lib.rs` | ✅ 4 inline DB connection patterns → 1 `open_db()` helper |
| P2.3 | Domain newtypes for `GasCost`, `RBarThreshold`, `QueueDepth` | `hkask-cns/src/energy.rs`, `hkask-agents/src/curator/curation_gate.rs` | ✅ Defined, `RBarThreshold` wired into `CurationConfidenceGate` |
| P2.4 | Template Method for `MemoryLoopAdapter` + `StorageRequest` struct | `hkask-agents/src/adapters/memory_loop_adapter.rs` | ✅ `StorageRequest` + `to_triple()`; all 3 store methods use it |
| P2.5 | Extract `github_api_url()` builder | `mcp-servers/hkask-mcp-github/src/main.rs` | ✅ 8 URL constructions → `github_api_url()` |
| P2.6 | Extract `parse_webid()` API helper | `hkask-api/src/routes/acp.rs` | ✅ 2 inline blocks → `parse_webid()` |
| P2.7 | Consolidate `MessageDispatch` priority queues | `hkask-agents/src/communication/dispatch.rs` | ✅ 3 `Arc<Mutex<Vec>>` → `Arc<Mutex<HashMap<MessagePriority, VecDeque>>>` |
| P2.8 | Define token error constants | `hkask-types/src/capability/verification.rs`, `mcp_runtime.rs`, `spec/main.rs` | ✅ `TOKEN_ERR_*` constants + `token_err_*()` helpers |

### P4 Items (Partially Complete)

| ID | Refactoring | Status |
|----|-------------|--------|
| P4.2 | Use `AgentKind` methods instead of string literals | ✅ `AgentKind::as_russell_persona()` added |
| P4.3 | Add `now_rfc3339()` helper | ✅ In `hkask-storage/src/store_macros.rs`, exported from lib.rs; used in escalation, goals, triples, etc. |
| P4.1 | Replace `.expect()` in production code | ❌ Not started |
| P4.4 | Audit `.to_string()` error conversions | ❌ Not started |
| P4.5 | Consider `tokio::sync::watch` for metacognition | ❌ Not started |

---

## Key Files Modified (P1 + P2 + P4)

- `crates/hkask-types/src/capability/mod.rs` — `DelegationAction` methods, `DelegationToken` methods, re-exports
- `crates/hkask-types/src/capability/verification.rs` — `VerificationOutcome`, `verify_delegation_token()`, `require_write_access()`, `require_read_access()`, `TOKEN_ERR_*` constants, `token_err_*()` helpers
- `crates/hkask-types/src/lib.rs` — re-exports updated
- `crates/hkask-types/src/agent_def.rs` — `AgentKind::as_russell_persona()` added
- `crates/hkask-storage/src/database.rs` — `in_memory_db()` added
- `crates/hkask-storage/src/lib.rs` — `in_memory_db`, `now_rfc3339` exported
- `crates/hkask-storage/src/store_macros.rs` — `now_rfc3339()` defined
- `crates/hkask-storage/src/goals.rs` — uses `now_rfc3339()`
- `crates/hkask-storage/src/nu_event_store.rs` — uses `now_rfc3339()`
- `crates/hkask-storage/src/standing_session.rs` — uses `now_rfc3339()`
- `crates/hkask-storage/src/triples.rs` — uses `now_rfc3339()`
- `crates/hkask-agents/src/error.rs` — `From` impls for `MemoryError`
- `crates/hkask-agents/src/adapters/memory_loop_adapter.rs` — `StorageRequest`, `require_write_access`/`require_read_access`, `?` operator
- `crates/hkask-agents/src/adapters/mcp_runtime.rs` — unified `verify_delegation_token()`, `TOKEN_ERR_*` constants
- `crates/hkask-agents/src/acp/mod.rs` — `AcpState` consolidated, single lock
- `crates/hkask-agents/src/escalation.rs` — `Store` impl, `lock_conn()`, removed private `now_rfc3339`
- `crates/hkask-agents/src/consent.rs` — `lock_conn()` usage
- `crates/hkask-agents/src/communication/dispatch.rs` — `HashMap<MessagePriority, VecDeque>` + single lock
- `crates/hkask-agents/src/curator/curation_gate.rs` — `RBarThreshold` newtype for thresholds
- `crates/hkask-agents/src/adapters/russell_acp.rs` — uses `AgentKind::as_russell_persona()`
- `crates/hkask-agents/src/registry_loader.rs` — uses `now_rfc3339()`
- `crates/hkask-cns/src/energy.rs` — `GasCost`, `RBarThreshold`, `QueueDepth` newtypes
- `crates/hkask-cns/src/lib.rs` — exports updated
- `crates/hkask-api/src/lib.rs` — `open_db()` helper, store initialization refactored
- `crates/hkask-api/src/routes/acp.rs` — `parse_webid()` helper
- `mcp-servers/hkask-mcp-spec/src/main.rs` — `TOKEN_ERR_*` constants
- `mcp-servers/hkask-mcp-github/src/main.rs` — `github_api_url()` builder
- `mcp-servers/hkask-mcp-keystore/src/main.rs` — dead code removed
- `mcp-servers/hkask-mcp-web/src/providers/exa.rs` — dead code removed

---

## What To Do Next: P3 Items

| ID | Refactoring | Files | Effort | Notes |
|----|-------------|-------|--------|-------|
| **P3.1** | **Unified error hierarchy** | All 6+ error enums across crates | L | Create `hkask-types/src/error.rs` with common variants; crate-level errors use `#[from]` for cross-crate conversion. Eliminates C2.1. |
| **P3.2** | **Split `McpRuntimeAdapter`** into `CapabilityOnlyAdapter` and `FullMcpAdapter` | `hkask-agents/src/adapters/mcp_runtime.rs` | M | The current adapter has an `Option<Arc<McpRuntime>>` and `Option<tokio::runtime::Handle>` — two "modes" that make impossible states representable. Split into two types. |
| **P3.3** | **Extract `RussellProcessManager`** | `hkask-agents/src/adapters/russell_acp.rs` | M | Extract process lifecycle (spawn, monitor, restart) from ACP protocol handling into a dedicated struct. |
| **P3.4** | **A2AMessage visitor pattern** | `hkask-agents/src/acp/mod.rs` | M | Add a `dispatch(&self, msg: A2AMessage)` method that delegates to per-variant handlers instead of inline destructuring in `send_message`. |
| **P3.5** | **Structured storage errors** | Multiple crates | M | Replace all `String`-based error variants with structured enums (e.g., `StorageError::NotFound { entity, id }` instead of `StorageError::Query(format!("not found: {}", id))`). |
| **P3.6** | **Extract escalation logic from metacognition** | `hkask-agents/src/curator_agent/metacognition.rs` | M | Extract `fn check_escalation_conditions(snapshot: &HealthSnapshot) -> Vec<Alert>` so escalation conditions are independently testable. |

## What To Do Next: Remaining P4 Items

| ID | Refactoring | Files | Effort | Notes |
|----|-------------|-------|--------|-------|
| **P4.1** | **Replace `.expect()` in production code** | `api/lib.rs`, `pod/manager.rs` | M | Replace `expect("...")` with `?` or proper error propagation. Panics at runtime are unacceptable. |
| **P4.4** | **Audit `.to_string()` error conversions** | Across all crates | M | Systematically find `.to_string()` in error paths and replace with `#[from]` impls or structured error variants. |
| **P4.5** | **Consider `tokio::sync::watch` for `MetacognitionLoop::last_snapshot`** | `metacognition.rs` | S | More idiomatic for single-value broadcast; currently uses `Arc<RwLock<Option<...>>>`. |

---

## Audit Reference (Full Findings)

### B1 — Long Method

| # | File | Method | Lines | Refactoring |
|---|------|--------|-------|-------------|
| B1.1 | `hkask-api/src/lib.rs` | `ApiState::new()` | ~280 | ✅ Extracted `open_db()` helper (P2.2) |
| B1.2 | `hkask-agents/src/adapters/russell_acp.rs` | `AcpPort::send_message` | ~100 | P3.3: Extract JSON-RPC dispatch |
| B1.3 | `hkask-agents/src/acp/mod.rs` | `AcpRuntime::register_agent` | ~60 | Extract token creation loop, storage persistence |
| B1.4 | `hkask-agents/src/curator_agent/metacognition.rs` | `sense()` + `act()` | ~80 each | P3.6: Extract escalation logic |

### B3 — Primitive Obsession

| # | Instance | Refactoring | Status |
|---|----------|-------------|--------|
| B3.1 | `String`-based error variants | Structured error enums | P3.5 |
| B3.2 | `agent_type` string matching | `AgentKind::as_russell_persona()` | ✅ P4.2 |
| B3.3 | `GasCost` (bare `u64`) | Newtype | ✅ P2.3 (defined, not yet wired into `GasEstimator`) |
| B3.4 | `RBarThreshold` (bare `f64`) | Newtype | ✅ P2.3 (wired into `CurationConfidenceGate`) |
| B3.5 | `QueueDepth` (bare `f64`) | Newtype | ✅ P2.3 (defined, not yet wired into `SetPoints`) |

### B4/B5 — Long Parameter List / Data Clumps

| # | Instance | Refactoring | Status |
|---|----------|-------------|--------|
| B4.1 | `store_episodic(producer, entity, attr, value, confidence, token)` | `StorageRequest` struct | ✅ P2.4 |
| B4.2 | `DelegationToken::new(6 params)` | Builder pattern already exists (`DelegationTokenBuilder`) | Keep |

### B6 — Refused Bequest

| # | Instance | Refactoring | Status |
|---|----------|-------------|--------|
| B6.1 | `McpRuntimeAdapter::new()` creates empty instance | P3.2: Split into two types | ❌ |
| B6.2 | `EnsembleError` unused variants | Consider splitting | Low priority |

### O2 — Switch Statements

| # | Instance | Refactoring | Status |
|---|----------|-------------|--------|
| O2.1 | `MessagePriority` match in `send()`/`receive()` | ✅ P2.7: Replaced with HashMap | Done |
| O2.2 | `agent_type` string match | ✅ P4.2: `AgentKind::as_russell_persona()` | Done |
| O2.3 | `A2AMessage` destructuring | P3.4: Visitor/dispatch pattern | ❌ |

### C1 — Divergent Change

| # | Struct | Refactoring | Status |
|---|--------|-------------|--------|
| C1.1 | `ApiState` | ✅ P2.2: Extracted `open_db()` helper | Partially done |
| C1.2 | `PodManagerBuilder` | Consider `SubsystemBundle` grouping | Low priority |

### C2 — Shotgun Surgery

| # | Change Type | Refactoring | Status |
|---|-------------|-------------|--------|
| C2.1 | New error variant | P3.1: Unified error hierarchy | ❌ |
| C2.2 | New storage operation | Generate port-adapter pairs via macro | Future |
| C2.3 | Token verification in new MCP server | ✅ P1.1 + P2.8: `verify_delegation_token()` + `TOKEN_ERR_*` constants | Done |

### D1 — Duplicate Code

| # | Pattern | Refactoring | Status |
|---|----------|-------------|--------|
| D1.1 | Token verification logic | ✅ P1.1: `verify_delegation_token()` in `hkask-types` | Done |
| D1.2 | Capability-denied guard | ✅ P1.6: `require_write_access()` / `require_read_access()` | Done |
| D1.3 | `Database::in_memory().expect()` | ✅ P1.4: `in_memory_db()` helper | Done |
| D1.4 | Lock poisoning guard | ✅ P1.2: `lock_conn()` in `EscalationQueue` and `ConsentManager` | Done |
| D1.5 | `.map_err()` chains | ✅ P1.3: `From` impls for `MemoryError` | Done |
| D1.6 | WebID parsing in API | ✅ P2.6: `parse_webid()` | Done |
| D1.7 | GitHub URL construction | ✅ P2.5: `github_api_url()` | Done |
| D1.8 | Token error strings | ✅ P2.8: `TOKEN_ERR_*` constants | Done |
| D1.9 | MessageDispatch priority queues | ✅ P2.7: HashMap + single lock | Done |
| D1.10 | Error message strings | ✅ P2.8: Centralized constants | Done |

### D2 — Dead Code

| # | Instance | Refactoring | Status |
|---|----------|-------------|--------|
| D2.1 | `Vault::new()` in keystore | ✅ P1.7: Removed | Done |
| D2.2 | `health()` in exa.rs | ✅ P1.7: Removed | Done |

### F1 — Feature Envy

| # | Method | Envy Target | Refactoring | Status |
|---|--------|-------------|-------------|--------|
| F1.1 | `PodContext::store_episodic` | `EpisodicStoragePort` | ✅ P1.6 + P2.4: `require_write_access()` + `StorageRequest` | Done |
| F1.2 | `PodContext::recall_episodic` | `EpisodicStoragePort` | ✅ P1.6: `require_read_access()` | Done |
| F1.3 | `McpRuntimeAdapter::invoke_tool` verification | `CapabilityChecker` + `DelegationToken` | ✅ P1.1: `verify_delegation_token()` | Done |

### F2 — Inappropriate Intimacy

| # | Instance | Refactoring | Status |
|---|----------|-------------|--------|
| F2.1 | `PodContext` inspects `token.action` directly | ✅ P1.6: `token.allows_write()` / `allows_read()` | Done |
| F2.2 | `russell_acp.rs` constructs JSON-RPC | P3.3: Encapsulate in `RussellProtocol` builder | ❌ |

### K2 — Form Template Method

| # | Pattern | Refactoring | Status |
|---|---------|-------------|--------|
| K2.1 | `store_episodic` / `store_semantic` identical except backend | ✅ P2.4: `StorageRequest` + `to_triple()` | Done |
| K2.2 | `recall_episodic` / `recall_semantic` identical guard + map | Could still apply template for recall; lower priority | ❌ |
| K2.3 | `sense()` / `act()` share escalation threshold logic | P3.6 | ❌ |

### K6 — Consolidate Duplicate Conditional

| # | Condition | Refactoring | Status |
|---|-----------|-------------|--------|
| K6.1 | `if token.action == DelegationAction::Read` (4×) | ✅ P1.6: `require_write_access()` / `allows_write()` | Done |
| K6.2 | `if let Some(checker) = ... verify()` (2×) | ✅ P1.1: `verify_delegation_token()` | Done |
| K6.3 | `Utc::now().to_rfc3339()` (3×) | ✅ P4.3: `now_rfc3339()` | Done |

### Arc<...> Wrapping Pattern

| # | Struct | Refactoring | Status |
|---|---------|-------------|--------|
| A1 | `AcpRuntime` (6 `Arc<RwLock<...>>`) | ✅ P2.1: Single `Arc<RwLock<AcpState>>` | Done |
| A2 | `CuratorContext` (optional Arcs) | Acceptable for now | — |
| A3 | `MetacognitionLoop` (Arc<RwLock<Vec/Option>>) | P4.5: Consider `tokio::sync::watch` | ❌ |
| A4 | `MessageDispatch` (3 Arc<Mutex<Vec>>) | ✅ P2.7: `HashMap<MessagePriority, VecDeque>` behind single lock | Done |
| A5 | `RussellAcpAdapter` | P3.3: Extract `RussellProcessManager` | ❌ |

---

## Design Constraints (from AGENTS.md)

- **No visual UI** — CLI/MCP/API only
- **No monitoring stacks** — CNS provides programmatic observability
- **No excess complexity** — No unused traits, stubs, `#[allow(dead_code)]`, feature flags

```bash
# Constraint verification
if grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"; then echo "VIOLATION: Headless"; exit 1; fi
if grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/; then echo "VIOLATION: P6/P7"; exit 1; fi
```

---

## Build Verification Command

```bash
cargo check --workspace && cargo test -p hkask-types -p hkask-agents -p hkask-storage -p hkask-memory
```

---

*Cumulative status: P1 (7/7 ✅), P2 (8/8 ✅), P4.2 ✅, P4.3 ✅. Remaining: P3 (0/6), P4.1, P4.4, P4.5.*