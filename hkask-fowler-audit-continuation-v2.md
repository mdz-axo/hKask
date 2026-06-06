---

## Continuation Prompt: Fowler Refactoring Audit — hKask Codebase v2

### Status

Phases 1–3 are substantially complete. Phases 4–6 remain. The continuation prompt below covers the exact remaining work, design constraints, verification commands, TDD methodology, and current file states.

---

### Project Root

`/home/mdz-axolotl/Clones/hKask`

### Architecture Constraints (Non-Negotiable — Read Before Touching Code)

From `docs/architecture/PRINCIPLES.md` and `AGENTS.md`:

| # | Constraint | Notes |
|---|------------|-------|
| **P1** | No trait without two consumers | Extract types only when ≥2 call sites warrant it |
| **P6** | Delete stubs, don't publish them | No `todo!` or `unimplemented!` |
| **P7** | Prefer deletion over deprecation | Remove, don't `#[deprecated]` |
| **C1** | A type must be worn before it's tailored | Use before abstract |
| **C4** | Repetition is a missing primitive | DRY violation → extract |
| **Headless** | No visual UI, no Grafana/Prometheus | CLI/MCP/API only. CNS provides programmatic observability |

**Verification**: `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`

---

### Skill Integration (Read Before Starting Work)

This project requires three skills to be active. Invoke them in this priority order:

#### 1. `coding-guidelines` — CRITICAL (always active)

Karpathy's four behavioral principles govern ALL code changes:

1. **Think Before Coding** — Surface assumptions, present alternatives, ask when uncertain. No silent interpretation.
2. **Simplicity First** — Minimum code that solves the problem. No speculative features, no abstractions for single-use code, no error handling for impossible scenarios. If 200 lines could be 50, rewrite it.
3. **Surgical Changes** — Touch only what you must. Match existing style. Don't refactor adjacent code. Every changed line must trace directly to the stated goal.
4. **Goal-Driven Execution** — Define verifiable success criteria before starting. Loop until verified.

**Anti-patterns to flag immediately:**
- Adding docstrings/type hints/formatting to code you weren't asked to change
- Creating abstractions (traits, interfaces, generic functions) for code used in exactly one place
- Adding "flexibility" nobody asked for
- Refactoring adjacent code "while you're in the area"
- Writing error handling for scenarios that can't happen

#### 2. `tdd` — HIGH (active for each structural change)

Matt Pocock's TDD methodology adapted for Rust. For each Phase 3–5 refactoring:

**Vertical tracer bullets, not horizontal slices:**

```
For each extraction (e.g., extracting GasBudgetManagement from CyberneticsLoop):
  RED:   Write test for the public interface of the new module → test fails
  GREEN: Move the code to the new module, wire it up → test passes
  REFACTOR: Clean up, remove duplication, deepen the interface
```

**Test philosophy:**
- **Good tests** exercise public interfaces, not implementation details. A test of `SetPoints::default()` verifies the values are correct — it doesn't care whether `SetPoints` lives in `cybernetics_loop.rs` or `set_points.rs`.
- **Bad tests** mock internal collaborators, test private methods, or verify through external means. If you rename an internal function and tests fail, those tests were testing implementation, not behavior.
- **Tests must survive refactors.** The existing 87 tests in `hkask-cns` and 59 tests in `hkask-agents` are the regression safety net. Every extraction must make existing tests pass before AND after the change.

**Rust conventions:**
- `#[cfg(test)] mod tests` for unit tests
- `tests/` directory for integration tests that exercise crate public APIs
- `#[tokio::test]` for async tests
- `tempfile` for filesystem tests — never write to project tree
- `assert!` with meaningful messages over bare `assert_eq!`
- Test error paths, not just happy paths
- **No `todo!()` or `unimplemented!()`** — return errors, don't panic

