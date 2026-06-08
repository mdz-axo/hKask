# HANDOFF.md — hKask Service Layer Extraction

**Session:** 12 (Audit) → 13 (Execution) → 14 (GoalService + Audit) → 15 (Depth-Tests + ConsolidationService + ServiceContext gaps)
**Status:** Build clean. Session 15 completed the depth-test decisions for 4 proposed service modules (all shallow — documented), extracted ConsolidationService, filled ServiceContext gaps (sovereignty_boundary_store, spec_store), wired sovereignty and spec CLI through ServiceContext, and deleted dead `open_sovereignty_store()`/`open_spec_store()`. **We are roughly 65% through.**

---

## 1. Session Context

### Session 13 (previous)

Fixed the broken build (11 errors → 0), replaced all `from_parts()` calls in API routes with `From<&ServiceContext>` derivation, wired CLI curator/pod/sovereignty/goal commands through `ServiceContext::build()`, deleted dead `open_registry_db()` and `open_consent_store()`, and passed full workspace verification.

### Session 14

**GoalService extraction completed.** Created `goal.rs` in `hkask-services` with `GoalContext` + `GoalService` (3 service methods + 3 parse helpers, 12 unit tests with `// REQ:` tags). Wired both API `routes/goal.rs` (3 sites) and CLI `commands/goal.rs` (3 sites) through `GoalService`. Eliminated all direct `goal_repo` access from both surfaces.

**Audited 4 unaudited API route files:**
- `routes/cns.rs` — ✅ Clean (3 sites, surface-only SSE bridge logic)
- `routes/episodic.rs` — ✅ Clean (3 sites, uses OCAP port membrane correctly)
- `routes/consolidation.rs` — ⚠️ MAJOR violation (bypasses ServiceContext entirely; opens own DB, does keystore passphrase verification, builds entire consolidation pipeline)
- `routes/spec.rs` — ✅ Clean (0 sites; stubs returning hardcoded data)

### Session 15 (current)

**Depth-test decisions completed for 4 proposed service modules:**
- TemplateService → **SHALLOW** (rejected). All 4 sites are thin delegations to `SqliteRegistry` methods + HTTP response mapping. No cross-surface duplication. Documented in `routes/templates.rs` module doc.
- BundleService → **SHALLOW** (rejected). All 5 sites are thin delegations to registry bundle methods. No CLI bundle commands with real logic. Documented in `routes/bundles.rs` module doc.
- AcpService → **SHALLOW** (rejected). All 3 sites are thin delegations to `AcpRuntime` methods. No CLI ACP commands. Documented in `routes/acp.rs` module doc.
- McpService → **SHALLOW** (rejected). All 3 sites are thin delegations to `McpRuntime`/`McpDispatcher` methods. No shared logic with CLI MCP. Documented in `routes/mcp.rs` module doc.

**ConsolidationService extracted.** Created `consolidation.rs` in `hkask-services` with `ConsolidationService` (2 service methods: `verify_passphrase`, `consolidate` + 2 unit tests with `// REQ:` tags). Wired `routes/consolidation.rs` through `ConsolidationService`. Eliminated all direct keystore access, DB opening, and pipeline assembly from the route. Added `ServiceError::Consolidation(String)` variant.

**ServiceContext gaps filled.** Added `sovereignty_boundary_store: SovereigntyBoundaryStore` and `spec_store: SqliteSpecStore` to `ServiceContext::build()`. Both are initialized with their own DB connections and schema initialization.

**CLI sites wired through ServiceContext:**
- `commands/sovereignty.rs` Status action: replaced `open_sovereignty_store()` with `svc_ctx.sovereignty_boundary_store`
- `commands/spec.rs` all 4 actions: replaced `open_spec_store()` with `ctx.spec_store` from `build_service_context()`

**Dead code deleted:**
- `commands/config.rs`: `open_sovereignty_store()` and `open_spec_store()` — no callers remaining

**Verification:** `cargo check --workspace` ✅ | `cargo clippy --workspace -- -D warnings` ✅ | `cargo test --workspace` ✅ (0 failures)

---

## 2. What Was Done

