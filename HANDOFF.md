# HANDOFF.md — hKask Service Layer Extraction

**Session:** 12 (Audit) → 13 (Execution)
**Status:** Build broken (11 compilation errors in `hkask-api`). Service layer less than halfway through strangler fig migration. Previous session incorrectly declared all open questions resolved.

---

## 1. Session Context

Session 12 audited 7 Tier 1 MEDIUM-priority open questions (F2, F3, F6, F14, F17, F18, F19) for the service layer extraction. It completed design audits but **made a critical error**: concluding every question was "by design" with no code extraction warranted, when the strangler fig pattern was incomplete — old direct-access paths in both CLI and API surfaces were never deleted after service modules were wired in. Mid-session, `ApiState` was refactored to hold `Arc<ServiceContext>` but left broken. The user caught the mistake and demanded honest documentation. **We are less than halfway through the real extraction work.**

---

## 2. What Was Done

### ApiState Refactoring (BROKEN — in progress)
- Replaced 20 individual domain fields in `ApiState` with single `service_context: Arc<ServiceContext>` field
- `from_service_context()` simplified to store `Arc::new(ctx)` plus 6 surface-specific fields
- **Left 11 compilation errors**: old `state.field` references in `lib.rs` methods and 2 syntax errors in `bundles.rs`
- File: `crates/hkask-api/src/lib.rs`

### Audit Findings (initially wrong — now corrected)
- F17: CLI curator commands open their own DB instead of using ServiceContext — initially closed as "by design", **must be reopened**
- F2/F3: Analysis findings correct but conclusions wrong — need re-evaluation
- F6: Boundary table is valid documentation
- F14: Closure valid (remaining ApiError constructions are HTTP-layer concerns)
- F18/F19: Closures valid (too divergent / CLI-only)

### Current Build State
```
cargo check -p hkask-api → 11 errors:
  - 2 syntax errors in bundles.rs (missing semicolons at L149, L182)
  - E0609: state.loop_system → state.service_context.loop_system (3 sites in lib.rs)
  - E0609: state.consent_manager → state.service_context.consent_manager (1 site in lib.rs)
  - E0609: state.session_manager → state.service_context.session_manager (1 site in lib.rs)
  - E0425: compose_bundle / get_bundle not found in bundles.rs (routing to non-existent functions)
  - 10 unused import warnings in lib.rs
```

---

## 3. What Remains

### HIGH — Fix the broken build

**Task 1: Fix `ApiState` method references in `lib.rs`**
- Replace `self.loop_system` → `self.service_context.loop_system` (3 sites: L235, L238, L247)
- Replace `self.consent_manager` → `self.service_context.consent_manager` (L180 in `with_consent_manager`)
- Replace `self.session_manager` → `self.service_context.session_manager` (L215 in `with_session_manager`)
- Delete unused imports (L55-62: `ConsentManager`, `EscalationQueue`, `LoopSystem`, `PodManager`, `EpisodicStoragePort`, `CnsRuntime`, `StandingSessionStore`, `SqliteGoalRepository`, `InferencePort`, `CapabilityChecker`, `WebID`)
- **BUT** `with_consent_manager` and `with_session_manager` are problematic: they mutate fields now inside `Arc<ServiceContext>`. Either remove these methods (preferred — ServiceContext is the single source of truth) or make ServiceContext fields mutable via interior mutability.

**Task 2: Fix syntax errors in `bundles.rs`**
- L149: Add missing semicolon after `serde_json::to_value(&bundle).unwrap_or(...)`
- L182: Add missing semicolon after `registry.find_bundle_by_skills(&request.skills)`

**Task 3: Fix `bundles.rs` routing — `compose_bundle` / `get_bundle`**
- The router references `compose_bundle` and `get_bundle` but the actual function names in the file are `compose_bundle` and `get_bundle` — check if they were accidentally renamed or if there's a visibility issue.

### HIGH — API route strangler fig completion

**Task 4: Replace `from_parts()` with `From<&ServiceContext>` derivation**
Add `From<&ServiceContext>` implementations in `hkask-services` and replace manual context construction:

| Route file | Current pattern | Sites | Target |
|-----------|----------------|-------|--------|
| `routes/curator.rs` | `CuratorContext::from_parts(state.service_context.escalation_queue.clone(), None, None)` | 4 | `CuratorContext::from(&state.service_context)` |
| `routes/sovereignty.rs` | `SovereigntyContext::from_parts(state.service_context.consent_manager.clone())` | 4 | `SovereigntyContext::from(&state.service_context)` |
| `routes/pods.rs` | `PodContext::from_parts(state.service_context.pod_manager.clone())` | 5 | `PodContext::from(&state.service_context)` |
| `routes/ensemble.rs` | `EnsembleContext::from_parts(state.service_context.session_manager.clone())` | 9+ | `EnsembleContext::from(&state.service_context)` |
| `routes/models.rs` | `InferenceContext::from_parts(...)` | 2 | `InferenceContext::from(&state.service_context)` |
| `routes/chat.rs` | `InferenceContext::from_parts(...)` | 1 | `InferenceContext::from(&state.service_context)` |
| `routes/goal.rs` | `state.service_context.goal_repo` (direct access) | 3 | `GoalContext::from(&state.service_context)` + `GoalService` |
| `routes/acp.rs` | `state.service_context.pod_manager.acp_runtime()` (direct access) | 3 | `AcpContext::from(&state.service_context)` + `AcpService` |