**TDD checklist per cycle:**
```
[ ] Test describes behavior, not implementation
[ ] Test uses public interface only (seam, not internals)
[ ] Test would survive internal refactor
[ ] Code is minimal for this test
[ ] No speculative features added
[ ] No todo!() or unimplemented!() stubs
[ ] cargo test -p <crate> passes
[ ] cargo clippy -p <crate> -- -D warnings passes
```

#### 3. `improve-codebase-architecture` — RECOMMENDED (use for Phase 3–5 decisions)

Before each structural extraction in Phases 3–5, apply the **deepening assessment**:

1. **Deletion test**: Imagine deleting the proposed module. If complexity vanishes, it was a pass-through. If complexity reappears across N callers, it earns its keep.
2. **Interface depth**: Is the proposed interface significantly smaller than its implementation? If the interface is nearly as complex as the code behind it, the module is shallow — reconsider.
3. **Seam test**: Two adapters = real seam. One adapter = hypothetical seam. Don't create seams for single-use cases (violates P1).
4. **Locality**: Does the extraction concentrate change-knowledge in one place? If callers still need to understand internals, locality hasn't improved.

Apply this especially to:
- **H9 (ApiState decomposition)**: Does grouping 5 sub-structs create real seams, or just reshuffle fields? If no adapter pattern exists (test vs production, mock vs real), the sub-structs may not be earning their keep.
- **H8 (CyberneticsLoop decomposition)**: Gas budget methods are called from both `CyberneticsLoop` and `GovernedTool`. That's two consumers — the extraction is justified. But directive handling methods are only called from within `CyberneticsLoop` itself — P1 says keep them inline unless a second consumer emerges.

---

### What's Been Done (Phases 1–3)

#### Phase 1.1 — FromSql/ToSql for Domain Types ✅
- `hkask-types/src/sql_impls.rs` behind `sql` feature flag
- `FromSql`/`ToSql` for 11 types; `DateTime<Utc>` stays as String (orphan rules)

#### Phase 1.2 — Macros and Store Refactoring ✅
- `collect_rows!` macro (two variants) + `impl_from_serde_json!` macro
- Applied to 8 store files

#### Phase 1.3 — ApiError Enum ✅
- `hkask-api/src/error.rs` — `ApiError` with `IntoResponse` + 15 `From` impls; all routes converted

#### Phase 1.4 — Error Propagation ✅
- `Stores::init()`, `ApiState::new/with_defaults/with_ensemble_inferencer()`, `build_loop_system()` all return `Result<_, ApiError>`

#### Phase 2 — Type System Strengthening ✅
- **H6 CuratorDirective** — 6-variant enum with helper methods; `LoopPayload::CurationDirective` is tuple variant
- **H7 SignalMetric** — 25-variant enum; `Signal::new` takes `SignalMetric` directly; bridge impls removed
- **H10 AccessControl** — Value type `{perspective, visibility, owner_webid}` with canonical constructors; `Triple` refactored
- **H10 TemporalBounds** — Value type `{valid_from, valid_to}` with `now()`, `is_current()`, `superseded()`; `Triple` refactored
- **M10–M14** — 30+ named constants extracted across CNS, loops, memory, and ensemble

#### Phase 3 — Structural Decomposition (Partial)
- **H8 (partial)** — Extracted `set_points.rs` module from `cybernetics_loop.rs`

---

### Remaining Work (With TDD Protocol)

#### Phase 3 — Complete Structural Decomposition

**H8: Further decompose CyberneticsLoop**

`cybernetics_loop.rs` is now ~1500 lines (down from 1720). Remaining targets:

1. **Gas budget management** (`register_gas_budget`, `can_proceed`, `agent_gas_status`, `reserve_gas`, `settle_gas`, `acquire_budget`, `replenish_all_budgets`, `replenish_agent_budget`):
   - **TDD approach**: These methods are called from `CyberneticsLoop` and `GovernedTool` — that's two consumers, justifying extraction.
   - **Tracer bullet**: Write a test that calls `CyberneticsLoop::register_gas_budget()` and `can_proceed()` through the public `HkaskLoop` trait — verify gas budget registration works. Move the methods to a `gas_budget_management` module. Run test. Runs green.
   - **Alternative considered**: Keep as `impl CyberneticsLoop` block within `cybernetics_loop.rs` but in a separate `impl` block for organizational clarity. This violates P1 if no second consumer exists, but we know `GovernedTool` calls gas budget methods directly.

2. **Directive handling** (`handle_curation_directive`, `apply_directive`, `apply_calibrate_threshold`, `apply_override_gas_budget`, `apply_clear_override`, `apply_replenish_budget`):
   - **TDD approach**: These are only called from within `CyberneticsLoop::process_inbox()` — single consumer. Per P1, **do not extract** into a separate module. Keep inline.
   - **Reconsider only if** a second consumer emerges (e.g., if `CuratorAgent` or `CurationLoop` begins calling `apply_directive` directly).

3. **HkaskLoop impl** (sense/compare/compute/act) — stays in `cybernetics_loop.rs`.

**H9: Decompose ApiState (20 fields)**

**Deepening assessment** (from `improve-codebase-architecture` skill):

`ApiState` has 20+ fields. The proposed sub-structs (`McpInfra`, `MemoryInfra`, `SessionInfra`, `GovernanceInfra`) would create seams. But do they have two adapters?

Current situation: All route handlers access `state.field_name` directly. Creating sub-structs means every handler changes to `state.mcp_infra.field_name`. That's 40+ call sites.

**Recommended approach**: Use **accessor methods** on `ApiState` that delegate to internal sub-structs. This way:
- Route handlers don't change (surgical changes principle)
- Sub-structs can be extracted without touching handlers
- The sub-structs are internal implementation details, not public API

```rust
// Before (route handler):
let runtime = &state.mcp_runtime;

// After accessor method approach:
let runtime = state.mcp_runtime(); // delegates to self.mcp_infra.mcp_runtime
```

This preserves P1 (two consumers: `ApiState` itself + the test infrastructure) and the surgical changes principle.

**TDD approach**:
1. Write a test for `ApiState::mcp_runtime()` accessor → GREEN
2. Create `McpInfra` sub-struct, move `mcp_runtime` field → existing test determines behavior
3. Add accessor method → test passes
4. Repeat for each sub-struct group
5. Refactor: remove direct field access from route handlers one at a time, running tests between each

**C4: Decompose REPL `run()` (1206 lines in `repl/mod.rs`)**

Low priority — REPL is not in the hot path. Extract `ReplSession` struct with methods only if there's a second consumer (e.g., test infrastructure needs to drive the REPL programmatically).

**H4: Extract `GovernedTool::invoke()` steps (180-line monolith)**

**TDD approach**:
1. Write a test for the full `GovernedTool::invoke()` pipeline (capability check → gas estimate → reserve → call → settle)
2. Extract `check_capability()` method → test still passes
3. Extract `estimate_and_reserve_gas()` → test still passes
4. Extract `execute_port_call()` → test still passes
5. Extract `settle_gas()` → test still passes
6. Verify all 87 tests in `hkask-cns` still pass

This is hot-path code. **Each extraction must preserve the exact gas-check → reserve → call → settle ordering.** No reordering. No new error paths that weren't there before.

#### Phase 4 — Cross-Cutting Deduplication

**TDD protocol**: For each deduplication, write a test that exercises the deduplicated path through the public API. Then make the change. Tests must pass before and after.