### Depth-test annotations (4 route files)
- `routes/templates.rs`: Added module doc explaining TemplateService was considered and rejected as shallow
- `routes/bundles.rs`: Added module doc explaining BundleService was considered and rejected as shallow
- `routes/acp.rs`: Added module doc explaining AcpService was considered and rejected as shallow
- `routes/mcp.rs`: Added module doc explaining McpService was considered and rejected as shallow

### ConsolidationService extraction
- `hkask-services/src/consolidation.rs` (new): `ConsolidationService` with `verify_passphrase()` and `consolidate()` methods
- `hkask-services/src/error.rs`: Added `ServiceError::Consolidation(String)` variant
- `hkask-services/src/lib.rs`: Added `pub mod consolidation` and `pub use consolidation::ConsolidationService`
- `hkask-api/src/routes/consolidation.rs`: Replaced direct keystore/DB/pipeline access with `ConsolidationService::verify_passphrase()` + `ConsolidationService::consolidate()`

### ServiceContext gaps
- `hkask-services/src/context.rs`: Added `sovereignty_boundary_store` and `spec_store` fields with DB connections and schema init in `build()`

### CLI wiring
- `hkask-cli/src/commands/sovereignty.rs`: Status action uses `svc_ctx.sovereignty_boundary_store` instead of `open_sovereignty_store()`
- `hkask-cli/src/commands/spec.rs`: All 4 actions use `ctx.spec_store` from `build_service_context()` instead of `open_spec_store()`

### Dead code deletion
- `hkask-cli/src/commands/config.rs`: Deleted `open_sovereignty_store()` and `open_spec_store()` (no remaining callers)

---

## 3. What Remains

**Honest assessment: We are ~65% through.** The major extractions are done (GoalService, ConsolidationService). The depth-test decisions resolved 4 proposed modules as shallow. The remaining work is: (1) ensemble global statics decision, (2) remaining `from_parts()` sites (mostly legitimate per documented decisions), (3) remaining dead code in config.rs.

### MEDIUM — Remaining direct-access patterns

| File | Pattern | Sites | Status |
|------|---------|-------|--------|
| `routes/ensemble.rs` | `state.service_context.session_manager.read().await.get_chat()` | 3 | ⚠️ Could route through EnsembleService but session_manager is already in ServiceContext |
| `routes/ensemble.rs` | `state.service_context.standing_session_store.clone()` | 1 | ⚠️ Direct access but store is already in ServiceContext |
| `routes/ensemble.rs` | `state.service_context.mcp_runtime.discover_tools()` | 1 | ✅ Documented as shallow (McpService rejected) |
| `routes/pods.rs` | `state.service_context.capability_checker.check_resource()` | 1 | ⚠️ OCAP gate — legitimate surface-level access |
| `routes/chat.rs` | `state.service_context.inference_port` + `from_parts()` fallback | 1 | ✅ Documented as legitimate fallback pattern |
| `commands/ensemble.rs` | 3 global statics + 8 `from_parts()` sites | 11 | ❌ Blocked on architecture decision |

### LOW — Remaining `from_parts()` sites (mostly legitimate per documented decisions)

| File | Sites | Status |
|------|-------|--------|
| `commands/chat.rs` | 1 | ✅ Legitimate standalone fallback (Decision 5) |
| `commands/compose.rs` | 1 | ✅ Legitimate standalone inference (Decision 5) |
| `commands/ensemble.rs` | 1 | ⚠️ Blocked on statics decision |
| `repl/handlers/model.rs` | 2 | ⚠️ ReplState doesn't hold ServiceContext; uses own fields |
| `repl/handlers/hhh.rs` | 1 | ✅ Legitimate gate inference port (REPL-specific) |
| `repl/init.rs` | 2 | ✅ Pre-onboarding, no ServiceContext yet |

### LOW — Dead code to delete (after migration)

| Function | Current callers | Delete when |
|----------|----------------|-------------|
| `create_disconnected_governed_dispatcher()` | config.rs | Audit callers |
| `create_mcp_dispatcher()` | config.rs | Audit callers |
| `create_mcp_dispatcher_with_servers()` | config.rs, commands/mcp.rs, commands/models.rs | After CLI MCP wiring through ServiceContext |
| `init_registry()` / `init_registry_with_secrets()` | config.rs | After full CLI migration |
| `registry_db_path()` | config.rs | After all config.rs helpers are dead |
| `resolve_acp_secret()` / `resolve_mcp_secret()` / `resolve_db_passphrase()` | config.rs | After all callers migrated to ServiceContext |

