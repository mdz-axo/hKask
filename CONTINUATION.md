# Continuation Prompt — hKask Service Layer Extraction

**Session 9 — Task 7b: Surface Assembly Migration**

---

## 0. Mandatory Pre-Work

**You MUST load these skills BEFORE writing any code. Do not skip this step.**

1. **`refactor-service-layer`** — The strangler fig process, deletion test, depth test, and verification checklist. This skill governs the entire extraction methodology. Every structural change must follow its Phase 5/Phase 6 process.
2. **`coding-guidelines`** — Think before coding. Surgical changes only. Every changed line must trace to the task.
3. **`tdd`** — RED→GREEN→REFACTOR per behavior. Every new path gets a `// REQ:` tagged test.
4. **`constraint-forces`** — Classify every design decision by force type (Prohibition > Guardrail > Guideline > Evidence > Hypothesis). Never silently relax a Prohibition or Guardrail.
5. **`zoom-out`** — Use BEFORE touching any surface assembly code. Map the four assembly paths, their dependencies, and their surface-specific fields. Produce the "before picture" before changing anything.
6. **`diagnose`** — If `ServiceContext::build()` output diverges from surface assembly output, use the disciplined diagnosis loop before trying to fix mismatches.

---

## 1. Session Context

This is **Session 9** of the hKask service layer extraction. Eight prior sessions completed:

| Session | Tasks | Key Outcome |
|---------|-------|-------------|
| 1 | 1–3 | `hkask-services` crate skeleton, `ServiceError`, `ServiceConfig`, `ServiceContext` |
| 2 | Re-audit + 4 start | Fixed 8 bugs, created `InferenceService` (3 functions, 4 tests) |
| 3 | 4 completion | Wired CLI (8 sites) + API (4 sites) to InferenceService |
| 4 | 5 | `CuratorService` (6 functions, 6 tests) — full strangler fig cycle |
| 5 | 6a | `EnsembleService` (8 functions, 11 tests) — standing/improv excluded |
| 6 | 6b | `PodService` (6 functions, 6 tests) — fixed CLI error-swallowing bug |
| 7 | 6c-skipped, 6d | `SovereigntyService` (9 functions + 2 types, 13 tests) |
| 8 | 6e/6f/6g-skipped, 7a | All three depth tests failed; added `From<&ServiceContext>` for all 5 context types |

**Current test count:** 46 service-layer tests. Workspace compiles clean with `clippy -D warnings`.

---

## 2. What Was Completed

### Task 6 — COMPLETE (5 extracted, 4 skipped)

| Module | Status | Functions | Tests | Skip Reason |
|--------|--------|-----------|-------|-------------|
| `inference.rs` | ✅ DONE | 3 | 4 | — |
| `curator.rs` | ✅ DONE | 6 | 6 | — |
| `ensemble.rs` | ✅ DONE | 8 | 11 | — |
| `pods.rs` | ✅ DONE | 6 | 6 | — |
| `sovereignty.rs` | ✅ DONE | 9 + 2 types | 13 | — |
| `memory.rs` | ❌ SKIPPED | — | — | 2 call sites, P1 OCAP-gated |
| `spec.rs` | ❌ SKIPPED | — | — | 4 sites, API stubs, surface-only ops |
| `goal.rs` | ❌ SKIPPED | — | — | Thin CRUD pass-throughs |
| `models.rs` | ❌ SKIPPED | — | — | Covered by InferenceService |

### Task 7a — COMPLETE

Added `From<&ServiceContext>` impls for all 5 context types, `session_manager` field to ServiceContext, `CuratorContext::from_service_context()` async method, and 6 infrastructure tests.

---

## 3. Current State of ServiceContext

`ServiceContext::build()` in `crates/hkask-services/src/context.rs` already implements the unified assembly. It has 20 fields:

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

All 5 context types can be derived via `From<&ServiceContext>`:

| Context | Derivation | Limitation |
|---------|-----------|------------|
| `InferenceContext` | `ctx.inference_port`, `ctx.config.default_model`, `ctx.config.okapi_base_url` | None |
| `PodContext` | `ctx.pod_manager.clone()` | None |
| `SovereigntyContext` | `ctx.consent_manager.clone()` | None |
| `CuratorContext` | `ctx.escalation_queue`, `cns_runtime: None`, `dispatch: Some(ctx.dispatch)` | Escalation-only; use `from_service_context(ctx).await` for full |
| `EnsembleContext` | `ctx.session_manager.clone()` | None |

---

## 4. What Remains: Task 7b

### Goal

Make `ApiState` and `ReplState` delegate to `ServiceContext::build()` instead of their own independent assembly paths. This eliminates ~930 lines of duplicated infrastructure code across 4 files.

### The Four Assembly Paths to Replace

| # | File | Lines | What It Does |
|---|------|-------|-------------|
| 1 | `crates/hkask-cli/src/repl/init.rs` | ~370 | Inference ports, onboarding, memory infra, CNS, loop system, curation, GovernedTool, HHH gate, PodManager |
| 2 | `crates/hkask-api/src/lib.rs` | ~280 | `ApiState::new()` + `with_defaults()`: stores, loop system via `build_loop_system()`, GovernedTool, PodManager |
| 3 | `crates/hkask-api/src/loop_system.rs` | ~170 | `build_loop_system()`: CNS, cybernetics, episodic/semantic loops, curation, snapshot loops |
| 4 | `crates/hkask-cli/src/commands/loops.rs` | ~113 | Standalone `kask loops` command: CNS, cybernetics, episodic/semantic, curation |

### Surface-Specific Fields That CANNOT Come From ServiceContext

**CLI (`ReplState`):**
- `onboarding_outcome` — interactive session result
- Per-agent memory DB (`hkask-memory-{agent}.db`) — agent-specific, not shared
- HHH gate inference port + config — CLI-specific alignment gate
- Session history — REPL interaction state
- REPL readline state — terminal I/O
- `ConsolidationService` — built from agent-specific memory DB

**API (`ApiState`):**
- `ensemble_inferencer: Option<Arc<InferencePortAdapter>>` — surface-specific inference with circuit breaker
- `spec_store: Option<Arc<dyn SpecStore>>` — optional, initialized separately
- `standing_sessions: Arc<RwLock<HashMap<String, Arc<RwLock<StandingSession>>>>>` — runtime map
- `gas_governance: Arc<dyn GasGovernancePort>` — ensemble gas governance
- `git_cas: Arc<GitCasAdapter>` — legacy template archival
- `git_cas_port: Arc<dyn GitCASPort>` — CAS hexagonal boundary
- `cns_runtime: Arc<CnsRuntime>` — **type mismatch**: ServiceContext has `Arc<RwLock<CnsRuntime>>`, ApiState has `Arc<CnsRuntime>`

### Recommended Strategy: Strangler Fig with Two-Phase Migration

**Phase 1 — API first (simpler surface):**
1. Add `ApiState::from_service_context(service_ctx: ServiceContext)` constructor
2. This constructor derives all shared fields from `ServiceContext` and adds API-specific fields
3. Verify: `cargo test -p hkask-api` passes
4. Make `ApiState::with_defaults()` call `ServiceContext::build()` internally
5. Delete `api/loop_system.rs::build_loop_system()` (no longer needed)
6. Verify full workspace

**Phase 2 — CLI (complex surface):**
1. Add `ReplState` field or method that composes `ServiceContext`
2. The CLI has more surface-specific concerns (onboarding, per-agent DB, HHH gate) so this requires careful decomposition
3. `commands/loops.rs` standalone command can use `ServiceContext::build()` directly
4. Delete old `init_repl_state()` assembly code
5. Verify full workspace

### Key Open Questions to Resolve Before Starting