- **Error `From` chain unification** — Replace 5+ hand-written match blocks with `?` operator + blanket `From` impls. Write a test for each error conversion path before changing.
- **`MemoryInfrastructure` factory** — Extract repeated construction into `MemoryInfrastructure::new()`. Write a test that creates a `MemoryInfrastructure` and exercises each storage port.
- **`DepletionSignal::from_alert()`** — Write a test that creates a `DepletionSignal` from `CnsHealth` data. Then extract the named constructor.
- **`ConsolidationResult` ↔ `ConsolidationOutcome` unification** — Verify structural equivalence with a static assertion (`assert_eq!(size_of::<...>())`) or conversion test, then unify.
- **MCP server boot pattern** — Identify the common `serve()` boilerplate across `hkask-mcp-*` servers, extract to `hkask-mcp::serve()`. Write a test that boots a minimal MCP server.

#### Phase 5 — Graph Simplification

- **M3: Inline `McpGovernor`** — Verify only consumer is `GovernedTool`. If yes, inline. If other consumers exist, leave as-is.
- **KillZoneDetector → CnsRuntime** — Verify only called from `CnsRuntime`. If yes, fold. Write test for `CnsRuntime::detect_kill_zone()` or equivalent.
- **M16: Delete `prompt_decomposition`** — Verify with `grep -r prompt_decomposition crates/ --include="*.rs"`. If no consumers, delete module + lib.rs export. Run tests.
- **M15: Bot route stubs** — Identify stubs in `hkask-api/src/routes/pods.rs`. If they return `StatusCode::NOT_IMPLEMENTED`, either implement or delete.

#### Phase 6 — Naming & Constants

Most constants already extracted (M10–M14). Remaining:
- Any unnamed numbers in `repl/commands.rs`, ensemble config, etc.
- Use `improve-codebase-architecture` skill to assess: if a number appears only once, it's not a "magic number" — it's just a number with clear context. Only extract if ≥2 occurrences or clear semantic value.

---

### Key Design Decisions (Binding)

1. **FromSql in `hkask-types`** — Orphan rules prevent implementing a foreign trait for a foreign type. The `sql` feature flag keeps the dependency optional.
2. **DateTime stays as String in DB** — `DateTime<Utc>` from `chrono` can't have `FromSql`/`ToSql` because neither the type nor the trait is local.
3. **`collect_rows!` two-variant macro** — Form 1 (`mapper` only) for direct conversions. Form 2 (`mapper` + `convert`) for two-step conversions.
4. **`ApiError` preserves JSON shape** — Single `error: String` field. Old `ErrorResponse` had `error`, `code`, `details`.
5. **`SignalMetric` bridges removed** — All callers use enum variants directly. `From<&str>` and `PartialEq<&str>` were temporary and are gone.
6. **`AccessControl` and `TemporalBounds` are value types** — They replace individual fields in `Triple`, not tables. DB columns remain separate.
7. **`SetPoints` extracted to its own module** — `set_points.rs` is a `pub mod` in `hkask-cns`. External API re-exported from `lib.rs` unchanged.
8. **Coding Guidelines (Karpathy principles)** — `coding-guidelines` skill is **critical** for all remaining work. Specifically:
   - **Simplicity First**: No abstractions for single-use code. Only extract when ≥2 call sites warrant it.
   - **Surgical Changes**: Touch only what's needed. Don't refactor adjacent code.
   - **Goal-Driven Execution**: Define success criteria before starting. Verify after each step.
   - **Think Before Coding**: Surface assumptions. If multiple interpretations exist, pause and ask.
9. **No visual UI** — hKask is headless. No Grafana, dashboards, or web frontends.
10. **Stubs are deleted, not deprecated** — `todo!`, `unimplemented!`, `#[deprecated]` are violations.

---

### Suggested Skills for Continuation Agent

