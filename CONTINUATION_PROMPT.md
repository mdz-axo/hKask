# CONTINUATION_PROMPT.md — Session 13: Service Layer Strangler Fig Completion

**Read `HANDOFF.md` first.** It contains the full context, file map, and decision record.

---

## What Happened

Session 12 audited 7 Tier 1 open questions and incorrectly closed all of them as "by design." The reality: the strangler fig pattern is incomplete. Service modules exist and are wired in, but old direct-access paths in both CLI and API surfaces were never deleted. Then `ApiState` was refactored to hold `Arc<ServiceContext>` but left broken (11 compilation errors). The P1 constraint was misinterpreted — "standalone commands CAN work without ServiceContext" was read as "MUST avoid ServiceContext," which is wrong.

## Where We Are Now

- **Build:** Broken. `cargo check -p hkask-api` → 11 errors (3 `E0609` field errors in `lib.rs`, 2 syntax errors in `bundles.rs`, 2 routing errors in `bundles.rs`, plus unused imports)
- **Service modules:** 5 extracted (InferenceService, CuratorService, EnsembleService, PodService, SovereigntyService), all with tests
- **From<&ServiceContext> impls:** Do NOT exist yet — must be added before routes can use them
- **API routes:** All still use `from_parts()` manual construction instead of `From<&ServiceContext>`
- **CLI commands:** Still open their own DBs, construct their own queues, use global statics
- **Strangler fig status:** Wire step done, delete step NEVER done

## Session 13 Task Plan

### Phase 1: Fix the broken build (Tasks 1–3)

These are blocking. Nothing else can proceed until `cargo check -p hkask-api` passes.

**Task 1: Fix `ApiState` method references in `lib.rs`**

File: `crates/hkask-api/src/lib.rs`

Three methods reference old direct fields that no longer exist on `ApiState`:

```rust
// L180: with_consent_manager — tries to set self.consent_manager
// This method is problematic. ServiceContext fields are behind Arc.
// Option A (preferred): Delete with_consent_manager entirely.
// Option B: Remove the method and handle at ServiceContext::build() level.
pub fn with_consent_manager(mut self, consent_manager: ConsentManager) -> Self {
    self.consent_manager = Arc::new(consent_manager);  // BROKEN
}

// L215: with_session_manager — same problem
pub fn with_session_manager(mut self, session_manager: Arc<RwLock<SessionManager>>) -> Self {
    self.session_manager = session_manager;  // BROKEN
}

// L232-248: start_loops / shutdown_loops reference self.loop_system
pub async fn start_loops(&self) -> ... {
    self.loop_system  // BROKEN → self.service_context.loop_system
}
pub fn shutdown_loops(&self) {
    self.loop_system  // BROKEN → self.service_context.loop_system
}
```

Action:
1. Delete `with_consent_manager` (L178-182) — ServiceContext is the single source of truth
2. Delete `with_session_manager` (L211-217) — same reason
3. Fix `start_loops`: `self.loop_system` → `self.service_context.loop_system`
4. Fix `shutdown_loops`: `self.loop_system` → `self.service_context.loop_system`
5. Delete all unused imports (L55-62: ConsentManager, EscalationQueue, LoopSystem, PodManager, EpisodicStoragePort, CnsRuntime, StandingSessionStore, SqliteGoalRepository, InferencePort, CapabilityChecker, WebID)
6. Keep only the imports actually used in the file

**Task 2: Fix syntax errors in `bundles.rs`**

File: `crates/hkask-api/src/routes/bundles.rs`

```rust
// L148-150: missing semicolon
let value =
    serde_json::to_value(&bundle).unwrap_or(serde_json::json!({"id": bundle.id}))
Ok(Json(value))  // ← needs semicolon after the line above

// L182-183: missing semicolon  
let existing = registry.find_bundle_by_skills(&request.skills)
let existing_match = ...  // ← needs semicolon after the line above
```

**Task 3: Fix `bundles.rs` routing errors**

The `bundles_router()` function references `compose_bundle` and `get_bundle`. These ARE defined in the file (L169 and L141) but may have visibility issues since they're async handler functions not marked `pub`. Axum routing doesn't require `pub` — verify they compile after fixing the syntax errors, since the E0425 errors might be cascading from the syntax errors above.

