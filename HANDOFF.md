# HANDOFF.md — hKask Service Layer Extraction

**Session:** 12 (Audit) → 13 (Execution) → 14 (Continue)
**Status:** Build clean. Phase 4 partial: API routes migrated, CLI curator/pod/sovereignty/goal wired through ServiceContext. **We are roughly 40% through the total extraction work.** The original spec massively underestimated the scope — every surface (API 16 route files, CLI 8+ command files, REPL 3 handler files) has direct-access paths that need individual migration.

---

## 1. Session Context

Session 13 fixed the broken build (11 errors → 0), replaced all `from_parts()` calls in API routes with `From<&ServiceContext>` derivation, wired CLI curator/pod/sovereignty/goal commands through `ServiceContext::build()`, deleted dead `open_registry_db()` and `open_consent_store()`, and passed full workspace verification. The strangler fig is advancing but **nowhere near done** — both surfaces still have ~80+ direct-access sites that bypass the service layer.

---

## 2. What Was Done

### Build fix (0 errors, 0 warnings)
- `bundles.rs`: fixed 2 missing semicolons (L149, L182)
- `lib.rs`: deleted `with_consent_manager()` and `with_session_manager()` (dead methods referencing old fields); fixed `start_loops()`/`shutdown_loops()` to use `self.service_context.loop_system`; removed 11 dead imports
- `lib.rs` tests: fixed 5 old field references (`state.registry` → `state.service_context.registry`, etc.)

### API routes: `from_parts()` → `From<&ServiceContext>` (28 sites across 5 route files)
- `routes/curator.rs`: 4 sites
- `routes/ensemble.rs`: 8 sites
- `routes/pods.rs`: 5 sites
- `routes/models.rs`: 2 sites
- `routes/sovereignty.rs`: 4 sites
- `routes/chat.rs`: 1 site kept (legitimate fallback for no shared port)

### CLI commands: wired through `ServiceContext::build()`
- `commands/curator.rs`: 4 functions use `build_service_context()` → `CuratorContext::from(&ctx)`; `curator_metacognition` uses `CuratorContext::from_service_context(&ctx).await`
- `commands/pod.rs`: 5 functions use `build_service_context()` → `PodContext::from(&ctx)`; eliminated all `PodManager::new_mock()` usage
- `commands/sovereignty.rs`: `build_ctx()` now uses `SovereigntyContext::from(&build_service_context())`
- `commands/goal.rs`: 3 functions use `build_service_context()` → `ctx.goal_repo`; eliminated `open_repository()`
- `commands/serve.rs`: removed `with_session_manager()` call (session manager now from ServiceContext)

### Dead code deletion
- `config::open_registry_db()` — deleted (no callers)
- `config::open_consent_store()` — deleted (no callers)

### Error type additions
- `CuratorError::Service(hkask_services::ServiceError)` variant added with `#[error("Service error: {0}")]`

### Verification
- `cargo check --workspace` ✅
- `cargo clippy --workspace -- -D warnings` ✅
- `cargo test --workspace` ✅ (0 failures)

---

## 3. What Remains

**Honest assessment: We are ~40% through.** The `From<&ServiceContext>` impls and API `from_parts` replacement were the easy part. The remaining work requires creating new service modules, wiring deeply-embedded state, and surgically removing parallel infrastructure.

### HIGH — API direct-access paths still bypassing service layer (~45 sites)

| Route file | Direct access pattern | Sites | Needs |
|-----------|---------------------|-------|-------|
| `routes/acp.rs` | `state.service_context.pod_manager.acp_runtime()` | 3 | `AcpService` + `AcpContext` |
| `routes/bundles.rs` | `state.service_context.registry.lock().await` | 5 | `BundleService` + `BundleContext` |
| `routes/templates.rs` | `state.service_context.registry.lock().await` | 4 | `TemplateService` + `TemplateContext` |
| `routes/mcp.rs` | `state.service_context.mcp_runtime.discover_tools()` | 2 | `McpService` + `McpContext` |
| `routes/pods.rs` | `state.service_context.capability_checker.check_resource()` | 1 | Move OCAP gate into `PodService` |
| `routes/ensemble.rs` | `state.service_context.session_manager.read().await.get_chat()` | 3 | Route through `EnsembleService` |
| `routes/ensemble.rs` | `state.service_context.mcp_runtime.discover_tools()` | 1 | `McpService` |
| `routes/ensemble.rs` | `state.service_context.standing_session_store.clone()` | 1 | Route through `EnsembleService` |
| `routes/goal.rs` | `state.service_context.goal_repo.create_goal()` etc. | 3 | `GoalService` + `GoalContext` |
| `routes/chat.rs` | `state.service_context.inference_port` + `from_parts()` fallback | 1 | Route through `InferenceService` |
| `routes/cns.rs`, `routes/episodic.rs`, `routes/consolidation.rs`, `routes/spec.rs` | Unknown — need audit | ~12 | Full audit needed |

### HIGH — CLI direct-access paths still bypassing service layer (~20 sites)

