# HANDOFF.md — hKask Service Layer Extraction

**Sessions:** 12–16 | **Status:** Build clean. ~75% through. | **Verification:** `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace` all pass.

---

## 1. Session History

### Session 12 (Audit)
Full codebase audit of direct-access patterns across CLI and API surfaces.

### Session 13 (Execution)
Fixed broken build (11 errors → 0), replaced all `from_parts()` calls in API routes with `From<&ServiceContext>` derivation, wired CLI curator/pod/sovereignty/goal commands through `ServiceContext::build()`, deleted dead `open_registry_db()` and `open_consent_store()`, passed full workspace verification.

### Session 14 (GoalService + API Audit)
Created `GoalService` (3 service methods + 3 parse helpers, 12 unit tests). Wired both API `routes/goal.rs` and CLI `commands/goal.rs` through `GoalService`. Audited 4 API route files — consolidation route was a major violation.

### Session 15 (Depth-Tests + ConsolidationService + ServiceContext Gaps)
Depth-tested 4 proposed service modules (all shallow — rejected with documentation). Created `ConsolidationService`. Filled ServiceContext gaps (`sovereignty_boundary_store`, `spec_store`). Wired CLI sovereignty + spec commands. Deleted dead `open_sovereignty_store()`/`open_spec_store()`.