### DECISION NEEDED — Ensemble global statics

The 3 `OnceLock` statics in `commands/ensemble.rs` provide cross-command session persistence. Options:
1. **Add `service_context: Arc<ServiceContext>` to ReplState** — Would enable `InferenceContext::from(&*state.service_context)` in REPL handlers AND provide a ServiceContext for ensemble CLI commands
2. **Build ServiceContext once at CLI main, thread through dispatch** — More surgical but requires signature changes
3. **Keep statics, document as intentional** — Acceptable if ensemble is CLI-only

This decision also affects: REPL handler `from_parts()` migration, compose/chat CLI wiring, and any future CLI command that needs ServiceContext.

---

## 4. Key Decisions to Preserve

1. **P1 Prohibition was misinterpreted in Session 12.** "Standalone CLI commands work without ServiceContext" means they CAN operate independently for simple operations, NOT that they MUST avoid ServiceContext. When a command needs shared infrastructure, it should use ServiceContext.

2. **ApiState holds `Arc<ServiceContext>` as single source of truth.** All domain objects come from `service_context.*`. Surface-specific fields (standing_sessions, ensemble_inferencer, git_cas, gas_governance) are the ONLY fields that don't come from ServiceContext. Do not add domain fields back to ApiState.

3. **`From<&ServiceContext>` is the derivation pattern for API routes.** Replace all `from_parts()` with this. API routes access `&state.service_context` (inside `Arc`), not owned `ServiceContext`.

4. **CLI commands use `build_service_context()` helper + `From<&ServiceContext>`.** Each CLI subcommand that needs ServiceContext builds one from `ServiceConfig::from_env()`. This is not ideal (rebuilds per call) but is correct and keeps subcommands independent. A future optimization should build ServiceContext once at CLI startup.

5. **`from_parts()` is acceptable for standalone inference port creation.** When no shared port is available (in-memory mode, fallback paths), `InferenceContext::from_parts(None, model, base_url)` is the correct pattern — it can't use `From<&ServiceContext>` because the ServiceContext itself may lack an inference port.

6. **F14/F18/F19 closures are valid.** ~11 `ApiError::` constructions in API routes are legitimate HTTP-layer concerns. Standing session CLI/API logic is too divergent to share. Improv is CLI-only.

7. **Ensemble global statics are an open architectural question.** Replacing them naively with per-call `ServiceContext::build()` would break CLI session continuity. Decision needed before ensemble migration continues.

8. **New service modules must pass the depth test.** Before creating any service module, apply the deletion test: delete the surface code — does complexity reappear in N callers? If the service module would just be a thin delegation, deepen or merge instead.

9. **CNS and Episodic routes do NOT need service modules.** `routes/cns.rs` (3 sites) does HTTP response mapping and SSE bridge logic — this is surface-only. `routes/episodic.rs` (3 sites) goes through the OCAP port membrane correctly. Creating CnsService or EpisodicService would be shallow pass-throughs that increase interface cost without adding behavior.

10. **Template, Bundle, ACP, and MCP routes do NOT need service modules.** Depth-tested in Session 15. All are thin delegations to domain crate methods with HTTP response mapping. Documented in module-level comments in each route file.

11. **ConsolidationService IS deep.** Passes deletion test: ~30 lines of infrastructure assembly (keystore + key derivation + per-agent DB + pipeline construction) would reappear in any caller. The route now delegates to `ConsolidationService::verify_passphrase()` + `ConsolidationService::consolidate()`.

12. **`routes/spec.rs` is all stubs.** No service module needed until spec persistence is implemented.

---

## 5. File Reference Map