**Task 5: Remove direct `manager.read().await` calls in `ensemble.rs`**
- `get_chat` (L156): `manager.read().await.get_chat(...)` → route through `EnsembleService`
- `improv_turn` (L270): `manager.read().await.get_chat(...)` → route through `EnsembleService`

### HIGH — CLI route strangler fig completion

**Task 6: Wire `commands/curator.rs` through `ServiceContext::build()`**
- 4 functions each call `open_registry_db()` + construct their own `EscalationQueue`:
  - `curator_escalations()` (L17)
  - `curator_resolve()` (L25)
  - `curator_dismiss()` (L33)
  - `curator_metacognition()` (L41) — also constructs standalone `CnsRuntime` + `MessageDispatch`
- Replace with `ServiceContext::build(config).await` then `CuratorContext::from(&ctx)`
- `run_curator` signature (L55) takes raw `registry`, `runtime`, `handle` — simplify to take `ServiceContext`

**Task 7: Wire `commands/sovereignty.rs` through `ServiceContext::build()`**
- `build_ctx()` (L22) calls `open_consent_store()` per invocation → replace with `ServiceContext` derivation
- Also opens `open_sovereignty_store()` separately (L44) — move to ServiceContext

**Task 8: Wire `commands/ensemble.rs` through `ServiceContext::build()`**
- 3 global statics creating parallel infrastructure:
  - `SESSION_MANAGER` (L24): `OnceLock<Arc<RwLock<SessionManager>>>`
  - `IMPROV_CLIENT` (L26): `OnceLock<Arc<CircuitBreakerInferenceAdapter>>`
  - `CYBERNETICS_LOOP` (L77): `OnceLock<Arc<RwLock<CyberneticsLoop>>>`
- Replace with `ServiceContext.session_manager`, `ServiceContext.cybernetics_loop`, etc.
- `get_session_manager()`, `get_improv_client()`, `get_cybernetics_loop()` → derive from ServiceContext
- `open_standing_session_store()` (L146) → use `ServiceContext.standing_session_store`

### MEDIUM — Cleanup

**Task 9: Remove dead code from `commands/config.rs`**
- `open_registry_db()` — if no longer called after Task 6
- `open_consent_store()` — if no longer called after Task 7
- `open_sovereignty_store()` — if no longer called after Task 7
- `open_spec_store()` — check for remaining callers

**Task 10: Remove `with_consent_manager` and `with_session_manager` from `ApiState`**
- These mutate fields inside `Arc<ServiceContext>` which breaks the single-source-of-truth invariant
- If CLI needs to share its SessionManager with the API, pass it during `ServiceContext::build()` via `ServiceConfig`

