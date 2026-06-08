# CONTINUATION.md — hKask Service Layer Extraction Session 16+

## Quick Start

Read `HANDOFF.md` in the project root for full context, file map, decision log, and session history.

---

## Where We Are

**Build: clean.** Sessions 12–15 built the service layer foundation, created 2 service modules (GoalService, ConsolidationService), depth-tested 4 proposed modules (all shallow — rejected with documentation), filled ServiceContext gaps, wired 6 CLI commands through ServiceContext, and deleted dead code. Full workspace passes `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`.

**We are ~65% through.** The deep service module extractions are done. What remains is wiring the last CLI commands that bypass ServiceContext, resolving one architectural blocker (ensemble statics), and deleting the legacy helper functions that become dead after migration.

---

## Honest Assessment: Remaining Work

### What's LEFT (sites that still bypass ServiceContext or use legacy helpers)

| Category | File | Pattern | Sites | Complexity |
|----------|------|---------|-------|------------|
| **Ensemble CLI** | `commands/ensemble.rs` | 3 OnceLock statics + 8 `EnsembleContext::from_parts(get_session_manager())` + `open_standing_session_store()` + `from_parts()` for improv | ~12 | HIGH — needs architectural decision first |
| **Agent CLI** | `commands/agent.rs` | `init_registry()` + `registry_yaml_path()` | 4 | MEDIUM — registry already in ServiceContext |
| **Chat CLI** | `commands/chat.rs` | `init_registry()` / `init_registry_with_secrets()` + `from_parts()` + `resolve_acp_secret()` + `registry_yaml_path()` | ~5 | MEDIUM — registry + inference in ServiceContext |
| **MCP CLI** | `commands/mcp.rs` | `create_mcp_dispatcher_with_servers()` | 1 | MEDIUM — mcp_runtime + mcp_dispatcher in ServiceContext |
| **Models CLI** | `commands/models.rs` | `create_mcp_dispatcher_with_servers()` | 1 | MEDIUM — same |
| **Web Search CLI** | `commands/web_search.rs` | `create_mcp_dispatcher_with_servers()` | 1 | MEDIUM — same |
| **Git CLI** | `commands/git_cmd.rs` | `resolve_acp_secret()` | 4 | LOW — secret in ServiceConfig |
| **CNS CLI** | `commands/cns.rs` | Creates own `CnsRuntime` standalone | ~3 | LOW — could use ServiceContext.cns_runtime |
| **REPL handlers** | `repl/handlers/model.rs` | `InferenceContext::from_parts()` | 2 | LOW — blocked on ReplState holding ServiceContext |
| **REPL hhh** | `repl/handlers/hhh.rs` | `InferenceContext::from_parts()` (gate port) | 1 | ✅ Legitimate — REPL-specific gate port |
| **REPL init** | `repl/init.rs` | `InferenceContext::from_parts()` (pre-onboarding) | 2 | ✅ Legitimate — no ServiceContext available yet |
| **API ensemble** | `routes/ensemble.rs` | 3 `session_manager` + 1 `standing_session_store` + 1 `mcp_runtime` | 5 | LOW — already in ServiceContext; shallow direct access |
| **API pods** | `routes/pods.rs` | 1 `capability_checker.check_resource()` | 1 | ✅ Legitimate OCAP surface gate |
| **API chat** | `routes/chat.rs` | 1 `from_parts()` fallback | 1 | ✅ Legitimate standalone fallback |

### Dead code in `commands/config.rs` (deletable after migration)

| Function | Current callers | Delete when |
|----------|----------------|-------------|
| `registry_db_path()` | `init_registry_with_secrets()` | After agent/chat CLI use ServiceContext |
| `registry_yaml_path()` | `agent.rs`, `chat.rs` (AgentRegistryLoader) | After agent/chat CLI use ServiceContext |
| `resolve_acp_secret()` | `git_cmd.rs` (4), `chat.rs` (1), `init_registry()` | After git/chat CLI use ServiceContext |
| `resolve_db_passphrase()` | `init_registry()` | After agent/chat CLI use ServiceContext |
| `resolve_mcp_secret()` | `create_mcp_dispatcher()`, `create_mcp_dispatcher_with_servers()` | After mcp/models/web_search CLI use ServiceContext |
| `create_disconnected_governed_dispatcher()` | `create_mcp_dispatcher()`, `create_mcp_dispatcher_with_servers()` | After all MCP dispatcher callers use ServiceContext |
| `create_mcp_dispatcher()` | No direct callers (dead) | Immediately |
| `create_mcp_dispatcher_with_servers()` | `mcp.rs`, `models.rs`, `web_search.rs` | After these 3 commands use ServiceContext |
| `init_registry()` | `agent.rs` (3), `chat.rs` (1) | After agent/chat CLI use ServiceContext |
| `init_registry_with_secrets()` | `chat.rs` (1) | After chat CLI uses ServiceContext |
| `ResolvedSecrets` struct | `chat.rs`, `config.rs` | After chat CLI uses ServiceContext — may need to keep for onboarding |

### Summary: ~28 remaining sites, ~10 dead functions, 1 architectural decision

