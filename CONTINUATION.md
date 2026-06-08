# CONTINUATION.md — hKask Service Layer Extraction Session 17+

## Quick Start

Read `HANDOFF.md` in the project root for full context, session history, file map, decision log, and honest assessment of remaining work.

---

## Where We Are

**Build: clean.** Sessions 12–16 built the service layer foundation, created 2 service modules (GoalService, ConsolidationService), depth-tested 4 proposed modules (all shallow — rejected), resolved the ensemble statics architectural decision (Option A), added `service_context: Arc<ServiceContext>` to ReplState, migrated ensemble CLI + all REPL ensemble handlers through ServiceContext, expanded ServiceContext with `acp_runtime` + `agent_registry_store`, and added `registry_yaml_path` to ServiceConfig. Full workspace passes `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`.

**We are ~75% through.** The deep service module extractions are done. The ReplState keystone is in. The ensemble domain is fully migrated. What remains is wiring the last CLI commands that bypass ServiceContext, adding missing ServiceContext fields (user_store), migrating the DB-opening commands, refactoring onboarding, and deleting the legacy helper functions.

---

## Honest Assessment: ~55 Sites Across ~12 Files Still Bypass ServiceContext

### Tier 1 — Mechanical `init_registry`/`resolve_*` replacements (~20 sites)

These are the highest-value, lowest-risk targets. ServiceContext now has everything needed (`acp_runtime`, `agent_registry_store`, `config.registry_yaml_path`, `config.acp_secret`).

| File | Sites | Key Change |
|------|-------|------------|
| `commands/agent.rs` | 7 | `init_registry()` → `build_service_context()`, `registry_yaml_path()` → `ctx.config.registry_yaml_path` |
| `commands/chat.rs` | 6 | `init_registry*()` → `build_service_context()`, `registry_yaml_path()` → `ctx.config.registry_yaml_path`, `resolve_acp_secret()` → `String::from_utf8_lossy(&ctx.config.acp_secret)` |
| `commands/mcp.rs` | 1 | `create_mcp_dispatcher_with_servers()` → `build_service_context()` + `ctx.mcp_runtime` + start servers + `ctx.mcp_dispatcher` |
| `commands/models.rs` | 1 | Same pattern as mcp.rs |
| `commands/web_search.rs` | 1 | Same pattern as mcp.rs |
| `commands/git_cmd.rs` | 4 | `resolve_acp_secret()` → `ctx.config.acp_secret` |

**Strategy:** For agent.rs, the `AgentRegistryLoader::new(registry_yaml_path(), acp, store, source)` call becomes `AgentRegistryLoader::new(ctx.config.registry_yaml_path.clone(), ctx.acp_runtime.clone(), ctx.agent_registry_store.clone(), Arc::new(FilesystemRegistrySource::new()))`. This eliminates 4 `init_registry()` calls.

**Strategy:** For mcp/models/web_search, the MCP dispatcher commands create isolated `McpRuntime` instances that start servers, invoke tools, then shut down. The ServiceContext already has `mcp_runtime` and `mcp_dispatcher`, but its runtime may not have the required servers started. Two approaches:
- **Option A (recommended):** Build a fresh ServiceContext for each command (same pattern as ensemble, curator, pod). Start the required servers on the ServiceContext's runtime. Use its `mcp_dispatcher` for invocation.
- **Option B:** Keep `create_mcp_dispatcher_with_servers()` as a local helper in each command file.

### Tier 2 — DB-opening commands + missing ServiceContext fields (~12 sites)

These commands open their own databases or build infrastructure that should come from ServiceContext.

| File | Sites | Key Change |
|------|-------|------------|
| `commands/helpers.rs` | 4 | `open_user_store()` → add `user_store` to ServiceContext |
| `commands/user.rs` | 1 | Replace `open_user_store()` with `ctx.user_store` |
| `git_archival.rs` | 4 | `registry_db_path()` + `resolve_db_passphrase()` + `Database::open` → ServiceContext's primary DB |
| `commands/consolidation.rs` | 4 | Replace standalone pipeline with ServiceContext's `ConsolidationService` (already exists) |
| `commands/compose.rs` | 2 | `Database::open` → ServiceContext's memory DB; `from_parts()` → `InferenceContext::from(ctx)` |
| `commands/embed_corpus.rs` | 1 | `Database::open` → ServiceContext's memory DB |

