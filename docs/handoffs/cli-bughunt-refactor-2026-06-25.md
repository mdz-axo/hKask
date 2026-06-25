# Handoff — hkask-cli Bug Hunt & Refactor Session

**Date:** 2026-06-25 | **Status:** Immediate fixes complete; broader refactors deferred

---

## 1. Session Context

This session audited the `hkask-cli` crate (`crates/hkask-cli/src/`) for bugs, architectural friction, and deepening opportunities, applying the **bug-hunt**, **coding-guidelines**, and **improve-codebase-architecture** skills. 10 immediate bugs/issues were fixed (all 57 CLI tests pass, zero warnings). 4 broader architectural refactors were identified but deferred — they touch ~15+ files and span the service-layer boundary, so they warrant a dedicated PR with contract tests.

---

## 2. What Was Done

### Bugs Fixed (Phase 1 — prior turn)

| Issue | File(s) Changed | What Changed |
|-------|----------------|-------------|
| Sovereignty `Revoke` silently ignored `--category` | `actions.rs`, `sovereignty.rs` | Removed unused `category` field from `SovereigntyAction::Revoke` (backend `ConsentManager::revoke_consent` only supports all-or-nothing). Updated user-facing message. |
| `SpecAction::Validate` used `id` instead of `spec_id` | `actions.rs`, `spec.rs` | Renamed field to `spec_id` to match `Evaluate`, `Cultivate`, `TestInvariant`. |
| `SpecAction::Cultivate` called `validate()` | `spec.rs` | Added explanatory comment; this is a service-layer gap (no `SpecService::cultivate` exists). |
| Duplicate CNS trace + dead `#[cfg]` block in Transcript handler | `main.rs` | Removed redundant `command_invoked` trace; removed dead `let _ = &path` suppression. |
| Unused `_rt` params (×7) | `main.rs`, `test.rs`, `registry.rs` | Removed from `check_fusion_startup`, `test::run`, `run_list`, `run_rm`, `list_styles`, `remove_style`. |

### Bugs Fixed (Phase 2 — this turn)

| Issue | File(s) Changed | What Changed |
|-------|----------------|-------------|
| `Cultivate` still used `id` (missed in Phase 1) | `actions.rs`, `spec.rs` | Renamed `id` → `spec_id` in `Cultivate` variant and all usages. |
| Fake CNS "Review Queue Depth" data | `cns.rs` | Removed hardcoded `Pending reviews: 0` / `IDLE` lines that queried nothing. |
| Serve tracing panic (double `set_global_default`) | `serve.rs` | Changed `expect()` to `let _ =` — avoids panic when `init_logging` already set the subscriber. |
| Consolidation identity mismatch | `consolidation.rs` | `kask consolidate --agent curator` was deriving `WebID::from_persona(b"curator")` instead of using the system `CuratorHandle`. Fixed to use `agent.is_none()` for the curator gate. |
| Sovereignty `Status` ignored `HKASK_WEBID` env var | `sovereignty.rs` | Changed from hardcoded `WebId::from_persona(b"cli-user")` to `resolve_user_webid()`, matching `Grant`/`Revoke`. |

### Test Status

- **`cargo check -p hkask-cli`**: 0 errors, 0 warnings
- **`cargo test -p hkask-cli`**: 57/57 passed (49 unit + 8 fuzz)

---

## 3. What Remains

### HIGH — Runtime Duplication (#16)

**Problem:** Six command handlers spawn their own `tokio::runtime::Runtime::new()` instead of using the shared runtime from `main()`:
- `export_cmd.rs` — `run_create()` (line 31), `run_upload()` (line 98)
- `kata.rs` — `start_kata()` (line 356)
- `qa.rs` — `run_script()` (line 434)
- `user.rs` — `replicant_rename()` (line 458), `replicant_delete()` (line 487)
- `wallet.rs` — `handle_fee()` (line 233), `handle_withdraw()` (line 437)

**Why it matters:** Each new runtime spawns a new thread pool. CNS spans from `main()` can't attribute work to these commands. The `command_completed` trace fires in `main()` before the spawned-runtime work finishes.

**Strategy:** Thread the shared tokio `Handle` or `Runtime` reference through these commands (as already done for `chat`, `curator`, `cns`, etc.). Most call sites accept `rt: &tokio::runtime::Runtime` — only the 6 internal functions that create fresh runtimes need adjustment.

**Files to change:** `export_cmd.rs`, `kata.rs`, `qa.rs`, `user.rs`, `wallet.rs`

