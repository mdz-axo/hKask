# Continuation Prompt — hKask Service Layer Extraction

**Session 10 — Tasks 7c/7d/7f, 8, 9**

---

## 0. Mandatory Pre-Work

**You MUST load these skills BEFORE writing any code. Do not skip this step.**

1. **`refactor-service-layer`** — The strangler fig process, deletion test, depth test, and verification checklist. Every structural change must follow its Phase 5/Phase 6 process.
2. **`coding-guidelines`** — Think before coding. Surgical changes only. Every changed line must trace to the task.
3. **`tdd`** — RED→GREEN→REFACTOR per behavior. Every new path gets a `// REQ:` tagged test.
4. **`constraint-forces`** — Classify every design decision by force type (Prohibition > Guardrail > Guideline > Evidence > Hypothesis). Never silently relax a Prohibition or Guardrail.
5. **`zoom-out`** — Use BEFORE touching CAS write-through or error mapping code. Map call sites, dependencies, and surface-specific vs shared concerns.
6. **`diagnose`** — If CAS or error mapping diverges after unification, use the disciplined diagnosis loop before trying to fix mismatches.
7. **`improve-codebase-architecture`** — F10 (ServiceContext god-object) may need sub-struct grouping. Use this skill to find the right decomposition before adding more fields.

---

## 1. Session Context

This is **Session 10** of the hKask service layer extraction. Nine prior sessions completed:

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
| 9 | 7b | **Surface assembly migration complete.** Both `ApiState` and `ReplState` compose `ServiceContext::build()`. Deleted 4 API modules (~460 lines). Fixed API CnsRuntime bug (disconnected instance → shared). 3 API tests added. |

**Current test count:** 49 tests (46 service-layer + 3 API). Workspace compiles clean with `clippy -D warnings`.

---

## 2. What Was Completed Through Session 9

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

### Task 7b — COMPLETE

**Phase 1 — API surface:**
- Added `ApiState::from_service_context(ctx, ensemble_inferencer)` — derives all shared fields from `ServiceContext`, initializes API-specific fields to defaults
- Refactored `ApiState::with_defaults()` from 7-param sync → 0-param async using `ServiceContext::build()` + `from_service_context()`
- Migrated `kask serve` to `ServiceContext::build(config).await` + `from_service_context()`
- Deleted `ApiState::new()`, `with_ensemble_inferencer()`, 4 API module files (~460 lines total)
- 3 new API tests

**Phase 2 — CLI surface:**
- Refactored `kask loops` from 113-line manual assembly → 44-line `ServiceContext::build()` call
- Refactored `init_repl_state()` to use `ServiceContext::build()` for shared infra; CLI-specific concerns remain (inference, onboarding, per-agent memory, GovernedTool, HHH gate, gas budget)
- ~183 lines of duplicated assembly eliminated

**Key bug fix:** Old `ApiState::new()` created a **disconnected** 3rd `CnsRuntime` instance. New path clones from ServiceContext's shared `Arc<RwLock<CnsRuntime>>`, ensuring the API's `cns_runtime` shares state with the loop system's CNS.

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

## 4. What Remains

### Task 7c — CAS Write-Through for ServiceContext Stores (F26) — MEDIUM

**Problem:** Old API `Stores::init()` added `.with_cas(git_cas_port)` to consent, goal, and standing session stores. `ServiceContext::build()` does not include CAS write-through. Per-mutation audit trails are lost in the new path.

**Investigation needed:**
1. Read the CAS write-through methods in `hkask-storage`: `ConsentStore::store_with_cas()`, `SqliteGoalRepository::with_cas()`, `StandingSessionStore::save_stored_session_with_cas()`
2. Determine whether CAS is a **shared concern** (should be in ServiceContext) or a **surface concern** (should be wired by surfaces after `ServiceContext::build()`)
3. Find all call sites that use the `*_with_cas()` methods vs regular methods

**Possible outcomes:**
- **Option A**: CAS is shared → Add `git_cas_port` field to ServiceContext; call `.with_cas()` during `build()`
- **Option B**: CAS is surface-specific → Surfaces call `.with_cas()` on the stores they get from ServiceContext after construction (requires stores to be mutable or re-constructable)
- **Option C**: CAS is deprecated → The old API path was the only consumer; remove `_with_cas` methods

**Constraint:** If adding a field to ServiceContext, it must pass the depth test (8+ call sites). F10 already flags ServiceContext at 20 fields.

