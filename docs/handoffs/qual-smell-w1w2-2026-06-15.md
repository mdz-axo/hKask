# Handoff — Code Quality & Smell Reduction, Waves 1–2

**Session date:** 2026-06-15
**Progress:** Waves 1–5 complete, Wave 6 pending
**Build status:** `cargo check --workspace` + `cargo clippy --workspace -D warnings` → clean

---

## 2) What Was Done

### Wave 1 — Security Boundary Correctness (✅ Complete)

**PR 1.1:** Created shared P4 Gate 1/2/3 startup verifier

- New file: `crates/hkask-mcp/src/startup.rs`
  - `verify_startup_gates(client, replicant, role, required_tools) -> StartupGateResult`
  - Performs `auth_query` (G1) → `assignment_query` (G2) → `capability_query` per tool (G3)
  - G3 denial is non-fatal — tools collected in `StartupGateResult::denied_tools`
  - No tracing in the shared function (tracing macros require constant `target:` values; callers do their own logging)
  - 6 tests with real Unix socket mock listener (same pattern as existing `daemon.rs` tests)
- Modified: `crates/hkask-mcp/src/lib.rs` — added `pub mod startup` + re-exports

**PR 1.2:** Adopted `verify_startup_gates` across all 10 MCP servers

- Removed ~320 lines of duplicated inline auth+assignment match code across 10 MCP servers
- `hkask-mcp-communication` gained daemon gate verification (previously had none)
- `hkask-mcp-condenser` has 7 G3 tools: `compress`, `classify`, `set_profile`, `stats`, `ping`, `persist`, `thread_summary`
- `hkask-mcp-research` retains its 3-tool list: `web_search`, `web_extract`, `web_browse`
- Other 8 servers use empty tool lists (`&[]`) — G3 infrastructure in place, tool lists can be added later

### Wave 2 — Runtime Reliability (✅ Complete)

**PR 2.1:** Fixed all 20 `.unwrap()` calls in `hkask-mcp-media` runtime code (7 helper methods + 13 tool handlers/main).

**PR 2.2:** Fixed 5 `.unwrap()` calls in `hkask-mcp-docproc` runtime code:
- `tools.rs`: 2 lock unwraps in `docproc_query` + `docproc_clear_index` → `span.internal_error()`
- `server.rs`: 2 lock unwraps in `index_passages` + `accumulate_and_check_drift` → `.expect()`
- `pipeline.rs`: 1 `primary_result.unwrap()` guarded by `is_none()` → `.expect()`

**PR 2.3:** Fixed 12 `.unwrap()` calls in `hkask-templates` `registry_sqlite.rs`:
- 4 `Result`-returning functions → `.map_err(|e| TemplateError::Database(InfrastructureError::from(e)))?`
- 7 `Option`/unit-returning functions → `.expect("Failed to lock registry connection for ...")`
- 1 `count()` returning `usize` → match with early return 0 on lock poison

### What Remains (not started)

- **Wave 6:** Sustainment (task 10 — CI quality gates)

---

## 5) Wave 5 — Module Depth + Safety Governance (✅ Complete)

### Task 8 — Public surface control and justification policy (✅ Complete)

**PR 8.1:** Added `PUBLIC_SURFACE-<crate>.md` justification docs for all 13 oversized crates:
- Each doc explains why the surface is large, lists mitigations, and passes the deletion test
- Covers: types, services, storage, agents, api, cns, improv, inference, keystore, mcp, memory, templates, wallet

**PR 8.2:** Skipped — reducing re-exports would be breaking; justification docs suffice

**PR 8.3:** Added `scripts/check-public-surface.sh`:
- Counts pub items per crate, flags crates >7 without PUBLIC_SURFACE.md
- All 16 crates pass (3 within threshold, 13 with justification docs)

### Task 9 — Enforce unsafe documentation policy (✅ Complete)

**PR 9.1:** Added `scripts/check-unsafe-safety.sh`:
- Scans all non-test Rust files for `unsafe {` blocks
- Flags any lacking a `SAFETY:` comment within 5 preceding lines
- Excludes test modules and test helper functions

**PR 9.2:** Moved one `SAFETY:` comment in `database.rs` to precede the `unsafe {` block
- All non-test unsafe blocks now have proximate SAFETY: comments

**PR 9.3:** No new violations detected — codebase was already compliant

---

## 4) Wave 4 — Architecture Convergence (✅ Complete)

### Task 5 — Settings-domain strangler extraction (✅ Complete)

**PR 5.1:** Added generic `load_settings<T>()` and `save_settings<T>()` to `hkask-services/src/settings.rs`:
- Shared file I/O for any `Serialize + DeserializeOwned + Default` type
- Uses `ServiceError::Infra` for typed error propagation
- 2 REQ-tagged tests (default fallback, save/load round-trip)

**PR 5.2:** Migrated CLI `commands/settings.rs` to delegate to service:
- Removed ~15 lines of duplicated `load_settings`/`save_settings`/`settings_path`
- REPL init (`repl/init.rs`) also updated to use service directly

**PR 5.3:** Migrated API `routes/settings.rs` to delegate to service:
- Thin wrappers preserve existing `fn load_settings() -> SettingsResponse` signature
- Removed ~15 lines of duplicated file I/O logic

**PR 5.4:** REPL already aligned — uses same `load_settings` from service via `repl/init.rs`

### Task 6 — CNS loop telemetry (✅ Complete)

**PR 6.1:** Added `LoopQuality` type to `hkask-types::loops`:
- Fields: `delay_ms` (loop latency), `gain` (actions/deviations ratio), `fidelity_score` (match quality)
- `from_cycle()` constructor computes metrics from deviations + actions
- 4 REQ-tagged tests (default zero, gain computation, no deviations, unmatched fidelity)