---

### HIGH — Wasteful `McpRuntime` Allocation (#14)

**Problem:** `main.rs:53` creates `McpRuntime::new()` unconditionally, but only `chat` and `curator` commands receive it. All other commands (`mcp`, `models`, `doctor`, `web-search`, `spec`, `daemon`, `loops`, etc.) waste the allocation.

**Strategy:** Move `McpRuntime::new()` inside the `Commands::Chat` and `Commands::Curator` match arms. Remove the `runtime` binding from `main()` scope. This is surgical — only 3 lines change in `main.rs`.

---

### MEDIUM — Command Registry Pattern (#18, #20)

**Problem:** Adding a new subcommand requires 4+ edits across 3+ files: `actions.rs` (enum variant), `cli/mod.rs` (clap definition), `main.rs` (match arm), `commands/mod.rs` (module + re-export). No compile-time guarantee that a variant has a handler — mismatches produce a silent no-op at runtime.

**Strategy:** Define a `CommandHandler` trait (or macro) in `commands/mod.rs`:
```rust
trait CommandHandler {
    type Action;
    fn run(rt: &tokio::runtime::Runtime, action: Self::Action, registry: &mut SqliteRegistry);
}
```
Then use a dispatch macro that maps `Commands` variants to handler impls at compile time. This also opens the door to dropping `or_exit` in favor of `Result` propagation (Architecture A from Phase 1).

**Files to change:** `main.rs`, `commands/mod.rs`, possibly a new `commands/registry.rs` or macro crate.

---

### MEDIUM — CNS Overkill (#19)

**Problem:** `build_cns_runtime()` in `cns.rs` builds a full `AgentService` (wallet, inference, storage, MCP runtime) just to extract a CNS handle, then discards it. For a lightweight `kask cns health` query, this is extreme.

**Strategy:** Either cache the CNS runtime in a lazy static, pass it from `main()` (since it's already created for the daemon command), or add a `CnsService::lightweight()` constructor that skips wallet/storage init.

---

### LOW — Serve Tracing Concern Escalation (#21)

**Problem:** `serve.rs` reconfigures tracing when `--json-logs` is set. This is a cross-cutting concern that should live in `main.rs` alongside `init_logging`. The fix applied (#13) prevents the panic but doesn't address the architectural concern.

**Strategy:** Move the `--json-logs` flag to the top-level `Cli` struct and handle it in `init_logging`.

---

## 4. Recommended Skills and Tools

- **`coding-guidelines`** — Activate before writing any code; enforces surgical changes and simplicity.
- **`rust-expertise`** — For the command registry trait design (ownership, type-driven design).
- **`deep-module`** — Apply the deletion test to any new abstraction (the registry trait must earn its existence).
- **`tdd`** — Write a contract test for the command registry pattern before implementing it.
- **`diagnose`** — If runtime deduplication causes subtle timing issues in wallet/export commands.

**Verification commands:**
```bash
cargo check -p hkask-cli -p hkask-services
cargo test -p hkask-cli
cargo clippy -p hkask-cli -- -D warnings
```

---

## 5. Key Decisions to Preserve

1. **`SovereigntyAction::Revoke` no longer has a `category` field.** The `ConsentManager::revoke_consent(&self, webid)` API only supports all-or-nothing revocation. If per-category revocation is added to the backend later, the field can be restored. Do NOT add it back without the backend support.

2. **`SpecAction::Validate` and `Cultivate` both use `--spec-id` (not `--id`).** This was aligned to match `Evaluate`, `TestInvariant`. Watch for scripts that used the old `--id` flag on `kask spec validate`.

3. **`Cultivate` delegates to `validate`.** There is no `SpecService::cultivate` method. The `SpecCurator::cultivate` trait exists in `hkask-storage` but isn't wired through the service layer. Adding a dedicated cultivate endpoint requires a service-layer change first.

4. **`Consolidation` identity gate.** When `--agent` is omitted, the system `CuratorHandle` identity is used (for OCAP-gated access to all agent stores). When `--agent` is provided, `WebID::from_persona` is used. The `agent_name` variable defaults to `"curator"` for DB path resolution but does NOT affect identity — do not conflate these.

5. **Serve tracing fix is a band-aid.** The `let _ = set_global_default()` in `serve.rs` prevents the panic but doesn't actually switch to JSON logging if `init_logging` already set a text subscriber. A proper fix would move `--json-logs` to the top-level CLI. Do not delete the `_ = ` suppression without also moving the flag to `Cli`.
