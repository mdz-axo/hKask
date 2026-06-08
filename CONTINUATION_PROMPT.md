# Continuation Prompt — hKask Service Layer, Session 12

## 0. Mandatory Pre-Work

**Read these two files in order before writing any code:**

1. **`HANDOFF.md`** — Full project state across 11 sessions, 46 key decisions, 5 service modules (InferenceService, CuratorService, EnsembleService, PodService, SovereigntyService), both surfaces composing `ServiceContext::build()`, 51 tests passing. Section 6 reflects completed Session 11 work (F9/F10/F7 all closed). Section 9 has full architectural context for all service modules.

2. **`OPEN_QUESTIONS.md`** — Current status of all open questions. F9/F10/F7 are CLOSED. Remaining MEDIUM items: F2 (session lifecycle), F3 (unified auth), F6 (REPL vs API boundary), F14 (dual error mapping), F17 (CuratorService standalone DB), F18/F19 (EnsembleService standing/improv extraction). All require design work before code.

**Mandatory skills:** Load ALL before any code changes:

1. `refactor-service-layer` — strangler fig, deletion test, depth test
2. `coding-guidelines` — surgical changes, simplicity first
3. `tdd` — RED→GREEN→REFACTOR per behavior
4. `constraint-forces` — classify every decision by force type
5. `zoom-out` — module map before cross-cutting changes
6. `improve-codebase-architecture` — depth test, deletion test for proposals
7. `diagnose` — disciplined diagnosis loop if regressions occur
8. `handoff` — capture state at session end

---

## 1. Session Context

**Eleven sessions** have completed the 9-task service layer extraction plan plus 3 post-extraction open questions. The service layer is mature: 5 modules extracted, 4 skipped via depth test, both surfaces composing `ServiceContext::build()`, 51 tests passing, workspace green.

Session 11 closed all three prioritized post-extraction questions:
- **F9 (HIGH)** — Production memory stores now respect `config.in_memory` (P1 User Sovereignty satisfied)
- **F10 (MEDIUM)** — `#[non_exhaustive]` on `ServiceContext`; sub-struct grouping rejected by depth test
- **F7 (MEDIUM)** — Default constants centralized; env-var reads audited

The service layer extraction is **fully complete**. Remaining work is architectural refinement and deferred open questions that require design before code.

---

## 2. What Was Completed Through Session 11

### 9-Task Service Layer Extraction — COMPLETE

All 9 tasks done: 5 service modules extracted (InferenceService, CuratorService, EnsembleService, PodService, SovereigntyService), 4 skipped via depth test (Memory, Spec, Goal, Models), surface assembly migrated to `ServiceContext::build()`, CAS dead code removed, secret resolution audited, error mapping unified, documentation updated.

### Post-Extraction Questions — ALL CLOSED

- **F9** — Memory persistence respects `config.in_memory`
- **F10** — `#[non_exhaustive]` prevents external `ServiceContext` construction
- **F7** — `DEFAULT_DB_PATH`/`DEFAULT_OKAPI_BASE_URL` public, re-exported, used by all call sites

---

## 3. Current State of ServiceConfig

`ServiceConfig` in `crates/hkask-services/src/config.rs` has 16 fields:

```rust
pub struct ServiceConfig {
    pub db_path: String,                    // Primary DB (hkask.db)
    pub db_passphrase: String,              // SQLCipher passphrase
    pub acp_secret: Vec<u8>,                // ACP HMAC key
    pub mcp_secret: Vec<u8>,                // MCP dispatch key
    pub okapi_base_url: String,             // Okapi server URL
    pub cns_threshold: u64,                 // CNS variety threshold
    pub gas_budget_cap: u64,                // Gas cap per session
    pub gas_replenish_rate: u64,            // Gas per turn
    pub in_memory: bool,                    // In-memory mode flag
    pub default_model: String,              // Default inference model
    pub gate_model: String,                 // HHH gate model
    pub agent_name: String,                 // Agent identity
    pub template_cache_path: String,        // Git CAS cache
    pub memory_db_path: Option<String>,     // Memory DB path (F9)
    pub memory_passphrase: Option<String>,  // Memory DB passphrase (F9)
}
```