### Session 16 (Keystone: ReplState + Ensemble + ServiceContext Expansion)
**Architectural decision resolved:** Added `service_context: Arc<ServiceContext>` to `ReplState` (Option A from prior session's decision matrix). This is the keystone that unblocks all remaining REPL/ensemble/cascade wiring.

**Ensemble CLI fully migrated:** Replaced 3 `OnceLock` statics (`SESSION_MANAGER`, `CYBERNETICS_LOOP`, `IMPROV_CLIENT`) and `open_standing_session_store()` with ServiceContext fields. All 12 ensemble command functions now take `&ServiceContext` as their first parameter. REPL slash commands (`/ensemble`, `/into`, `/filter`, `/mode`) pass `state.service_context`. CLI subcommand path uses `build_service_context()` helper. `serve.rs` now builds improv client from ServiceContext instead of global static.

**REPL model handler migrated:** 2 `InferenceContext::from_parts()` sites → `InferenceContext::from(state.service_context.as_ref())`.

**ServiceContext expanded with agent infrastructure:**
- Added `acp_runtime: Arc<AcpRuntime>` (was created internally but moved into PodManager; now cloned and stored)
- Added `agent_registry_store: AgentRegistryStore` (created from primary DB with schema init + ACP state restoration)
- Added `registry_yaml_path: PathBuf` to `ServiceConfig` (resolved from `HKASK_REGISTRY_PATH` env var)

**Added `chrono` dependency** to `hkask-services` for RFC3339 timestamp parsing in ACP state restoration.

**Cleaned up clippy:** All `&*state.service_context` → `&state.service_context` or `.as_ref()` patterns.

---

## 2. What Was Done (Session 16)

| Change | Files |
|--------|-------|
| Add `service_context: Arc<ServiceContext>` to ReplState | `repl/mod.rs` |
| Populate `service_context` in `init_repl_state()` | `repl/init.rs` |
| Wire model handler through ServiceContext | `repl/handlers/model.rs` |
| Replace 3 OnceLock statics + `open_standing_session_store()` | `commands/ensemble.rs` |
| All ensemble functions take `&ServiceContext` | `commands/ensemble.rs` |
| Wire REPL ensemble/into/filter/mode handlers | `repl/handlers/ensemble.rs`, `into.rs`, `ensemble_ops.rs` |
| Wire REPL status/ask/turn through ServiceContext | `repl/handlers/status.rs`, `ask.rs`, `turn.rs` |
| Wire REPL commands.rs call sites | `repl/commands.rs` |
| Wire serve.rs improv client from ServiceContext | `commands/serve.rs` |
| Add `acp_runtime` + `agent_registry_store` to ServiceContext | `services/context.rs` |
| ACP state restoration in ServiceContext::build() | `services/context.rs` |
| Add `registry_yaml_path` to ServiceConfig | `services/config.rs` |
| Add `chrono` dep to hkask-services | `services/Cargo.toml` |

---

## 3. What Remains — Honest Assessment

We are **~75% through**. The deep service module extractions (GoalService, ConsolidationService) are done. The ensemble migration is done. The ReplState keystone is in. But there are still **~55 sites** across **~12 files** that bypass ServiceContext using legacy patterns. The remaining work breaks into four tiers:

### Tier 1 — CLI commands with direct `init_registry`/`resolve_*` usage (~20 sites, 4 files)

These are the most mechanical — swap `init_registry()` → `build_service_context()`, swap `registry_yaml_path()` → `ctx.config.registry_yaml_path`, swap `resolve_acp_secret()` → `ctx.config.acp_secret`.

| File | Pattern | Sites | Notes |
|------|----------|-------|-------|
| `commands/agent.rs` | `init_registry()` + `registry_yaml_path()` | 7 | ServiceContext now has `acp_runtime` + `agent_registry_store` |
| `commands/chat.rs` | `init_registry*()` + `registry_yaml_path()` + `resolve_acp_secret()` + `from_parts()` | 6 | Complex — registry + inference + secrets |
| `commands/mcp.rs` | `create_mcp_dispatcher_with_servers()` | 1 | Use ServiceContext's `mcp_runtime` + `mcp_dispatcher` + start servers |
| `commands/models.rs` | `create_mcp_dispatcher_with_servers()` | 1 | Same pattern as mcp.rs |
| `commands/web_search.rs` | `create_mcp_dispatcher_with_servers()` | 1 | Same pattern as mcp.rs |
| `commands/git_cmd.rs` | `resolve_acp_secret()` | 4 | Use `ctx.config.acp_secret` |

### Tier 2 — CLI commands that open their own DB / build infrastructure (~12 sites, 5 files)

These build per-agent memory DBs, consolidation pipelines, or user stores from scratch. Some can use ServiceContext's shared stores; others need per-agent DBs that ServiceContext can't own (they're agent-scoped).

| File | Pattern | Sites | Notes |
|------|----------|-------|-------|
| `commands/consolidation.rs` | `hkask_keystore::resolve_db_passphrase()` + `Database::open` + builds full pipeline | 4 | CLI consolidation builds per-agent memory DB. ServiceContext already has `ConsolidationService` in API path. CLI path could use it too. |
| `commands/compose.rs` | `Database::open` + `from_parts()` | 2 | Opens per-agent DB for compose inference |
| `commands/embed_corpus.rs` | `Database::open` | 1 | Opens per-agent DB for embedding |
| `commands/helpers.rs` | `open_user_store()` + `registry_db_path()` + `resolve_db_passphrase()` + `Database::open` | 4 | Need `user_store` in ServiceContext |
| `commands/user.rs` | calls `open_user_store()` | 1 | Depends on helpers.rs |
| `git_archival.rs` | `registry_db_path()` + `resolve_db_passphrase()` + `Database::open` | 4 | Uses primary DB directly |

### Tier 3 — Onboarding module (~15 sites, 1 file)

**This is the hardest domain.** Onboarding runs BEFORE ServiceContext exists — it's the bootstrap that creates secrets, registers agents, and provisions the first replicant. It inherently needs `init_registry()`, `Database::open()`, and `hkask_keystore::*`.

| File | Pattern | Sites | Notes |
|------|----------|-------|-------|
| `onboarding.rs` | `init_registry*()` + `registry_db_path()` + `resolve_db_passphrase()` + `Database::open` + `hkask_keystore::*` | ~15 | Bootstrap — runs before ServiceContext. Most calls are **legitimate**. |

### Tier 4 — REPL init residuals + remaining `from_parts()` (~5 sites, 3 files)

| File | Pattern | Sites | Notes |
|------|----------|-------|-------|
| `repl/init.rs` | `resolve_mcp_secret()` + `Database::open` (per-agent memory) | 2 | init builds ServiceContext then adds per-agent memory on top. Per-agent DB is REPL-specific. |
| `repl/handlers/hhh.rs` | `from_parts()` for gate port | 1 | Legitimate — HHH gate port is REPL-specific |
| `commands/chat.rs` | `from_parts()` for standalone inference | 1 | May be legitimate fallback (Decision 5) |
| `commands/compose.rs` | `from_parts()` for standalone inference | 1 | Legitimate standalone (Decision 5) |

### API residual (2 sites, 2 files)

| File | Pattern | Sites | Notes |
|------|----------|-------|-------|
| `routes/chat.rs` | `from_parts()` fallback | 1 | Documented as legitimate (Decision 5) |
| `middleware/auth.rs` | `hkask_keystore::resolve_mcp_security_key()` | 1 | Auth middleware resolves key per-request — could use ServiceContext.config |

### Dead code in `config.rs` (deletable after migration)

| Function | Current callers outside config.rs | Delete when |
|----------|----------------------------------|-------------|
| `registry_db_path()` | `onboarding.rs`, `git_archival.rs`, `helpers.rs` | After Tier 2 migration |
| `registry_yaml_path()` | `agent.rs`, `chat.rs` | After Tier 1 migration |
| `resolve_acp_secret()` | `git_cmd.rs`, `chat.rs`, `init_registry()` | After Tier 1 migration |
| `resolve_db_passphrase()` | `consolidation.rs`, `helpers.rs`, `git_archival.rs`, `onboarding.rs`, `init_registry()` | After Tier 2 migration |
| `resolve_mcp_secret()` | `repl/init.rs`, `create_mcp_dispatcher*()` | After Tier 4 + MCP migration |
| `create_disconnected_governed_dispatcher()` | `create_mcp_dispatcher*()` | After MCP commands migrated |
| `create_mcp_dispatcher()` | None (already dead) | **Immediately** |
| `create_mcp_dispatcher_with_servers()` | `mcp.rs`, `models.rs`, `web_search.rs` | After Tier 1 MCP migration |
| `init_registry()` | `agent.rs`, `chat.rs`, `onboarding.rs` | After Tier 1 + onboarding refactor |
| `init_registry_with_secrets()` | `chat.rs`, `onboarding.rs` | After Tier 1 + onboarding refactor |

### ServiceContext gaps still needed

| Gap | Needed by | Priority |
|-----|-----------|----------|
| `user_store` (or `Arc<Mutex<UserStore>>`) | `helpers.rs:open_user_store()`, `user.rs` | Tier 2 |
| CLI consolidation using ServiceContext's memory infrastructure | `consolidation.rs` | Tier 2 — needs design |
| Per-agent memory DB path derivation in ServiceConfig | `compose.rs`, `embed_corpus.rs`, `consolidation.rs` | Tier 2 — config already has `effective_memory_db_path()` |

---

## 4. Key Decisions to Preserve

1–12. **All prior decisions still hold** (see previous HANDOFF versions).

13. **`service_context: Arc<ServiceContext>` is in ReplState.** This unblocks ensemble, model handler, and future REPL wiring. ReplState duplicates some ServiceContext fields (cns, cybernetics_loop, etc.) — these will be removed in a future cleanup pass once all consumers are migrated.

14. **Ensemble CLI functions take `&ServiceContext` as first parameter.** All 12 ensemble functions changed from using static singletons to parameter injection. CLI subcommand path uses `build_service_context()`. REPL path uses `state.service_context`.

15. **`From<&ServiceContext>` works for `InferenceContext`, `EnsembleContext`, `CuratorContext`, `GoalContext`, `PodContext`, `SovereigntyContext`.** When `Arc<ServiceContext>` is the container, use `.as_ref()` to get `&ServiceContext` for the `From` impl.

16. **ServiceContext now owns `acp_runtime` and `agent_registry_store`.** Both are populated in `ServiceContext::build()` with ACP state restoration. This eliminates the need for `init_registry()` in any command that only needs ACP + agent store.

17. **`registry_yaml_path` is in ServiceConfig.** Resolved from `HKASK_REGISTRY_PATH` env var, defaults to `registry/bots`. Commands that need AgentRegistryLoader can use `ctx.config.registry_yaml_path` instead of the legacy `config::registry_yaml_path()`.

18. **Onboarding is pre-ServiceContext by design.** It runs before any shared infrastructure exists. Most of its legacy calls are legitimate and should be documented, not migrated. However, `init_registry_with_secrets()` should be replaced with a lighter-weight approach that uses `ServiceConfig::from_secrets()` instead.

19. **MCP dispatcher commands need a standalone runtime.** `create_mcp_dispatcher_with_servers()` creates an isolated `McpRuntime`, starts MCP servers, then creates a dispatcher. ServiceContext's `mcp_runtime` already has servers started by `serve.rs` or REPL init. For standalone CLI commands (`kask mcp invoke`, `kask models list`), two options:
    - **Option A:** Build a fresh ServiceContext (they already do this implicitly)
    - **Option B:** Keep `create_mcp_dispatcher_with_servers()` as a standalone helper (it creates isolated runtimes for one-shot commands)
    Recommended: Option A — build ServiceContext, start servers on its runtime, use its dispatcher.

---

## 5. File Reference Map (Updated)

| File | Role | Status |
|------|------|--------|
| `crates/hkask-services/src/context.rs` | ServiceContext | ✅ Now includes `acp_runtime`, `agent_registry_store`, ACP restoration |
| `crates/hkask-services/src/config.rs` | ServiceConfig | ✅ Now includes `registry_yaml_path` |
| `crates/hkask-services/src/lib.rs` | Services public API | ✅ Exports 7 service modules |
| `crates/hkask-services/src/goal.rs` | GoalService | ✅ Created Session 14 |
| `crates/hkask-services/src/consolidation.rs` | ConsolidationService | ✅ Created Session 15 |
| `crates/hkask-services/src/error.rs` | ServiceError | ✅ Includes Consolidation variant |
| `crates/hkask-cli/src/repl/mod.rs` | ReplState | ✅ Now holds `service_context: Arc<ServiceContext>` |
| `crates/hkask-cli/src/repl/init.rs` | REPL init | ✅ Stores ctx in Arc, populates service_context |
| `crates/hkask-cli/src/repl/handlers/model.rs` | Model handler | ✅ Uses `InferenceContext::from(state.service_context.as_ref())` |
| `crates/hkask-cli/src/repl/handlers/ensemble.rs` | Ensemble handler | ✅ Takes `svc_ctx: &ServiceContext` |
| `crates/hkask-cli/src/repl/handlers/into.rs` | Into handler | ✅ Takes `svc_ctx: &ServiceContext` |
| `crates/hkask-cli/src/repl/handlers/ensemble_ops.rs` | Filter/Mode handlers | ✅ Take `svc_ctx: &ServiceContext` |
| `crates/hkask-cli/src/repl/handlers/status.rs` | Status handler | ✅ Uses `state.service_context` for ensemble config |
| `crates/hkask-cli/src/repl/handlers/ask.rs` | Ask handler | ✅ Uses `state.service_context` for ensemble send |
| `crates/hkask-cli/src/repl/turn.rs` | Ensemble turn | ✅ Uses `state.service_context` for improv |
| `crates/hkask-cli/src/repl/commands.rs` | Slash command dispatch | ✅ Passes `state.service_context` to ensemble/into/filter/mode |
| `crates/hkask-cli/src/commands/ensemble.rs` | Ensemble CLI | ✅ All functions take `&ServiceContext`, no statics |
| `crates/hkask-cli/src/commands/serve.rs` | API server | ✅ Builds improv client from ServiceContext |
| `crates/hkask-cli/src/commands/curator.rs` | Curator CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/pod.rs` | Pod CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/sovereignty.rs` | Sovereignty CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/goal.rs` | Goal CLI | ✅ Uses `GoalService` |
| `crates/hkask-cli/src/commands/spec.rs` | Spec CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/agent.rs` | Agent CLI | ❌ `init_registry()` + `registry_yaml_path()` (7 sites) |
| `crates/hkask-cli/src/commands/chat.rs` | Chat CLI | ❌ `init_registry*()` + `registry_yaml_path()` + `resolve_acp_secret()` + `from_parts()` (6 sites) |
| `crates/hkask-cli/src/commands/mcp.rs` | MCP CLI | ❌ `create_mcp_dispatcher_with_servers()` (1 site) |
| `crates/hkask-cli/src/commands/models.rs` | Models CLI | ❌ `create_mcp_dispatcher_with_servers()` (1 site) |
| `crates/hkask-cli/src/commands/web_search.rs` | Web Search CLI | ❌ `create_mcp_dispatcher_with_servers()` (1 site) |
| `crates/hkask-cli/src/commands/git_cmd.rs` | Git CLI | ❌ `resolve_acp_secret()` (4 sites) |
| `crates/hkask-cli/src/commands/consolidation.rs` | Consolidation CLI | ❌ `hkask_keystore::resolve_db_passphrase()` + `Database::open` + builds pipeline (4 sites) |
| `crates/hkask-cli/src/commands/compose.rs` | Compose CLI | ❌ `Database::open` + `from_parts()` (2 sites) |
| `crates/hkask-cli/src/commands/embed_corpus.rs` | Embed corpus CLI | ❌ `Database::open` (1 site) |
| `crates/hkask-cli/src/commands/helpers.rs` | Shared helpers | ❌ `open_user_store()` + `registry_db_path()` + `resolve_db_passphrase()` + `Database::open` (4 sites) |
| `crates/hkask-cli/src/commands/user.rs` | User CLI | ❌ Calls `open_user_store()` (1 site) |
| `crates/hkask-cli/src/git_archival.rs` | Git archival | ❌ `registry_db_path()` + `resolve_db_passphrase()` + `Database::open` (4 sites) |
| `crates/hkask-cli/src/onboarding.rs` | Onboarding | ⚠️ ~15 legacy calls — **most are legitimate pre-ServiceContext** |
| `crates/hkask-cli/src/commands/config.rs` | Legacy helpers | ⚠️ 10 functions remaining; `create_mcp_dispatcher()` is already dead |
| `crates/hkask-api/src/routes/chat.rs` | Chat API | ⚠️ 1 `from_parts()` fallback (legitimate per Decision 5) |
| `crates/hkask-api/src/middleware/auth.rs` | Auth middleware | ⚠️ 1 `hkask_keystore::resolve_mcp_security_key()` |

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*