**PR 6.2:** Instrumented `CyberneticsLoop`:
- Added `loop_quality: Arc<RwLock<LoopQuality>>` field
- Overrode `tick()` to measure elapsed time and compute `LoopQuality`
- Added `loop_quality()` accessor method
- Debug tracing emits delay_ms, gain, fidelity per cycle
- 2 REQ-tagged tests (default quality, tick updates quality)

**PR 6.3:** REQ tests included in PR 6.1 + 6.2 above

### Task 7 — Strengthen span typing (✅ Complete)

**PR 7.1:** Added `SpanKind` enum + `Span::from_kind()` to `hkask-types::event`:
- 13 variants covering the most common spans (tool, gas, curation, agent_pod, variety)
- Each variant maps to a canonical (namespace, path) pair — no string typos possible
- 1 REQ-tagged test verifying all variant paths

**PR 7.2:** Migrated 2 high-traffic call sites in `CyberneticsLoop`:
- `persist_directive_acknowledgment` → `SpanKind::CurationDirectiveAcknowledged`
- `act()` algedonic alert → `SpanKind::VarietyAlgedonicAlert`
- Removed now-unused `SpanNamespace` import

**PR 7.3:** Existing `Span::new()` remains as compatibility adapter for non-migrated sites

---

## 3) Wave 3 — Spec Traceability + Dead Abstraction Removal (✅ Complete)

### Task 3 — P8 REQ coverage expansion

**PR 3.1 — hkask-types:** 20 new REQ-tagged tests:
- `event.rs`: 8 tests (NuEvent defaults, builder chain, SpanNamespace parse/category, SpanCategory, Phase backward-compat, Span construction)
- `error.rs`: 6 tests (McpErrorKind retryable/intervention, InfrastructureError From impls, Display)
- `capability/tokens.rs`: 2 tests (ConsolidationToken issuer verification)
- `capability/verification.rs`: 4 tests (CapabilityChecker, verify_delegation_token, require_write_access)

**PR 3.2 — hkask-agents:** 6 new REQ-tagged tests:
- `pod/mod.rs`: 6 tests (lifecycle transitions, pod defaults, is_active, voice round-trip, error Display)

**PR 3.3 — hkask-api:** 4 new REQ-tagged tests:
- `error.rs`: 3 tests (ApiError status codes, Display, IntoResponse)
- `routes/settings.rs`: 1 test (seed field merge semantics)

### Task 4 — Replace pass-through capability validator

**PR 4.1–4.3 (combined):** Replaced `CapabilityAwareValidator` stub with real implementation:
- Uses `capabilities_match` from `hkask-types::capability` for action-hierarchy-aware comparison
- Returns `TemplateError::CapabilityDenied` with details about unsatisfied requirements
- Handles malformed requirements, empty requirements, multiple requirements
- 9 new REQ-tagged tests: empty pass, satisfied pass, unsatisfied fail, action hierarchy (Execute≥Write≥Read), malformed error, multiple requirements, no tokens fail

**Validation:**
```
✅ cargo test -p hkask-types --lib      → 41/41 passed
✅ cargo test -p hkask-agents --lib     → 14/14 passed
✅ cargo test -p hkask-api --lib        → 6/6 passed
✅ cargo test -p hkask-templates --lib  → 20/20 passed
```

---

## 4) Recommended Skills and Tools

For next session, load these skills after context reset:

| Skill | Reason |
|-------|--------|
| `coding-guidelines` | Before writing any code — enforces simplicity, surgical changes, goal-driven |
| `condenser-continuation` | If continuing from this handoff in a fresh session |

Key commands:
```bash
cargo check -p hkask-mcp-media          # Verify after each batch of unwrap fixes
cargo clippy -p hkask-mcp-media -- -D warnings
cargo test -p hkask-mcp                  # Re-run startup gate tests
cargo check --workspace                  # Full workspace check before moving to PR 2.2
```

---

## 5) Key Decisions to Preserve

1. **No tracing in `verify_startup_gates`** — Tracing macros (`info!`, `warn!`, etc.) require constant `target:` values at compile time. Passing a runtime `trace_target` string fails with `E0435`. The shared function returns a structured result; each caller logs with its own constant target string.

2. **G3 capability denial is non-fatal** — Matches the research server's existing behavior. Servers start in degraded mode; denied tools are reported via `StartupGateResult::denied_tools`. Reason: OCAP principle is that tools are individually gated, not the whole server. Denying one tool shouldn't block the server.

3. **Empty tool lists for G3 are valid** — 8 of 10 servers use `&[]` for `required_tools`. This means Gate 3 is a no-op (no capability queries). Tool lists can be populated as each server's capability requirements are defined. The infrastructure is in place.

4. **For lock poisoning in helper methods: `String` error propagation** — Since these helpers return `Result<T, String>` (not `McpToolError`), the lock poison error is converted to a `format!("Gallery state lock error: {}", e)` string. Callers already handle `String` errors via span guards or error accumulation. This avoids changing function signatures.

5. **For lock poisoning in tool handlers: `span.internal_error()`** — These handlers return `String` directly. The `span` guard is already in scope. Using `span.internal_error()` produces an `McpErrorKind::Internal` error in the JSON output, which is the correct taxonomy for infrastructure faults like lock poisoning.

6. **Communication server now has `try_daemon_flow`** — Previously the only server without daemon verification. Added with role `"communication"` and empty tool list. The `_daemon_client` is created but not yet wired into the `CommunicationServer` struct (that struct has no `daemon` or `replicant` fields — a separate task).
