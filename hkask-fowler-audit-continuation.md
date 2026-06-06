# hKask Fowler Refactoring Audit — Continuation Prompt

## Session Purpose

Execute Martin Fowler's refactoring audit for the hKask codebase. The full audit document is in the conversation history (45 findings across 5 severity levels, 6-phase execution plan). This document captures the exact state so a fresh agent can continue.

---

## Completed Work

### Phase 1: Foundation Refactorings ✅ COMPLETE

**1.1 — FromSql/ToSql for Domain Types [C3]** ✅
- `hkask-types/src/sql_impls.rs` — FromSql/ToSql impls for WebID, Id<T> types, Visibility, GoalState, AgentKind, **Confidence** (behind `sql` feature)
- Migrated stores to use direct `row.get::<_, Type>(N)?` instead of `String` + `FromStr`

**1.2 — collect_rows! and define_store_error! macros [C1/C2]** ✅ Already existed

**1.3 — ApiError Enum [C5]** ✅ COMPLETE
- Created `crates/hkask-api/src/error.rs` with `ApiError` enum
- Migrated 3 route files, removed dead `error_response()` helper
- **Remaining**: 10+ other route files still use `(StatusCode, Json<ErrorResponse>)` — can be migrated incrementally

**1.4 — row_to_triple_row() [H1]** ✅ Already existed

---

### Phase 2: Type System Strengthening — ✅ COMPLETE

**2.1 — CuratorDirective enum replacing string dispatch [H6/H7]** ✅ COMPLETE (committed in `8e1fc2a38`)
- `CuratorDirective` enum in `hkask-types/src/loops/curation.rs`
- `LoopPayload::CurationDirective` changed from struct-with-strings to `CurationDirective(CuratorDirective)`
- CNS `cybernetics_loop.rs`, `dampener.rs`, `dispatch.rs` all use enum matching

**2.2 — SignalMetric enum replacing string metrics [H7]** ✅ COMPLETE
- 25 variants in `crates/hkask-types/src/loops/mod.rs`
- `Signal.metric` field changed from `String` to `SignalMetric`
- All call sites in `hkask-cns`, `hkask-agents`, `hkask-memory` migrated to enum variants

**2.3 — AccessControl struct replacing data clump [H10]** ✅ COMPLETE
- `struct AccessControl { perspective, visibility, owner_webid }` in `crates/hkask-types/src/visibility.rs`
- `Triple` uses `access: AccessControl` field
- `AccessControl::to_semantic()` strips perspective, sets visibility to Shared
- Canonical constructors: `new(owner)`, `episodic(perspective, owner)`, `semantic(owner)`

**2.4 — Confidence newtype replacing bare f64 [H8/H9]** ✅ COMPLETE
- `struct Confidence(f64)` in `crates/hkask-types/src/visibility.rs` with clamping to [0.0, 1.0]
- Methods: `new()`, `full()`, `zero()`, `value()`, `into_inner()`, `decay(rate, time)`
- `From<f64>` and `From<Confidence> for f64` conversions
- `Display` impl (formats to 4 decimal places)
- `FromSql`/`ToSql` impls behind `sql` feature flag
- `Triple.confidence` changed from `f64` to `Confidence`
- `Triple::with_confidence()` accepts `impl Into<Confidence>`
- `TripleStore::update()` accepts `impl Into<Confidence>` for new_confidence
- `TripleRow.confidence` changed from `f64` to `Confidence` (uses FromSql)
- `EpisodicStoragePort`/`SemanticStoragePort` traits use `Confidence` instead of `f64`
- `PodContext` methods accept `impl Into<Confidence>`
- `MemoryLoopAdapter::StorageRequest.confidence` changed to `Confidence`
- `bayesian::decay()` standalone function removed; replaced by `Confidence::decay()` method
- `bayesian.rs` now contains only `DEFAULT_DECAY_HALF_LIFE_SECS` and `DEFAULT_DECAY_RATE` constants + tests using `Confidence::decay()`
- All tracing calls use `%confidence` (Display format) instead of bare field logging
- Threshold parameters in SQL queries (`count_semantic_below_confidence(threshold: f64)`) remain `f64` — these are SQL query params, not domain values

---

### Phases 3–6: NOT STARTED

