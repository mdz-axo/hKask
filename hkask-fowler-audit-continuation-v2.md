# hKask Fowler Refactoring Audit — Continuation Prompt v2

> **Read `hkask-fowler-audit-continuation.md` first.** That file has the full state of Phases 1–2 (all complete) including verification, design decisions, key files, and test results. This file adds context for the next agent.

## What Has Changed Since the Previous Continuation Doc

The previous continuation doc was written during Phase 2.2 when `hkask-agents` and `hkask-memory` didn't compile. Since then:

1. **Phase 2.2 (SignalMetric)** — completed and committed
2. **Phase 2.3 (AccessControl)** — was already done before this session
3. **Phase 2.4 (Confidence newtype)** — completed in this session

**All of Phase 2 is done.** The working tree has only 2 minor uncommitted CNS files from the 2.2 CuratorDirective cleanup (import reorganization, `OverrideRecord` moved to `gas_budget_management.rs`).

### Verification (as of this writing)

```
✅ cargo check — all 11 crates compile, 0 errors, 0 warnings
✅ cargo clippy -D warnings — clean on all touched crates
✅ cargo test — 210 tests pass (52 types + 21 storage + 3 memory + 87 CNS + 2 agents + doc tests)
✅ git diff --stat HEAD — only 6 files changed (2 CNS + continuation doc + 3 unrelated)
```

---

## Phase 3: Structural Decomposition — What You'll Find

The audit plan says "decompose X" for each target. Before you start cutting, **zoom out first** (use the `zoom-out` skill). Some decomposition has already happened organically:

### 3.1 — CyberneticsLoop (currently 1365 lines, down from 1681)

**Already extracted:**
- `gas_budget_management.rs` (431 lines) — `GasBudgetManager`, `OverrideRecord`
- `set_points.rs` (209 lines) — `SetPoints`, `SetPointsConfig`, `CurationThresholdConfig`
- `energy.rs` (484 lines) — `GasBudget`, `AgentGasStatus`, `GasError`
- `dampener.rs` (454 lines) — `Dampener` with `CuratorDirective`-based override cooldown
- `circuit_breaker.rs` (6225 bytes) — `CircuitBreaker`
- `algedonic.rs` (15604 bytes) — algedonic alert infrastructure
- `variety.rs` (2892 bytes) — variety counting

**What remains in `cybernetics_loop.rs` (1365 lines):**
- `CyberneticsLoop` struct (8 fields)
- Constructor + builder methods (~60 lines)
- `sense()` — produces signals from gas, variety, queue depth (~50 lines)
- `compare()` — delegates to `Deviation::from_signal` (~5 lines)
- `compute()` — match on `SignalMetric` variants to produce `LoopAction`s (~100 lines)
- `act()` — dispatch via `LoopMessage` (~30 lines)
- `process_inbox()` — handle `LoopPayload` messages (~60 lines)
- `apply_directive()` — match on `CuratorDirective` variants (~80 lines)
- `replenish_all_budgets()`, `replenish_agent_budget()` — delegate to `GasBudgetManager`
- ~500 lines of integration tests

**Assessment:** At 1365 lines with 500+ being tests, the core logic is already well-decomposed. The remaining file is **not a high-priority decomposition target**. The extraction to modules has already happened. Further splitting would scatter a tightly-coupled sense→compare→compute→act cycle across files without clear benefit.

**Recommendation:** Skip 3.1 or do a minor cleanup only (extract tests to a separate `#[cfg(test)]` module file). The real win was the `GasBudgetManager` and `Dampener` extractions that already happened.

### 3.2 — ApiState (currently 20+ fields, 673 lines total in lib.rs)