Public constants: `DEFAULT_DB_PATH`, `DEFAULT_OKAPI_BASE_URL`
Helper: `effective_memory_db_path()` — derives `{db_path}-memory.db` when `in_memory: false`

---

## 4. Current State of ServiceContext

`ServiceContext::build()` in `crates/hkask-services/src/context.rs` has 20 fields, now `#[non_exhaustive]`:

```rust
#[non_exhaustive]
pub struct ServiceContext {
    pub registry: Arc<Mutex<SqliteRegistry>>,
    pub mcp_runtime: Arc<McpRuntime>,
    pub mcp_dispatcher: Arc<McpDispatcher>,
    pub cns_runtime: Arc<RwLock<CnsRuntime>>,
    pub cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,
    pub loop_system: Arc<LoopSystem>,
    pub dispatch: Arc<MessageDispatch>,
    pub inference_port: Option<Arc<dyn InferencePort>>,
    pub episodic_storage: Arc<dyn EpisodicStoragePort>,
    pub semantic_storage: Arc<dyn SemanticStoragePort>,
    pub escalation_queue: Arc<EscalationQueue>,
    pub consent_manager: Arc<ConsentManager>,
    pub goal_repo: Arc<SqliteGoalRepository>,
    pub pod_manager: Arc<PodManager>,
    pub capability_checker: Arc<CapabilityChecker>,
    pub system_webid: WebID,
    pub event_sink: Arc<dyn NuEventSink>,
    pub standing_session_store: Arc<StandingSessionStore>,
    pub session_manager: Arc<RwLock<SessionManager>>,
    pub config: ServiceConfig,
}
```

All 5 context types derive via `From<&ServiceContext>`:

| Context | Derivation | Limitation |
|---------|-----------|------------|
| `InferenceContext` | `ctx.inference_port`, `ctx.config.default_model`, `ctx.config.okapi_base_url` | None |
| `PodContext` | `ctx.pod_manager.clone()` | None |
| `SovereigntyContext` | `ctx.consent_manager.clone()` | None |
| `CuratorContext` | `ctx.escalation_queue`, `cns_runtime: None`, `dispatch: Some(ctx.dispatch)` | Escalation-only; use `from_service_context(ctx).await` for full |
| `EnsembleContext` | `ctx.session_manager.clone()` | None |

---

## 5. What Remains — Open Questions

All HIGH-priority items are closed. Remaining items are MEDIUM and below, organized by actionability.

### Tier 1 — Requires design before code (MEDIUM)

| ID | Question | Design Prerequisite | Strategy |
|----|----------|---------------------|----------|
| F2 | Session lifecycle across surfaces | Specify durability semantics (when does a session start/end? what persists across restart?) | Zoom out on session creation paths in CLI + API; write durability spec; then implement |
| F3 | Unified authentication context | Define `AuthContext` struct in services; determine what fields both surfaces need | Audit auth paths in CLI (`repl/`) + API (`routes/auth.rs`); propose `AuthContext` with depth test |
| F6 | REPL vs API state boundary | Write boundary table: which state is shared vs surface-specific | Zoom out on `ReplState` vs `ApiState`; map each field to shared/surface; document |
| F17 | CuratorService standalone commands open DB each time | Decide: wire through `ServiceContext::build()` or document independence | Audit `kask curator` standalone commands; measure cost of forcing `ServiceContext` vs keeping per-invocation DB opens |
| F18 | EnsembleService standing session extraction | Design surface-specific adapter for CLI (YAML) vs API (JSON+MCP+gas) | Apply refactor-service-layer Phase 1 audit; if divergence too high, document why extraction is blocked |
| F19 | EnsembleService improv operation extraction | Design inferencer abstraction that works for both CLI (global static) and API (`ApiState.ensemble_inferencer_with_breaker()`) | Same as F18 — audit first, extract only if depth test passes |

### Tier 2 — Partially addressed, track for completion (MEDIUM)