### Task 7d — Secret Resolution Extraction — MEDIUM

**Status:** Partially done — `ServiceConfig::from_env()` already resolves ACP/MCP/DB secrets from keystore.

**Remaining:** Audit all surfaces for remaining `resolve_acp_secret()` / `resolve_capability_key()` / `resolve_mcp_secret()` calls that bypass `ServiceConfig`. Ensure every secret resolution goes through `ServiceConfig`.

**Known remaining call sites:**
- `crates/hkask-cli/src/commands/config.rs` — `resolve_mcp_secret()` called in `init_repl_state()` at line 108-109
- Any other direct keystore calls in CLI or API

### Task 7f — Error Mapping Unification — MEDIUM

**Problem:** API has dual error paths:
1. `From<ServiceError> for ApiError` adapter (in `api/error.rs` lines 353-465)
2. 14 direct `ApiError` constructions in `api/routes/*.rs` that bypass ServiceError

**Goal:** Audit all `api/routes/*.rs` files for direct `ApiError::Internal`, `ApiError::NotFound`, etc. that should instead flow through `ServiceError` → `ApiError`. Add missing `From<ServiceError> for ApiError` arms if needed.

**CLI side:** CLI error adapters (`cli/errors.rs`) already have `From<ServiceError>` for `CuratorError`, `AgentError`, `EnsembleError`, `UserError`. Check for direct domain-error constructions that should flow through ServiceError.

**Approach:**
1. Grep `api/routes/*.rs` for `ApiError::` direct constructions
2. Classify each: legitimate surface concern (e.g., HTTP status selection) vs. should-flow-through-ServiceError
3. Add missing `From<ServiceError>` arms for domain errors that are now in ServiceError
4. Verify: `cargo test -p hkask-api`

### Task 8 — Verification — MEDIUM

```
[ ] Depth test every module in hkask-services
[ ] Dependency direction check (no circular deps)
[ ] cargo check --workspace
[ ] cargo clippy --workspace -- -D warnings
[ ] cargo test --workspace
[ ] Deletion test for remaining modules (what breaks if a service module is removed?)
[ ] No todo!/unimplemented! in hkask-services
[ ] No MCP→services dependency (P1 preserved)
[ ] Verify ServiceContext fields are all used by 2+ surfaces
```

### Task 9 — Documentation — LOW

- Update `docs/status/test-inventory.md` with 49+ test entries
- Update `docs/architecture/hKask-architecture-master.md` with service layer section
- Write `OPEN_QUESTIONS.md` for F1–F26

---

## 5. Constraint Forces

These are non-negotiable. Classify every design decision by force type.

| Constraint | Force | Implication |
|-----------|-------|-------------|
| MCP servers do NOT depend on `hkask-services` | Prohibition (P1) | Never modify `mcp-servers/` code |
| OCAP gates stay in domain crates / surfaces | Prohibition (P1) | Service layer never decides access |
| Both old and new paths must work before deleting old | Prohibition (P3 Strangler Fig) | Add new path alongside, verify, then delete |
| No `todo!` or `unimplemented!` in `hkask-services` | Prohibition (P6/P7) | Write real code or return errors |
| Dependency direction: CLI/API → services → domain | Guideline (P12) | Never the reverse |
| One domain per commit | Guideline (P5) | One task at a time |
| Depth test: 8+ call sites | Guideline | If not met, deepen or skip |
| ServiceContext ≤ 20 fields | Guardrail (F10) | Any new field needs strong justification |
| Surface context pattern | Guideline | Each service module has its own lightweight context |

---

## 6. Key Files to Read First

| File | Why |
|------|-----|
| `HANDOFF.md` | Authoritative state — Sections 5 (39 decisions), 6 (what remains), 7 (F1–F26), 9 (architecture) |
| `crates/hkask-services/src/context.rs` | `ServiceContext::build()` — 20 fields, ~380 lines |
| `crates/hkask-services/src/config.rs` | `ServiceConfig` — 3 constructors, 8 defaults |
| `crates/hkask-services/src/error.rs` | `ServiceError` — 31 variants across 9 domain groups |
| `crates/hkask-api/src/error.rs` | `ApiError` + `From<ServiceError>` adapter (353-465) + 14+ direct domain `From` impls |
| `crates/hkask-api/src/lib.rs` | `ApiState::from_service_context()` — shows current API wiring |
| `crates/hkask-cli/src/errors.rs` | CLI `From<ServiceError>` adapters for 4 error enums |
| `crates/hkask-cli/src/repl/init.rs` | `init_repl_state()` — shows current CLI wiring via ServiceContext |
| `crates/hkask-cli/src/commands/config.rs` | `resolve_mcp_secret()` — potential remaining direct secret resolution |
| `crates/hkask-storage/src/consent_store.rs` | `store_with_cas()` — CAS write-through pattern |
| `crates/hkask-storage/src/goals.rs` | `with_cas()` — CAS builder pattern |
| `crates/hkask-storage/src/standing_session.rs` | `save_stored_session_with_cas()` — CAS write-through |