**ServiceContext gaps to fill first:**
- `user_store: Arc<Mutex<UserStore>>` — needed by helpers.rs, user.rs
- For consolidation/compose/embed_corpus: These open per-agent memory DBs. ServiceConfig already has `effective_memory_db_path()`. Consider adding a `build_per_agent_memory_db(agent_name)` method to ServiceContext that opens a memory DB using the same passphrase derivation.

### Tier 3 — Onboarding refactoring (~15 sites)

**This domain requires careful design, not mechanical migration.**

Onboarding runs BEFORE ServiceContext exists — it's the bootstrap process. However, much of its complexity can be simplified:
- Replace `init_registry()` / `init_registry_with_secrets()` with direct `AcpRuntime::new()` + `AgentRegistryStore::new()` calls (the same thing ServiceContext::build() does internally)
- Replace `registry_db_path()` + `resolve_db_passphrase()` + `Database::open` with `ServiceConfig::from_env()` + `Database::open(&config.db_path, &config.db_passphrase)`
- The key insight: onboarding doesn't need ServiceContext, but it shouldn't need the legacy `config.rs` helpers either. It should use `ServiceConfig` directly.

### Tier 4 — REPL init residuals + legitimate `from_parts()` (~5 sites)

| File | Sites | Assessment |
|------|-------|------------|
| `repl/init.rs` | 2 | `resolve_mcp_secret()` can use `ctx.config.mcp_secret` (already available since ServiceContext is built). Per-agent `Database::open` is REPL-specific — legitimate. |
| `repl/handlers/hhh.rs` | 1 | `from_parts()` for gate port — legitimate (REPL-specific port, not shared) |
| `commands/chat.rs` | 1 | `from_parts()` fallback — legitimate per Decision 5 |
| `commands/compose.rs` | 1 | `from_parts()` standalone — legitimate per Decision 5 |

### API residual (2 sites)

| File | Sites | Assessment |
|------|-------|-----------|
| `routes/chat.rs` | 1 | `from_parts()` fallback — legitimate (Decision 5) |
| `middleware/auth.rs` | 1 | `hkask_keystore::resolve_mcp_security_key()` — could use `state.service_context.config.mcp_secret` via extension |

### Dead code deletion (after all tiers complete)

After all migration is done, delete these from `config.rs`:
- `registry_db_path()`, `registry_yaml_path()`, `resolve_acp_secret()`, `resolve_mcp_secret()`, `resolve_db_passphrase()`
- `create_disconnected_governed_dispatcher()`, `create_mcp_dispatcher()`, `create_mcp_dispatcher_with_servers()`
- `init_registry()`, `init_registry_with_secrets()`
- Keep: `ResolvedSecrets` struct (still used by onboarding), `ServiceConfig::from_env()` patterns

**Delete `create_mcp_dispatcher()` immediately** — it has zero callers outside config.rs.

---

## What To Do Next (Priority Order)

### 0. Load skills first

Load these skills before any code changes:
- **`refactor-service-layer`** (required — this IS the methodology)
- **`coding-guidelines`** (surgical changes only)
- **`constraint-forces`** (classify decisions)
- **`zoom-out`** (map each domain before extracting)
- **`tdd`** (any new ServiceContext fields or service methods need tracer bullets)

### 1. Delete immediately-dead `create_mcp_dispatcher()` from config.rs

It has zero callers. Quick win.

### 2. Wire agent CLI through ServiceContext (7 sites)

- Add `build_service_context()` helper to `commands/agent.rs`
- Replace `init_registry()` with `build_service_context()`
- Replace `registry_yaml_path()` with `ctx.config.registry_yaml_path.clone()`
- Use `ctx.acp_runtime.clone()` instead of `init_registry()`-returned AcpRuntime
- Use `ctx.agent_registry_store.clone()` instead of `init_registry()`-returned store
- Verify: `cargo check -p hkask-cli`

