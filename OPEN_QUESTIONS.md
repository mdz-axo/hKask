# Open Questions — hKask Service Layer Extraction

Questions raised during the service layer extraction (strangler fig pattern).
Each entry tracks priority, current status, and next action where applicable.

## Status Summary

| ID | Question | Priority | Status |
|----|----------|----------|--------|
| F1 | Streaming response support | LOW | Deferred |
| F2 | Session lifecycle across surfaces | MEDIUM | **CLOSED (Session 12)** |
| F3 | Unified authentication context | MEDIUM | **CLOSED (Session 12)** |
| F4 | MCP server service access | LOW | By design |
| F5 | Test seam depth for `ServiceContext::build()` | HIGH | CLOSED |
| F6 | REPL vs API state boundary | MEDIUM | **CLOSED (Session 12)** |
| F7 | `ServiceConfig` vs environment variables | MEDIUM | CLOSED |
| F8 | `GovernedTool` membrane boundary | LOW | Deferred |
| F9 | Production memory stores use `in_memory_db()` | HIGH | CLOSED |
| F10 | `ServiceContext` approaching god-object (20 fields) | MEDIUM | CLOSED |
| F11 | `InvalidPassphrase` vs `LoginFailed` security | LOW | Track |
| F12 | `ValidationError(String)` too generic | LOW | Track |
| F13 | `CapabilityChecker` secret inconsistency | LOW | CLOSED |
| F14 | Dual error mapping in API | MEDIUM | **CLOSED (Session 12)** |
| F15 | `InferenceContext` vs `ServiceContext` | LOW | CLOSED |
| F16 | Embedding concern separation | LOW | Track |
| F17 | `CuratorService` standalone commands open DB each time | MEDIUM | **CLOSED (Session 12)** |
| F18 | `EnsembleService` standing session extraction | MEDIUM | **CLOSED (Session 12)** |
| F19 | `EnsembleService` improv operation extraction | MEDIUM | **CLOSED (Session 12)** |
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

### F2 — Session lifecycle across surfaces

**Status:** CLOSED (Session 12) — By design. Audit confirmed CLI and API have fundamentally different session models; shared parts already extracted.

**Audit findings:**

| Aspect | CLI REPL | API |
|--------|----------|-----|
| Session scope | REPL process lifetime | HTTP request → standing session |
| Session creation | `/ensemble` command → `active_session = Some(id)` | POST /api/ensemble/standing → insert into HashMap |
| Session destruction | `/into` leaves, or REPL exits | DELETE or server restart |
| Session persistence | None (in-memory) | StandingSessionStore for metadata, in-memory HashMap for live state |
| Multi-session | Only one `active_session` at a time | Multiple standing sessions in HashMap |

The shared session concern (SessionManager for ensemble chat/deliberation) is already
unified via `EnsembleService`. The divergent concerns (`active_session` navigation vs
`standing_sessions` management) are surface-specific and correctly placed. Migrating
session state into `ServiceContext` would add surface-specific fields to a shared
context — violating the F10 god-object guardrail. No durability spec needed because
each surface's session lifecycle is self-contained.

**Constraint forces:** Forcing `ServiceContext::build()` for standalone CLI session
operations is a P1 Prohibition violation (standalone commands must work without full
`ServiceContext`).

### F3 — Unified authentication context

**Status:** CLOSED (Session 12) — By design. Three surfaces have fundamentally different auth models; a unified `AuthContext` in `hkask-services` fails the depth test.

**Audit findings:**

| Surface | Auth Model | Identity Source | Capability Check |
|---------|-----------|-----------------|------------------|
| API | Cryptographic token verification | `Authorization: Bearer` → HMAC-SHA256 → `AuthContext { token, webid }` | `capability_checker.check_resource(&auth.token, ...)` |
| CLI REPL | Trusted operator at terminal | `WebID::from_persona(agent_name)` — no token verification | No formal capability check (root access) |
| CLI standalone | Trusted operator, direct DB access | DB passphrase from env/keychain | No auth layer at all |
| MCP servers | Out-of-process, env var secrets | N/A (excluded by P1 design) | N/A |

A service-layer `AuthContext { webid: WebID, token: Option<DelegationToken> }` would be
a data-only container with no behavior — shallow module, fails depth test. The API's
`AuthContext` is correctly placed in API middleware (depends on Axum `Extension`). The
CLI doesn't need a formal auth layer. OCAP capability checks are a P1 Prohibition that
must stay in domain crates/surfaces.

**Depth test:** Delete API `AuthContext` → complexity reappears in 25+ route handlers.
Delete proposed service-layer `AuthContext` → complexity vanishes (it has no behavior).

### F6 — REPL vs API state boundary

**Status:** CLOSED (Session 12) — Boundary table documented. Shared fields are already in `ServiceContext`; surface-specific fields are correctly placed.

**Boundary table:**

| Category | Shared (ServiceContext) | CLI-only | API-only |
|----------|------------------------|----------|----------|
| Infrastructure | registry, mcp_runtime, mcp_dispatcher, loop_system, cns_runtime, consent_manager, escalation_queue, session_manager, standing_session_store, goal_repo, service_config, system_webid, inference_port, episodic_storage, pod_manager, event_sink | — | capability_checker, git_cas, git_cas_port |
| Inference | inference_port | inference_loop, gate_inference_port, hhh_*, governed_tool | ensemble_inferencer, gas_governance |
| Memory | episodic_storage | semantic_storage, consolidation_service | — |
| Session | session_manager, standing_session_store | active_session, session_history, current_model, current_agent | standing_sessions (live map) |
| Persona/Manifest | — | persona_constraints, tool_prompt_section, manifest_executor, process_manifest, resolved_secrets | spec_store |

