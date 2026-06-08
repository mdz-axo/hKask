# Continuation Prompt — hKask Service Layer Post-Extraction

**Session 11 — Open Questions F9, F10, F7**

---

## 0. Mandatory Pre-Work

**You MUST load these skills BEFORE writing any code. Do not skip this step.**

1. **`refactor-service-layer`** — Strangler fig process, deletion test, depth test, verification checklist. Any ServiceContext restructuring or further extraction must follow this skill.
2. **`coding-guidelines`** — Think before coding. Surgical changes only. Every changed line traces to the task.
3. **`tdd`** — RED→GREEN→REFACTOR per behavior. Every new path gets a `// REQ:` tagged test.
4. **`constraint-forces`** — Classify every decision by force type. F9 is P1 (Prohibition); F10 is Guardrail. Never silently relax.
5. **`zoom-out`** — Use BEFORE touching ServiceContext structuring, memory store wiring, or env-var consolidation.
6. **`improve-codebase-architecture`** — Required for F10 sub-struct decomposition.
7. **`diagnose`** — If memory persistence or ServiceContext refactoring introduces regressions, use the disciplined diagnosis loop.

---

## 1. Session Context

The 9-task service layer extraction plan is **COMPLETE** (Sessions 1–10). All 5 service modules are extracted and wired, surface assembly migrated, CAS dead code removed, secret resolution audited, error mapping unified, full verification passed, documentation updated.

| Session | Tasks | Key Outcome |
|---------|-------|-------------|
| 1 | 1–3 | `hkask-services` crate skeleton, `ServiceError`, `ServiceConfig`, `ServiceContext` |
| 2 | Re-audit + 4 start | Fixed 8 bugs, created `InferenceService` (3 functions, 4 tests) |
| 3 | 4 completion | Wired CLI (8 sites) + API (4 sites) to InferenceService |
| 4 | 5 | `CuratorService` (6 functions, 6 tests) — full strangler fig cycle |
| 5 | 6a | `EnsembleService` (8 functions, 11 tests) — standing/improv excluded |
| 6 | 6b | `PodService` (6 functions, 6 tests) — fixed CLI error-swallowing bug |
| 7 | 6c-skipped, 6d | `SovereigntyService` (9 functions + 2 types, 13 tests) |
| 8 | 6e/6f/6g-skipped, 7a | Depth tests failed for 3 domains; added `From<&ServiceContext>` for all 5 contexts |
| 9 | 7b | Surface assembly migration — both `ApiState` and `ReplState` compose `ServiceContext::build()`. Deleted ~460 lines of duplicated assembly code |
| 10 | 7c/7d/7f, 8, 9 | CAS dead code removed, secret resolution audited, error mapping unified, full verification, documentation |

**Current test count:** 49 tests (46 service-layer + 3 API). Workspace compiles clean with `clippy -D warnings`.

This session shifts from the 9-task extraction plan to **addressing open questions** from `OPEN_QUESTIONS.md`, prioritized by force type and user impact.

---

## 2. What Was Completed Through Session 10

### Task 6 — COMPLETE (5 extracted, 4 skipped)

| Module | Status | Functions | Tests | Skip Reason |
|--------|--------|-----------|-------|-------------|
| `inference.rs` | ✅ DONE | 3 | 4 | — |
| `curator.rs` | ✅ DONE | 6 | 6 | — |
| `ensemble.rs` | ✅ DONE | 8 | 11 | — |
| `pods.rs` | ✅ DONE | 6 | 6 | — |
| `sovereignty.rs` | ✅ DONE | 9 + 2 types | 13 | — |
| `memory.rs` | ⏭ SKIPPED | — | — | Depth test failed (2 sites, P1 OCAP-gated) |
| `spec.rs` | ⏭ SKIPPED | — | — | Depth test failed (4 sites, API stubs) |
| `goal.rs` | ⏭ SKIPPED | — | — | Depth test failed (CRUD pass-throughs) |
| `models.rs` | ⏭ SKIPPED | — | — | Covered by InferenceService |

### Task 7 — COMPLETE (all sub-tasks done)