**Task 11: Full workspace verification**
```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

### LOW — Documentation

**Task 12: Update OPEN_QUESTIONS.md**
- Reopen F17 with correct assessment
- Re-evaluate F2/F3 conclusions
- Update F6 with boundary table reference

**Task 13: Update docs/status/test-inventory.md**
- Add service-layer seam tests
- Update existing tests that reference old ApiState fields

---

## 4. Recommended Skills and Tools

### Required Skills (load at session start)

1. **`refactor-service-layer`** — The core methodology. Strangler fig sequence, depth test, one-domain-per-commit. This IS the work.
2. **`coding-guidelines`** — Karpathy's surgical change principle. Every changed line must trace to the extraction.
3. **`tdd`** — Vertical tracer-bullet discipline. RED→GREEN per behavior, never horizontal slices.
4. **`constraint-forces`** — Classify the P1 Prohibition misinterpretation. "Standalone CLI commands CAN work without ServiceContext" ≠ "MUST avoid ServiceContext".
5. **`diagnose`** — For the broken build: reproduce, anchor, hypothesize, fix, verify.
6. **`zoom-out`** — Use before starting to re-verify the module map after Session 12's partial changes.

### Tools

- `cargo check -p hkask-api` — Primary verification during API fixes
- `cargo check -p hkask-cli` — Primary verification during CLI fixes
- `cargo check --workspace` — Full verification after each domain migration
- `cargo clippy -p <crate> -- -D warnings` — Lint enforcement (P7)
- `cargo test -p <crate>` — Test verification per domain
- `grep` — Find remaining `from_parts`, `open_registry_db`, `open_consent_store` call sites
- `read_file` / `edit_file` — Precise surgical edits

### Verification Commands Per Task

| Task | Verify |
|------|--------|
| 1-3 | `cargo check -p hkask-api` |
| 4 | `cargo check -p hkask-api && cargo test -p hkask-api` |
| 5 | `cargo check -p hkask-api` |
| 6 | `cargo check -p hkask-cli && cargo test -p hkask-cli` |
| 7 | `cargo check -p hkask-cli` |
| 8 | `cargo check -p hkask-cli && cargo test -p hkask-cli` |
| 9-10 | `cargo check --workspace` |
| 11 | `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace` |

---

## 5. Key Decisions to Preserve

1. **P1 Prohibition was misinterpreted.** "Standalone CLI commands work without ServiceContext" means they CAN operate independently for simple operations, NOT that they MUST avoid ServiceContext when they need shared infrastructure (escalation queue, consent manager, CNS). When a CLI command needs infrastructure that ServiceContext provides, it should use ServiceContext.

2. **ApiState holds `Arc<ServiceContext>` as single source of truth.** All domain objects come from `service_context.*`. Surface-specific fields (standing_sessions, ensemble_inferencer, git_cas, gas_governance) are the ONLY fields that don't come from ServiceContext. This is the canonical architecture — do not add domain fields back to ApiState.

3. **`From<&ServiceContext>` is the derivation pattern.** Replace all `from_parts()` manual construction with `impl From<&ServiceContext> for XxxContext`. This ensures surfaces can't accidentally construct contexts with stale or mismatched components.

4. **F14 closure is valid.** The ~11 remaining `ApiError::` constructions in API routes are legitimate HTTP-layer concerns (input validation, OCAP gates, auth failures). Do not extract these to the service layer.

5. **F18/F19 closures are valid.** Standing session CLI/API logic is too divergent to share. Improv operations are CLI-only. Do not force extraction where there's no duplication.

6. **Strangler fig step "DELETE old code" was never done.** Service modules were wired in but old direct-access paths remained. The sequence is: wire → verify → DELETE. The deletion step must happen for each domain.

7. **Global statics in `commands/ensemble.rs` are parallel infrastructure.** `SESSION_MANAGER`, `IMPROV_CLIENT`, `CYBERNETICS_LOOP` create their own CyberneticsLoops, session managers, and inference adapters outside the ServiceContext. This violates the single-assembly-point principle and must be replaced.

---

## File Reference Map

| File | Role | Status |
|------|------|--------|
| `crates/hkask-api/src/lib.rs` | ApiState definition | BROKEN — 11 errors |
| `crates/hkask-api/src/routes/bundles.rs` | Bundle routes | 2 syntax errors |
| `crates/hkask-api/src/routes/curator.rs` | Curator API routes | Uses `from_parts()` (4 sites) |
| `crates/hkask-api/src/routes/sovereignty.rs` | Sovereignty API routes | Uses `from_parts()` (4 sites) |
| `crates/hkask-api/src/routes/pods.rs` | Pod API routes | Uses `from_parts()` (5 sites) |
| `crates/hkask-api/src/routes/ensemble.rs` | Ensemble API routes | Uses `from_parts()` (9+ sites) + direct `manager.read().await` |
| `crates/hkask-api/src/routes/models.rs` | Model listing routes | Uses `from_parts()` (2 sites) |
| `crates/hkask-api/src/routes/chat.rs` | Chat route | Uses `from_parts()` (1 site) |
| `crates/hkask-api/src/routes/goal.rs` | Goal routes | Direct `state.service_context.goal_repo` access (3 sites) |
| `crates/hkask-api/src/routes/acp.rs` | ACP routes | Direct `state.service_context.pod_manager.acp_runtime()` (3 sites) |
| `crates/hkask-cli/src/commands/curator.rs` | Curator CLI commands | Opens own DB (4 sites) |
| `crates/hkask-cli/src/commands/sovereignty.rs` | Sovereignty CLI commands | Opens own consent store |
| `crates/hkask-cli/src/commands/ensemble.rs` | Ensemble CLI commands | 3 global statics + own DB |
| `crates/hkask-cli/src/commands/config.rs` | DB/store helpers | `open_registry_db()`, `open_consent_store()`, etc. |
| `crates/hkask-services/src/context.rs` | ServiceContext definition | Complete, `From` impls needed |
| `crates/hkask-services/src/curator.rs` | CuratorService + CuratorContext | `from_parts()` only, no `From<&ServiceContext>` |
| `crates/hkask-services/src/sovereignty.rs` | SovereigntyService + SovereigntyContext | `from_parts()` only |
| `crates/hkask-services/src/pods.rs` | PodService + PodContext | `from_parts()` only |
| `crates/hkask-services/src/ensemble.rs` | EnsembleService + EnsembleContext | `from_parts()` only |
| `crates/hkask-services/src/inference.rs` | InferenceService + InferenceContext | `from_parts()` only |