| ID | Question | Priority | Impact on 7b |
|----|----------|----------|---------------|
| F5 | Test seam depth for `ServiceContext::build()` | HIGH | Need integration test that `ServiceContext::build()` produces same output as surface assemblies |
| F10 | ServiceContext god-object (20 fields) | MEDIUM | Consider sub-structs (InfraContext, LoopContext, AgentContext) before adding more fields |
| F13 | CapabilityChecker secret inconsistency (3 checkers, 2 secrets) | MEDIUM | `ServiceContext::build()` creates `CapabilityChecker::new(&config.mcp_secret)` at L328, but `PodManager` uses `acp_secret` at L330,344. Investigate before wiring. |
| F9 | Production memory stores use `in_memory_db()` | HIGH | `ServiceContext::build()` L252 uses `in_memory_db()` for memory. Production API may need persistent memory. P1 concern. |

---

## 5. Constraint Forces

These are non-negotiable. Classify every design decision by force type.

| Constraint | Force | Implication |
|-----------|-------|-------------|
| MCP servers do NOT depend on `hkask-services` | Prohibition (P1) | Never modify `mcp-servers/` code |
| OCAP gates stay in domain crates / surfaces | Prohibition (P1) | Service layer never decides access |
| Both old and new paths must work before deleting old | Prohibition (P3 Strangler Fig) | Add new path alongside, verify, then delete |
| One domain per commit | Guideline (P5) | One surface at a time (API first, then CLI) |
| Dependency direction: CLI/API → services → domain | Guideline (P12) | Never the reverse |
| No `todo!` or `unimplemented!` in `hkask-services` | Prohibition (P6/P7) | Write real code or return errors |
| Surface context pattern | Guideline | Each service module has its own lightweight context |
| Depth test: 8+ call sites | Guideline | If not met, deepen or skip |

---

## 6. Key Files to Read First

| File | Why |
|------|-----|
| `HANDOFF.md` | Authoritative state — Sections 5 (39 decisions), 6 (what remains), 7 (F1–F25), 9 (architecture) |
| `crates/hkask-services/src/context.rs` | `ServiceContext::build()` + `From<&ServiceContext>` impls + 6 infrastructure tests |
| `crates/hkask-services/src/lib.rs` | Module re-exports |
| `crates/hkask-services/src/error.rs` | `ServiceError` variants (31+ — may need new ones for surface migration) |
| `crates/hkask-api/src/lib.rs` | `ApiState` struct + `new()` + `with_defaults()` — the target for Phase 1 migration |
| `crates/hkask-api/src/loop_system.rs` | `build_loop_system()` — will be deleted after migration |
| `crates/hkask-api/src/stores.rs` | `Stores::init()` — DB/Store init currently in API surface |
| `crates/hkask-api/src/governed_tool.rs` | `build_governed_mcp_tool()` — will be deleted after migration |
| `crates/hkask-cli/src/repl/init.rs` | `init_repl_state()` — the target for Phase 2 migration |
| `crates/hkask-cli/src/commands/loops.rs` | Standalone `kask loops` command |
| `crates/hkask-cli/src/errors.rs` | CLI error adapters (`From<ServiceError>` impls) |
| `crates/hkask-api/src/error.rs` | API error adapters (`From<ServiceError>` impl) |

---

## 7. Detailed Task Sequence

### Task 7b — Phase 1: API Surface Migration

**Step 1: Investigate F13 (CapabilityChecker secret inconsistency)**
- Read `context.rs` L328-344 where ServiceContext creates 3 CapabilityCheckers
- Read `api/lib.rs` to see how ApiState creates its checker
- Document any inconsistency; decide on resolution

**Step 2: Add `ApiState::from_service_context()` constructor**
- This is an ALTERNATIVE constructor, not a replacement
- Takes `ServiceContext` + API-specific fields (ensemble_inferencer, git_cas, etc.)
- Derives all shared fields from ServiceContext
- Write test: construct ApiState both ways, verify same field values

**Step 3: Refactor `ApiState::with_defaults()` to use `ServiceContext::build()`**
- Replace the inline assembly with `ServiceContext::build(config).await?`
- Add API-specific fields on top
- Verify: `cargo test -p hkask-api` passes

**Step 4: Delete `api/loop_system.rs::build_loop_system()`**
- No longer called by any surface code
- Verify: `cargo check --workspace && cargo clippy --workspace -- -D warnings`

**Step 5: Delete `api/governed_tool.rs::build_governed_mcp_tool()`**
- If ServiceContext already constructs GovernedTool + McpDispatcher
- Verify: `cargo test --workspace`