| File | Role | Status |
|------|------|--------|
| `crates/hkask-api/src/lib.rs` | ApiState definition | ✅ Clean |
| `crates/hkask-api/src/routes/curator.rs` | Curator API routes | ✅ Uses `From<&ServiceContext>` |
| `crates/hkask-api/src/routes/sovereignty.rs` | Sovereignty API routes | ✅ Uses `From<&ServiceContext>` |
| `crates/hkask-api/src/routes/pods.rs` | Pod API routes | ✅ Uses `From<&ServiceContext>` |
| `crates/hkask-api/src/routes/ensemble.rs` | Ensemble API routes | ✅ Uses `From<&ServiceContext>` (3 direct `session_manager` access remain — shallow) |
| `crates/hkask-api/src/routes/models.rs` | Model listing routes | ✅ Uses `From<&ServiceContext>` |
| `crates/hkask-api/src/routes/chat.rs` | Chat route | ⚠️ 1 `from_parts()` fallback (legitimate) |
| `crates/hkask-api/src/routes/acp.rs` | ACP routes | ✅ Shallow — depth-test documented |
| `crates/hkask-api/src/routes/bundles.rs` | Bundle routes | ✅ Shallow — depth-test documented |
| `crates/hkask-api/src/routes/templates.rs` | Template routes | ✅ Shallow — depth-test documented |
| `crates/hkask-api/src/routes/mcp.rs` | MCP routes | ✅ Shallow — depth-test documented |
| `crates/hkask-api/src/routes/goal.rs` | Goal API routes | ✅ Uses `GoalService` via `GoalContext` |
| `crates/hkask-api/src/routes/consolidation.rs` | Consolidation API route | ✅ Uses `ConsolidationService` (Session 15) |
| `crates/hkask-api/src/routes/spec.rs` | Spec API route | ✅ Clean — stubs returning hardcoded data |
| `crates/hkask-api/src/routes/cns.rs` | CNS API route | ✅ Clean — surface-only SSE bridge |
| `crates/hkask-api/src/routes/episodic.rs` | Episodic API route | ✅ Clean — OCAP port membrane |
| `crates/hkask-services/src/lib.rs` | Services public API | ✅ Exports 7 service modules |
| `crates/hkask-services/src/goal.rs` | GoalService | ✅ Created Session 14 |
| `crates/hkask-services/src/consolidation.rs` | ConsolidationService | ✅ Created Session 15 |
| `crates/hkask-services/src/context.rs` | ServiceContext | ✅ Now includes sovereignty_boundary_store + spec_store |
| `crates/hkask-services/src/error.rs` | ServiceError | ✅ Includes Consolidation variant |
| `crates/hkask-cli/src/commands/curator.rs` | Curator CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/pod.rs` | Pod CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/sovereignty.rs` | Sovereignty CLI | ✅ Uses ServiceContext + sovereignty_boundary_store (Session 15) |
| `crates/hkask-cli/src/commands/goal.rs` | Goal CLI | ✅ Uses `GoalService` via `GoalContext` |
| `crates/hkask-cli/src/commands/spec.rs` | Spec CLI | ✅ Uses ServiceContext + spec_store (Session 15) |
| `crates/hkask-cli/src/commands/ensemble.rs` | Ensemble CLI | ❌ 3 global statics + 8 `from_parts()` sites |
| `crates/hkask-cli/src/commands/agent.rs` | Agent CLI | ❌ `init_registry()` + `registry_yaml_path()` (4 sites) |
| `crates/hkask-cli/src/commands/chat.rs` | Chat CLI | ❌ `init_registry*()` + `from_parts()` + `resolve_acp_secret()` (~5 sites) |
| `crates/hkask-cli/src/commands/mcp.rs` | MCP CLI | ❌ `create_mcp_dispatcher_with_servers()` (1 site) |
| `crates/hkask-cli/src/commands/models.rs` | Models CLI | ❌ `create_mcp_dispatcher_with_servers()` (1 site) |
| `crates/hkask-cli/src/commands/web_search.rs` | Web Search CLI | ❌ `create_mcp_dispatcher_with_servers()` (1 site) |
| `crates/hkask-cli/src/commands/git_cmd.rs` | Git CLI | ❌ `resolve_acp_secret()` (4 sites) |
| `crates/hkask-cli/src/commands/cns.rs` | CNS CLI | ❌ Standalone `CnsRuntime` (~3 sites) |
| `crates/hkask-cli/src/commands/compose.rs` | Compose CLI | ⚠️ `from_parts()` (1 site, legitimate standalone) |
| `crates/hkask-cli/src/commands/config.rs` | Legacy helpers | ⚠️ ~10 functions remaining; 2 deleted in Session 15 |
| `crates/hkask-cli/src/repl/` | REPL handlers | ⚠️ `from_parts()` (5 sites, 2 actionable — ReplState doesn't hold ServiceContext) |