---

## 7. Detailed Task Sequence

### Recommended Order: 7c → 7d → 7f → 8 → 9

**Task 7c should go first** because it may require changes to ServiceContext, which affects all downstream tasks.

### Task 7c — CAS Write-Through Investigation

**Step 1: Zoom out on CAS usage**
- Grep all `*_with_cas()` and `.with_cas()` call sites across the entire workspace
- Determine: who calls these methods? How many call sites exist?
- Classify: shared concern (both surfaces need it) vs. surface-specific (only API uses it)

**Step 2: Assess F10 impact**
- If CAS requires a new ServiceContext field, does it pass the depth test?
- Can CAS be added without growing ServiceContext (e.g., by wiring it after `build()`)?

**Step 3: Implement the chosen approach**
- If CAS is shared: add to ServiceContext, write test, verify
- If CAS is surface-specific: document how surfaces wire CAS after `ServiceContext::build()`
- If CAS is unused: remove dead `_with_cas` methods from storage crate

**Step 4: Verify**
- `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`

### Task 7d — Secret Resolution Extraction

**Step 1: Audit all keystore calls**
- Grep `resolve_acp_secret`, `resolve_mcp_secret`, `resolve_db_passphrase`, `resolve_capability_key` across CLI and API
- Identify any that bypass `ServiceConfig::from_env()` or `ServiceConfig::from_secrets()`

**Step 2: Migrate remaining direct calls**
- Move any remaining direct keystore calls to go through ServiceConfig
- If CLI's `init_repl_state()` still has `resolve_mcp_secret()` at line 108, ensure it flows through `ServiceConfig`

**Step 3: Verify**
- `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`

### Task 7f — Error Mapping Unification

**Step 1: Audit API routes for direct ApiError construction**
```bash
grep -rn "ApiError::" crates/hkask-api/src/routes/ --include="*.rs"
```
- Classify each as: legitimate surface concern vs. should-flow-through-ServiceError
- Count how many direct constructions exist (known: ~14)

**Step 2: Audit CLI for direct error construction that should use ServiceError**
```bash
grep -rn "CuratorError::\|AgentError::\|EnsembleError::\|UserError::" crates/hkask-cli/src/commands/ --include="*.rs"
```

**Step 3: Add missing From<ServiceError> arms**
- Only add arms that are actually needed by call sites
- Don't add speculative arms "just in case"
- Each new arm should trace to a specific call site that would benefit

**Step 4: Verify**
- `cargo test -p hkask-api && cargo test -p hkask-cli`

### Task 8 — Verification

**Depth test every service module:**

| Module | Public Functions | Known Call Sites | Passes Depth Test? |
|--------|-----------------|-------------------|---------------------|
| `inference.rs` | 3 | CLI + API (8+ sites) | ✅ |
| `curator.rs` | 6 | CLI + API (12+ sites) | ✅ |
| `ensemble.rs` | 8 | CLI + API (16+ sites) | ✅ |
| `pods.rs` | 6 | CLI + API (12+ sites) | ✅ |
| `sovereignty.rs` | 9 + 2 types | CLI + API (18+ sites) | ✅ |

**Dependency direction verification:**
```bash
# No circular deps: services should not depend on CLI or API
grep -rn "hkask_cli\|hkask_api" crates/hkask-services/src/ --include="*.rs"
# Should find nothing

# MCP servers should not depend on services
grep -rn "hkask_services" mcp-servers/ --include="*.rs"
# Should find nothing
```