---

## What To Do Next (Priority Order)

### 0. Load skills first

Load these skills before any code changes:
- **`refactor-service-layer`** (required — this IS the methodology)
- **`coding-guidelines`** (surgical changes only)
- **`constraint-forces`** (classify decisions)
- **`zoom-out`** (map each domain before extracting)
- **`tdd`** (any new service modules need tracer bullets)

### 1. Resolve ensemble global statics architecture decision

The 3 `OnceLock` statics in `commands/ensemble.rs` provide cross-command session persistence.

**Option A: Add `service_context: Arc<ServiceContext>` to ReplState** ⭐ RECOMMENDED
- Pro: Unblocks ALL remaining REPL/ensemble work in one shot. REPL init already builds a ServiceContext at line 127 of `init.rs` — just store it.
- Pro: `repl/handlers/model.rs` can use `InferenceContext::from(&*state.service_context)` instead of `from_parts()`.
- Con: ReplState will temporarily hold some duplicated fields (inference_port, service_config) that also exist in ServiceContext. These can be removed later.
- Implementation: Add `pub(crate) service_context: Arc<ServiceContext>` field to ReplState, populate from `ctx` in `init_repl_state()`, then wire ensemble and model handlers.

**Option B: Build ServiceContext once at CLI main, thread through dispatch**
- Con: Requires signature changes across many `run_*` functions. `kask serve` already has its own ServiceContext. More surgical but higher blast radius.

**Option C: Keep statics, document as intentional**
- Con: Blocks ensemble CLI migration. Violates single assembly point principle. No forward progress on service layer for ensemble.

**Decision criteria (constraint-forces):**
- Option A is a **Guideline** — best practice, relaxable with reason.
- Option C violates the **Guideline** that ServiceContext is the single assembly point.
- The REPL already builds a ServiceContext — adding it to ReplState is zero-cost.

### 2. Wire ensemble CLI through ServiceContext (after decision)

Once Option A is chosen:
- Replace 8 `EnsembleContext::from_parts(get_session_manager())` → `EnsembleContext::from(&*state.service_context)` in REPL paths, or `EnsembleContext::from(&build_service_context())` in non-REPL paths
- Replace `SESSION_MANAGER` static: already in `service_context.session_manager`
- Replace `CYBERNETICS_LOOP` static: already in `service_context.cybernetics_loop`
- Replace `IMPROV_CLIENT` static: needs ServiceContext's inference port + circuit breaker
- Replace `open_standing_session_store()` → `service_context.standing_session_store`
- Delete dead `open_standing_session_store()` function from ensemble.rs

### 3. Wire agent/chat CLI through ServiceContext

- `commands/agent.rs`: Replace `init_registry()` with `build_service_context()`. AgentRegistryLoader needs `registry_yaml_path()` — add to ServiceConfig or pass separately.
- `commands/chat.rs`: Replace `init_registry()`/`init_registry_with_secrets()` with `build_service_context()`. Replace `resolve_acp_secret()` with `service_context.config.acp_secret`.
- After migration: delete `init_registry()`, `init_registry_with_secrets()`, `registry_db_path()`, `resolve_acp_secret()`, `resolve_db_passphrase()` from config.rs (if no other callers remain).

### 4. Wire MCP dispatcher commands through ServiceContext

- `commands/mcp.rs`: Replace `create_mcp_dispatcher_with_servers()` with `build_service_context()` → use `service_context.mcp_dispatcher` + `service_context.mcp_runtime`
- `commands/models.rs`: Same pattern
- `commands/web_search.rs`: Same pattern
- After migration: delete `create_mcp_dispatcher()`, `create_mcp_dispatcher_with_servers()`, `create_disconnected_governed_dispatcher()`, `resolve_mcp_secret()` from config.rs

### 5. Wire git CLI and CNS CLI

- `commands/git_cmd.rs`: Replace 4 `resolve_acp_secret()` calls with ServiceContext-derived secret
- `commands/cns.rs`: Replace standalone `CnsRuntime::with_threshold()` with `service_context.cns_runtime`

### 6. Wire REPL model.rs through ServiceContext (after Option A)

- `repl/handlers/model.rs`: 2 `InferenceContext::from_parts()` → `InferenceContext::from(&*state.service_context)`

### 7. Delete all dead code from config.rs

After all commands are wired through ServiceContext:
- Delete: `registry_db_path()`, `registry_yaml_path()`, `resolve_acp_secret()`, `resolve_mcp_secret()`, `resolve_db_passphrase()`, `create_disconnected_governed_dispatcher()`, `create_mcp_dispatcher()`, `create_mcp_dispatcher_with_servers()`, `init_registry()`, `init_registry_with_secrets()`
- Keep: `ResolvedSecrets` (may still be needed for onboarding), `ServiceConfig::from_env()` patterns

### 8. Verify API route coverage is complete

All API routes should now be either:
- ✅ Using service modules (GoalService, ConsolidationService, CuratorService, etc.)
- ✅ Documented as shallow (templates, bundles, acp, mcp, cns, episodic)
- ✅ Legitimate fallback/OCAP patterns (chat, pods)