| ID | Question | Current State | Next Action |
|----|----------|---------------|------------|
| F14 | Dual error mapping in API | 3 sovereignty routes fixed; ~11 direct `ApiError::` constructions remain (legitimate surface concerns) | Verify no new direct constructions appear; consider `ApiError::from` for any that have `ServiceError` paths |
| F22 | `SovereigntyBoundaryStore` reads in CLI Status | Per-user boundary data from persisted store; service returns default boundary; surface merges | Document the merge pattern; consider a `get_merged_status` service method if both surfaces need it |

### Tier 3 — Track only (LOW)

| ID | Question |
|----|----------|
| F1 | Streaming response support |
| F4 | MCP server service access (by design) |
| F8 | GovernedTool membrane boundary |
| F11 | InvalidPassphrase vs LoginFailed security |
| F12 | ValidationError(String) too generic |
| F16 | Embedding concern separation |

---

## 6. Constraint Forces

| Constraint | Force | Implication |
|-----------|-------|-------------|
| MCP servers do NOT depend on `hkask-services` | Prohibition (P1) | Never modify `mcp-servers/` to import from services |
| OCAP gates stay in domain crates / surfaces | Prohibition (P1) | Service layer never decides access |
| Standalone CLI commands work without `ServiceContext` | Prohibition (P1) | Don't force `ServiceContext::build()` just for `db_path` |
| ServiceContext is `#[non_exhaustive]` | Guardrail (F10) | External crates can't construct it; only `ServiceContext::build()` |
| Memory stores respect `config.in_memory` | Guardrail (F9/P1) | File-backed DB when false, in-memory when true |
| No `todo!` or `unimplemented!` in `hkask-services` | Prohibition (P6/P7) | Write real code or return errors |
| Dependency direction: CLI/API → services → domain | Guideline (P12) | Never the reverse |
| New `ServiceContext` fields need depth justification | Guardrail (F10) | 8+ consumer sites or compelling alternative |
| One domain per commit | Guideline (P5) | One task at a time |

---

## 7. Key Files to Read First

Read these IN ORDER before writing any code:

1. **`HANDOFF.md`** — Authoritative project state, 46 key decisions, architectural context for all 5 service modules
2. **`OPEN_QUESTIONS.md`** — Full F1–F26 status, priorities, next-actions
3. **`crates/hkask-services/src/context.rs`** — ServiceContext::build() — the heart of the service layer
4. **`crates/hkask-services/src/config.rs`** — ServiceConfig — where all config fields live
5. **`crates/hkask-services/src/error.rs`** — ServiceError hierarchy
6. **`docs/architecture/hKask-architecture-master.md`** — Architecture master doc

---

## 8. Recommended Task Sequence

The service layer extraction is complete. Session 12 should focus on **design work** for Tier 1 items, not code changes. The right approach is: audit → propose → discuss → implement.

### Task 1 — Audit F2/F3/F6/F17/F18/F19 (design prerequisites)

For each Tier 1 question:
1. Zoom out on the relevant code paths
2. Produce an audit (duplicated logic, divergence, depth test result)
3. Write a proposal in `OPEN_QUESTIONS.md` or a new RFC document
4. Only proceed to code if the depth test passes

**Priority order for audits:**
1. **F17** — Simplest; standalone curator commands opening DB is a measurable waste. Audit cost/benefit of wiring through `ServiceContext::build()`.
2. **F3** — Auth context is a bounded concern; audit auth paths in both surfaces.
3. **F2** — Session lifecycle spans the most code; needs durability semantics spec before any extraction.
4. **F6** — REPL vs API boundary is documentation work; low risk, high clarity value.
5. **F18/F19** — These were explicitly deferred in Sessions 5–6 due to divergence. Re-audit to see if anything changed.

### Task 2 — If any audit reveals extraction opportunity

Follow the refactor-service-layer skill:
1. Phase 0 — Zoom out
2. Phase 1 — Audit and classify (apply depth test)
3. Phase 2 — Classify constraint forces
4. Phase 3 — Design the service module
5. Phase 4 — RED/GREEN/Wire/Verify (one tracer bullet at a time)

### Task 3 — Update documentation