### Task 7b — Phase 2: CLI Surface Migration

**Step 6: Make `kask loops` command use `ServiceContext::build()`**
- Standalone command — simplest CLI migration target
- Replace manual assembly with `ServiceContext::build(config).await?`
- Derive loop_system from ServiceContext
- Verify: `cargo test -p hkask-cli`

**Step 7: Add `ReplState` composition via ServiceContext**
- Most complex: onboarding happens BEFORE ServiceContext, per-agent DB is surface-specific
- Pattern: ServiceContext provides shared infra; ReplState adds CLI-specific fields
- Investigate: Can `init_repl_state()` call `ServiceContext::build()` after onboarding?
- Verify: `cargo test -p hkask-cli`

**Step 8: Delete `cli/repl/init.rs` inline assembly**
- Remove duplicated DB init, CNS construction, loop system registration
- Verify: `cargo check --workspace && cargo test --workspace`

### Task 7f — Error Mapping Unification

After surfaces compose ServiceContext:
- Audit `api/routes/*.rs` for direct `ApiError` construction that should use `ServiceError`
- Add missing `From<ServiceError> for ApiError` arms
- Verify: `cargo test -p hkask-api`

### Task 8 — Verification

```
[ ] Depth test every module in hkask-services
[ ] Dependency direction check (no circular deps)
[ ] cargo check --workspace
[ ] cargo clippy --workspace -- -D warnings
[ ] cargo test --workspace
[ ] Deletion test: removing build_loop_system() doesn't break anything
[ ] No todo!/unimplemented! in hkask-services
```

### Task 9 — Documentation

- Update `docs/status/test-inventory.md` with 46+ test entries
- Update `docs/architecture/hKask-architecture-master.md` with service layer section
- Write `OPEN_QUESTIONS.md` for F1–F25

---

## 8. Recommended Tools and Commands

```bash
# Verify current state before any change
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test -p hkask-services --lib

# After each step
cargo check -p hkask-api       # or hkask-cli
cargo test -p hkask-api        # or hkask-cli
cargo clippy -p hkask-services -- -D warnings

# Full verification after each surface migration
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace

# Check for violations
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/hkask-services/src/ --include="*.rs"
grep -r "hkask_services" mcp-servers/ --include="*.rs"  # Should find nothing
```

---

## 9. Key Decisions to Preserve (39 Total)

Read `HANDOFF.md` Section 5 for the full list. Most critical for this session:

| # | Decision | Force |
|---|----------|-------|
| 2 | `ServiceContext::build()` is async | Guideline |
| 3 | Strangler fig: build alongside, don't replace yet | Guideline (now Prohibition P3 for deletion phase) |
| 12 | Dependency direction: CLI/API → services → domain | Guideline |
| 15 | `ApiState` stores `ServiceConfig` | Guideline |
| 20 | `From<&ServiceContext>` for CuratorContext is escalation-only | Guideline |
| 37 | `session_manager` added to ServiceContext (20 fields) | Guideline |
| 38 | All 5 context types now have `From<&ServiceContext>` | Guideline |
| 39 | Surface code NOT yet changed for Task 7 | Prohibition (P3) — about to change |

---

## 10. Anti-Patterns to Avoid

1. **Big bang replacement** — Don't replace both surfaces at once. API first, then CLI.
2. **Breaking the build** — Every step must leave the workspace compiling and all tests passing.
3. **Adding fields to ServiceContext without depth justification** — F10 is already a concern at 20 fields.
4. **Moving OCAP gates into service layer** — P1 Prohibition. Auth/capability checks stay in surfaces.
5. **MCP servers depending on `hkask-services`** — P1 Prohibition. Never modify mcp-servers/.
6. **Horizontal slicing** — Don't write all the new constructors then test them all. One constructor → one test → verify → next.
7. **Relaxing the depth test** — If a proposed ServiceContext field has fewer than 8 consumer sites, don't add it. Keep it surface-specific.

---

*ℏKask — A Minimal Viable Container for Agents — v0.23.0*