Verify: `cargo check -p hkask-api`

### Phase 2: Add `From<&ServiceContext>` implementations (Task 4 prerequisite)

Before replacing `from_parts()` in routes, the `From<&ServiceContext>` impls must exist. Check if they already do (the handoff from sessions 3-8 says they were written but never used).

**Task 4a: Verify existing `From<&ServiceContext>` impls**

Search `crates/hkask-services/src/{curator,sovereignty,pods,ensemble,inference}.rs` for `impl From<&ServiceContext>`. If they exist, proceed to Task 4b. If not, write them:

```rust
// In curator.rs:
impl From<&ServiceContext> for CuratorContext {
    fn from(ctx: &ServiceContext) -> Self {
        CuratorContext::from_parts(
            ctx.escalation_queue.clone(),
            Some(ctx.cns_runtime.read().await.clone().into()),
            Some(ctx.dispatch.clone()),
        )
    }
}

// In sovereignty.rs:
impl From<&ServiceContext> for SovereigntyContext {
    fn from(ctx: &ServiceContext) -> Self {
        SovereigntyContext::from_parts(ctx.consent_manager.clone())
    }
}

// In pods.rs:
impl From<&ServiceContext> for PodContext {
    fn from(ctx: &ServiceContext) -> Self {
        PodContext::from_parts(ctx.pod_manager.clone())
    }
}

// In ensemble.rs:
impl From<&ServiceContext> for EnsembleContext {
    fn from(ctx: &ServiceContext) -> Self {
        EnsembleContext::from_parts(ctx.session_manager.clone())
    }
}

// In inference.rs:
impl From<&ServiceContext> for InferenceContext {
    fn from(ctx: &ServiceContext) -> Self {
        InferenceContext::from_parts(
            ctx.inference_port.clone(),
            &ctx.config.default_model,
            &ctx.config.okapi_base_url,
        )
    }
}
```

**Task 4b: Replace `from_parts()` in API routes**

For each route file, replace manual `from_parts()` calls with `From<&ServiceContext>` derivation:

```rust
// BEFORE:
let ctx = hkask_services::CuratorContext::from_parts(
    state.service_context.escalation_queue.clone(),
    None,
    None,
);

// AFTER:
let ctx = hkask_services::CuratorContext::from(&*state.service_context);
```

Files and sites:
- `routes/curator.rs`: 4 sites (L124, L170, L202, L230)
- `routes/sovereignty.rs`: 4 sites (L101, L137, L171, L212)
- `routes/pods.rs`: 5 sites (L68, L97, L116, L130, L144, L158)
- `routes/ensemble.rs`: 9+ sites (L134, L156, L178, L192, L226, L270, L317, L342, L368, L401)
- `routes/models.rs`: 2 sites (L84, L130)
- `routes/chat.rs`: 1 site (L80)

**Task 4c: Handle direct-access routes (goal.rs, acp.rs)**

These access ServiceContext fields directly without going through a service context type. Two approaches:

**Option A (preferred):** Create thin `GoalContext` + `AcpContext` and `From<&ServiceContext>` impls:
```rust
// In hkask-services (new or add to existing modules):
pub struct GoalContext {
    pub goal_repo: Arc<SqliteGoalRepository>,
}
impl From<&ServiceContext> for GoalContext {
    fn from(ctx: &ServiceContext) -> Self {
        GoalContext { goal_repo: ctx.goal_repo.clone() }
    }
}
```

**Option B (simpler, acceptable):** Leave direct access through `state.service_context.goal_repo` since these are CRUD operations with no business logic to extract. The depth test says: if deleting the surface code doesn't cause complexity to reappear in N callers, don't extract.

Recommend Option B for goal.rs (3 direct accesses, CRUD only) and evaluate ACP separately.

Verify: `cargo check -p hkask-api && cargo test -p hkask-api`

### Phase 3: CLI strangler fig completion (Tasks 6–8)

**Task 6: Wire `commands/curator.rs` through `ServiceContext::build()`**

Current: 4 functions each open their own DB and construct their own `EscalationQueue`.