**Phase 3: Structural Decomposition**
- 3.1: Decompose `CyberneticsLoop` (1681 lines) → mod.rs + gas_budget.rs + directives.rs + set_points.rs
- 3.2: Decompose `ApiState` (20 fields) → McpInfra + MemoryInfra + SessionInfra + GovernanceInfra
- 3.3: Decompose REPL `run()` (1206 lines) → ReplSession struct with methods
- 3.4: Extract `GovernedTool::invoke()` steps (180 lines) → check_capability(), reserve_gas(), dispatch(), settle(), emit_outcome()

**Phase 4: Cross-Cutting Deduplication**
- 4.1: Unify error From impls (after define_store_error! macro)
- 4.2: Create MemoryInfrastructure factory
- 4.3: Extract DepletionSignal::from_alert()
- 4.4: Unify ConsolidationResult/ConsolidationOutcome
- 4.5: MCP server boot pattern deduplication

**Phase 5: Graph Simplification**
- 5.1: Replace McpGovernor with Arc<CapabilityChecker>
- 5.2: Fold KillZoneDetector into CnsRuntime
- 5.3: Delete prompt_decomposition module (it's just a relocation comment)
- 5.4: Implement or remove stub routes (bots.rs capabilities, spec.rs, cns.rs)
- 5.5: Unify ConsolidationService and ConsolidationBridge

**Phase 6: Naming & Constants Cleanup (N1–N14)**
- Extract magic numbers/strings into named constants across 14 locations

---

## Verification State

### Compiles Clean ✅
All 11 crates compile without errors or warnings:
- `cargo check -p hkask-types --features sql` ✅
- `cargo check -p hkask-storage` ✅
- `cargo check -p hkask-cns` ✅
- `cargo check -p hkask-memory` ✅
- `cargo check -p hkask-agents` ✅
- `cargo check -p hkask-api` ✅
- `cargo check -p hkask-cli` ✅
- `cargo check -p hkask-mcp` ✅
- `cargo check -p hkask-templates` ✅
- `cargo check -p hkask-ensemble` ✅
- `cargo check -p hkask-keystore` ✅

### Clippy Clean ✅
- `cargo clippy -p hkask-types --features sql -p hkask-storage -p hkask-memory -p hkask-cns -p hkask-agents -p hkask-api -p hkask-cli -- -D warnings` ✅

### Tests Pass ✅
- `cargo test -p hkask-types --features sql` — 52/52 ✅
- `cargo test -p hkask-storage` — 21/21 ✅
- `cargo test -p hkask-memory` — 3/3 ✅
- `cargo test -p hkask-cns` — 87/87 ✅
- `cargo test -p hkask-agents` — 2/2 ✅

### Uncommitted Changes
```
M crates/hkask-cns/src/cybernetics_loop.rs  (Phase 2.2 leftover: import cleanup, OverrideRecord removal)
M crates/hkask-cns/src/lib.rs               (Phase 2.2 leftover: gas_budget_management re-export)
```
These are minor cleanups from the CuratorDirective migration — the OverrideRecord struct was inlined into a sub-module. Should be committed before starting Phase 3.

---

## Key Files Reference

| File | Role |
|------|------|
| `crates/hkask-types/src/loops/mod.rs` | SignalMetric enum (25 variants), Signal struct, Loop trait, ActionType |
| `crates/hkask-types/src/loops/curation.rs` | CuratorDirective enum, CuratorHandle |
| `crates/hkask-types/src/loops/dispatch.rs` | LoopPayload, LoopMessage, DispatchTarget |
| `crates/hkask-types/src/visibility.rs` | AccessControl, **Confidence**, TemporalBounds, Visibility |
| `crates/hkask-types/src/sql_impls.rs` | FromSql/ToSql impls (including Confidence) |
| `crates/hkask-types/src/lib.rs` | Re-exports (Confidence added) |
| `crates/hkask-storage/src/triples.rs` | Triple (confidence: Confidence), TripleStore |
| `crates/hkask-memory/src/episodic.rs` | Uses Confidence::decay() instead of bayesian::decay() |
| `crates/hkask-memory/src/semantic_loop.rs` | Uses %confidence in tracing |
| `crates/hkask-memory/src/bayesian.rs` | Constants only (decay fn removed) |
| `crates/hkask-agents/src/ports/memory_storage.rs` | Port traits use Confidence |
| `crates/hkask-agents/src/adapters/memory_loop_adapter.rs` | StorageRequest.confidence: Confidence |
| `crates/hkask-agents/src/pod/context.rs` | store_episodic/store_semantic accept impl Into<Confidence> |
| `crates/hkask-api/src/routes/episodic.rs` | Confidence::new() for API confidence param |
| `crates/hkask-cli/src/commands/chat.rs` | Confidence::new(0.7) for CLI store |
| `crates/hkask-cns/src/cybernetics_loop.rs` | CyberneticsLoop (1681 lines) — Phase 3.1 target |
| `crates/hkask-cns/src/dampener.rs` | Dampener — uses CuratorDirective |
| `crates/hkask-cns/src/governed_tool.rs` | GovernedTool::invoke() — Phase 3.4 target |
| `crates/hkask-api/src/lib.rs` | ApiState (20 fields) — Phase 3.2 target |
| `crates/hkask-cli/src/repl/mod.rs` | REPL run() (1206 lines) — Phase 3.3 target |

---

## Design Decisions Made

### Confidence newtype design
- **Transparent newtype** with private inner `f64` — callers must use `value()` or `into_inner()` to get raw f64
- **Clamping on construction**: `Confidence::new(1.5) == Confidence(1.0)`, `Confidence::new(-0.3) == Confidence(0.0)`
- **`impl Into<Confidence>` on public APIs** (PodContext methods) so callers can pass `0.7_f64` or `Confidence::new(0.7)` interchangeably
- **`Confidence` on trait boundaries** (EpisodicStoragePort, SemanticStoragePort) — not `impl Into` since trait objects need concrete types
- **Threshold parameters remain `f64`** — SQL query thresholds are not domain confidence values; they're comparison parameters
- **bayesian::decay() removed** — superseded by `Confidence::decay()` method which also clamps the result

### Port trait design
- `EpisodicStoragePort.store_episodic(confidence: Confidence)` — concrete type for dyn compatibility
- `EpisodicStoragePort.store_episodic_classified(confidence_override: Option<Confidence>)` — consistent
- `SemanticStoragePort.store_semantic(confidence: Confidence)` — concrete type
- `PodContext.store_episodic(confidence: impl Into<Confidence>)` — ergonomic for callers
- `PodContext.store_semantic(confidence: impl Into<Confidence>)` — ergonomic for callers

---

## Skills Recommended for Next Agent

Use these skills from `.agents/skills/`:
- **coding-guidelines**: Enforce surgical changes, simplicity-first, no over-engineering
- **tdd**: Write tests before refactoring each Phase 3 decomposition
- **improve-codebase-architecture**: Find deepening opportunities as you decompose CyberneticsLoop, ApiState, etc.
- **zoom-out**: Before decomposing a 1600-line file, map its module boundaries first

---

## Immediate Next Steps (in order)

1. **Commit the 2 uncommitted files** (CNS import cleanup from Phase 2.2)
2. **Phase 3.1**: Decompose `CyberneticsLoop` (1681 lines in `crates/hkask-cns/src/cybernetics_loop.rs`)
   - Extract gas budget management → `gas_budget.rs`
   - Extract directive handling → `directives.rs`
   - Extract set-points → `set_points.rs` (may already exist as module)
   - Keep core sense→compare→compute→act cycle in `mod.rs`
   - **TDD approach**: Write tests first, then move code, then verify tests still pass
3. **Phase 3.2**: Decompose `ApiState` (20 fields in `crates/hkask-api/src/lib.rs`)
   - Group into: McpInfra, MemoryInfra, SessionInfra, GovernanceInfra
4. **Phase 3.3**: Decompose REPL `run()` (1206 lines in `crates/hkask-cli/src/repl/mod.rs`)
   - Extract ReplSession struct with methods
5. **Phase 3.4**: Extract `GovernedTool::invoke()` steps (180 lines)
   - check_capability(), reserve_gas(), dispatch(), settle(), emit_outcome()
6. **Phase 4**: Cross-cutting deduplication (after structural decomposition makes patterns visible)
7. **Phases 5–6**: Per the audit plan

---

*ℏKask - A Minimal Viable Container for Agents — v0.22.0*