**Current `ApiState` fields** (lines 176–221):
```rust
pub struct ApiState {
    pub registry: Arc<Mutex<SqliteRegistry>>,           // Template infra
    pub mcp_runtime: Arc<McpRuntime>,                   // MCP infra
    pub mcp_dispatcher: Arc<McpDispatcher>,              // MCP infra
    pub pod_manager: Arc<PodManager>,                    // Agent infra
    pub capability_checker: Arc<CapabilityChecker>,     // Governance infra
    pub system_webid: WebID,                             // Identity
    pub ensemble_inferencer: Option<Arc<...>>,          // Inference infra
    pub spec_store: Option<Arc<dyn SpecStore>>,          // Spec infra
    pub consent_manager: Arc<ConsentManager>,             // Sovereignty infra
    pub escalation_queue: Arc<EscalationQueue>,          // Curation infra
    pub git_cas: Arc<GitCasAdapter>,                     // MCP infra
    pub standing_sessions: Arc<RwLock<HashMap<...>>>,   // Session infra
    pub standing_session_store: Option<Arc<...>>,        // Session infra
    pub session_manager: Arc<RwLock<SessionManager>>,    // Session infra
    pub goal_repo: Arc<SqliteGoalRepository>,            // Goal infra
    pub loop_system: Arc<LoopSystem>,                    // CNS infra
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,  // Memory infra
    pub cns_runtime: Arc<CnsRuntime>,                    // CNS infra
    pub inference_port: Option<Arc<dyn InferencePort>>, // Inference infra
}
```

**Natural groupings:**
- **McpInfra** — `mcp_runtime`, `mcp_dispatcher`, `git_cas`
- **MemoryInfra** — `episodic_storage`, `consent_manager`, `goal_repo`
- **SessionInfra** — `standing_sessions`, `standing_session_store`, `session_manager`
- **GovernanceInfra** — `capability_checker`, `system_webid`, `escalation_queue`
- **InferenceInfra** — `ensemble_inferencer`, `inference_port`

**However:** `ApiState` is a data container passed to route handlers via Axum's `State`. Decomposing into sub-structs would require updating every route handler's signature. The audit finding was H-level (helpful, not critical). Consider whether the grouping adds real clarity or just adds indirection.

**Recommendation:** If you proceed, create the sub-structs but keep `ApiState` as the single `State` type with convenience accessor methods. Don't change route handler signatures.

### 3.3 — REPL `run()` (1206 lines)

The `run()` function in `crates/hkask-cli/src/repl/mod.rs` starts at line 166. It's a monolithic REPL loop with inline chat handling, model switching, command parsing, and memory storage. The audit recommends extracting a `ReplSession` struct with methods.

### 3.4 — GovernedTool::invoke() (728 lines in governed_tool.rs)

The `invoke()` method is the core of the OCAP membrane. The audit recommends extracting:
- `check_capability()` — verify the token grants access
- `reserve_gas()` — reserve from the gas budget
- `dispatch()` — call the underlying tool
- `settle()` — settle the gas reservation
- `emit_outcome()` — emit the CNS span

---

## Skills to Use

From `.agents/skills/`:
- **coding-guidelines** — Surgical changes. Don't refactor adjacent code. Every changed line must trace to the task.
- **tdd** — Write tests before refactoring. Verify each extraction doesn't break behavior.
- **improve-codebase-architecture** — Use the "deletion test" before extracting: "If I delete this module, does complexity vanish or reappear?"
- **zoom-out** — Before decomposing, map the module boundaries and data flow.

---

## Key Constraints (from AGENTS.md)

- **No visual UI** — CLI/MCP/API only. No dashboards.
- **No monitoring stacks** — CNS provides programmatic observability.
- **No excess complexity** — No unused traits, stubs, `todo!()`, `unimplemented!()`, deprecated features.
- **Violations get deleted.** See `docs/architecture/PRINCIPLES.md`.

---

## Practical Tips for Phase 3

1. **Start with 3.4 (GovernedTool)** — smallest scope, clearest extraction, best TDD opportunity.
2. **Then 3.3 (REPL run)** — extract `ReplSession` struct, move loop state into it, methods for each command.
3. **Then 3.2 (ApiState)** — only if the grouping adds real clarity. Consider skipping if it just adds layers.
4. **3.1 (CyberneticsLoop)** — likely not worth further decomposition. The 1365-line file has 500+ lines of tests and the core logic is already spread across extracted modules.

---

## Uncommitted Files (commit before Phase 3)

```
M crates/hkask-cns/src/cybernetics_loop.rs  (import cleanup, OverrideRecord extraction from Phase 2.2)
M crates/hkask-cns/src/lib.rs               (gas_budget_management re-export)
M crates/hkask-cns/src/gas_budget_management.rs (OverrideRecord moved here)
M docs/architecture/DDMVSS.md               (unrelated documentation)
M mcp-servers/hkask-mcp-spec/src/types.rs   (unrelated MCP types)
```

These should be committed before starting Phase 3 to keep the working tree clean.