**Full workspace verification:**
```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

**No violations:**
```bash
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/hkask-services/src/ --include="*.rs"
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/hkask-api/src/ --include="*.rs"
```

### Task 9 — Documentation

**Step 1:** Update `docs/status/test-inventory.md` with 49+ test entries grouped by module:
- InferenceService: 4 tests
- CuratorService: 6 tests
- EnsembleService: 11 tests
- PodService: 6 tests
- SovereigntyService: 13 tests
- Infrastructure: 6 tests
- API: 3 tests

**Step 2:** Update `docs/architecture/hKask-architecture-master.md` with service layer section covering:
- Architecture diagram (CLI/API → services → domain)
- ServiceContext composition pattern
- Surface-specific vs shared concerns boundary
- Depth test results for all modules

**Step 3:** Write `OPEN_QUESTIONS.md` for F1–F26 with current status, priority, and next-action for each.

---

## 8. Key Decisions to Preserve (39+ Total)

Read `HANDOFF.md` Section 5 for the full list. Most critical for this session:

| # | Decision | Force | Impact |
|---|----------|-------|--------|
| 2 | `ServiceContext::build()` is async | Guideline | All callers `.await` it |
| 12 | Dependency direction: CLI/API → services → domain | Guideline | Never the reverse |
| 39 | Surface code uses ServiceContext for assembly | Guideline (now Prohibition) | Both surfaces compose `ServiceContext::build()` |
| 40 | API's `cns_runtime` shares state with loop system's CNS | Bug fix | Old `ApiState::new()` created a disconnected instance |
| 41 | GovernedTool stays surface-specific | Guideline | Only CLI needs it. Fails depth test for ServiceContext |
| 42 | CAS write-through not in ServiceContext stores | Open (F26) | May need to add or make surface-level |
| 43 | F13 CapabilityChecker secrets are by design | CLOSED | 1 MCP + 2 ACP checkers, same pattern in both surfaces |
| 44 | `with_defaults()` signature changed (0 params) | Guideline | No live callers existed |

---

## 9. Anti-Patterns to Avoid

1. **Adding fields to ServiceContext without depth justification** — F10 flags 20 fields as a god-object risk. Any new field needs 8+ consumer sites or a compelling alternative.
2. **Breaking the build** — Every step must leave the workspace compiling and all tests passing.
3. **Adding speculative `From` impls** — Only add `From<ServiceError>` arms that are actually needed by call sites.
4. **Moving OCAP gates into service layer** — P1 Prohibition. Auth/capability checks stay in surfaces.
5. **MCP servers depending on `hkask-services`** — P1 Prohibition. Never modify mcp-servers/.
6. **Horizontal slicing** — Don't audit all routes then fix them all. One route → verify → next.
7. **Treating CAS as shared without evidence** — If only the API ever used CAS write-through, it may be a surface concern. Verify call sites before adding to ServiceContext.

---

## 10. Open Questions Requiring Attention

| ID | Question | Priority | Status |
|----|----------|----------|--------|
| F9 | Production memory stores use `in_memory_db()` | HIGH | Track — P1 User Sovereignty |
| F10 | ServiceContext approaching god-object (20 fields) | MEDIUM | Guard with sub-structs; investigate grouping |
| F26 | ServiceContext stores lack CAS write-through | MEDIUM | Needs investigation (Task 7c) |
| F14 | Dual error mapping in API (14 direct + ServiceError adapter) | MEDIUM | Planned for Task 7f |
| F7 | ServiceConfig vs environment variables (3 places read HKASK_DB_PATH) | MEDIUM | Track — may affect Task 7d |

---

## 11. Recommended Tools and Commands

```bash
# Verify current state before any change
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test -p hkask-services --lib
cargo test -p hkask-api

# After each step
cargo check -p hkask-api       # or hkask-cli, hkask-services
cargo test -p hkask-api        # or relevant crate
cargo clippy -p hkask-services -- -D warnings

# Full verification after each task completion
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace

# Check for violations
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/hkask-services/src/ --include="*.rs"
grep -r "hkask_services" mcp-servers/ --include="*.rs"  # Should find nothing

# CAS usage audit
grep -rn "with_cas\|_with_cas" crates/ --include="*.rs"

# Secret resolution audit
grep -rn "resolve_acp_secret\|resolve_mcp_secret\|resolve_db_passphrase\|resolve_capability" crates/hkask-cli/ crates/hkask-api/ --include="*.rs"

# API direct error construction audit
grep -rn "ApiError::" crates/hkask-api/src/routes/ --include="*.rs"

# Dependency direction check
grep -rn "hkask_cli\|hkask_api" crates/hkask-services/src/ --include="*.rs"  # Should find nothing
```

---

*ℏKask — A Minimal Viable Container for Agents — v0.23.0*