```rust
// BEFORE (each function does this):
let conn = open_registry_db()?;
let queue = Arc::new(hkask_agents::EscalationQueue::new(conn)?);
let ctx = CuratorContext::from_parts(queue, None, None);

// AFTER:
let config = hkask_services::ServiceConfig::from_env()?;
let ctx = hkask_services::ServiceContext::build(config).await?;
let curator_ctx = hkask_services::CuratorContext::from(&ctx);
```

Also simplify `run_curator` signature from taking raw `registry`, `runtime`, `handle` to taking `ServiceContext`.

**Task 7: Wire `commands/sovereignty.rs` through `ServiceContext::build()`**

Current: `build_ctx()` opens its own consent store per call.

```rust
// BEFORE:
fn build_ctx() -> SovereigntyContext {
    let consent_store = commands::config::open_consent_store()?;
    let consent_manager = Arc::new(ConsentManager::new(consent_store));
    SovereigntyContext::from_parts(consent_manager)
}

// AFTER:
fn build_ctx() -> Result<SovereigntyContext, ...> {
    let config = hkask_services::ServiceConfig::from_env()?;
    let ctx = hkask_services::ServiceContext::build(config).await?;
    Ok(SovereigntyContext::from(&ctx))
}
```

Note: `open_sovereignty_store()` at L44 is separate from `ConsentManager` — it reads boundary data directly. This should also come from ServiceContext (add `sovereignty_store` field if not already present).

**Task 8: Wire `commands/ensemble.rs` through `ServiceContext::build()`**

Current: 3 global statics create parallel infrastructure:
- `SESSION_MANAGER` (L24) — own `SessionManager`
- `IMPROV_CLIENT` (L26) — own `CircuitBreakerInferenceAdapter`
- `CYBERNETICS_LOOP` (L77) — own `CyberneticsLoop` with in-memory DB

These must be replaced with ServiceContext's fields. This is the most complex CLI migration because the statics are used across the module. Strategy:

1. Change function signatures to take `&ServiceContext` instead of using statics
2. Replace `get_session_manager()` with `ctx.session_manager.clone()`
3. Replace `get_cybernetics_loop()` with `ctx.cybernetics_loop.clone()`
4. Replace `get_improv_client(port)` with constructing from `ctx.inference_port`
5. Replace `open_standing_session_store()` with `ctx.standing_session_store.clone()`
6. Update `run_ensemble` to build ServiceContext first, then pass to operations
7. Delete the 3 statics and their helper functions

Verify: `cargo check -p hkask-cli && cargo test -p hkask-cli`

### Phase 4: Cleanup (Tasks 9–11)

**Task 9:** Remove dead `open_*()` functions from `commands/config.rs` if no longer called after Tasks 6-8.

**Task 10:** Verify `with_consent_manager` and `with_session_manager` are already deleted (Task 1).

**Task 11:** Full workspace verification:
```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

### Phase 5: Documentation (Tasks 12–13)

**Task 12:** Update OPEN_QUESTIONS.md — reopen F17, re-evaluate F2/F3.

**Task 13:** Update test inventory.

---

## Required Skills

Load these at session start before writing any code:

1. **`refactor-service-layer`** — Strangler fig pattern, depth test, one-domain-per-commit
2. **`coding-guidelines`** — Surgical changes, think before coding
3. **`tdd`** — RED→GREEN per behavior
4. **`constraint-forces`** — Classify P1 misinterpretation
5. **`diagnose`** — For build fix
6. **`zoom-out`** — Verify module map before changes

## Anti-Patterns to Avoid

1. **Horizontal migration** — Don't migrate all domains before verifying any surface works
2. **Big-bang deletion** — Don't delete old code before verifying new path works
3. **Shallow service module** — Don't create pass-through contexts with no behavior
4. **Surface types in service signatures** — No `Json<T>`, no `ApiError`, no HTTP concepts
5. **Feature creep** — Don't add new functionality during migration
6. **"By design" without evidence** — Every closure needs a deletion test or depth test justification

## Key Constraint Reminder

**P1 Prohibition CORRECT interpretation:** CLI commands CAN operate without ServiceContext for trivial operations (reading a path, printing a help message). They SHOULD use ServiceContext when they need shared infrastructure (escalation queue, consent manager, session manager, CNS, goal repo). The constraint protects simplicity for trivial commands, not avoidance of shared infrastructure for substantial ones.