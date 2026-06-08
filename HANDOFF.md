# HANDOFF.md — hKask Service Layer Extraction

**Sessions:** 12–18 | **Status:** ✅ Complete. All dead code deleted, ReplState deduplication done, full workspace verified (check + clippy + test + legacy pattern audit). | **Verification:** `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace` all pass. No legacy pattern violations found.

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
Added `service_context: Arc<ServiceContext>` to `ReplState`. Ensemble CLI fully migrated. ServiceContext expanded with `acp_runtime`, `agent_registry_store`, `registry_yaml_path`.

### Session 17 (Tier 1 Wiring Complete)
Systematically executed steps 1–9: migrated all Tier 1 CLI commands through ServiceContext, filled remaining ServiceContext gaps (`user_store`), wired Tier 2 commands (consolidation, git_archival, user), refactored onboarding to use `ServiceConfig` directly, wired API auth middleware, and cleaned up REPL init residuals.

### Session 18 (Final Steps: Dead Code + ReplState Dedup + Verification)
Completed Steps 10–12:
- **Step 10:** Deleted `commands/config.rs` entirely. Moved `ResolvedSecrets` into `onboarding.rs` (its natural home). Updated 3 import sites. Fixed `errors.rs` doc comment.
- **Step 11:** Removed all 5 duplicated ReplState fields (`service_config`, `dispatch`, `loop_system`, `cybernetics_loop`, `cns`). All consumers now read from `state.service_context.<field>`. Cleaned up 6 unused imports.
- **Step 12:** Full workspace verification passed. Legacy pattern audit found zero violations — all remaining patterns are legitimate and documented.

---

## 2. What Was Done (Session 18)

| Step | Change | Files |
|------|--------|-------|
| 10a | Moved `ResolvedSecrets` to `onboarding.rs` | `onboarding.rs`, `commands/chat.rs`, `repl/mod.rs` |
| 10b | Deleted `commands/config.rs` entirely (9 dead functions) | `commands/config.rs` (deleted), `commands/mod.rs` |
| 10c | Fixed `errors.rs` doc comment | `errors.rs` |
| 11a | Removed `service_config` from ReplState | `repl/mod.rs`, `repl/init.rs`, `repl/handlers/hhh.rs` |
| 11b | Removed `dispatch` from ReplState | `repl/mod.rs`, `repl/init.rs`, `repl/cns_display.rs` |
| 11c | Removed `loop_system` from ReplState | `repl/mod.rs`, `repl/init.rs`, `repl/cns_display.rs`, `repl/handlers/status.rs` |
| 11d | Removed `cybernetics_loop` from ReplState | `repl/mod.rs`, `repl/init.rs`, `repl/turn.rs`, `repl/hhh_loop.rs` |
| 11e | Removed `cns` from ReplState | `repl/mod.rs`, `repl/init.rs`, `repl/cns_display.rs`, `repl/handlers/status.rs` |

---

## 3. Remaining Legitimate Legacy Patterns (Do NOT Migrate)

These are architecturally correct and should not be changed:

| Pattern | Location | Why legitimate |
|---------|----------|---------------|
| `InferenceContext::from_parts()` fallback | `routes/chat.rs:80`, `commands/chat.rs:233` | Decision 5: fallback when shared port unavailable |
| `InferenceContext::from_parts()` for HHH gate | `repl/handlers/hhh.rs:55`, `repl/init.rs:53,77` | REPL-specific gate port, not in ServiceContext |
| `InferenceContext::from_parts()` for compose | `commands/compose.rs:275` | Standalone command, uses user-provided DB |
| `Database::open` for per-agent memory | `repl/init.rs:194`, `commands/compose.rs:110`, `commands/embed_corpus.rs:106` | Per-agent DBs with user-provided passphrase |
| `Database::open` in onboarding | `onboarding.rs` (5 sites) | Bootstrap — must open DB before ServiceContext |
| `Database::open` in consolidation | `commands/consolidation.rs:41` | Derives secrets from user-provided passphrase for verification |
| `hkask_keystore::*` in onboarding | `onboarding.rs` (5 sites) | Bootstrap — runs before ServiceContext |
| `hkask_keystore::*` in bootstrap | `bootstrap.rs` | Bootstraps secrets into keychain |
| `hkask_keystore::*` in keystore command | `commands/keystore.rs` | Keychain CLI — is the keystore surface |
| `hkask_keystore::*` in consolidation | `commands/consolidation.rs:78` | Derives secrets from user-provided passphrase |
| `ResolvedSecrets` struct | `onboarding.rs` | Now in its natural home — onboarding |
| `AuthService::new()` | `middleware/auth.rs:40` | Legacy path — `from_config()` is now used; `new()` kept for tests/standalone |

---

## 4. Key Decisions to Preserve

1–12. **All prior decisions still hold** (see previous HANDOFF versions).

13. **`service_context: Arc<ServiceContext>` is in ReplState.** All 5 duplicated fields have been removed. Consumers read from `state.service_context.<field>`.

14. **Ensemble CLI functions take `&ServiceContext` as first parameter.** All 12 ensemble functions use parameter injection.