| Skill | Priority | Reason |
|-------|----------|--------|
| `coding-guidelines` | **Critical** | Must govern all refactoring. Surgical changes, simplicity first, no speculative abstractions. Invoke at the start of each extraction and verify after. |
| `tdd` | **High** | Red-green-refactor for each structural change. Write a test for the public interface before extracting. Never refactor while RED. |
| `improve-codebase-architecture` | **Recommended** | Deepening assessment before each extraction. Apply deletion test and seam test. Use especially for H9 (ApiState) and H8 (CyberneticsLoop) decisions. |
| `diagnose` | **Optional** | Use if bugs surface during refactoring. Build a feedback loop first, then bisect. |
| `zoom-out` | **Optional** | Use if you get lost in a file's weeds and need to map the module boundaries and caller graph. |

---

### Open Questions & Risks

| # | Question | Risk | TDD Guidance |
|---|----------|------|-------------|
| 1 | How far to decompose `CyberneticsLoop`? | Medium | Gas budget methods have 2 consumers (CyberneticsLoop + GovernedTool) → extract. Directive methods have 1 consumer → keep inline per P1. |
| 2 | ApiState decomposition strategy | High | Use accessor methods that delegate to sub-structs, so route handlers don't change. Write test for `state.mcp_runtime()` before extracting `McpInfra`. |
| 3 | REPL decomposition scope | Low | Only extract if a second consumer emerges (e.g., test harness). Not urgent. |
| 4 | `GovernedTool::invoke()` extraction risk | Medium | This is hot-path code. Write a full pipeline test FIRST. Each extraction step must preserve exact ordering. |
| 5 | `McpGovernor` inlining | Medium | Verify single consumer with grep before inlining. |
| 6 | `prompt_decomposition` deletion | Low | Verify no consumers with grep. Delete module + lib.rs export. |
| 7 | `ConsolidationResult` vs `ConsolidationOutcome` | Low | Write a conversion test + static size assertion before unifying. |

---

### Approach Guidance for Remaining Phases

**Per-extraction TDD cycle:**

```
For each Phase 3–5 task:
  1. [coding-guidelines] Think: What's the minimal change? Is there a second consumer? (P1)
  2. [tdd] Write a test for the PUBLIC INTERFACE of what you're about to change
  3. Run test → RED (if it's a new interface) or GREEN (if it tests existing behavior)
  4. Make the change (extract, deduplicate, inline, etc.)
  5. Run test → GREEN
  6. Run cargo test --workspace → all pass
  7. Run cargo clippy --workspace -- -D warnings → clean
  8. [tdd] Refactor if needed, tests stay green
  9. Commit
```

**For ApiState decomposition specifically:**

The `improve-codebase-architecture` skill's deepening assessment suggests accessor methods rather than direct sub-struct field access. This follows the "Surgical Changes" principle — route handlers don't change unless they need new functionality:

```rust
// BEFORE: direct field access (current)
pub struct ApiState {
    pub mcp_runtime: Arc<McpRuntime>,
    pub mcp_dispatcher: Arc<McpDispatcher>,
    // ... 18 more fields
}

// AFTER: sub-struct + accessor method (recommended)
pub struct ApiState {
    mcp_infra: McpInfra,       // private
    memory_infra: MemoryInfra,   // private
    // ...
}

impl ApiState {
    pub fn mcp_runtime(&self) -> &Arc<McpRuntime> {
        &self.mcp_infra.mcp_runtime
    }
    // ... accessor methods delegate to sub-structs
}
```

This preserves backward compatibility at the call site while allowing the internal structure to evolve.

---

### Verification Commands

```bash
# Full workspace check (must be clean)
cargo check --workspace

# Lint (must pass with -D warnings)
cargo clippy --workspace -- -D warnings

# Tests (must all pass)
cargo test --workspace

# Pattern violations (must return empty)
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"
grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"
```

### TDD Verification Per Extraction

```bash
# Per-craterun cycle:
cargo test -p <crate>              # Specific crate tests
cargo clippy -p <crate> -- -D warnings  # Lint
cargo check -p <crate>             # Type-check

# Full cycle after each major extraction:
cargo test --workspace              # All tests
cargo clippy --workspace -- -D warnings  # Full lint
```

---