| Sub-task | Status | Key Change |
|----------|--------|------------|
| 7a | ✅ | `From<&ServiceContext>` for all 5 context types; `session_manager` added |
| 7b | ✅ | Surface assembly migration; ~460 lines deleted; both surfaces compose `ServiceContext::build()` |
| 7c | ✅ | CAS dead code removed; `define_store_cas!` macro deleted; 6 stores simplified |
| 7d | ✅ | Secret resolution audit: all main paths through `ServiceConfig` |
| 7e | ✅ | CNS/Loop/EventSink wiring absorbed into 7b |
| 7f | ✅ | 3 sovereignty routes use `ApiError::from`; remaining direct constructions are legitimate |

### Tasks 8 & 9 — COMPLETE

Full workspace verification passed. Documentation updated (test-inventory.md, architecture-master.md, OPEN_QUESTIONS.md).

---

## 3. Current State of ServiceContext

`ServiceContext::build()` in `crates/hkask-services/src/context.rs` has 20 fields:

```rust
pub struct ServiceContext {
    pub registry: Arc<Mutex<SqliteRegistry>>,
    pub mcp_runtime: Arc<McpRuntime>,
    pub mcp_dispatcher: Arc<McpDispatcher>,
    pub cns_runtime: Arc<RwLock<CnsRuntime>>,
    pub cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,
    pub loop_system: Arc<LoopSystem>,
    pub dispatch: Arc<MessageDispatch>,
    pub inference_port: Option<Arc<dyn InferencePort>>,
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,
    pub semantic_storage: Arc<dyn SemanticStoragePort>,
    pub escalation_queue: Arc<EscalationQueue>,
    pub consent_manager: Arc<ConsentManager>,
    pub goal_repo: Arc<SqliteGoalRepository>,
    pub pod_manager: Arc<PodManager>,
    pub capability_checker: Arc<CapabilityChecker>,
    pub system_webid: WebID,
    pub event_sink: Arc<dyn NuEventSink>,
    pub standing_session_store: Arc<StandingSessionStore>,
    pub session_manager: Arc<RwLock<SessionManager>>,
    pub config: ServiceConfig,
}
```

All 5 context types derive via `From<&ServiceContext>`:

| Context | Derivation | Limitation |
|---------|-----------|------------|
| `InferenceContext` | `ctx.inference_port`, `ctx.config.default_model`, `ctx.config.okapi_base_url` | None |
| `PodContext` | `ctx.pod_manager.clone()` | None |
| `SovereigntyContext` | `ctx.consent_manager.clone()` | None |
| `CuratorContext` | `ctx.escalation_queue`, `cns_runtime: None`, `dispatch: Some(ctx.dispatch)` | Escalation-only; use `from_service_context(ctx).await` for full |
| `EnsembleContext` | `ctx.session_manager.clone()` | None |

---

## 4. What Remains — Open Questions

Organized by priority. Read `OPEN_QUESTIONS.md` for full details on each.

### HIGH — F9: Production memory stores use `in_memory_db()`

**Problem:** Episodic/semantic memory stores use `in_memory_db()` even when `config.in_memory: false`. Production deployments lose all memories on restart. This is a **P1 User Sovereignty Guardrail** — the user configured persistent storage and got ephemeral.

**Task:** Add `memory_db_path` and `memory_passphrase` to `ServiceConfig`; wire file-backed DB construction when `in_memory: false` in `ServiceContext::build()`; add integration test verifying persistence across restart.

**Strategy:**
1. Zoom out on `ServiceContext::build()` memory store construction (lines ~228-250 in `context.rs`)
2. Add two fields to `ServiceConfig`: `memory_db_path: Option<String>`, `memory_passphrase: Option<String>`
3. In `ServiceContext::build()`, when `!config.in_memory`, construct file-backed DB for episodic/semantic stores using `memory_db_path` or derive from `db_path` (e.g., `hkask-memory-episodic.db`, `hkask-memory-semantic.db`)
4. Write test: construct ServiceContext with `in_memory: false`, store a triple, drop ServiceContext, reconstruct, verify triple persists
5. Verify: `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`