No route should call `hkask_keystore::*`, `Database::open()`, or construct domain infrastructure directly.

### 9. Full workspace verification

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace
```

Verify:
- No `from_parts()` in API routes (except documented legitimate fallbacks)
- No `open_*_store()` or `init_registry*` calls in any CLI command
- No direct `hkask_keystore::*` calls outside `hkask-services` and `hkask-keystore`
- All dead code in `config.rs` deleted
- Dependency direction: CLI → services → domain (no circular deps)

---

## Strategy Notes

**One domain per cycle.** Each CLI command group (ensemble, agent/chat, mcp-dispatcher, git/cns) gets its own wire → verify cycle. Never migrate two command groups in the same edit.

**The REPL ServiceContext addition is the keystone.** It unblocks ensemble, model handler, and future REPL work. Do it first, verify it, then cascade.

**AgentRegistryLoader is the tricky edge case.** It takes `registry_yaml_path()` as input, which is a filesystem path to YAML agent definitions. This is config, not infrastructure — it should come from `ServiceConfig` or be passed as a CLI argument. Don't create a service method for it; just thread the path through.

**The MCP dispatcher commands have a subtle issue.** `create_mcp_dispatcher_with_servers()` creates a fresh `McpRuntime`, starts MCP servers on it, then creates a dispatcher. The ServiceContext's `mcp_runtime` already has servers started by `serve.rs` or REPL init. For standalone CLI commands (`kask mcp invoke`, `kask models list`), we need a standalone runtime with servers. Two options:
1. Build a fresh ServiceContext for these commands (they already do this implicitly)
2. Keep `create_mcp_dispatcher_with_servers()` as a standalone helper (it's not dead code — it creates isolated runtimes for one-shot commands)

**Be honest about what's a service module vs. a config path.** `registry_yaml_path()` is just a filesystem path derived from an env var. It doesn't need a service module. It needs to be accessible from ServiceConfig or passed as a parameter.

---

## Tools

```bash
cargo check --workspace           # Full type-check
cargo clippy --workspace -- -D warnings  # Lint
cargo test --workspace            # Full test suite
cargo test -p hkask-services      # Service layer tests only
cargo test -p hkask-api            # API tests only
cargo test -p hkask-cli            # CLI tests only
grep -rn 'from_parts\|open_registry_db\|open_consent_store\|init_registry\|create_mcp_dispatcher' crates/  # Find remaining legacy patterns
grep -rn 'hkask_keystore::' crates/hkask-api/src/ crates/hkask-cli/src/  # Find direct keystore access outside services
grep -rn 'Database::open' crates/hkask-api/src/ crates/hkask-cli/src/  # Find direct DB opening outside services
```

---

## Completed Work (Sessions 12–15)

| Session | What | Files |
|---------|------|-------|
| 12 | Audit | Full codebase audit of direct-access patterns |
| 13 | Build fix + from_parts migration | `lib.rs`, `routes/*.rs`, `commands/curator.rs`, `commands/pod.rs`, `commands/sovereignty.rs`, `commands/goal.rs`, `commands/serve.rs`, `config.rs` |
| 14 | GoalService + API audit | `hkask-services/src/goal.rs` (new), `routes/goal.rs` (wired), `commands/goal.rs` (wired), `lib.rs` (exports), audited `routes/cns.rs`, `routes/episodic.rs`, `routes/consolidation.rs`, `routes/spec.rs` |
| 15 | Depth-tests + ConsolidationService + ServiceContext gaps | `hkask-services/src/consolidation.rs` (new), `routes/consolidation.rs` (wired), `routes/templates.rs`/`bundles.rs`/`acp.rs`/`mcp.rs` (depth-test docs), `context.rs` (sovereignty_boundary_store + spec_store), `error.rs` (Consolidation variant), `commands/sovereignty.rs` (wired), `commands/spec.rs` (wired), `commands/config.rs` (dead code deleted) |

## Remaining Work Inventory

| Priority | Task | Sites | Status |
|----------|------|-------|--------|
| 0 | Load skills (refactor-service-layer, coding-guidelines, constraint-forces, zoom-out, tdd) | — | Before any code |
| 1 | Resolve ensemble statics decision (Option A recommended) | 12 (ensemble.rs) | Decision needed |
| 2 | Add `service_context: Arc<ServiceContext>` to ReplState | 1 (init.rs) | Blocked on #1 |
| 3 | Wire ensemble CLI through ServiceContext | 12 | Blocked on #2 |
| 4 | Wire agent/chat CLI through ServiceContext | ~9 | Independent |
| 5 | Wire MCP dispatcher commands through ServiceContext | 3 | Independent |
| 6 | Wire git CLI + CNS CLI through ServiceContext | ~7 | Independent |
| 7 | Wire REPL model.rs through ServiceContext | 2 | Blocked on #2 |
| 8 | Delete dead code from config.rs | ~10 functions | After #3-#6 |
| 9 | Verify API routes fully covered | — | After all above |
| 10 | Full workspace verification | — | After all above |