### Key Files Modified Since `origin/main` (4 commits + working tree)

| File | Change |
|------|--------|
| `hkask-types/Cargo.toml` | Added `rusqlite` optional dep + `sql` feature |
| `hkask-types/src/lib.rs` | Added `sql_impls`, `AccessControl`, `TemporalBounds` exports |
| `hkask-types/src/sql_impls.rs` | **NEW** — FromSql/ToSql for 11 types |
| `hkask-types/src/loops/curation.rs` | **NEW** — `CuratorDirective` enum |
| `hkask-types/src/loops/dispatch.rs` | `LoopPayload::CurationDirective` → tuple variant |
| `hkask-types/src/loops/mod.rs` | `SignalMetric` expanded to 25 variants; bridges removed; `Signal::new` takes `SignalMetric` |
| `hkask-types/src/visibility.rs` | **NEW** additions — `AccessControl` and `TemporalBounds` value types |
| `hkask-storage/Cargo.toml` | `hkask-types` now has `features = ["sql"]` |
| `hkask-storage/src/store_macros.rs` | `collect_rows!` + `impl_from_serde_json!` macros |
| `hkask-storage/src/triples.rs` | Uses `AccessControl` + `TemporalBounds`; `collect_rows!` |
| `hkask-storage/src/*.rs` | Applied `collect_rows!` and `impl_from_serde_json!` to 7 files |
| `hkask-api/src/error.rs` | **NEW** — `ApiError` enum + `IntoResponse` + 15 `From` impls |
| `hkask-api/src/lib.rs` | Error propagation; `build_loop_system` returns `Result` |
| `hkask-api/src/routes/*.rs` | All converted to `ApiError` |
| `hkask-cns/src/cybernetics_loop.rs` | `apply_directive` pattern-matches `CuratorDirective`; `sense`/`compute` use `SignalMetric`; gas budget methods, directives still inline |
| `hkask-cns/src/set_points.rs` | **NEW** — Extracted from `cybernetics_loop.rs` |
| `hkask-cns/src/algedonic.rs` | Extracted 7 named constants |
| `hkask-cns/src/circuit_breaker.rs` | Extracted 3 named constants |
| `hkask-cns/src/energy.rs` | Extracted `DEFAULT_GAS_ALERT_THRESHOLD` |
| `hkask-cns/src/variety.rs` | Extracted `DEFAULT_VARIETY_WINDOW_SECS` |
| `hkask-cns/src/lib.rs` | Added `set_points` module; updated re-exports |
| `hkask-agents/src/curator/curation_loop.rs` | Uses `SignalMetric` enum variants |
| `hkask-agents/src/communication/communication_loop.rs` | Uses `SignalMetric::QueueDepth`, `RegisteredLoops` |
| `hkask-agents/src/inference_loop.rs` | Uses `SignalMetric::CircuitBreakerState` etc. |
| `hkask-agents/src/curator_agent/metacognition.rs` | Uses `SignalMetric` + extracted config constants |
| `hkask-agents/src/loop_system.rs` | Extracted tick interval constants |
| `hkask-agents/src/adapters/memory_loop_adapter.rs` | Uses `AccessControl` value type |
| `hkask-memory/src/triples.rs` | Uses `AccessControl` + `TemporalBounds` |
| `hkask-memory/src/{episodic,semantic,consolidation}.rs` | Updated `Triple` field access |
| `hkask-memory/src/semantic_loop.rs` | Extracted `DEFAULT_SEMANTIC_STORAGE_BUDGET` |
| `hkask-memory/src/episodic_loop.rs` | Uses `SignalMetric::StorageUsage`, `DecayRate` |
| `hkask-ensemble/src/chat.rs` | Extracted `GasBudgetConfig` constants |
| `hkask-ensemble/src/standing_session.rs` | Uses `GasBudgetConfig` constants |

---

*ℏKask — A Minimal Viable Container for Agents — v0.22.0*