**Constraint forces:**
- F9 is a **P1 User Sovereignty Guardrail** — user configured persistent storage, got ephemeral
- F10 is a **Guardrail** — adding `memory_db_path`/`memory_passphrase` grows ServiceConfig, not ServiceContext. ServiceContext stays at 20 fields
- P6/P7 **Prohibition** — no `todo!` or `unimplemented!`
- **Guideline** — memory DB path should default from `db_path` base (e.g., `hkask-memory-episodic.db` next to `hkask.db`) so no explicit config needed for standard deployments

### MEDIUM — F10: ServiceContext god-object (20 fields)

**Problem:** `ServiceContext` has 20 public `Arc` fields. Each new concern (memory persistence, session lifecycle, auth context) will add more. Guard against growth by grouping into sub-structs and adding `#[non_exhaustive]`.

**Task:** Propose sub-struct grouping; implement; verify no call sites break.

**Strategy:**
1. Use `improve-codebase-architecture` skill to analyze ServiceContext usage patterns
2. Group fields by domain concern:
   - **InfraContext**: registry, mcp_runtime, mcp_dispatcher, capability_checker, config
   - **LoopContext**: cns_runtime, cybernetics_loop, loop_system, dispatch, event_sink
   - **AgentContext**: inference_port, episodic_storage, semantic_storage, escalation_queue, consent_manager, goal_repo, pod_manager, system_webid, standing_session_store, session_manager
3. `ServiceContext` becomes: `pub infra: InfraContext`, `pub loops: LoopContext`, `pub agents: AgentContext`
4. Update all `From<&ServiceContext>` impls to navigate sub-structs
5. Update `ServiceContext::build()` to construct sub-structs
6. Update both surfaces (`ApiState`, `ReplState`) to navigate sub-structs
7. Verify: `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`

**Constraint forces:**
- F10 is a **Guardrail** — measured boundary, user-overridable
- **Prohibition (P1)** — do NOT add new fields during restructuring; only reorganize existing ones
- **Guideline** — every `pub` field in a sub-struct needs 2+ consumers or it should stay flat
- **Guideline** — sub-structs should be `#[non_exhaustive]` to prevent external construction

**Warning:** This is the highest-risk task in this session. ServiceContext is used by 5 context types, 2 surfaces, and the `build()` function. A bad decomposition will break everything. **Do F9 first** (simpler, independent), then attempt F10 only if time permits.

### MEDIUM — F7: ServiceConfig vs environment variables

**Problem:** `HKASK_DB_PATH` is read in 3 places: `ServiceConfig::from_env()`, `ServiceConfig::from_secrets()`, and `registry_db_path()` in `cli/commands/config.rs`. Should be resolved once.

**Task:** Consolidate all env-var reads into `ServiceConfig`; ensure `registry_db_path()` uses `ServiceConfig.db_path` or accepts it as a parameter.

**Strategy:**
1. Grep all `std::env::var("HKASK_DB_PATH")` and `std::env::var("OKAPI_BASE_URL")` call sites
2. For standalone CLI commands that don't have a `ServiceConfig`, add `db_path` parameter to functions like `open_registry_db()`, `open_spec_store()`, `open_sovereignty_store()`
3. For main assembly paths, ensure they use `ServiceConfig.db_path` (already done)
4. Verify: `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`

**Constraint forces:**
- F7 is a **Guideline** — best practice, relaxable with reason
- **Prohibition (P1)** — standalone CLI commands must still work without a full `ServiceContext`; don't force them to construct one just to get `db_path`

---

## 5. Constraint Forces

These are non-negotiable. Classify every design decision by force type.

| Constraint | Force | Implication |
|-----------|-------|-------------|
| MCP servers do NOT depend on `hkask-services` | Prohibition (P1) | Never modify `mcp-servers/` code |
| OCAP gates stay in domain crates / surfaces | Prohibition (P1) | Service layer never decides access |
| F9: User configured persistence, got ephemeral | Guardrail (P1 User Sovereignty) | Must fix — memory stores must respect `in_memory: false` |
| F10: ServiceContext ≤ 20 fields | Guardrail | Any new field or restructuring needs strong justification |
| No `todo!` or `unimplemented!` in `hkask-services` | Prohibition (P6/P7) | Write real code or return errors |
| Dependency direction: CLI/API → services → domain | Guideline (P12) | Never the reverse |
| One domain per commit | Guideline (P5) | One task at a time |
| Depth test: 8+ call sites | Guideline | If not met, deepen or skip |