Both surfaces derive from `ServiceContext::build()`. Surface-specific fields have no
counterpart in the other surface. The boundary is clean — no extraction needed.

### F14 — Dual error mapping in API

**Status:** CLOSED (Session 12) — All remaining direct `ApiError::` constructions are legitimate surface concerns.

**Audit of remaining direct constructions:**

| Category | Variants | Justification |
|----------|----------|---------------|
| Input validation | `BadRequest` | Parsing: Visibility, GoalState, GoalID, UUID, field validation — HTTP-layer input concern |
| OCAP capability | `Forbidden` | `check_resource(&auth.token, ...)` — P1 Prohibition, must stay in surface |
| Auth failures | `Unauthorized` | Passphrase mismatch, missing auth — HTTP-layer auth concern |
| Surface-only entities | `NotFound` | Standing sessions, bundles — don't exist in service layer |
| Infrastructure | `Internal` | Consolidation DB open, episodic memory, git operations — surface-wired infrastructure without ServiceError path |

None of these should flow through `ServiceError` — they're HTTP-layer concerns that
have no service-layer counterpart.

### F17 — CuratorService standalone commands open DB each time

**Status:** CLOSED (Session 12) — By design. P1 Prohibition protects standalone CLI pattern.

**Audit findings:**

4 standalone `commands/curator.rs` functions each call `open_registry_db()` to create
a fresh `EscalationQueue` + `CuratorContext`. The `commands/sovereignty.rs` `build_ctx()`
similarly opens a fresh `ConsentStore` + `ConsentManager`.

**Cost of forcing `ServiceContext::build()`:** Initializes entire dependency graph
(registry, MCP runtime, CNS, loops, escalation queue, episodic/semantic stores,
consent manager, goal repo, pod manager, capability checker, event sink, session
manager). 90%+ of this infrastructure is unused by standalone commands.

**Cost of current pattern:** 1 SQLite connection open per one-shot CLI invocation.
Negligible for commands that run once and exit.

**Constraint forces:** P1 Prohibition — "Standalone CLI commands work without
`ServiceContext`". Forcing `ServiceContext::build()` for a `kask curator resolve <id>`
invocation violates this principle. The single SQLite connection overhead is acceptable
for one-shot commands.

### F18 — EnsembleService standing session extraction

**Status:** CLOSED (Session 12) — By design. Divergence between CLI and API standing session flows is wider than previously documented; common logic too shallow to extract.

**Audit findings:**

| Aspect | CLI | API |
|--------|-----|-----|
| Config source | YAML file on disk | JSON HTTP body |
| MCP tool discovery | None | Yes (discover_tools + with_available_tools) |
| Gas governance | None | Yes (with_gas_governance from CyberneticsLoop) |
| Session persistence | No explicit persist call | Yes (persist_session + post_initial_messages) |
| Session lifecycle | Bootstrap and forget | Start + store in HashMap + status queries |

The common logic is 2 lines: `StandingSession::from_config(config)` + `.with_store(store)`.
Too shallow to justify a service method (fails depth test). The real complexity is in
surface-specific setup (tool discovery, gas governance, persistence, state management).

### F19 — EnsembleService improv operation extraction

**Status:** CLOSED (Session 12) — By design. Improv operations are CLI-only; no API counterpart exists. No duplication to extract.

**Audit findings:**

CLI has 4 improv operations: `improv_turn`, `improv_config`, `set_threshold`, `set_mode`.
API has no improv endpoints. Moving these to `hkask-services` would be pass-through
delegation with no error normalization benefit — the service function would just call
through to `SessionManager`/`ChatSession` methods with no added logic.

---

## Open Questions

### F1 — Streaming response support

**Priority:** LOW · **Status:** Deferred

Current API returns single `ChatResponse`. Neither surface supports SSE chunked
inference. The service layer needs a `ChatStream` result type that both surfaces adapt.

**Next Action:** Implement when a surface requires streaming; design `ChatStream` service result type then.

---

### F4 — MCP server service access

**Priority:** LOW · **Status:** By design

MCP servers are separate processes and can't share `ServiceContext` by reference.
They need shared primitives (`hkask-storage`, `hkask-keystore`,
`InferenceService` factory pattern).

---

### F8 — GovernedTool membrane boundary

**Priority:** LOW · **Status:** Deferred

REPL creates `GovernedTool` with shared `CyberneticsLoop`; API creates in
`build_governed_mcp_tool()`; CLI commands create "disconnected" one. Service
layer should own canonical construction, but one-shot CLI commands need design.

**Next Action:** Design a `GovernedToolFactory` in `hkask-services` that surfaces call; CLI one-shot path gets a disconnected variant.

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

### F16 — Embedding concern separation

**Priority:** LOW · **Status:** Track

`OkapiEmbedding` still uses `OkapiConfig` directly rather than going through
`InferenceService`. Should embedding be a first-class service or remain
coupled to Okapi config?

**Next Action:** Evaluate whether `OkapiEmbedding` should call `InferenceService::embed()` or stay config-coupled; decide and document.

---

### F22 — SovereigntyBoundaryStore reads in CLI Status

**Priority:** Guideline · **Status:** Per-user boundary data from persisted store

CLI `status` command reads `SovereigntyBoundaryStore` to display per-user
boundary data. This is a guideline-level concern — the data comes from the
persisted store via `ServiceContext`, which is correct.

**Next Action:** Document the read path in architecture docs if not already present.