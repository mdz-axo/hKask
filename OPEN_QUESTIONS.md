# Open Questions — hKask Service Layer Extraction

Questions raised during the service layer extraction (strangler fig pattern).
Each entry tracks priority, current status, and next action where applicable.

## Status Summary

| ID | Question | Priority | Status |
|----|----------|----------|--------|
| F1 | Streaming response support | LOW | Deferred |
| F2 | Session lifecycle across surfaces | MEDIUM | Deferred |
| F3 | Unified authentication context | MEDIUM | Deferred |
| F4 | MCP server service access | LOW | By design |
| F5 | Test seam depth for `ServiceContext::build()` | HIGH | CLOSED |
| F6 | REPL vs API state boundary | MEDIUM | Deferred |
| F7 | `ServiceConfig` vs environment variables | MEDIUM | CLOSED |
| F8 | `GovernedTool` membrane boundary | LOW | Deferred |
| F9 | Production memory stores use `in_memory_db()` | HIGH | CLOSED |
| F10 | `ServiceContext` approaching god-object (20 fields) | MEDIUM | CLOSED |
| F11 | `InvalidPassphrase` vs `LoginFailed` security | LOW | Track |
| F12 | `ValidationError(String)` too generic | LOW | Track |
| F13 | `CapabilityChecker` secret inconsistency | LOW | CLOSED |
| F14 | Dual error mapping in API | MEDIUM | Partially addressed |
| F15 | `InferenceContext` vs `ServiceContext` | LOW | CLOSED |
| F16 | Embedding concern separation | LOW | Track |
| F17 | `CuratorService` standalone commands open DB each time | MEDIUM | Track |
| F18 | `EnsembleService` standing session extraction | MEDIUM | Deferred |
| F19 | `EnsembleService` improv operation extraction | MEDIUM | Deferred |
| F20 | `EnsembleService` `list_deliberation_sessions` depth test | LOW | Closed |
| F21 | Memory domain depth test result | LOW | CLOSED |
| F22 | `SovereigntyBoundaryStore` reads in CLI Status | Guideline | Per-user boundary data from persisted store |
| F23 | Spec domain depth test result | LOW | CLOSED |
| F24 | Goal domain depth test result | LOW | CLOSED |
| F25 | Models domain depth test result | LOW | CLOSED |
| F26 | `ServiceContext` stores lack CAS write-through | MEDIUM | CLOSED |

---

## Closed Questions

### F5 — Test seam depth for `ServiceContext::build()`

**Status:** CLOSED — 3 API tests + 6 infrastructure tests prove `ServiceContext` produces valid state.

### F13 — CapabilityChecker secret inconsistency

**Status:** CLOSED — By design. 1 MCP secret, 2 ACP checkers, same pattern in both surfaces.

### F15 — InferenceContext vs ServiceContext

**Status:** CLOSED — Decided: lightweight `InferenceContext` for surfaces, `ServiceContext` for full composition.

### F21 — Memory domain depth test result

**Status:** CLOSED — Skipped. 2 call sites, P1 OCAP-gated.

### F23 — Spec domain depth test result

**Status:** CLOSED — Skipped. 4 call sites, API stubs.

### F24 — Goal domain depth test result

**Status:** CLOSED — Skipped. CRUD pass-throughs.

### F25 — Models domain depth test result

**Status:** CLOSED — Skipped. Covered by `InferenceService`.

### F26 — ServiceContext stores lack CAS write-through

**Status:** CLOSED (Session 10) — CAS store write-through is dead code (0 call sites). Removed `define_store_cas!` macro, `*_with_cas()` methods, `.with_cas()` builders, `cas_port` fields from 6 stores. Read-only `git_cas_port` for git operations (archive, verify, diff, log, snapshot) remains alive and untouched. No `ServiceContext` field added (F10 preserved).

### F9 — Production memory stores use in_memory_db()

**Status:** CLOSED (Session 11) — Added `memory_db_path: Option<String>` and `memory_passphrase: Option<String>` to `ServiceConfig`. `ServiceContext::build()` now respects `config.in_memory`: when false, opens file-backed encrypted DB for episodic/semantic stores via `Database::open()`; when true, keeps `in_memory_db()`. Path defaults to `{db_path}-memory.db` (e.g., `hkask.db` → `hkask-memory.db`) via `ServiceConfig::effective_memory_db_path()`. Passphrase defaults to `db_passphrase`. `HKASK_MEMORY_DB_PATH` env var supported. 5 new tests (3 config unit + 2 context integration). P1 User Sovereignty Guardrail satisfied: user configured persistence, user gets persistence.

---

## Open Questions

### F1 — Streaming response support

**Priority:** LOW · **Status:** Deferred

Current API returns single `ChatResponse`. Neither surface supports SSE chunked
inference. The service layer needs a `ChatStream` result type that both surfaces adapt.

**Next Action:** Implement when a surface requires streaming; design `ChatStream` service result type then.

---

### F2 — Session lifecycle across surfaces

**Priority:** MEDIUM · **Status:** Deferred

REPL `active_session` and API `standing_sessions` should migrate to
`ServiceContext`, but durability semantics need specification.

**Next Action:** Specify durability semantics (in-memory vs persisted, timeout policy) before migrating session state into `ServiceContext`.

---

### F3 — Unified authentication context

**Priority:** MEDIUM · **Status:** Deferred

API `AuthContext`, CLI `ResolvedSecrets`, MCP env vars — need single
`AuthContext` abstraction.

**Next Action:** Define a unified `AuthContext` struct in `hkask-services` that all three surfaces construct from their respective sources.

---

### F4 — MCP server service access

**Priority:** LOW · **Status:** By design