| File | Direct access pattern | Sites | Needs |
|------|----------------------|-------|-------|
| `commands/ensemble.rs` | `get_session_manager()` global static (8 sites) | 8 | Replace with ServiceContext |
| `commands/ensemble.rs` | `IMPROV_CLIENT` global static | 1 | Replace with ServiceContext |
| `commands/ensemble.rs` | `CYBERNETICS_LOOP` global static | 1 | Replace with ServiceContext |
| `commands/ensemble.rs` | `open_standing_session_store()` | 1 | Use `ServiceContext.standing_session_store` |
| `commands/sovereignty.rs` | `open_sovereignty_store()` | 1 | Add `SovereigntyBoundaryStore` to ServiceContext |
| `commands/spec.rs` | `open_spec_store()` | 4 | Add `SqliteSpecStore` to ServiceContext or create SpecService |
| `commands/compose.rs` | `InferenceContext::from_parts()` | 1 | Wire through ServiceContext |
| `commands/chat.rs` | `InferenceContext::from_parts()` | 1 | Wire through ServiceContext |

### HIGH — REPL direct-access paths (~4 sites)

| File | Pattern | Sites | Needs |
|------|---------|-------|-------|
| `repl/init.rs` | `InferenceContext::from_parts()` (pre-onboarding) | 2 | REPL already uses ServiceContext; clean up pre-onboarding path |
| `repl/handlers/model.rs` | `InferenceContext::from_parts()` | 2 | Use `InferenceContext::from(&*state.service_context)` |
| `repl/handlers/hhh.rs` | `InferenceContext::from_parts()` | 1 | Same pattern |

### MEDIUM — New service modules needed in `hkask-services`

| Module | Purpose | Dependencies |
|--------|---------|-------------|
| `bundle.rs` | BundleService + BundleContext | `ServiceContext.registry` |
| `template.rs` | TemplateService + TemplateContext | `ServiceContext.registry` |
| `mcp.rs` | McpService + McpContext | `ServiceContext.mcp_runtime`, `mcp_dispatcher` |
| `acp.rs` | AcpService + AcpContext | `ServiceContext.pod_manager.acp_runtime()` |
| `goal.rs` | GoalService + GoalContext | `ServiceContext.goal_repo` |

### MEDIUM — ServiceContext gaps

| Missing field | Used by | Action |
|--------------|---------|--------|
| `SovereigntyBoundaryStore` | sovereignty CLI Status | Add to ServiceContext::build() |
| `SqliteSpecStore` | spec CLI commands | Add to ServiceContext::build() |

### MEDIUM — Dead code to delete (after migration)

| Function | Current callers | Delete when |
|----------|----------------|-------------|
| `open_sovereignty_store()` | sovereignty.rs L46 | After SovereigntyBoundaryStore in ServiceContext |
| `open_spec_store()` | spec.rs (4 sites) | After SqliteSpecStore in ServiceContext or SpecService |
| `create_disconnected_governed_dispatcher()` | config.rs | Audit callers |
| `create_mcp_dispatcher()` | config.rs | Audit callers |
| `create_mcp_dispatcher_with_servers()` | config.rs | Audit callers |
| `init_registry()` / `init_registry_with_secrets()` | config.rs | After full CLI migration |

### LOW — Ensemble global statics architecture decision

The 3 `OnceLock` statics in `commands/ensemble.rs` provide cross-command session persistence in the CLI. Replacing them with `ServiceContext::build()` would create a *different* session manager per subcommand call, breaking CLI session continuity. Options:

1. **ServiceContext singleton in CLI** — Build ServiceContext once at CLI startup, store in `OnceLock`, derive all contexts from it. This preserves session sharing while eliminating parallel infrastructure.
2. **Pass ServiceContext through CLI command dispatch** — Refactor `kask` main to build ServiceContext once and thread it through all subcommand handlers. More surgical but requires signature changes across many `run_*` functions.
3. **Keep ensemble statics, document as intentional** — If the ensemble module is truly CLI-only for these operations, the statics may be acceptable. But this violates the "single assembly point" principle.

This is an architectural decision that should be made before continuing ensemble migration.

---

## 4. Recommended Skills and Tools

### Required Skills (load at session start)

1. **`refactor-service-layer`** — The core methodology. Strangler fig sequence, depth test, one-domain-per-commit, P1-P6 principles. **This IS the work.**
2. **`coding-guidelines`** — Surgical change principle. Every changed line must trace to the extraction. No "while we're in the area."
3. **`constraint-forces`** — Classify whether direct access is a Prohibition violation (it's not — it's a Guideline that should become practice) vs. an acceptable surface-specific concern.
4. **`diagnose`** — When builds break or tests fail during migration.
5. **`zoom-out`** — Use before starting each new domain extraction to map the call graph.
6. **`tdd`** — For new service modules (BundleService, TemplateService, etc.): RED→GREEN per behavior.

### Tools