After each audit or code change:
- Update `OPEN_QUESTIONS.md` (status, next action, or closure)
- Update `HANDOFF.md` Section 5 (key decisions) and Section 6 (what remains)
- Update test counts in Section 4

---

## 9. Key Decisions to Preserve (46 Total)

Read `HANDOFF.md` Section 5 for the full list. Most critical for this session:

| # | Decision | Force | Impact |
|---|----------|-------|--------|
| 2 | `ServiceContext::build()` is async | Guideline | All callers `.await` it |
| 12 | Dependency direction: CLI/API → services → domain | Guideline | Never the reverse |
| 39 | Surface code uses ServiceContext for assembly | Prohibition | Both surfaces compose `ServiceContext::build()` |
| 44 | Memory stores respect `config.in_memory` | Guardrail (P1) | F9 CLOSED |
| 45 | `#[non_exhaustive]` on ServiceContext; no sub-structs | Guardrail | F10 CLOSED |
| 46 | Default constants centralized; env-var reads audited | Guideline | F7 CLOSED |

---

## 10. Anti-Patterns to Avoid

1. **Adding fields to ServiceContext without depth justification** — F10 flags 20 fields as a god-object risk. Any new field needs 8+ consumer sites or a compelling alternative.
2. **Breaking the build** — Every step must leave the workspace compiling and all tests passing.
3. **Restructuring ServiceContext without understanding all consumers** — 5 context types, 2 surfaces, and `build()`. A bad decomposition cascades.
4. **Forcing standalone CLI commands through ServiceContext** — P1: standalone commands should work without a full ServiceContext. Use parameters, not forced construction.
5. **Extracting without depth test** — Every proposed service module must pass the 8-call-site threshold or be documented as intentionally skipped.
6. **Horizontal slicing** — Don't audit everything then fix everything. One question → one audit → one proposal → verify.
7. **Adding speculative `From` impls** — Only add `From<ServiceError>` arms that are actually needed by call sites.
8. **Creating shallow sub-structs** — F10 analysis showed all proposed groupings are data-only containers with no behavior (shallow modules). Don't repeat this mistake.

---

## 11. Open Questions Requiring Attention

| ID | Question | Priority | Status |
|----|----------|----------|--------|
| F2 | Session lifecycle across surfaces | MEDIUM | Deferred — specify durability semantics first |
| F3 | Unified authentication context | MEDIUM | Deferred — define `AuthContext` struct |
| F6 | REPL vs API state boundary | MEDIUM | Deferred — write boundary table |
| F14 | Dual error mapping in API | MEDIUM | Partially addressed — remaining are legitimate |
| F17 | CuratorService standalone commands open DB each time | MEDIUM | Track — wire through `ServiceContext::build()` or document independence |
| F18 | EnsembleService standing session extraction | MEDIUM | Deferred — needs adapter design |
| F19 | EnsembleService improv operation extraction | MEDIUM | Deferred — needs inferencer abstraction |

---

## 12. Recommended Tools and Commands

```bash
# Verify current state before any change
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test -p hkask-services --lib
cargo test -p hkask-api

# After each step
cargo check -p hkask-services       # or hkask-cli, hkask-api
cargo test -p hkask-services --lib    # or relevant crate
cargo clippy -p hkask-services -- -D warnings

# Full verification after each task completion
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace

# Check for violations
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/hkask-services/src/ --include="*.rs"
grep -rn "hkask_services" mcp-servers/ --include="*.rs"  # Should find nothing

# ServiceContext consumer audit
grep -rn "ServiceContext" crates/ --include="*.rs" | grep -v "test" | grep -v "doc" | wc -l

# Auth path audit (for F3)
grep -rn "WebID\|capability_checker\|acp_secret\|CapabilityChecker" crates/hkask-cli/src/ crates/hkask-api/src/ --include="*.rs"

# Session lifecycle audit (for F2)
grep -rn "SessionManager\|session_manager\|standing_session" crates/hkask-cli/src/ crates/hkask-api/src/ --include="*.rs"
```

---

*ℏKask — A Minimal Viable Container for Agents — v0.23.0*