MCP servers are separate processes and can't share `ServiceContext` by reference.
They need shared primitives (`hkask-storage`, `hkask-keystore`,
`InferenceService` factory pattern).

---

### F6 — REPL vs API state boundary

**Priority:** MEDIUM · **Status:** Deferred

Document which fields stay in surface vs service.

**Next Action:** Audit `ServiceContext` fields and surface-local state; write a boundary table in the architecture docs.

---

### F7 — ServiceConfig vs environment variables

**Priority:** MEDIUM · **Status:** CLOSED — Default constants centralized; env-var reads audited.

Default values (`DEFAULT_DB_PATH`, `DEFAULT_OKAPI_BASE_URL`) made public in
`ServiceConfig` and re-exported from `hkask-services`. All 4 leaked call sites
(`commands/config.rs`, `commands/ensemble.rs`, `commands/compose.rs`, `repl/init.rs`)
now use centralized constants instead of duplicated string literals. Remaining
direct env-var reads in standalone CLI paths are by design — standalone commands
must work without a full `ServiceContext` (P1 Prohibition).

---

### F8 — GovernedTool membrane boundary

**Priority:** LOW · **Status:** Deferred

REPL creates `GovernedTool` with shared `CyberneticsLoop`; API creates in
`build_governed_mcp_tool()`; CLI commands create "disconnected" one. Service
layer should own canonical construction, but one-shot CLI commands need design.

**Next Action:** Design a `GovernedToolFactory` in `hkask-services` that surfaces call; CLI one-shot path gets a disconnected variant.

---

### F9 — Production memory stores use in_memory_db()

**Priority:** HIGH · **Status:** CLOSED — See Closed Questions section. P1 User Sovereignty Guardrail satisfied in Session 11.

---

### F10 — ServiceContext approaching god-object (20 fields)

**Priority:** MEDIUM · **Status:** CLOSED — `#[non_exhaustive]` applied; sub-struct grouping rejected by depth test.

`ServiceContext` now has `#[non_exhaustive]`, preventing external crates from
constructing it with struct literal syntax — `ServiceContext::build()` is the
sole constructor. Full sub-struct grouping (InfraContext, LoopContext,
AgentContext) was analyzed but rejected: each proposed sub-struct is a
data-only container with no behavior (shallow module, fails depth test).
The cost (changing every `ctx.field` to `ctx.group.field` across 7+ call
sites in 2 surfaces) outweighs the benefit. `#[non_exhaustive]` alone
achieves F10's goal of guarding against growth.

---

### F11 — InvalidPassphrase vs LoginFailed security

**Priority:** LOW · **Status:** Track

`InvalidPassphrase(String)` leaks whether the passphrase was wrong vs user not
found. `LoginFailed(String)` is deliberately opaque. Should unify for security
or document the distinction.

**Next Action:** Audit call sites; either unify both to `LoginFailed` or add a doc comment explaining the intentional distinction.

---

### F12 — ValidationError(String) too generic

**Priority:** LOW · **Status:** Track

`ValidationError(String)` is extremely broad. Consider `ConfigValidation(String)`
or domain-specific variants.

**Next Action:** Enumerate all `ValidationError` construction sites; replace with domain-specific variants where the error source is unambiguous.

---

### F14 — Dual error mapping in API

**Priority:** MEDIUM · **Status:** Partially addressed

`hkask-api/src/error.rs` has 14 direct `From<DomainError>` impls PLUS the
`From<ServiceError>` adapter. Each domain error is mapped to `ApiError` twice.
3 sovereignty routes now use `ApiError::from`; remaining direct constructions
are legitimate surface concerns (input validation, OCAP gates, auth checks).

**Next Action:** Audit remaining direct `ApiError` constructions; convert any that are pure service-layer error propagations to `ApiError::from` via `ServiceError`.

---

### F16 — Embedding concern separation

**Priority:** LOW · **Status:** Track

`OkapiEmbedding` still uses `OkapiConfig` directly rather than going through
`InferenceService`. Should embedding be a first-class service or remain
coupled to Okapi config?

**Next Action:** Evaluate whether `OkapiEmbedding` should call `InferenceService::embed()` or stay config-coupled; decide and document.

---

### F17 — CuratorService standalone commands open DB each time

**Priority:** MEDIUM · **Status:** Track

`CuratorService` standalone CLI commands (sovereignty verify, etc.) open a
new database connection on each invocation instead of reusing the one in
`ServiceContext`.

**Next Action:** Wire standalone commands through `ServiceContext::build()` path that shares the DB pool, or document why independent connections are acceptable.

---

### F18 — EnsembleService standing session extraction

**Priority:** MEDIUM · **Status:** Deferred

Divergent CLI/API flows for standing session management make extraction into
a unified service method non-trivial.

**Next Action:** Map CLI vs API session creation differences; design a unified `ensure_standing_session()` service method that accepts a surface-specific adapter.

---

### F19 — EnsembleService improv operation extraction

**Priority:** MEDIUM · **Status:** Deferred

Divergent inferencer setup between CLI and API improv paths blocks a single
service method.

**Next Action:** Abstract inferencer construction behind a trait or factory; then extract `run_improv()` into `EnsembleService`.

---

### F20 — EnsembleService list_deliberation_sessions depth test

**Priority:** LOW · **Status:** Pass-through

Stays as direct `SessionManager` call — too thin to warrant a service method.

---

### F22 — SovereigntyBoundaryStore reads in CLI Status

**Priority:** Guideline · **Status:** Per-user boundary data from persisted store

CLI `status` command reads `SovereigntyBoundaryStore` to display per-user
boundary data. This is a guideline-level concern — the data comes from the
persisted store via `ServiceContext`, which is correct.

**Next Action:** Document the read path in architecture docs if not already present.