```bash
cargo check -p hkask-api          # API verification
cargo check -p hkask-cli          # CLI verification
cargo check --workspace           # Full verification
cargo clippy --workspace -- -D warnings  # Lint
cargo test --workspace           # Full test suite
grep -rn 'from_parts\|open_registry_db\|open_consent_store\|open_sovereignty_store\|open_spec_store' crates/  # Find remaining direct-access sites
grep -rn 'state\.service_context\.\(registry\|pod_manager\|session_manager\|mcp_runtime\|capability_checker\|standing_session_store\)' crates/hkask-api/src/routes/  # Find remaining API direct access
```

---

## 5. Key Decisions to Preserve

1. **P1 Prohibition was misinterpreted in Session 12.** "Standalone CLI commands work without ServiceContext" means they CAN operate independently for simple operations, NOT that they MUST avoid ServiceContext. When a command needs shared infrastructure, it should use ServiceContext.

2. **ApiState holds `Arc<ServiceContext>` as single source of truth.** All domain objects come from `service_context.*`. Surface-specific fields (standing_sessions, ensemble_inferencer, git_cas, gas_governance) are the ONLY fields that don't come from ServiceContext. Do not add domain fields back to ApiState.

3. **`From<&ServiceContext>` is the derivation pattern for API routes.** Replace all `from_parts()` with this. API routes access `&state.service_context` (inside `Arc`), not owned `ServiceContext`.

4. **CLI commands use `build_service_context()` helper + `From<&ServiceContext>`.** Each CLI subcommand that needs ServiceContext builds one from `ServiceConfig::from_env()`. This is not ideal (rebuilds per call) but is correct and keeps subcommands independent. A future optimization should build ServiceContext once at CLI startup.

5. **`from_parts()` is acceptable for standalone inference port creation.** When no shared port is available (in-memory mode, fallback paths), `InferenceContext::from_parts(None, model, base_url)` is the correct pattern — it can't use `From<&ServiceContext>` because the ServiceContext itself may lack an inference port.

6. **F14/F18/F19 closures are valid.** ~11 `ApiError::` constructions in API routes are legitimate HTTP-layer concerns. Standing session CLI/API logic is too divergent to share. Improv is CLI-only.

7. **Ensemble global statics are an open architectural question.** Replacing them naively with per-call `ServiceContext::build()` would break CLI session continuity. Decision needed before ensemble migration continues.

8. **New service modules must pass the depth test.** Before creating `BundleService`, `TemplateService`, etc., apply the deletion test: delete the surface code — does complexity reappear in N callers? If the service module would just be a thin delegation, deepen or merge instead.

---

## File Reference Map

| File | Role | Status |
|------|------|--------|
| `crates/hkask-api/src/lib.rs` | ApiState definition | ✅ Clean |
| `crates/hkask-api/src/routes/curator.rs` | Curator API routes | ✅ Uses `From<&ServiceContext>` |
| `crates/hkask-api/src/routes/sovereignty.rs` | Sovereignty API routes | ✅ Uses `From<&ServiceContext>` |
| `crates/hkask-api/src/routes/pods.rs` | Pod API routes | ✅ Uses `From<&ServiceContext>` |
| `crates/hkask-api/src/routes/ensemble.rs` | Ensemble API routes | ✅ Uses `From<&ServiceContext>` (but 3 direct `session_manager` access remain) |
| `crates/hkask-api/src/routes/models.rs` | Model listing routes | ✅ Uses `From<&ServiceContext>` |
| `crates/hkask-api/src/routes/chat.rs` | Chat route | ⚠️ 1 `from_parts()` fallback (legitimate) |
| `crates/hkask-api/src/routes/acp.rs` | ACP routes | ❌ Direct `pod_manager.acp_runtime()` (3 sites) |
| `crates/hkask-api/src/routes/bundles.rs` | Bundle routes | ❌ Direct `registry.lock().await` (5 sites) |
| `crates/hkask-api/src/routes/templates.rs` | Template routes | ❌ Direct `registry.lock().await` (4 sites) |
| `crates/hkask-api/src/routes/mcp.rs` | MCP routes | ❌ Direct `mcp_runtime` access (2 sites) |
| `crates/hkask-api/src/routes/goal.rs` | Goal API routes | ❌ Direct `goal_repo` access (3 sites) |
| `crates/hkask-cli/src/commands/curator.rs` | Curator CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/pod.rs` | Pod CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/sovereignty.rs` | Sovereignty CLI | ⚠️ Mostly ServiceContext, but `open_sovereignty_store()` (1 site) |
| `crates/hkask-cli/src/commands/goal.rs` | Goal CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/ensemble.rs` | Ensemble CLI | ❌ 3 global statics + `from_parts()` (8 sites) |
| `crates/hkask-cli/src/commands/spec.rs` | Spec CLI | ❌ `open_spec_store()` (4 sites) |
| `crates/hkask-cli/src/commands/compose.rs` | Compose CLI | ⚠️ `from_parts()` (1 site) |
| `crates/hkask-cli/src/commands/config.rs` | DB/store helpers | ⚠️ `open_sovereignty_store()`, `open_spec_store()` remain |
| `crates/hkask-cli/src/repl/` | REPL handlers | ⚠️ `from_parts()` (5 sites) |
| `crates/hkask-services/src/context.rs` | ServiceContext | ✅ Complete with `From` impls |