---

## 6. Key Files to Read First

Read these IN ORDER before writing any code:

1. **`HANDOFF.md`** — Authoritative project state, 43 key decisions, architectural context for all 5 service modules
2. **`OPEN_QUESTIONS.md`** — Full F1–F26 status, priorities, next-actions
3. **`crates/hkask-services/src/context.rs`** — ServiceContext::build() — the heart of the service layer
4. **`crates/hkask-services/src/config.rs`** — ServiceConfig — where new config fields go
5. **`crates/hkask-services/src/error.rs`** — ServiceError hierarchy
6. **`crates/hkask-storage/src/store_macros.rs`** — Store macro definitions (recently cleaned up)
7. **`docs/architecture/hKask-architecture-master.md`** — Architecture master doc (recently updated with service layer section)

---

## 7. Detailed Task Sequence

### Recommended Order: F9 → F10 → F7

**F9 should go first** because it's the highest priority (P1 User Sovereignty), is self-contained (only touches ServiceConfig and ServiceContext::build() memory store construction), and doesn't depend on F10 restructuring. If F10 restructuring happens first, F9's changes would need to be rebased into the new structure — wasted work.

**F10 is the highest-risk task.** ServiceContext restructuring touches 5 context types, 2 surfaces, and the build function. Only attempt if F9 is complete and workspace is verified green.

**F7 is the simplest task.** If F9 and F10 take the full session, F7 can be deferred to a future session.

### Task F9 — Production memory persistence

**Step 1: Zoom out on memory store construction**
- Read `ServiceContext::build()` in `context.rs` — find where episodic_storage and semantic_storage are constructed
- Read `ServiceConfig` fields — identify what exists vs what's needed
- Read `MemoryLoopAdapter::in_memory_unchecked()` — understand what's used for in-memory mode

**Step 2: Design ServiceConfig additions**
- Add `memory_db_path: Option<String>` (defaults to `hkask-memory.db` derived from `db_path`)
- Add `memory_passphrase: Option<String>` (defaults to `db_passphrase` if not set)
- Update `from_env()`, `from_secrets()`, `in_memory()` constructors

**Step 3: Wire file-backed memory stores**
- In `ServiceContext::build()`, when `!config.in_memory`, construct file-backed DB for episodic/semantic stores
- When `config.in_memory`, keep existing `in_memory_db()` behavior
- Test: RED first — write test that expects memory to persist across ServiceContext instances

**Step 4: Verify**
- `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`

### Task F10 — ServiceContext sub-struct grouping (if time permits)

**Step 1: Analyze ServiceContext usage**
- Use `improve-codebase-architecture` skill
- Map which fields are used together by each context type
- Identify natural groupings

**Step 2: Propose sub-struct decomposition**
- Write the proposed struct definitions
- Count consumers for each field — ensure 2+ per sub-struct field
- Check for circular dependencies between sub-structs

**Step 3: Implement and verify**
- Restructure `ServiceContext::build()` to construct sub-structs
- Update all `From<&ServiceContext>` impls
- Update both surfaces
- Full workspace verification

**Step 4: Add `#[non_exhaustive]`**
- Add to `ServiceContext` and each sub-struct
- Verify external code can't construct sub-structs directly

### Task F7 — ServiceConfig env-var consolidation (if time permits)

**Step 1: Audit all env-var reads**
```bash
grep -rn "env::var\|std::env::var" crates/hkask-cli/ crates/hkask-api/ crates/hkask-services/ --include="*.rs" | grep -i "hkask\|okapi"
```

**Step 2: Consolidate**
- Ensure `from_env()` reads all env vars once
- Ensure `from_secrets()` reads env vars (except secrets) once
- Add `db_path` parameter to standalone CLI functions that need it

**Step 3: Verify**
- `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`

---

## 8. Key Decisions to Preserve (43+ Total)

Read `HANDOFF.md` Section 5 for the full list. Most critical for this session:

| # | Decision | Force | Impact |
|---|----------|-------|--------|
| 2 | `ServiceContext::build()` is async | Guideline | All callers `.await` it |
| 12 | Dependency direction: CLI/API → services → domain | Guideline | Never the reverse |
| 39 | Surface code uses ServiceContext for assembly | Guideline (now Prohibition) | Both surfaces compose `ServiceContext::build()` |
| 40 | CAS store write-through is dead code | Evidence (F26 CLOSED) | Removed from 6 stores; read-only `git_cas_port` untouched |
| 41 | Secret resolution: all main paths through ServiceConfig | Evidence | Remaining direct calls are by design |
| 42 | Sovereignty API routes use `ApiError::from` | Guideline | 3 routes fixed; remaining direct constructions are legitimate |
| 43 | Remaining direct `ApiError::` constructions are surface concerns | Guideline | ~11 constructions for input validation, OCAP gates, auth |

---

## 9. Anti-Patterns to Avoid

1. **Adding fields to ServiceContext without depth justification** — F10 flags 20 fields as a god-object risk. Any new field needs 8+ consumer sites or a compelling alternative.
2. **Breaking the build** — Every step must leave the workspace compiling and all tests passing.
3. **Treating F9 as a Guideline** — F9 is a **Guardrail** (P1 User Sovereignty). User configured persistence, got ephemeral. Must fix.
4. **Restructuring ServiceContext without understanding all consumers** — F10 touches 5 context types, 2 surfaces, and build(). A bad decomposition cascades.
5. **Forcing standalone CLI commands through ServiceContext** — P1: standalone commands should work without a full ServiceContext. Use parameters, not forced construction.
6. **Horizontal slicing** — Don't audit all env vars then fix them all. One site → verify → next.
7. **Adding speculative `From` impls** — Only add `From<ServiceError>` arms that are actually needed by call sites.

---

## 10. Open Questions Requiring Attention

| ID | Question | Priority | Status |
|----|----------|----------|--------|
| F9 | Production memory stores use `in_memory_db()` | HIGH | **P1 User Sovereignty Guardrail** — must fix |
| F10 | ServiceContext god-object (20 fields) | MEDIUM | Guard with sub-structs; investigate grouping |
| F7 | ServiceConfig vs environment variables | MEDIUM | Track — consolidate env-var reads |
| F2 | Session lifecycle across surfaces | MEDIUM | Deferred — specify durability semantics first |
| F3 | Unified authentication context | MEDIUM | Deferred — define `AuthContext` struct |
| F6 | REPL vs API state boundary | MEDIUM | Deferred — write boundary table |
| F14 | Dual error mapping in API | MEDIUM | Partially addressed — remaining are legitimate |
| F17 | CuratorService standalone commands open DB each time | MEDIUM | Track |
| F18 | EnsembleService standing session extraction | MEDIUM | Deferred — needs adapter design |
| F19 | EnsembleService improv operation extraction | MEDIUM | Deferred — needs inferencer abstraction |

---

## 11. Recommended Tools and Commands

```bash
# Verify current state before any change
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test -p hkask-services --lib
cargo test -p hkask-api

# After each step
cargo check -p hkask-services       # or hkask-cli, hkask-api
cargo test -p hkask-services --lib    # or relevant crate
cargo clippy -p hkask-services -- -D warnings

# Full verification after each task completion
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace

# Check for violations
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/hkask-services/src/ --include="*.rs"
grep -rn "hkask_services" mcp-servers/ --include="*.rs"  # Should find nothing

# Memory store audit
grep -rn "in_memory_db\|MemoryLoopAdapter" crates/hkask-services/src/ --include="*.rs"
grep -rn "in_memory_db\|MemoryLoopAdapter" crates/hkask-agents/src/ --include="*.rs"

# Env-var audit
grep -rn "env::var\|std::env::var" crates/hkask-cli/ crates/hkask-api/ crates/hkask-services/ --include="*.rs" | grep -i "hkask\|okapi"

# ServiceContext consumer audit
grep -rn "ServiceContext" crates/ --include="*.rs" | grep -v "test" | grep -v "doc" | wc -l
```

---

*ℏKask — A Minimal Viable Container for Agents — v0.23.0*