### 3. Wire git CLI through ServiceContext (4 sites)

- Add `build_service_context()` helper to `commands/git_cmd.rs`
- Replace 4 `resolve_acp_secret()` with `ctx.config.acp_secret`
- Verify: `cargo check -p hkask-cli`

### 4. Wire MCP dispatcher commands through ServiceContext (3 files, 3 sites)

- For `mcp.rs`, `models.rs`, `web_search.rs`: build ServiceContext, start required servers on its `mcp_runtime`, use its `mcp_dispatcher` + issue a capability token
- Each command will need its own `build_service_context()` helper (same pattern as ensemble.rs)
- Verify: `cargo check -p hkask-cli`

### 5. Wire chat CLI through ServiceContext (6 sites)

- This is the most complex Tier 1 command. It uses `init_registry*()`, `registry_yaml_path()`, `resolve_acp_secret()`, and `from_parts()`.
- Replace registry init with `build_service_context()` + `ctx.acp_runtime` + `ctx.agent_registry_store`
- Replace `resolve_acp_secret()` with `ctx.config.acp_secret`
- Replace `from_parts()` with `InferenceContext::from(ctx)` (or keep as documented fallback)
- Verify: `cargo check -p hkask-cli`

### 6. Fill ServiceContext gaps + wire Tier 2 commands

- Add `user_store: Arc<Mutex<UserStore>>` to ServiceContext::build()
- Add `build_per_agent_memory_db()` method or helper for compose/embed_corpus/consolidation CLI
- Wire `commands/consolidation.rs` through ServiceContext's ConsolidationService
- Wire `commands/compose.rs` through ServiceContext
- Wire `commands/embed_corpus.rs` through ServiceContext
- Wire `commands/helpers.rs` → `ctx.user_store` (replaces `open_user_store()`)
- Wire `git_archival.rs` through ServiceContext's primary DB connection
- Verify: `cargo check -p hkask-cli`

### 7. Refactor onboarding to use ServiceConfig directly

- Replace `init_registry()` / `init_registry_with_secrets()` with direct AcpRuntime + AgentRegistryStore construction
- Replace `registry_db_path()` + `resolve_db_passphrase()` + `Database::open` with ServiceConfig-derived paths
- Replace `config::resolve_db_passphrase()` with `hkask_keystore::resolve_db_passphrase()` directly
- Document remaining legitimate `hkask_keystore::*` calls as pre-ServiceContext bootstrap
- Verify: `cargo check -p hkask-cli`

### 8. Wire REPL init residuals

- Replace `crate::commands::config::resolve_mcp_secret()` in `repl/init.rs` with `ctx.config.mcp_secret`
- Verify: `cargo check -p hkask-cli`

### 9. Wire API auth middleware

- Replace `hkask_keystore::resolve_mcp_security_key()` with `state.service_context.config.mcp_secret`
- Verify: `cargo check -p hkask-api`

### 10. Delete all dead code from config.rs

After all callers are migrated:
- Delete: `registry_db_path`, `registry_yaml_path`, `resolve_acp_secret`, `resolve_mcp_secret`, `resolve_db_passphrase`, `create_disconnected_governed_dispatcher`, `create_mcp_dispatcher`, `create_mcp_dispatcher_with_servers`, `init_registry`, `init_registry_with_secrets`
- Keep: `ResolvedSecrets` struct, `ServiceConfig::from_env()` patterns
- Verify: `cargo check --workspace`

### 11. Remove duplicated ReplState fields

Now that `service_context` is in ReplState, these fields are redundant:
- `cns` (use `service_context.cns_runtime`)
- `cybernetics_loop` (use `service_context.cybernetics_loop`)
- `loop_system` (use `service_context.loop_system`)
- `dispatch` (use `service_context.dispatch`)
- `service_config` (use `service_context.config`)

