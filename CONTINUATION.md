# CONTINUATION.md — hKask Service Layer Extraction Session 14+

## Quick Start

Read `HANDOFF.md` in the project root for full context, file map, and decision log. The summary below gives you the operational picture.

---

## Where We Are

**Build: clean.** Session 13 fixed 11 compilation errors, replaced 28 `from_parts()` sites in API routes, wired 4 CLI command modules through `ServiceContext::build()`, and deleted dead code. Full workspace passes `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`.

**But we are only ~40% through the extraction.** The original spec massively underestimated the work. There are ~80+ remaining direct-access sites across API routes, CLI commands, and REPL handlers that bypass the service layer. Creating the new service modules (BundleService, TemplateService, McpService, AcpService, GoalService) and wiring them through is the bulk of the remaining work.

---

## What To Do Next (Priority Order)

### 1. Load skills first

Load these skills before any code changes:
- **`refactor-service-layer`** (required — this IS the methodology)
- **`coding-guidelines`** (surgical changes only)
- **`constraint-forces`** (classify decisions)
- **`zoom-out`** (map each domain before extracting)
- **`tdd`** (new service modules need tracer bullets)

### 2. Audit unaudited API routes

4 route files have never been audited for direct-access patterns:
- `routes/cns.rs`
- `routes/episodic.rs`
- `routes/consolidation.rs`
- `routes/spec.rs`

Use `zoom-out` skill, then `grep` for `state.` and `State(state)` patterns. Add findings to the "API direct-access paths" table in HANDOFF.md §3.

### 3. Create new service modules (one domain at a time)

For each domain, follow the strangler fig sequence from `refactor-service-layer` skill Phase 4:

**Recommended order (by dependency and risk):**

| Priority | Module | What it wraps | API sites | CLI sites | Depth test result |
|----------|--------|---------------|-----------|-----------|-------------------|
| 1 | `GoalService` | `goal_repo` CRUD | 3 (goal.rs) | 0 (already wired) | Likely deep — goal logic is non-trivial |
| 2 | `TemplateService` | `registry` template ops | 4 (templates.rs) | 0 | Medium — template listing is thin but search adds depth |
| 3 | `BundleService` | `registry` bundle ops | 5 (bundles.rs) | 0 | Medium — bundle CRUD with skill-matching logic |
| 4 | `AcpService` | `pod_manager.acp_runtime()` | 3 (acp.rs) | 0 | Thin delegation? Apply depth test carefully |
| 5 | `McpService` | `mcp_runtime` tool discovery | 3 (mcp.rs + ensemble) | 0 | Thin delegation? Tool discovery may not warrant service |

**For each module:**
1. Apply depth test before creating — would complexity reappear in N callers? If not, don't create the module; deepen or merge instead.
2. RED: Write one failing test per service operation with `// REQ:` tag
3. GREEN: Implement minimal code to pass
4. Wire API route to call service
5. Wire CLI command to call service (if applicable)
6. Delete duplicated logic from surface
7. Verify: `cargo check --workspace && cargo test -p hkask-services && cargo test -p hkask-api`

### 4. Resolve ensemble global statics architecture

Before touching `commands/ensemble.rs`, decide on the approach (see HANDOFF.md §3 "LOW — Ensemble global statics architecture decision"). The 3 `OnceLock` statics provide cross-command session persistence — naive replacement with per-call `ServiceContext::build()` breaks this. Options:
1. CLI-level ServiceContext singleton
2. Thread ServiceContext through command dispatch
3. Keep statics as documented exception

### 5. Fill ServiceContext gaps

Add to `ServiceContext::build()`:
- `SovereigntyBoundaryStore` — used by sovereignty CLI Status action
- `SqliteSpecStore` — used by spec CLI commands (4 sites)

Then delete `open_sovereignty_store()` and `open_spec_store()` from `config.rs`.

### 6. Wire remaining CLI commands

- `commands/spec.rs` — 4 sites use `open_spec_store()`
- `commands/ensemble.rs` — 8 `from_parts()` sites + 3 global statics + `open_standing_session_store()`
- `commands/compose.rs` — 1 `from_parts()` site
- `commands/chat.rs` — 1 `from_parts()` site

### 7. Wire REPL handlers

- `repl/handlers/model.rs` — 2 `from_parts()` sites → `InferenceContext::from(&*state.service_context)`
- `repl/handlers/hhh.rs` — 1 `from_parts()` site → same
- `repl/init.rs` — 2 `from_parts()` sites (pre-onboarding inference, gate model)

### 8. Audit and delete remaining dead code

After all surfaces are wired:
- `config.rs`: `open_sovereignty_store()`, `open_spec_store()`, `create_disconnected_governed_dispatcher()`, `create_mcp_dispatcher()`, `create_mcp_dispatcher_with_servers()`, `init_registry()`, `init_registry_with_secrets()`
- `config.rs`: `registry_db_path()`, `registry_yaml_path()`, `resolve_acp_secret()`, `resolve_mcp_secret()`, `resolve_db_passphrase()` — check if these are only used by deleted functions

### 9. Full workspace verification

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace
```

### 10. Update documentation

- Update `OPEN_QUESTIONS.md` with remaining items
- Update `docs/status/test-inventory.md` with service-layer seam tests
- Mark completed items in HANDOFF.md file reference map

---

## Strategy Notes

**One domain per session.** Don't try to create all 5 service modules in one go. Each domain needs its own RED→GREEN→REFACTOR cycle with full workspace verification between domains.

**Apply the depth test ruthlessly.** If a proposed service module is just `self.inner.method()` with no added logic, it's shallow. Shallow modules increase interface cost without adding behavior. Merge or deepen instead.

**Don't touch what works.** The `from_parts()` calls in REPL handlers and chat fallbacks use `InferenceContext::from_parts()` with a shared port or standalone port — these are legitimate patterns that don't bypass ServiceContext. Only replace them if the replacement is provably simpler.

**The `state.service_context.registry.lock().await` pattern in templates/bundles routes** is direct registry access, not service-layer access. The question is whether template/bundle listing is "business logic" (extract to service) or "data access" (keep in surface). Apply the deletion test: if you delete the route handler, does the registry-query complexity reappear in another caller? If yes → extract. If no → it's surface-specific.

**Keep the handoff document updated.** After each domain migration, update the file reference map in HANDOFF.md with ✅/❌/⚠️ status. This keeps the next agent from re-auditing what's already done.