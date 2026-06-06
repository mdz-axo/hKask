# hKask Fowler Refactoring Audit — Continuation Prompt

## Session Purpose

Execute Martin Fowler's refactoring audit for the hKask codebase. The full audit document is in the conversation history (45 findings across 5 severity levels, 6-phase execution plan). This document captures the exact state so a fresh agent can continue.

---

## Completed Work

### Phase 1: Foundation Refactorings ✅ COMPLETE

**1.1 — FromSql/ToSql for Domain Types [C3]** ✅
- `hkask-types/src/sql_impls.rs` already had FromSql/ToSql impls for WebID, Id<T> types, Visibility, GoalState, AgentKind (behind `sql` feature)
- **Migrated stores** to use direct `row.get::<_, Type>(N)?` instead of `String` + `FromStr`:
  - `crates/hkask-storage/src/goals.rs` — ~20 manual parse sites replaced
  - `crates/hkask-storage/src/user_store.rs` — ~10 manual parse sites replaced
  - `crates/hkask-storage/src/nu_event_store.rs` — EventID, WebID, Option<EventID>, Visibility migrated
  - `crates/hkask-storage/src/agent_registry.rs` — `kind.as_str()` → `kind` in SQL params
- `triples.rs` was already migrated before this session
- `DateTime<Utc>` does NOT have FromSql (chrono feature not enabled for rusqlite) — timestamps remain String → manual parse

**1.2 — collect_rows! and define_store_error! macros [C1/C2]** ✅ Already existed
- `crates/hkask-storage/src/store_macros.rs` already has: `define_store!`, `impl_from_rusqlite!`, `impl_from_serde_json!`, `collect_rows!`
- `triples.rs` already uses `collect_rows!` and `row_to_triple_row()`

**1.3 — ApiError Enum [C5]** ✅ COMPLETE
- Created `crates/hkask-api/src/error.rs` with `ApiError` enum (NotFound, Unauthorized, Forbidden, BadRequest, Conflict, RateLimited, Internal) + `IntoResponse` impl
- Added `From` impls for: GoalRepositoryError, MemoryError, ConsentError, EscalationError, GitError, InfrastructureError
- Migrated 3 route files: `goal.rs`, `episodic.rs`, `consolidation.rs`
- Removed dead `error_response()` helper from `routes/mod.rs`
- **Remaining**: 10+ other route files still use `(StatusCode, Json<ErrorResponse>)` — can be migrated incrementally

**1.4 — row_to_triple_row() [H1]** ✅ Already existed

---

### Phase 2: Type System Strengthening — IN PROGRESS

**2.1 — CuratorDirective enum replacing string dispatch [H6/H7]** ✅ COMPLETE (committed in `8e1fc2a38`)
- Added `ClearOverride { agent: WebID }` variant to `CuratorDirective` in `crates/hkask-types/src/loops/curation.rs`
- Added helper methods: `variant_name()`, `agent_target()`, `is_metacognitive()`
- Changed `LoopPayload::CurationDirective` from `{ directive_type: String, target: WebID, parameters: Value }` to `CurationDirective(CuratorDirective)` in `crates/hkask-types/src/loops/dispatch.rs`
- Updated `apply_directive()` in `crates/hkask-cns/src/cybernetics_loop.rs` to match on enum variants
- Updated `Dampener` in `crates/hkask-cns/src/dampener.rs` to use `CuratorDirective` instead of strings
- Updated `send_curator_directive()` in `crates/hkask-agents/src/communication/dispatch.rs` to pass enum directly
- 87 CNS tests pass including 8 dampener tests

**2.2 — SignalMetric enum replacing string metrics [H7]** 🔴 IN PROGRESS — BLOCKED

**Current state (UNCOMMITTED, in working tree):**
- `SignalMetric` enum exists in `crates/hkask-types/src/loops/mod.rs` with 9 variants:
  - EnergyRemaining, VarietyDeficit, ErrorRate, ConnectorLatency, CommunicationQueueDepth (original CNS metrics)
  - StorageUsage, DecayRate (Episodic Loop)
  - TripleCount, LowConfidenceCount (Semantic Loop)
- Has `as_str()`, `Display`, `From<&str>`, `PartialEq<&str>` impls
- `Signal.metric` field changed from `String` to `SignalMetric`
- `Signal::new()` changed from `metric: impl Into<SignalMetric>` to `metric: SignalMetric` (no more silent string conversion)
- CNS `cybernetics_loop.rs` fully migrated: `sense()`, `compute()`, and all test `Signal::new()` calls use enum variants

**BLOCKER: `hkask-agents` and `hkask-memory` don't compile** because they use string metric names NOT in the current `SignalMetric` enum:

| File | String metrics used | Need new variants? |
|------|---------------------|-------------------|
| `hkask-agents/src/curator/curation_loop.rs` | `"algedonic_events"`, `"pending_escalations"`, `"consolidation_candidates"`, `"goal_stale_count"`, `"goal_expired_count"`, `"spec_drift_alert_count"` + `metric.as_str()` match | Yes — 6 new variants |
| `hkask-agents/src/curator_agent/metacognition.rs` | `"metacognition_variety_deficit"`, `"metacognition_critical_alerts"`, `"metacognition_bot_failures"` + `metric.as_str()` match | Yes — 3 new variants |
| `hkask-agents/src/communication/tool_dispatch.rs` | `"tool_dispatch_queue_depth"` | Yes — 1 new variant |
| `hkask-agents/src/communication/communication_loop.rs` | `"queue_depth"`, `"registered_loops"` | Yes — 2 new variants |
| `hkask-agents/src/inference_loop.rs` | `"circuit_breaker_state"`, `"inference_available"`, `"inference_gas_remaining"`, `"inference_model_available"` + `metric.as_str()` match | Yes — 4 new variants |
| `hkask-memory/src/episodic_loop.rs` | `"storage_usage"`, `"decay_rate"` + `metric == "storage_usage"` | Already in enum! Just needs `Signal::new()` call sites updated |
| `hkask-memory/src/semantic_loop.rs` | `"triple_count"`, `"low_confidence_count"` + `metric.as_str()` match | Already in enum! Just needs call sites updated |

**To complete 2.2:**
1. Add 16 new `SignalMetric` variants to cover the above metrics
2. Add corresponding `as_str()` match arms and `From<&str>` match arms
3. Update all `Signal::new()` call sites in the above files to use enum variants
4. Update all `metric.as_str()` and `metric == "..."` comparisons to use enum matching
5. Run `cargo check` and `cargo test` for all affected crates

**OR** consider an alternative design: since `SignalMetric` is growing large (25+ variants) and metrics are loop-specific, consider making `Signal.metric` generic or having per-loop metric enums. But the simplest fix is to just add all variants — they're all known at compile time.

---

### Phases 2.3–2.6 + Phases 3–6: NOT STARTED

**2.3 — Extract AccessControl and TemporalBounds value types [H10]**
- `perspective: Option<WebID>`, `visibility: Visibility`, `owner_webid: WebID` always appear together in `Triple`
- Plan: Create `struct AccessControl { perspective, visibility, owner_webid }` in `hkask-types`
- Add `Triple::to_semantic(&self) -> Triple` that sets `perspective: None, visibility: Shared`

**2.4 — Extract Confidence newtype [Finding from storage/memory]**
- Bare `f64` confidence everywhere — should be `struct Confidence(f64)` with clamping to [0.0, 1.0]
- Add `fn decay(&self, rate: f64, time: f64) -> Confidence`

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
- `cargo check -p hkask-types --features sql` ✅
- `cargo check -p hkask-storage` ✅
- `cargo check -p hkask-cns` ✅
- `cargo check -p hkask-api` ✅
- `cargo check -p hkask-cli` ✅
- `cargo check -p hkask-keystore` ✅
- `cargo check -p hkask-mcp` ✅
- `cargo check -p hkask-templates` ✅
- `cargo check -p hkask-ensemble` ✅

### Does NOT Compile ❌ (blocked on Phase 2.2)
- `cargo check -p hkask-agents` ❌ — Signal::new() expects SignalMetric, got &str
- `cargo check -p hkask-memory` ❌ — Signal::new() expects SignalMetric, got &str

### Tests Pass ✅ (for compiling crates)
- `cargo test -p hkask-types --features sql` — 51/51 ✅
- `cargo test -p hkask-storage` — 21/21 ✅
- `cargo test -p hkask-cns` — 87/87 ✅

### Uncommitted Changes
```
M crates/hkask-cns/src/cybernetics_loop.rs  (SignalMetric enum migration for CNS)
M crates/hkask-types/src/loops/mod.rs       (4 new SignalMetric variants, PartialEq<&str>, Signal::new() direct)
```

---

## Key Files Reference

| File | Role |
|------|------|
| `crates/hkask-types/src/loops/mod.rs` | SignalMetric enum, Signal struct, Loop trait, ActionType |
| `crates/hkask-types/src/loops/curation.rs` | CuratorDirective enum, CuratorHandle |
| `crates/hkask-types/src/loops/dispatch.rs` | LoopPayload, LoopMessage, DispatchTarget |
| `crates/hkask-types/src/sql_impls.rs` | FromSql/ToSql impls for domain types |
| `crates/hkask-cns/src/cybernetics_loop.rs` | CyberneticsLoop (1681 lines) — Phase 3 target |
| `crates/hkask-cns/src/dampener.rs` | Dampener — now uses CuratorDirective |
| `crates/hkask-cns/src/governed_tool.rs` | GovernedTool::invoke() — Phase 3.4 target |
| `crates/hkask-api/src/error.rs` | ApiError enum — Phase 1.3 deliverable |
| `crates/hkask-storage/src/store_macros.rs` | define_store!, impl_from_rusqlite!, collect_rows! |
| `crates/hkask-cli/src/repl/mod.rs` | REPL run() (1206 lines) — Phase 3.3 target |
| `crates/hkask-api/src/lib.rs` | ApiState (20 fields) — Phase 3.2 target |

---

## Immediate Next Steps (in order)

1. **Fix Phase 2.2 blocker**: Add 16 missing SignalMetric variants + migrate all call sites in hkask-agents and hkask-memory. See the table above for exact metrics per file.
2. **Commit the working tree** after 2.2 is complete and all crates compile.
3. **Phase 2.3**: Extract `AccessControl` struct from `Triple`'s perspective/visibility/owner_webid fields.
4. **Phase 2.4**: Extract `Confidence` newtype.
5. **Phase 3**: Structural decomposition (CyberneticsLoop, ApiState, REPL run(), GovernedTool::invoke).
6. **Phases 4–6**: Per the audit plan.