This is a separate cleanup pass. Migrate each consumer one at a time. Don't rush this — it touches every REPL handler.

### 12. Full workspace verification

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace
```

Verify:
- No `from_parts()` in API routes (except documented legitimate fallbacks)
- No `open_*_store()` or `init_registry*` calls in any CLI command (except onboarding)
- No direct `hkask_keystore::*` calls outside `hkask-services`, `hkask-keystore`, and `onboarding.rs`
- No `Database::open` calls outside `hkask-services`, `hkask-storage`, and `onboarding.rs`
- All dead code in `config.rs` deleted
- Dependency direction: CLI → services → domain (no circular deps)

---

## Strategy Notes

**One domain per cycle.** Each CLI command group gets its own wire → verify cycle. Never migrate two command groups in the same edit.

**The agent.rs migration is the highest-value Tier 1 target.** It validates the new `acp_runtime` + `agent_registry_store` fields in ServiceContext. If the pattern works here, it works for chat.rs too.

**The MCP dispatcher commands have a subtle issue.** ServiceContext's `mcp_runtime` may not have the required servers started for standalone CLI commands. Solution: build a fresh ServiceContext (like ensemble does) and start the needed servers before using the dispatcher.

**Onboarding is NOT a standard migration.** Don't try to make it use ServiceContext — it's pre-ServiceContext by design. Instead, simplify it to use ServiceConfig + direct domain construction instead of the legacy config.rs helpers.

**The consolidation CLI is the Tier 2 keystone.** The API already uses `ConsolidationService`. The CLI should too, instead of building the pipeline from scratch. This validates that the service module works for both surfaces.

**Don't forget the `create_mcp_dispatcher()` dead code.** It has zero callers and should be deleted immediately as a quick win.

---

## Tools

```bash
cargo check --workspace           # Full type-check
cargo clippy --workspace -- -D warnings  # Lint
cargo test --workspace            # Full test suite
cargo test -p hkask-services      # Service layer tests only
cargo test -p hkask-api            # API tests only
cargo test -p hkask-cli            # CLI tests only
grep -rn 'from_parts\|open_registry_db\|init_registry\|create_mcp_dispatcher\|resolve_acp_secret\|resolve_mcp_secret\|resolve_db_passphrase\|Database::open' crates/hkask-cli/src/ crates/hkask-api/src/  # Find remaining legacy patterns
grep -rn 'hkask_keystore::' crates/hkask-api/src/ crates/hkask-cli/src/  # Find direct keystore access outside services
```

---

## Completed Work (Sessions 12–16)

| Session | What | Key Files |
|---------|------|-----------|
| 12 | Audit | Full codebase audit of direct-access patterns |
| 13 | Build fix + from_parts migration | `lib.rs`, `routes/*.rs`, `commands/curator.rs`, `commands/pod.rs`, `commands/sovereignty.rs`, `commands/goal.rs`, `commands/serve.rs`, `config.rs` |
| 14 | GoalService + API audit | `hkask-services/src/goal.rs` (new), `routes/goal.rs` (wired), `commands/goal.rs` (wired), `lib.rs` (exports) |
| 15 | Depth-tests + ConsolidationService + ServiceContext gaps | `hkask-services/src/consolidation.rs` (new), `routes/consolidation.rs` (wired), `context.rs` (sovereignty + spec stores), `commands/sovereignty.rs` (wired), `commands/spec.rs` (wired), `config.rs` (dead code deleted) |
| 16 | ReplState keystone + Ensemble migration + ServiceContext expansion | `repl/mod.rs` (+service_context), `repl/init.rs` (Arc wrap), `repl/handlers/model.rs`, `repl/handlers/ensemble.rs`, `repl/handlers/into.rs`, `repl/handlers/ensemble_ops.rs`, `repl/handlers/status.rs`, `repl/handlers/ask.rs`, `repl/turn.rs`, `repl/commands.rs`, `commands/ensemble.rs` (no statics), `commands/serve.rs`, `services/context.rs` (+acp_runtime, +agent_registry_store), `services/config.rs` (+registry_yaml_path) |