15. **`From<&ServiceContext>` works for `InferenceContext`, `EnsembleContext`, `CuratorContext`, `GoalContext`, `PodContext`, `SovereigntyContext`.** Use `.as_ref()` from `Arc<ServiceContext>`.

16. **ServiceContext owns `acp_runtime`, `agent_registry_store`, and `user_store`.** All populated in `ServiceContext::build()`.

17. **`registry_yaml_path` is in `ServiceConfig`.** Resolved from `HKASK_REGISTRY_PATH` env var.

18. **Onboarding is pre-ServiceContext by design.** It uses `ServiceConfig` directly and `init_registry_from_config()` — not `ServiceContext::build()`.

19. **MCP dispatcher commands build fresh ServiceContext.** They use `build_service_context(rt, servers)` helpers that construct ServiceContext and start MCP servers on its runtime.

20. **`ServiceConfig::from_secrets()` returns `Self` directly** (not `Result`). It takes `(acp_secret, db_passphrase, mcp_secret, default_agent)`.

21. **`ResolvedSecrets` is in `onboarding.rs`, not `commands/config.rs`.** The `config.rs` module has been deleted entirely.

22. **`AgentRegistryStore` and `UserStore` derive `Clone`** via the `define_store!` macro — they can be cloned from ServiceContext without Arc/Mutex wrapping. `UserStore` is stored as `Arc<Mutex<UserStore>>` in ServiceContext for write-safety.

23. **`compose.rs` and `embed_corpus.rs` use user-provided `--db` and `--passphrase` CLI args.** Their `Database::open` calls are legitimate, not legacy patterns.

24. **API auth middleware uses `AuthService::from_config()`.** This avoids per-request keystore resolution by using the already-resolved `config.mcp_secret`.

25. **`commands/config.rs` is deleted.** All 9 dead functions removed. `ResolvedSecrets` moved to `onboarding.rs` where it belongs conceptually.

26. **ReplState has zero duplicated ServiceContext fields.** The 5 fields (`cns`, `cybernetics_loop`, `loop_system`, `dispatch`, `service_config`) were all removed. Consumers use `state.service_context.<field>`.

---

## 5. File Reference Map (Final)

| File | Role | Status |
|------|------|--------|
| `crates/hkask-services/src/context.rs` | ServiceContext | ✅ All fields populated |
| `crates/hkask-services/src/config.rs` | ServiceConfig | ✅ Includes `registry_yaml_path`, `from_secrets()`, `from_env()` |
| `crates/hkask-services/src/lib.rs` | Services public API | ✅ Exports 7 service modules |
| `crates/hkask-services/src/goal.rs` | GoalService | ✅ Created Session 14 |
| `crates/hkask-services/src/consolidation.rs` | ConsolidationService | ✅ Created Session 15 |
| `crates/hkask-services/src/error.rs` | ServiceError | ✅ Includes Consolidation variant |
| `crates/hkask-cli/src/repl/mod.rs` | ReplState | ✅ No duplicated fields; `service_context: Arc<ServiceContext>` |
| `crates/hkask-cli/src/repl/init.rs` | REPL init | ✅ Stores ctx in Arc, populates service_context |
| `crates/hkask-cli/src/repl/handlers/model.rs` | Model handler | ✅ Uses `From<&ServiceContext>` |
| `crates/hkask-cli/src/repl/handlers/ensemble.rs` | Ensemble handler | ✅ Takes `&ServiceContext` |
| `crates/hkask-cli/src/repl/cns_display.rs` | CNS display | ✅ Uses `state.service_context.cns_runtime`, `.loop_system`, `.dispatch` |
| `crates/hkask-cli/src/repl/handlers/status.rs` | Status handler | ✅ Uses `state.service_context.cns_runtime`, `.loop_system` |
| `crates/hkask-cli/src/repl/hhh_loop.rs` | HHH loop | ✅ Uses `state.service_context.cybernetics_loop` |
| `crates/hkask-cli/src/repl/turn.rs` | Chat turn | ✅ Uses `state.service_context.cybernetics_loop` |
| `crates/hkask-cli/src/commands/agent.rs` | Agent CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/chat.rs` | Chat CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/mcp.rs` | MCP CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/models.rs` | Models CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/web_search.rs` | Web Search CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/git_cmd.rs` | Git CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/consolidation.rs` | Consolidation CLI | ✅ Uses ServiceConfig |
| `crates/hkask-cli/src/commands/user.rs` | User CLI | ✅ Uses ServiceContext |
| `crates/hkask-cli/src/commands/config.rs` | ~~Legacy helpers~~ | ❌ Deleted (Step 10) |
| `crates/hkask-cli/src/onboarding.rs` | Onboarding | ✅ Owns `ResolvedSecrets`; uses `ServiceConfig` + `init_registry_from_config()` |
| `crates/hkask-cli/src/git_archival.rs` | Git archival | ✅ Uses ServiceContext |
| `crates/hkask-api/src/middleware/auth.rs` | Auth middleware | ✅ Uses `AuthService::from_config()` |
| `crates/hkask-api/src/routes/chat.rs` | Chat API | ✅ 1 `from_parts()` fallback (legitimate) |
| `crates/hkask-api/src/lib.rs` | ApiState | ✅ Uses `ServiceContext::build()` |

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*