# Handoff — hKask Service Layer Extraction

## 1. Session Context

Twelve sessions have completed the 9-task service layer extraction plan plus all post-extraction open questions. All extraction, migration, infrastructure unification, verification, and documentation tasks are complete. Session 11 closed three prioritized post-extraction questions (F9, F10, F7). Session 12 audited all remaining Tier 1 MEDIUM open questions (F2, F3, F6, F14, F17, F18, F19) — all resolved as "by design" via depth test and constraint analysis. No MEDIUM-priority open questions remain. This handoff covers all work done and the LOW/track-only questions that remain for future work.

**Session 1** (Tasks 1–3): Created the `hkask-services` crate skeleton, extracted `ServiceError`, `ServiceConfig`, and `ServiceContext`. Left 3 clippy errors and no tests.

**Session 2** (Re-audit + Fixes + Task 4 start): Activated all 5 mandatory skills. Ran full Phase 0→1→2 re-audit, found 4 MUST FIX bugs. Fixed all 4 plus 4 SHOULD FIX items. Created `InferenceService` module with 3 public functions and 4 tests. Did NOT wire CLI or API surfaces.

**Session 3** (Task 4 completion — Phases 4c–4f): Wired all CLI (8 sites) and API (4 sites) to call InferenceService. Introduced `InferenceContext` as a lightweight alternative to `ServiceContext` for surface layers. Removed all `OkapiConfig::local_dev()` and `OkapiInference::new()` calls from CLI and API inference sites. All workspace checks pass: `cargo check`, `cargo clippy -D warnings`, `cargo test`.

**Session 4** (Task 5 — CuratorService): Extracted `CuratorService` with 6 service operations, `CuratorContext`, and `MetacognitionSummary`. Full strangler fig cycle completed: RED (6 tests) → GREEN (all pass) → Wire CLI → Wire API → Delete duplication → Verify workspace.

**Session 5** (Task 6a — EnsembleService): Extracted `EnsembleService` with 8 service operations, `EnsembleContext`, and `map_participant_role` helper. Full strangler fig cycle completed for chat/deliberation operations. Standing sessions and improv operations intentionally excluded (divergent/surface-only). All workspace checks pass.

**Session 6** (Task 6b — PodService): Extracted `PodService` with 6 service operations (5 domain + 1 helper), `PodContext`, and `normalize_pod_error` helper. Full strangler fig cycle completed. Fixed CLI bug where `deactivate_pod` silently swallowed errors. Both CLI and API now route through `PodService`. All workspace checks pass.

**Session 7** (Task 6c-skipped, Task 6d — SovereigntyService): Skipped memory.rs (depth test failed). Extracted `SovereigntyService` with 9 public functions + 2 return types (AccessCheck, SovereigntyStatus) and 13 tests. Fixed CLI `revoke_consent` bug. Both surfaces route through SovereigntyService.

**Session 8** (Task 6e/6f/6g — all skipped via depth test): Applied depth test to spec.rs, goal.rs, and models.rs. All three fail the 8-call-site threshold. Task 6 is now complete (5 extracted, 4 skipped). Proceeding to Task 7.

**Session 9** (Task 7b — Surface Assembly Migration): Migrated both API and CLI surfaces to compose `ServiceContext::build()` instead of their own independent assembly paths. Added `ApiState::from_service_context()` and refactored `ApiState::with_defaults()`. Migrated `kask serve` and `kask loops` to use `ServiceContext::build()`. Refactored `init_repl_state()` to use `ServiceContext::build()` for shared infrastructure. Deleted 4 API modules (loop_system, governed_tool, stores, ensemble) and ~460 lines of duplicated assembly code. 49 tests passing (46 service + 3 API).

**Session 10** (Tasks 7c, 7d, 7f, 8, 9 — CAS investigation, secret audit, error mapping, verification, documentation): Closed F26 (CAS store write-through is dead code — 0 call sites; removed `define_store_cas!` macro, 6 `*_with_cas()` methods, `.with_cas()` builders from 6 stores). Audited secret resolution (all main paths flow through ServiceConfig; remaining direct keystore calls are by design). Unified 3 sovereignty API routes to use `ApiError::from` instead of `ApiError::Internal`. Full workspace verification passes. Updated test-inventory.md, architecture-master.md, wrote OPEN_QUESTIONS.md (F1–F26). 49 tests passing.

**Session 11** (F9, F10, F7 — Post-extraction open questions): Closed all three prioritized post-extraction questions. F9 (HIGH/P1 User Sovereignty): `ServiceContext::build()` now respects `config.in_memory` — file-backed encrypted DB for episodic/semantic stores when false, in-memory when true. Added `memory_db_path`/`memory_passphrase` to `ServiceConfig`, `effective_memory_db_path()` helper, `HKASK_MEMORY_DB_PATH` env var, 5 new tests. F10 (MEDIUM): `#[non_exhaustive]` applied to `ServiceContext`; sub-struct grouping analyzed and rejected by depth test (data-only containers = shallow modules). F7 (MEDIUM): `DEFAULT_DB_PATH`/`DEFAULT_OKAPI_BASE_URL` made public and re-exported; 4 leaked call sites now use centralized constants. 51 tests passing (46 prior + 5 new).

**Session 12** (F2, F3, F6, F14, F17, F18, F19 — Tier 1 design audits): Audited all 7 remaining MEDIUM-priority open questions. Every question resolved as "by design" — no code extraction warranted. F2: CLI and API have fundamentally different session models; shared parts already extracted via EnsembleService. F3: Three surfaces have fundamentally different auth models; unified AuthContext would be shallow data-only container (fails depth test). F6: Boundary table documented; shared fields already in ServiceContext, surface-specific fields correctly placed. F14: All remaining direct ApiError constructions are legitimate HTTP-layer concerns. F17: Standalone CLI commands intentionally avoid ServiceContext (P1 Prohibition); single SQLite open per one-shot command is negligible. F18: CLI/API standing session divergence too wide; 2-line common logic too shallow to extract. F19: Improv operations are CLI-only with no API counterpart. 51 tests passing (unchanged — design session, no code changes).

## 2. What Was Done

### Session 2 — Re-Audit Fixes

| # | Fix | File(s) |
|---|-----|---------|
| F1 | Added `ServiceError::Keystore(String)` variant; moved keystore errors out of `Cns(String)` | `error.rs`, `config.rs`, `api/error.rs` |
| F2 | Converted `ServiceContext::build()` from sync to `async fn build()` | `context.rs` |
| F3 | Added `ApiError::ServiceUnavailable { reason }` (503) for keystore errors | `api/error.rs` |
| F4 | Fixed memory adapter sharing — uses same `mem_conn` as loops, documented pattern | `context.rs` |
| F5 | Added `template_cache_path` field to `ServiceConfig`; removed hardcoded path | `config.rs`, `context.rs` |
| F6 | Extracted 8 default constants to module level in `config.rs` | `config.rs` |
| F7 | CNS event sink now uses `primary_conn` instead of `in_memory_db()` | `context.rs` |
| F8 | Fixed 3 clippy errors (redundant closures, missing semicolons) | `context.rs` |

### Session 2 — Phase 0–3 Audit Results

**Phase 0 (Zoom-Out):** Produced module map, caller graph, data flow, boundary summary, key invariants, and depth/deletion tests for all 3 existing modules. Key finding: `ServiceContext` and `ServiceConfig` are scaffolding not yet consumed by any surface.

**Phase 1 (Audit):** Produced RDF triples for 17 duplicated operations across CLI and API. Classified each as Identical (7), Divergent (8), Surface-only (1), or Pass-through (1). Produced mermaid ER diagram.

**Phase 2 (Constraint Classification):** Classified 10 design decisions. Critical findings:
- Decision #6 (in-memory memory stores) is a **P1 User Sovereignty Guardrail** — user configured persistent storage, got ephemeral
- Decision #2 (sync build with dropped runtime) promoted from Hypothesis to **confirmed bug** — dangling `Handle` in `FullMcpAdapter`

**Phase 3 (Design):** Produced depth-test results for 11 planned modules. **2 modules rejected by depth test:**
- `chat.rs` → Pass-through (agent chat is REPL-specific; raw inference is InferenceService)
- `cns.rs` → Pass-through (`ctx.cns_runtime.health()` is single-line delegation)

**9 modules approved for creation:**

| Module | Functions | Classification | Depth Test |
|--------|-----------|---------------|------------|
| `inference.rs` | 3 | Identical | PASS |
| `curator.rs` | 6 | Divergent | PASS |
| `ensemble.rs` | 8 | Identical/Divergent mix | PASS |
| `pods.rs` | 5 | Divergent | PASS |
| `models.rs` | 2 | Divergent | PASS |
| `memory.rs` | 5 | Identical | PASS |
| `sovereignty.rs` | 4 | Identical | PASS |
| `spec.rs` | 4 | Identical | PASS |
| `goal.rs` | 3 | Identical | PASS |

### Session 2 — InferenceService (Task 4, partial)

Created `hkask-services/src/inference.rs` with:
- `InferenceService::resolve_port(ctx, model)` — REQ tags: svc-inf-001, svc-inf-002, svc-inf-003
- `InferenceService::list_models(ctx)` — REQ tag: svc-inf-004
- `InferenceService::search_models(ctx, query)` — REQ tag: svc-inf-005
- `ModelInfo` struct with `From<OkapiModelEntry>` conversion
- 4 unit tests (all passing)

### Session 3 — InferenceService Wiring (Task 4, Phases 4c–4f)

**Key Design Decision: InferenceContext**

Introduced `InferenceContext` as a lightweight struct containing only the 3 fields needed for inference: `shared_port`, `default_model`, `okapi_base_url`. This avoids requiring a full `ServiceContext` (which opens databases and starts loops) at call sites that only need inference port resolution.

- `InferenceContext::from_parts(shared_port, default_model, okapi_base_url)` — for CLI/API surfaces that construct from their own state
- `From<&ServiceContext> for InferenceContext` — for future use when ServiceContext is fully composed (Task 7b)
- Changed `InferenceService` method signatures from `&ServiceContext` to `&InferenceContext`

**CLI Wiring (8 sites across 7 files):**

| File | What Changed |
|------|-------------|
| `cli/repl/init.rs` | Default + gate inference ports via `InferenceService::resolve_port()`. Added `ServiceConfig` construction from onboarding secrets. Removed `OkapiConfig::local_dev()`. |
| `cli/repl/mod.rs` | Replaced `okapi_config: OkapiConfig` with `service_config: ServiceConfig` in `ReplState`. |
| `cli/repl/handlers/hhh.rs` | Gate model switch via `InferenceService::resolve_port()` using `state.service_config`. Removed `OkapiInference::new()`. |
| `cli/repl/handlers/model.rs` | Model listing/search via `InferenceService::search_models()` using `state.service_config`. Uses `ModelInfo` fields instead of `OkapiModelEntry.details`. |
| `cli/commands/chat.rs` | Fallback inference port via `InferenceService::resolve_port()`. Removed `OkapiConfig::local_dev()` + `OkapiInference::new()`. |
| `cli/commands/compose.rs` | Generation inference port via `InferenceService::resolve_port()`. Kept `OkapiConfig` for embedding (different concern). Removed `OkapiInference::new()`. |
| `cli/commands/ensemble.rs` | Ensemble improv inference via `InferenceService::resolve_port()`. Removed `OkapiInference::new()`. |

**API Wiring (4 sites across 3 files):**

| File | What Changed |
|------|-------------|
| `api/lib.rs` | Added `service_config: ServiceConfig` to `ApiState`. `with_ensemble_inferencer()` uses `InferenceService::resolve_port()`. |
| `api/routes/chat.rs` | Fallback inference via `InferenceService::resolve_port()` using `state.service_config`. |
| `api/routes/models.rs` | `list_models` and `search_models` via `InferenceService::list_models()` / `search_models()`. Uses `ModelInfo` fields directly. |

### Session 4 — CuratorService (Task 5, complete)

Created `hkask-services/src/curator.rs` with:

- `CuratorContext` — lightweight context with `escalation_queue`, optional `cns_runtime`, optional `dispatch`
- `CuratorService` — 6 service operations (all wired to both surfaces)
- `MetacognitionSummary` — service-layer type capturing public `HealthSnapshot` fields
- 6 unit tests with `// REQ:` tags (svc-cur-001 through svc-cur-006)

**Key Design Decisions for CuratorService:**

| Decision | Force | Rationale |
|----------|-------|-----------|
| `resolve_escalation`/`dismiss_escalation` verify existence before mutating | Guideline | Normalizes behavior: API checked, CLI didn't. Both surfaces now get `ServiceError::EscalationNotFound` |
| `CuratorContext.cns_runtime` and `dispatch` are `Option<Arc<..>>` | Guideline | Escalation-only ops don't need them; follows `InferenceContext.shared_port` pattern |
| `CuratorAgent` constructed fresh per `run_metacognition` call | Hypothesis | Matches CLI's current behavior. Shared MetacognitionLoop is future work |
| `EscalationStats` re-exported from `hkask_agents::escalation` | Guideline | Clean domain type, no need for service-layer wrapper |
| `MetacognitionSummary` is a new service-layer type | Guideline | `HealthSnapshot.bot_status_reports` is `pub(crate)`, so we expose public fields only |
| `From<&ServiceContext> for CuratorContext` deferred | Guideline | Requires async `.read().await` on `RwLock<CnsRuntime>`; add in Task 7b |

**CLI Wiring (4 operations in `cli/commands/curator.rs`):**

| Function | What Changed |
|----------|-------------|
| `curator_escalations()` | Creates `CuratorContext` from `EscalationQueue::new()`, calls `CuratorService::list_escalations()` |
| `curator_resolve(id)` | Creates `CuratorContext`, calls `CuratorService::resolve_escalation()` |
| `curator_dismiss(id)` | Creates `CuratorContext`, calls `CuratorService::dismiss_escalation()` |
| `curator_metacognition()` | Creates `CuratorContext` with CNS + dispatch, calls `CuratorService::run_metacognition()`, returns `summary.summary_text` |

**API Wiring (4 operations in `api/routes/curator.rs`):**

| Function | What Changed |
|----------|-------------|
| `list_escalations` | Creates `CuratorContext` from `state.escalation_queue`, calls `CuratorService::list_escalations()` |
| `resolve_escalation` | Creates `CuratorContext`, calls `CuratorService::resolve_escalation()` (existence check now in service) |
| `dismiss_escalation` | Creates `CuratorContext`, calls `CuratorService::dismiss_escalation()` (existence check now in service) |
| `metacognition_status` | Creates `CuratorContext`, calls `CuratorService::escalation_stats()` |

**Duplication removed:**
- CLI no longer directly calls `EscalationQueue::list_pending()`, `queue.resolve()`, `queue.dismiss()`, or constructs `CuratorAgent` + `MetacognitionLoop`
- API no longer directly calls `queue.list_pending()`, `queue.get()`, `queue.resolve()`, `queue.dismiss()`, `queue.stats()`
- Both surfaces route through `CuratorService`, which normalizes behavior (existence check before resolve/dismiss)

### Session 5 — EnsembleService (Task 6a, complete)

Created `hkask-services/src/ensemble.rs` with:

- `EnsembleContext` — lightweight context with `session_manager: Arc<RwLock<SessionManager>>`
- `EnsembleService` — 8 async service operations (7 normalization + 1 thin CRUD)
- `map_participant_role` — public helper normalizing `"orchestrator"` → `ParticipantRole::Curator`
- 11 unit tests with `// REQ:` tags (svc-ens-001 through svc-ens-008, plus svc-ens-003a)

**Key Design Decisions for EnsembleService:**

| Decision | Force | Rationale |
|----------|-------|-----------|
| Standing sessions NOT extracted | Divergent | CLI reads YAML from file; API constructs from JSON + MCP tool discovery + gas governance. Too different to unify. Stay in surface code. |
| Improv operations NOT extracted | Divergent/Surface-only | `improv_turn` needs surface-specific inferencer (CLI: global static; API: `ensemble_inferencer_with_breaker()`). `improv_config`, `set_threshold`, `set_mode` are CLI-only. |
| `EnsembleContext` has only `session_manager` | Guideline | Chat/deliberation ops only need the session manager. Standing ops would need `StandingSessionStore` + `GasGovernancePort` but those are surface-specific. |
| `map_participant_role` is a public function, not a method | Guideline | Surfaces need it for formatting output. Both CLI and API had identical mapping logic. |
| `list_deliberation_sessions` stays as direct call in both surfaces | Pass-through | Thin delegation that doesn't normalize errors. Not worth a service method. |
| `get_chat` API handler stays as direct `SessionManager` call | Surface-only | Only in API, not in CLI. Not duplicated. |

**CLI Wiring (9 functions in `cli/commands/ensemble.rs`):**

| Function | What Changed |
|----------|-------------|
| `ensemble_chat_create` | Routes through `EnsembleService::create_chat()` |
| `ensemble_chat_register` | Routes through `EnsembleService::register_participant()` (role mapping now in service) |
| `ensemble_chat_send` | Routes through `EnsembleService::send_message()` |
| `ensemble_chat_list` | Routes through `EnsembleService::list_chat_sessions()` |
| `ensemble_deliberation_create` | Routes through `EnsembleService::create_deliberation()` |
| `ensemble_deliberation_start` | Routes through `EnsembleService::start_deliberation()` |
| `ensemble_deliberation_record` | Routes through `EnsembleService::record_deliberation_response()` |
| `ensemble_deliberation_synthesize` | Routes through `EnsembleService::synthesize_deliberation()` |
| `ensemble_deliberation_list` | Stays as direct `SessionManager` call (thin pass-through) |

**API Wiring (8 handlers in `api/routes/ensemble.rs`):**

| Handler | What Changed |
|---------|-------------|
| `create_chat` | Routes through `EnsembleService::create_chat()` |
| `list_chats` | Routes through `EnsembleService::list_chat_sessions()` |
| `register_bot` | Routes through `EnsembleService::register_participant()` (role mapping now in service) |
| `send_message` | Routes through `EnsembleService::send_message()` |
| `create_deliberation` | Routes through `EnsembleService::create_deliberation()` |
| `start_deliberation` | Routes through `EnsembleService::start_deliberation()` |
| `record_response` | Routes through `EnsembleService::record_deliberation_response()` |
| `synthesize_deliberation` | Routes through `EnsembleService::synthesize_deliberation()` |
| `list_deliberations` | Stays as direct `SessionManager` call (thin pass-through) |

**Duplication removed:**
- CLI no longer has `ParticipantRole` matching logic (`"orchestrator" => Curator`)
- API no longer has `ParticipantRole` matching logic
- Both surfaces get consistent `ServiceError::SessionNotFound` for missing sessions
- `AgentResponse`, `ChatParticipant`, `ParticipantRole` imports removed from API routes
- `AgentResponse`, `ChatParticipant`, `ParticipantRole` imports removed from CLI commands

### Session 6 — PodService (Task 6b, complete)

Created `hkask-services/src/pods.rs` with:

- `PodContext` — lightweight context with `pod_manager: Arc<PodManager>`
- `PodService` — 5 async service operations + 1 sync helper
- `normalize_pod_error` — internal helper mapping `AgentPodError::PodNotFound` → `ServiceError::PodNotFound`
- 6 unit tests with `// REQ:` tags (svc-pod-001 through svc-pod-006)

**Key Design Decisions for PodService:**

| Decision | Force | Rationale |
|----------|-------|-----------|
| `PodContext` holds only `Arc<PodManager>` | Guideline | Matches `EnsembleContext` pattern (single field). Pod lifecycle ops need only the pod manager. |
| `PodService::parse_pod_id()` centralizes UUID parsing | Guideline | Was duplicated in 6 call sites. Returns `ServiceError::PodNotFound` for invalid UUIDs. |
| `PodService::deactivate_pod()` fixes CLI error swallowing | Guardrail | CLI previously did `let _ = manager.deactivate_pod(...)` — silently ignoring errors. Service now propagates consistently. |
| Auth/capability check stays in API for `create_pod` | Prohibition (P1) | OCAP capability gating is user sovereignty. Service layer doesn't decide who can create pods. |
| Persona parsing stays in surface | Guideline | CLI reads persona YAML from file; API receives `persona_yaml` in JSON body. File I/O and deserialization are surface concerns. |
| CLI pod ops use `PodManager::new_mock()` per invocation | Hypothesis | CLI pod operations are stateless (transient PodManager). Service layer normalizes ops, not PodManager lifecycle. |

**CLI Wiring (5 functions in `cli/commands/pod.rs`):**

| Function | What Changed |
|----------|-------------|
| `get_pod_status` | Routes through `PodService::get_pod_status()`, UUID parsing removed |
| `list_pods` | Routes through `PodService::list_pods()`, PodManagerBuilder stays in surface |
| `create_pod` | Routes through `PodService::create_pod()`, file I/O stays in surface |
| `activate_pod` | Routes through `PodService::activate_pod()`, UUID parsing removed |
| `deactivate_pod` | Routes through `PodService::deactivate_pod()`, **error swallowing fixed** |

**API Wiring (5 handlers in `api/routes/pods.rs`):**

| Handler | What Changed |
|---------|-------------|
| `list_pods` | Routes through `PodService::list_pods()` |
| `create_pod` | Routes through `PodService::create_pod()`, auth check stays in surface |
| `activate_pod` | Routes through `PodService::activate_pod()`, UUID parsing removed |
| `deactivate_pod` | Routes through `PodService::deactivate_pod()`, UUID parsing removed |
| `pod_status` | Routes through `PodService::get_pod_status()`, UUID parsing removed |

**Duplication removed:**
- CLI no longer has `Uuid::parse_str` + `PodID::from_uuid` calls (was in 3 functions)
- API no longer has `Uuid::parse_str` + `PodID::from_uuid` calls (was in 3 handlers)
- Both surfaces get consistent `ServiceError::PodNotFound` for missing/invalid pod IDs
- `uuid` import removed from both CLI and API pod files
- `PodID` import removed from both CLI and API pod files

**New `ServiceError` variants:**
- `PodNotFound(String)` — sentinel for UUID parse errors and not-found normalization
- `Pod(#[from] AgentPodError)` — catch-all for pod domain errors

**API error mappings added** (`From<ServiceError> for ApiError`):
- `PodNotFound(id)` → `NotFound { resource: "pod", id }`
- `Pod(AgentPodError::PodNotFound(_))` → `NotFound { resource: "pod", id: e.to_string() }`
- `Pod(AgentPodError::PersonaParseError(msg))` → `BadRequest { message: "Invalid persona: {msg}" }`
- `Pod(AgentPodError::InvalidStateTransition(from, to))` → `Conflict { message: "Invalid pod state transition: {from} -> {to}" }`
- `Pod(_)` → `Internal { message: e.to_string() }`

**Task 6c — memory.rs (SKIPPED)**

Depth test FAILED. Memory domain does not warrant extraction:
- **Episodic store/recall/usage** — P1 OCAP-gated (EpisodicStoragePort membrane). Service layer MUST NOT bypass OCAP. No duplication across surfaces.
- **Semantic store/recall** — CLI-only (compose, embed_corpus). No API counterpart. No duplication.
- **Consolidation trigger** — Only 2 call sites (CLI + API) with high divergence (API has rate limiting, different WebID resolution, different passphrase flow). Below 8-call-site threshold.
- **Memory infrastructure construction** (DB open, passphrase resolve, memory build) — belongs in Task 7 (ServiceContext::build()), not a service module.

**Task 6d — SovereigntyService (COMPLETE)**

9 public functions + 2 return types, 13 tests, all passing:
- `parse_data_category()` — centralizes identical string-to-DataCategory mapping from CLI helpers and API route
- `get_boundary()` — returns `DataSovereigntyBoundary::hkask_default()` (previously duplicated in both surfaces)
- `requires_affirmative_consent()` — boundary policy check
- `grant_consent()` — delegates to ConsentManager
- `revoke_consent()` — delegates to ConsentManager (fixes CLI bug: spurious grant_consent call before revoke)
- `has_consent()` — delegates to ConsentManager, fails closed
- `get_granted_categories()` — delegates to ConsentManager
- `check_access()` — combines boundary classification + consent check; returns AccessCheck struct
- `get_status()` — combines boundary data + consent state; returns SovereigntyStatus struct

Key achievements for 6d:
- `SovereigntyContext` with `Arc<ConsentManager>` — follows PodContext pattern (single field)
- CLI `Revoke` bug fixed: removed spurious `grant_consent` call before `revoke_consent`
- CLI `Check` now shows classification + access_required + has_consent (was showing only granted/denied)
- API `sovereignty_status` now uses `SovereigntyService::get_status()` instead of inline boundary + consent logic
- API `sovereignty_check_access` now uses `SovereigntyService::check_access()` instead of inline boundary classification
- API `sovereignty_grant_consent` now uses `SovereigntyService::grant_consent()` + `get_granted_categories()`
- API `sovereignty_revoke_consent` now uses `SovereigntyService::revoke_consent()` (previously called `state.consent_manager.revoke_consent()` directly)
- P1 Prohibition preserved: `check_access()` returns data + consent state; the surface decides to grant/deny
- Constraint: Guideline — `parse_data_category` centralized from 2 duplicate sites
- Constraint: Guardrail — `revoke_consent` in service only revokes (no spurious grant)
- All 40 service-layer tests passing, workspace compiles clean with clippy `-D warnings`

**Session 8 — Depth Tests for spec.rs, goal.rs, models.rs (all SKIPPED)**

All three remaining Task 6 submodules fail the depth test (8-call-site threshold):

| Module | Call Sites | Reason to Skip | Force |
|--------|-----------|----------------|-------|
| `spec.rs` | 4 | API routes are stubs (hardcoded responses); CLI render/test-invariant/test-verify are surface-only; validate/cultivate have no API counterpart with real logic; capture logic diverges (CLI persists, API returns) | Evidence (measured) |
| `goal.rs` | 12 across 6 ops | CRUD ops are thin pass-throughs to `SqliteGoalRepository`; parsing helpers too thin to justify full service module; `open_repository()` infrastructure wiring belongs in Task 7 | Evidence (measured) |
| `models.rs` | 0 | Already fully covered by `InferenceService::list_models/search_models`; no additional duplication | Evidence (measured) |

**Task 6 is now COMPLETE**: 5 modules extracted (inference, curator, ensemble, pods, sovereignty), 4 modules skipped (memory, spec, goal, models). All 40 service-layer tests passing.

**Session 8 — Task 7a (Infrastructure: From<&ServiceContext> impls + session_manager)**

Added `From<&ServiceContext>` impls for all 5 context types, enabling surfaces to derive their service contexts from a single `ServiceContext` instance:

| Context Type | `From<&ServiceContext>` | Notes |
|-------------|------------------------|-------|
| `InferenceContext` | ✅ Already existed | Pre-existing from Session 3 |
| `PodContext` | ✅ Added | `ctx.pod_manager.clone()` |
| `SovereigntyContext` | ✅ Added | `ctx.consent_manager.clone()` |
| `CuratorContext` | ✅ Added (escalation-only) | `cns_runtime: None`, `dispatch: Some(...)` |
| `EnsembleContext` | ✅ Added | `ctx.session_manager.clone()` |

Added `CuratorContext::from_service_context(ctx)` async method for full context (with CNS runtime).

Added `session_manager: Arc<RwLock<SessionManager>>` field to `ServiceContext` (needed by EnsembleService from both surfaces).

6 new infrastructure tests (svc-infra-001 through svc-infra-006) in `context.rs` verifying all `From<&ServiceContext>` impls work end-to-end with `ServiceContext::build(ServiceConfig::in_memory())`.

Key decisions for Session 8:

| Decision | Force | Rationale |
|----------|-------|-----------|
| `From<&ServiceContext>` for CuratorContext is escalation-only | Guideline | Can't extract `Arc<CnsRuntime>` from `Arc<RwLock<CnsRuntime>>` synchronously. `from_service_context()` async method provides full context. |
| `session_manager` added to ServiceContext | Guideline | Both CLI and API need SessionManager for ensemble operations. Enables `From<&ServiceContext> for EnsembleContext`. |
| Surface code NOT changed yet | Prohibition (P3 Strangler Fig) | Both old and new paths must work before deleting old code. Surfaces still use their own assembly. |

### Session 9 — Task 7b: Surface Assembly Migration

**Phase 1 — API surface:**

Added `ApiState::from_service_context(ctx, ensemble_inferencer)` async constructor that derives all shared infrastructure from `ServiceContext` and initializes API-specific fields (gas governance, git CAS, standing sessions, spec store) to defaults. This replaces the old `ApiState::new()` which manually assembled stores, loop system, governed tool, and ensemble session.

Refactored `ApiState::with_defaults()` from 7-parameter sync function to 0-parameter async function that resolves `ServiceConfig::from_env()`, calls `ServiceContext::build(config).await`, then `from_service_context(ctx, None).await`. Eliminated ~40 lines of inline PodManager/MemoryLoopAdapter assembly.

Migrated `kask serve` (`cli/commands/serve.rs`) from `ApiState::new(registry, mcp_runtime, ...)` to `ServiceContext::build(config).await` + `ApiState::from_service_context(ctx, Some(adapter)).await`. Eliminated `resolve_capability_secret()` function (secrets now resolved inside `ServiceConfig::from_env()`).

Deleted dead code: `ApiState::new()`, `ApiState::with_ensemble_inferencer()`, `build_loop_system()`, `build_governed_mcp_tool()`, `build_ensemble_session()`, `Stores::init()`, `open_db()`, `init_git_cas()`, `GitCasBundle`, `GovernedMcpTool`, `EnsembleSession`. Deleted 4 module files: `api/loop_system.rs`, `api/governed_tool.rs`, `api/stores.rs`, `api/ensemble.rs`.

3 new API tests: `from_service_context_produces_valid_state`, `from_service_context_with_ensemble_inferencer`, `with_defaults_uses_service_context`.

**Phase 2 — CLI surface:**

Refactored `kask loops` command (`cli/commands/loops.rs`) from 113-line manual assembly (CNS, cybernetics, episodic/semantic/curation loops, escalation queue, ACP secret resolution) to 44-line `ServiceContext::build(config)` call. ~69 lines eliminated.

Refactored `init_repl_state()` (`cli/repl/init.rs`) to use `ServiceContext::build()` for all shared infrastructure (CNS, loop system, curation loop, cybernetics loop, dispatch, MCP runtime, event sink, pod manager, registry, consent manager, escalation queue, session manager). CLI-specific concerns remain: inference port/loop, onboarding, per-agent memory DB, GovernedTool (for tool discovery), HHH gate, gas budget registration. ~183 lines of duplicated CNS/loop/curation/GovernedTool assembly eliminated.

Key design decisions for Session 9:

| Decision | Force | Rationale |
|----------|-------|-----------|
| `ApiState::from_service_context()` is async | Guideline | Extracting `Arc<CnsRuntime>` from `Arc<RwLock<CnsRuntime>>` requires `.await`. |
| API's `cns_runtime` now shares state with loop system's CNS | Bug fix | Old `ApiState::new()` created a disconnected `CnsRuntime` instance. New path clones from ServiceContext's shared runtime. |
| GovernedTool stays surface-specific (not in ServiceContext) | Guideline | Only CLI needs it for `discover_tools()/get_tool_info()`. API uses McpDispatcher. Fails depth test for ServiceContext field. |
| CLI creates its own GovernedTool from ServiceContext fields | Guideline | Uses `ctx.mcp_runtime`, `ctx.cybernetics_loop`, `ctx.event_sink`, `ctx.loop_system.dispatch_sender()`. |
| `with_defaults()` signature changed from 7 params to 0 params | Guideline | No live callers existed. Breaking change is safe. |
| CAS write-through not in ServiceContext stores | Open question (F26) | Old `Stores::init()` added `.with_cas()`. ServiceContext::build() doesn't. Per-mutation audit trails lost. Needs investigation. |
| F13 CapabilityChecker secrets are by design | CLOSED | 1 MCP checker (top-level) + 2 ACP checkers (adapter + PodManager). Same in both surfaces. Not inconsistent. |

**Total test count**: 49 tests (46 service-layer + 3 API)

Service-layer tests: 4 inference + 6 curator + 11 ensemble + 6 pods + 13 sovereignty + 6 infrastructure
API tests: from_service_context_produces_valid_state + from_service_context_with_ensemble_inferencer + with_defaults_uses_service_context

## 3. Current Module Structure

```
hkask-services/src/
├── lib.rs           — re-exports: ServiceConfig, DEFAULT_DB_PATH, DEFAULT_OKAPI_BASE_URL, ServiceContext, ServiceError, InferenceContext, InferenceService, ModelInfo, CuratorContext, CuratorService, MetacognitionSummary, EnsembleContext, EnsembleService, map_participant_role, PodContext, PodService, SovereigntyContext, SovereigntyService, SovereigntyStatus, AccessCheck, parse_data_category
├── error.rs         — 31 variants across 9 domain groups + SessionNotFound + Keystore + EscalationNotFound + Cns
├── config.rs        — ServiceConfig with 3 constructors + 2 public default constants (DEFAULT_DB_PATH, DEFAULT_OKAPI_BASE_URL) + 6 private constants + memory_db_path/memory_passphrase/effective_memory_db_path + 3 config tests
├── context.rs       — ServiceContext (#[non_exhaustive])::async build() with 20 Arc fields + 8 infrastructure tests
├── inference.rs     — InferenceContext + InferenceService (3 functions) + ModelInfo struct + From<&ServiceContext> + 4 tests
├── curator.rs       — CuratorContext + CuratorService (6 functions) + MetacognitionSummary + From<&ServiceContext> (escalation-only) + from_service_context (async) + 6 tests
├── ensemble.rs      — EnsembleContext + EnsembleService (8 functions) + map_participant_role + From<&ServiceContext> + 11 tests
├── pods.rs          — PodContext + PodService (6 functions) + normalize_pod_error + From<&ServiceContext> + 6 tests
└── sovereignty.rs   — SovereigntyContext + SovereigntyService (9 functions) + AccessCheck + SovereigntyStatus + parse_data_category + From<&ServiceContext> + 13 tests
```

## 4. Verification Status

```
cargo check --workspace                    ✅
cargo clippy --workspace -- -D warnings   ✅
cargo test --workspace                    ✅ (all tests passing)
cargo test -p hkask-services              ✅ (51 tests: 4 inference + 6 curator + 11 ensemble + 6 pods + 13 sovereignty + 8 infrastructure + 3 config)
cargo test -p hkask-api                   ✅ (3 tests: from_service_context_produces_valid_state + from_service_context_with_ensemble_inferencer + with_defaults_uses_service_context)
No todo!/unimplemented! in hkask-services ✅
No todo!/unimplemented! in hkask-api     ✅
ServiceContext is #[non_exhaustive]           ✅ (F10: external construction prevented)
Memory stores respect config.in_memory       ✅ (F9: file-backed DB when false, in-memory when true)
DEFAULT_DB_PATH/DEFAULT_OKAPI_BASE_URL public ✅ (F7: centralized, re-exported)
No EscalationQueue direct calls in CLI/API curator routes ✅
No CuratorAgent/MetacognitionLoop direct calls in CLI ✅
No direct EscalationQueue calls in API curator routes ✅
No SessionManager direct calls in wired CLI ensemble functions ✅
No SessionManager direct calls in wired API ensemble handlers ✅
No Uuid/PodID direct calls in CLI pod functions ✅
No Uuid/PodID direct calls in API pod handlers ✅
No PodManager direct calls in wired CLI pod functions ✅
No PodManager direct calls in wired API pod handlers ✅
MCP servers do NOT depend on hkask-services ✅ (P1 preserved)
Dependency direction: CLI/API → services → domain ✅ (no reverse)
Deleted: api/loop_system.rs, api/governed_tool.rs, api/stores.rs, api/ensemble.rs ✅
Deleted: ApiState::new(), ApiState::with_ensemble_inferencer() ✅
ApiState::from_service_context() replaces ApiState::new() ✅
kask serve uses ServiceContext::build() ✅
kask loops uses ServiceContext::build() ✅
init_repl_state() uses ServiceContext::build() for shared infra ✅
```

## 5. Key Decisions

1. **Flat error hierarchy, not nested.** `ServiceError` composes domain errors via `#[from]`. `Keystore(String)` for secret resolution failures.
2. **`ServiceContext::build()` is async.** No more `Runtime::new()` + `block_on()` + `drop(rt)`. Callers `.await` it.
3. **Strangler fig: build alongside, don't replace yet.** Neither `ReplState` nor `ApiState` compose `ServiceContext`. They use `InferenceContext`/`CuratorContext`/`EnsembleContext`/`PodContext`/`SovereigntyContext` + `ServiceConfig` instead. All context types now have `From<&ServiceContext>` but surfaces haven't migrated yet (Task 7b).
4. **MCP servers do NOT depend on `hkask-services`.** They use `hkask-templates` primitives directly.
5. **`InferenceService` does NOT cache ports by model.** Each non-default model call creates a fresh `OkapiInference`. Caching is a future Hypothesis.
6. **`InferenceService::resolve_port()` reuses shared port for default model.** Falls back to fresh instance for other models.
7. **No `chat.rs` module.** Agent-specific chat logic is REPL-only. Raw inference is in `InferenceService`.
8. **No `cns.rs` module.** `CnsRuntime` methods are direct delegations. Surfaces call `ctx.cns_runtime` directly.
9. **Memory adapter and loops share the same database connection** via `Arc<Connection>`. Different object instances, same underlying SQLite DB.
10. **CNS event sink uses `primary_conn`** for production persistence, not `in_memory_db()`.
11. **Template cache path is configurable** via `HKASK_TEMPLATE_CACHE_PATH` or `ServiceConfig.template_cache_path`.
12. **Dependency direction: CLI/API → services → domain crates.** Never the reverse.
13. **`InferenceContext` is the surface-facing type.** CLI and API use `InferenceContext::from_parts()` to avoid building a full `ServiceContext`. `From<&ServiceContext>` impl added for future use when `ServiceContext` is composed (Task 7b).
14. **`ReplState` stores `ServiceConfig` instead of `OkapiConfig`.** The `service_config` field provides `okapi_base_url`, `default_model`, and `gate_model` for `InferenceContext` construction.
15. **`ApiState` stores `ServiceConfig`** initialized from `ServiceConfig::from_env()` at construction time.
16. **`embed_corpus.rs` and `compose.rs` embedding paths keep `OkapiConfig`.** `InferenceService` handles inference ports only, not embedding. `OkapiEmbedding` is a separate concern.
17. **`CuratorService` normalizes existence check before resolve/dismiss.** API previously checked, CLI didn't. Both surfaces now get consistent `ServiceError::EscalationNotFound`.
18. **`CuratorContext` uses `Option<Arc<CnsRuntime>>` and `Option<Arc<MessageDispatch>>`.** Escalation-only operations don't need them. `run_metacognition` requires both and returns `ServiceError::Cns` if missing.
19. **`MetacognitionSummary` is a service-layer type** because `HealthSnapshot.bot_status_reports` is `pub(crate)` in `hkask-agents`.
20. **`From<&ServiceContext> for CuratorContext` is escalation-only.** Extracting `Arc<CnsRuntime>` from `Arc<RwLock<CnsRuntime>>` requires async, which can't be done in a `From` impl. The `From` impl sets `cns_runtime: None`. `CuratorContext::from_service_context(ctx).await` provides full context.
21. **`EnsembleService` normalizes session-not-found across 10+ call sites.** Both CLI and API now get consistent `ServiceError::SessionNotFound` instead of ad-hoc string formatting.
22. **`map_participant_role` is a public function in `hkask-services`.** Both surfaces had identical `"orchestrator" => Curator` mapping. Now centralized.
23. **Standing sessions excluded from `EnsembleService`.** CLI reads YAML from file path; API constructs from JSON body + MCP tool discovery + gas governance wiring. Too divergent to unify — would require parameterizing surface-specific logic that adds more complexity than it removes.
24. **Improv operations excluded from `EnsembleService`.** `improv_turn` needs a surface-specific inferencer (CLI uses global static; API uses `ApiState.ensemble_inferencer_with_breaker()`). `improv_config`, `set_threshold`, `set_mode` are CLI-only surface operations.
25. **`list_deliberation_sessions` stays as direct `SessionManager` call.** Thin pass-through with no error normalization. Doesn't pass depth test.
26. **`PodContext` holds only `Arc<PodManager>`.** Matches `EnsembleContext` pattern (single field). Pod lifecycle operations need only the pod manager.
27. **`PodService::parse_pod_id()` centralizes UUID parsing.** Was duplicated in 6 call sites (3 CLI: activate, deactivate, status + 3 API: activate, deactivate, status). Returns `ServiceError::PodNotFound` for invalid UUIDs.
28. **`PodService::deactivate_pod()` fixes CLI error swallowing bug.** CLI previously did `let _ = manager.deactivate_pod(...)` — silently ignoring errors. Service layer now propagates errors consistently.
29. **Auth/capability check stays in API surface for `create_pod`.** P1 Prohibition: OCAP capability gating is user sovereignty. Service layer doesn't decide who can create pods.
30. **Persona parsing stays in surface.** CLI reads persona YAML from file; API receives `persona_yaml` string in JSON body. File I/O and request deserialization are surface concerns.
31. **`ServiceError::PodNotFound(String)` sentinel for UUID parse errors and not-found normalization.** Follows `EscalationNotFound`/`SessionNotFound` pattern.
32. **`ServiceError::Pod(AgentPodError)` catch-all for domain errors.** `From<AgentPodError>` maps `PodNotFound(PodID)` to `PodNotFound(String)` sentinel; all other variants map to `Pod(AgentPodError)`.
33. **Spec domain skipped (depth test failed).** 4 meaningful call sites. API spec routes are stubs returning hardcoded values. CLI render/test-invariant/test-verify are surface-only (file I/O, minijinja rendering, print formatting). Capture logic diverges: CLI persists to SpecStore, API returns constructed spec as JSON without persisting. Validate/cultivate use DefaultSpecCurator in CLI only — no API counterpart.
34. **Goal domain skipped (depth test failed).** 12 call sites across 6 operations, but CRUD operations are thin pass-throughs to `SqliteGoalRepository` methods. Parsing helpers (Visibility, GoalState, GoalID) are too thin to justify a full service module. Infrastructure wiring (`open_repository()`) belongs in Task 7.
35. **Models domain skipped (depth test N/A).** Already fully covered by `InferenceService::list_models/search_models`. No additional duplication to extract.
36. **`From<&ServiceContext>` for CuratorContext is escalation-only.** `cns_runtime` is behind `Arc<RwLock<CnsRuntime>>` in ServiceContext and requires async extraction. The `From` impl sets `cns_runtime: None` (suitable for escalation ops only). `CuratorContext::from_service_context(ctx).await` provides full context for metacognition.
37. **`session_manager` added to ServiceContext.** Both CLI and API need SessionManager for ensemble operations. Enables `From<&ServiceContext> for EnsembleContext`. ServiceContext now has 20 fields.
38. **All 5 context types now have `From<&ServiceContext>`.** InferenceContext (pre-existing), PodContext, SovereigntyContext, CuratorContext (escalation-only), EnsembleContext. This is the foundational step for Task 7b where surfaces will compose ServiceContext and derive their contexts.
39. **Surface code now uses ServiceContext for assembly.** Both `ApiState` and `ReplState` compose `ServiceContext::build()` for shared infrastructure. `ApiState::from_service_context()` is the canonical constructor. `init_repl_state()` uses `ServiceContext::build()` internally. `kask serve` and `kask loops` both route through `ServiceContext::build()`. Old assembly code (`ApiState::new()`, `build_loop_system()`, `build_governed_mcp_tool()`, `build_ensemble_session()`, `Stores::init()`) has been deleted.
40. **CAS store write-through is dead code.** 0 call sites for any `*_with_cas()` method or `.with_cas()` builder. The old `Stores::init()` was the only consumer and was deleted in Session 9. Removed `define_store_cas!` macro, 6 `*_with_cas()` methods, `.with_cas()` builders, and `cas_port` fields from 6 stores in `hkask-storage`. All 5 affected stores now use `define_store!`. `SqliteGoalRepository` also had hand-written CAS code removed. The read-only `git_cas_port` (used by API git routes, CLI git commands, MCP git server, CNS snapshot loop) is separate and untouched. F26 CLOSED.
41. **Secret resolution audit: all main paths flow through ServiceConfig.** The 4 main assembly paths (API, serve, loops, REPL) all route through `ServiceConfig::from_env()` or `ServiceConfig::from_secrets()`. Remaining direct `hkask_keystore::resolve_*` calls are by design: (a) domain crate internals that MUST NOT depend on `hkask-services` (P1), (b) standalone CLI commands that don't need a full `ServiceContext`, (c) API route auth checks. No migration needed.
42. **Sovereignty API routes now use `ApiError::from` instead of `ApiError::Internal`.** 3 routes (`get_status`, `grant_consent`, `check_access`) were wrapping `ServiceError` as `ApiError::Internal { message: e.to_string() }` instead of using the `From<ServiceError> for ApiError` adapter. This meant legitimate business errors (e.g., bad category) got 500 instead of 400. Now they use `.map_err(ApiError::from)?` like all other service-wired routes. F14 PARTIALLY ADDRESSED.
43. **Remaining direct `ApiError::` constructions are legitimate surface concerns.** ~11 direct constructions remain in `api/routes/`: `BadRequest` for input parsing/validation, `Forbidden` for OCAP capability gates, `Unauthorized` for auth failures, `NotFound` for surface-only entities (standing sessions, bundles), `Internal` for infrastructure failures without ServiceError path (consolidation, episodic memory, git). These are NOT errors that should flow through ServiceError — they're HTTP-layer concerns.
44. **F9 CLOSED: Production memory stores now respect `config.in_memory`.** `ServiceContext::build()` previously used `in_memory_db()` unconditionally for episodic/semantic stores regardless of `config.in_memory`. Fix: when `!config.in_memory`, opens file-backed encrypted DB via `Database::open()`; when `config.in_memory`, keeps `in_memory_db()`. `ServiceConfig` gains `memory_db_path: Option<String>` (defaults to `{db_path}-memory.db`) and `memory_passphrase: Option<String>` (defaults to `db_passphrase`). `HKASK_MEMORY_DB_PATH` env var supported. P1 User Sovereignty Guardrail satisfied: user configured persistence, user gets persistence.
45. **F10 CLOSED: `#[non_exhaustive]` applied; sub-struct grouping rejected.** Full sub-struct grouping (InfraContext, LoopContext, AgentContext) was analyzed by usage patterns across both surfaces + 5 From impls. Each proposed sub-struct is a data-only container with no behavior — fails depth test. The cost (7+ call sites × 3 surfaces changing `ctx.field` to `ctx.group.field`) outweighs the benefit. `#[non_exhaustive]` alone achieves F10's goal: external crates can't construct `ServiceContext` with struct literals; must use `ServiceContext::build()`.
46. **F7 CLOSED: Default constants centralized; env-var reads audited.** `DEFAULT_DB_PATH` and `DEFAULT_OKAPI_BASE_URL` made public in `ServiceConfig` and re-exported from `hkask-services`. 4 leaked call sites (`commands/config.rs`, `commands/ensemble.rs`, `commands/compose.rs`, `repl/init.rs`) now reference centralized constants instead of duplicating string literals. Remaining direct env-var reads in standalone CLI paths are by design (P1: standalone commands don't need a full `ServiceContext`). `HKASK_MEMORY_DB_PATH` added to `from_env()` and `from_secrets()` as part of F9.

## 6. What Remains

**The original 9-task service layer extraction plan is COMPLETE.** All tasks (1–9) are done: 5 service modules extracted, 4 skipped via depth test, surface assembly migrated, CAS dead code removed, secret resolution audited, error mapping unified, full verification passed, and documentation updated.

**All MEDIUM and HIGH open questions are now CLOSED.** Sessions 11–12 closed all 10 post-extraction questions:
- **F9 (HIGH)** — P1 User Sovereignty: Production memory stores now respect `config.in_memory`
- **F10 (MEDIUM)** — `#[non_exhaustive]` applied; sub-struct grouping rejected by depth test
- **F7 (MEDIUM)** — Default constants centralized; env-var reads audited
- **F2 (MEDIUM)** — By design: CLI and API have fundamentally different session models; shared parts already extracted
- **F3 (MEDIUM)** — By design: Three surfaces have fundamentally different auth models; unified AuthContext fails depth test
- **F6 (MEDIUM)** — Boundary table documented; shared fields in ServiceContext, surface-specific fields correctly placed
- **F14 (MEDIUM)** — All remaining direct ApiError constructions are legitimate surface concerns
- **F17 (MEDIUM)** — By design: P1 Prohibition protects standalone CLI pattern (no ServiceContext forced)
- **F18 (MEDIUM)** — By design: CLI/API standing session divergence too wide; common logic too shallow to extract
- **F19 (MEDIUM)** — By design: Improv operations are CLI-only with no API counterpart

**No further service layer extraction work is warranted.** Every candidate has been audited against the depth test and constraint forces. Remaining open questions are LOW priority or track-only:

### LOW — Track-only questions

| ID | Question | Note |
|----|----------|------|
| F1 | Streaming response support | Implement when a surface requires streaming |
| F8 | GovernedTool membrane boundary | Design GovernedToolFactory if 3+ surfaces need it |
| F11 | InvalidPassphrase vs LoginFailed security | Unify or document distinction |
| F12 | ValidationError(String) too generic | Replace with domain-specific variants |
| F16 | Embedding concern separation | Evaluate OkapiEmbedding vs InferenceService |
| F22 | SovereigntyBoundaryStore reads in CLI Status | Guideline: per-user boundary data from persisted store |

## 7. Open Questions (F1–F26)

**No MEDIUM or HIGH questions remain open.** All have been resolved through code changes or design audits.

| ID | Question | Priority | Status |
|----|----------|----------|--------|
| F1 | Streaming response support | LOW | Deferred |
| F2 | Session lifecycle across surfaces | MEDIUM | **CLOSED (Session 12)** — By design: different session models; shared parts already extracted |
| F3 | Unified authentication context | MEDIUM | **CLOSED (Session 12)** — By design: different auth models; unified AuthContext fails depth test |
| F4 | MCP server service access (by design — out of process) | LOW | By design |
| F5 | Test seam depth for ServiceContext::build() | HIGH | **CLOSED** — 3 API tests + 6 infrastructure tests |
| F6 | REPL vs API state boundary | MEDIUM | **CLOSED (Session 12)** — Boundary table documented |
| F7 | ServiceConfig vs environment variables | MEDIUM | **CLOSED (Session 11)** — Default constants centralized; env-var reads audited |
| F8 | GovernedTool membrane boundary | LOW | Deferred |
| F9 | Production memory stores use `in_memory_db()` | HIGH | **CLOSED (Session 11)** — P1 User Sovereignty Guardrail satisfied |
| F10 | ServiceContext approaching god-object (20 fields) | MEDIUM | **CLOSED (Session 11)** — `#[non_exhaustive]`; sub-structs rejected by depth test |
| F11 | InvalidPassphrase vs LoginFailed security concern | LOW | Track |
| F12 | ValidationError(String) too generic | LOW | Track |
| F13 | CapabilityChecker secret inconsistency | LOW | **CLOSED** — By design |
| F14 | Dual error mapping in API | MEDIUM | **CLOSED (Session 12)** — All remaining direct constructions are legitimate surface concerns |
| F15 | InferenceContext vs ServiceContext | LOW | **CLOSED** — Decided |
| F16 | Embedding concern separation | LOW | Track |
| F17 | CuratorService standalone commands open DB each time | MEDIUM | **CLOSED (Session 12)** — By design: P1 Prohibition protects standalone CLI pattern |
| F18 | EnsembleService standing session extraction | MEDIUM | **CLOSED (Session 12)** — By design: divergence too wide; common logic too shallow |
| F19 | EnsembleService improv operation extraction | MEDIUM | **CLOSED (Session 12)** — By design: CLI-only, no API counterpart |
| F20 | EnsembleService `list_deliberation_sessions` | LOW | **CLOSED** — Pass-through |
| F21 | Memory domain depth test result | LOW | **CLOSED** — Skipped |
| F22 | SovereigntyBoundaryStore reads in CLI Status | Guideline | Per-user boundary data from persisted store |
| F23 | Spec domain depth test result | LOW | **CLOSED** — Skipped |
| F24 | Goal domain depth test result | LOW | **CLOSED** — Skipped |
| F25 | Models domain depth test result | LOW | **CLOSED** — Skipped |
| F26 | ServiceContext stores lack CAS write-through | MEDIUM | **CLOSED (Session 10)** — Dead code removed |

## 8. Mandatory Skills for Next Session

**The service layer extraction is complete. No further extraction work is warranted.** These skills remain useful for any future architecture work or new feature development that touches the service layer:

1. **`refactor-service-layer`** — The strangler fig process, deletion test, depth test. Any future service extraction or architecture work must follow this skill's process.
2. **`coding-guidelines`** — Assess before implementing. Surgical changes only. Every changed line must trace to the task.
3. **`tdd`** — RED→GREEN→REFACTOR per behavior. Every new path gets a `// REQ:` tagged test.
4. **`constraint-forces`** — Classify every design decision by force type. Never silently relax a Prohibition or Guardrail.
5. **`zoom-out`** — Use BEFORE touching ServiceContext or any cross-cutting concern.
6. **`improve-codebase-architecture`** — For evaluating architectural proposals. Use deletion test and depth test.
7. **`diagnose`** — If any work introduces regressions, use the disciplined diagnosis loop.
8. **`handoff`** — Use at session end to capture state for the next agent.

## 9. Architectural Context for Continuation Agent

### InferenceService Design (implemented + wired)

```rust
// inference.rs — InferenceContext + InferenceService (3 public functions)
pub struct InferenceContext {
    pub shared_port: Option<Arc<dyn InferencePort>>,
    pub default_model: String,
    pub okapi_base_url: String,
}

impl InferenceContext {
    pub fn from_parts(shared_port, default_model, okapi_base_url) -> Self
}

impl From<&ServiceContext> for InferenceContext { ... }

pub struct InferenceService;
impl InferenceService {
    pub fn resolve_port(ctx: &InferenceContext, model: &str) -> Result<Arc<dyn InferencePort>, ServiceError>
    pub async fn list_models(ctx: &InferenceContext) -> Result<Vec<ModelInfo>, ServiceError>
    pub async fn search_models(ctx: &InferenceContext, query: &str) -> Result<Vec<ModelInfo>, ServiceError>
}

pub struct ModelInfo {
    pub name: String,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
    pub size_bytes: Option<u64>,
}
```

### CuratorService Design (implemented + wired)

```rust
// curator.rs — CuratorContext + CuratorService (6 public functions) + MetacognitionSummary
pub struct CuratorContext {
    pub escalation_queue: Arc<EscalationQueue>,
    pub cns_runtime: Option<Arc<CnsRuntime>>,        // Required for run_metacognition
    pub dispatch: Option<Arc<MessageDispatch>>,        // Required for run_metacognition
}

impl CuratorContext {
    pub fn from_parts(
        escalation_queue: Arc<EscalationQueue>,
        cns_runtime: Option<Arc<CnsRuntime>,
        dispatch: Option<Arc<MessageDispatch>,
    ) -> Self
    pub async fn from_service_context(ctx: &ServiceContext) -> Self  // Full context with CNS
}
impl From<&ServiceContext> for CuratorContext {  // Escalation-only (cns_runtime: None) }

pub struct MetacognitionSummary {
    pub summary_text: String,
    pub cns_health: String,
    pub variety_counters: Vec<(String, u64)>,
    pub critical_alerts: usize,
    pub total_alerts: usize,
}

pub struct CuratorService;
impl CuratorService {
    // REQ: svc-cur-001
    pub fn list_escalations(ctx: &CuratorContext) -> Result<Vec<EscalationEntry>, ServiceError>
    // REQ: svc-cur-002
    pub fn get_escalation(ctx: &CuratorContext, id: &str) -> Result<Option<EscalationEntry>, ServiceError>
    // REQ: svc-cur-003 — verifies existence before resolving
    pub fn resolve_escalation(ctx: &CuratorContext, id: &str, resolved_by: &str) -> Result<(), ServiceError>
    // REQ: svc-cur-004 — verifies existence before dismissing
    pub fn dismiss_escalation(ctx: &CuratorContext, id: &str, dismissed_by: &str) -> Result<(), ServiceError>
    // REQ: svc-cur-005
    pub fn escalation_stats(ctx: &CuratorContext) -> Result<EscalationStats, ServiceError>
    // REQ: svc-cur-006 — requires cns_runtime and dispatch in context
    pub async fn run_metacognition(ctx: &CuratorContext) -> Result<MetacognitionSummary, ServiceError>
}
```

### EnsembleService Design (implemented + wired)

```rust
// ensemble.rs — EnsembleContext + EnsembleService (8 public functions) + map_participant_role
pub struct EnsembleContext {
    pub session_manager: Arc<RwLock<SessionManager>>,
}

impl EnsembleContext {
    pub fn from_parts(session_manager: Arc<RwLock<SessionManager>>) -> Self
}
impl From<&ServiceContext> for EnsembleContext {  // ctx.session_manager.clone() }

pub fn map_participant_role(role: &str) -> ParticipantRole

pub struct EnsembleService;
impl EnsembleService {
    // REQ: svc-ens-001
    pub async fn create_chat(ctx: &EnsembleContext, session_id: &str) -> Result<(), ServiceError>
    // REQ: svc-ens-002
    pub async fn list_chat_sessions(ctx: &EnsembleContext) -> Result<Vec<String>, ServiceError>
    // REQ: svc-ens-003 — normalizes role mapping + checks existence
    pub async fn register_participant(ctx: &EnsembleContext, session_id: &str, webid: WebID, role: &str, capabilities: Vec<String>) -> Result<(), ServiceError>
    // REQ: svc-ens-004 — checks session existence before sending
    pub async fn send_message(ctx: &EnsembleContext, session_id: &str, sender_webid: WebID, content: &str) -> Result<(), ServiceError>
    // REQ: svc-ens-005
    pub async fn create_deliberation(ctx: &EnsembleContext, session_id: &str) -> Result<(), ServiceError>
    // REQ: svc-ens-006 — checks existence before starting
    pub async fn start_deliberation(ctx: &EnsembleContext, session_id: &str) -> Result<(), ServiceError>
    // REQ: svc-ens-007 — checks existence before recording
    pub async fn record_deliberation_response(ctx: &EnsembleContext, session_id: &str, agent_webid: WebID, content: String, confidence: f64) -> Result<(), ServiceError>
    // REQ: svc-ens-008 — checks existence before synthesizing
    pub async fn synthesize_deliberation(ctx: &EnsembleContext, session_id: &str) -> Result<String, ServiceError>
}
```

### PodService Design (implemented + wired)

```rust
// pods.rs — PodContext + PodService (5 domain operations + 1 helper) + normalize_pod_error
pub struct PodContext {
    pub pod_manager: Arc<PodManager>,
}

impl PodContext {
    pub fn from_parts(pod_manager: Arc<PodManager>) -> Self
}
impl From<&ServiceContext> for PodContext {  // ctx.pod_manager.clone() }

pub struct PodService;
impl PodService {
    // REQ: svc-pod-001 — parse_pod_id validates UUID format
    pub fn parse_pod_id(id: &str) -> Result<PodID, ServiceError>
    // REQ: svc-pod-002 — get_pod_status normalizes not-found errors
    pub async fn get_pod_status(ctx: &PodContext, pod_id: &str) -> Result<PodStatus, ServiceError>
    // REQ: svc-pod-003 — list_pods delegates to PodManager with consistent error mapping
    pub async fn list_pods(ctx: &PodContext) -> Result<Vec<PodStatus>, ServiceError>
    // REQ: svc-pod-004 — create_pod delegates to PodManager with consistent error mapping
    pub async fn create_pod(ctx: &PodContext, template: &str, persona: &AgentPersona, name: Option<String>) -> Result<String, ServiceError>
    // REQ: svc-pod-005 — activate_pod normalizes not-found errors
    pub async fn activate_pod(ctx: &PodContext, pod_id: &str) -> Result<(), ServiceError>
    // REQ: svc-pod-006 — deactivate_pod normalizes not-found errors (fixes CLI error swallowing)
    pub async fn deactivate_pod(ctx: &PodContext, pod_id: &str) -> Result<(), ServiceError>
}
```

### SovereigntyService Design (implemented + wired)

```rust
// sovereignty.rs — SovereigntyContext + SovereigntyService + AccessCheck + SovereigntyStatus
pub struct SovereigntyContext {
    pub consent_manager: Arc<ConsentManager>,
}

impl SovereigntyContext {
    pub fn from_parts(consent_manager: Arc<ConsentManager>) -> Self
}
impl From<&ServiceContext> for SovereigntyContext {  // ctx.consent_manager.clone() }

pub struct AccessCheck {
    pub classification: String,
    pub access_required: String,
    pub has_consent: bool,
}

pub struct SovereigntyStatus {
    pub explicit_consent: bool,
    pub requires_affirmative_consent: bool,
    pub sovereign_data: Vec<String>,
    pub shared_data: Vec<String>,
    pub public_data: Vec<String>,
    pub granted_categories: Vec<String>,
}

pub fn parse_data_category(s: &str) -> DataCategory

pub struct SovereigntyService;
impl SovereigntyService {
    // REQ: svc-sov-001 — parse_data_category maps string to DataCategory
    pub fn parse_data_category(s: &str) -> DataCategory
    // REQ: svc-sov-002 — get_boundary returns the default Magna Carta classification
    pub fn get_boundary() -> DataSovereigntyBoundary
    // REQ: svc-sov-003 — requires_affirmative_consent reflects boundary policy
    pub fn requires_affirmative_consent() -> bool
    // REQ: svc-sov-004 — grant_consent delegates to ConsentManager
    pub fn grant_consent(ctx: &SovereigntyContext, webid: &str, category: &DataCategory) -> Result<(), ServiceError>
    // REQ: svc-sov-005 — revoke_consent revokes all consent for the WebID
    pub fn revoke_consent(ctx: &SovereigntyContext, webid: &str) -> Result<(), ServiceError>
    // REQ: svc-sov-006 — has_consent returns Ok(bool), fails closed
    pub fn has_consent(ctx: &SovereigntyContext, webid: &str, category: &DataCategory) -> Result<bool, ServiceError>
    // REQ: svc-sov-007 — get_granted_categories returns category names
    pub fn get_granted_categories(ctx: &SovereigntyContext, webid: &str) -> Result<Vec<String>, ServiceError>
    // REQ: svc-sov-008 — check_access returns classification, access_required, and has_consent
    pub fn check_access(ctx: &SovereigntyContext, webid: &str, category: &DataCategory) -> Result<AccessCheck, ServiceError>
    // REQ: svc-sov-009 — get_status combines boundary and consent state
    pub fn get_status(ctx: &SovereigntyContext, webid: &str) -> Result<SovereigntyStatus, ServiceError>
}
```

### Surface Wiring Pattern

CLI and API surfaces construct context structs from their own state:

```rust
// CLI ensemble (standalone commands using global SESSION_MANAGER)
let ctx = hkask_services::EnsembleContext::from_parts(get_session_manager());
EnsembleService::create_chat(&ctx, &session).await.map_err(|e| e.to_string())?;

// CLI sovereignty (standalone commands using ConsentManager per-invocation)
let ctx = hkask_services::SovereigntyContext::from_parts(Arc::new(ConsentManager::new(consent_store)));
SovereigntyService::grant_consent(&ctx, &webid.to_string(), &category).map_err(|e| e.to_string())?;

// API sovereignty (from ApiState)
let ctx = hkask_services::SovereigntyContext::from_parts(state.consent_manager.clone());
SovereigntyService::check_access(&ctx, &webid_str, &category).map_err(ApiError::from)?;
```

**Future path (Task 7b):** When surfaces compose a `ServiceContext`, contexts are derived:

```rust
// After Task 7b: surfaces compose ServiceContext once at init
let service_ctx = ServiceContext::build(config).await?;
// Derive any context type from the shared ServiceContext
let ens_ctx: EnsembleContext = (&service_ctx).into();
let pod_ctx: PodContext = (&service_ctx).into();
let sov_ctx: SovereigntyContext = (&service_ctx).into();
let inf_ctx: InferenceContext = (&service_ctx).into();
let cur_ctx: CuratorContext = (&service_ctx).into(); // escalation-only
let cur_ctx = CuratorContext::from_service_context(&service_ctx).await; // full
```

### Completed Call Site Replacements

**CLI (all inference + curator + ensemble sites wired):**
1. `cli/repl/init.rs` — Default + gate inference ports → `InferenceService::resolve_port()`
2. `cli/repl/handlers/hhh.rs` — Gate model switch → `InferenceService::resolve_port()`
3. `cli/repl/handlers/model.rs` — Model listing/search → `InferenceService::search_models()`
4. `cli/commands/chat.rs` — Fallback inference port → `InferenceService::resolve_port()`
5. `cli/commands/compose.rs:275-284` — Generation inference → `InferenceService::resolve_port()`
6. `cli/commands/ensemble.rs:130-140` — Ensemble improv → `InferenceService::resolve_port()`
7. `cli/commands/curator.rs` — All 4 curator operations → `CuratorService::*`
8. `cli/commands/ensemble.rs` — 9 ensemble chat/deliberation operations → `EnsembleService::*`
9. `cli/commands/pod.rs` — 5 pod lifecycle operations → `PodService::*`
10. `cli/commands/sovereignty.rs` — 4 sovereignty operations → `SovereigntyService::*` + `parse_data_category`

**API (all inference + curator + ensemble + pods + sovereignty sites wired):**
1. `api/lib.rs` — `with_ensemble_inferencer()` → `InferenceService::resolve_port()`
2. `api/routes/chat.rs` — Fallback inference → `InferenceService::resolve_port()`
3. `api/routes/models.rs` — `list_models` → `InferenceService::list_models()`
4. `api/routes/models.rs` — `search_models` → `InferenceService::search_models()`
5. `api/routes/curator.rs` — All 4 curator routes → `CuratorService::*`
6. `api/routes/ensemble.rs` — 8 chat/deliberation handlers → `EnsembleService::*`
7. `api/routes/pods.rs` — 5 pod lifecycle handlers → `PodService::*`
8. `api/routes/sovereignty.rs` — 4 sovereignty handlers → `SovereigntyService::*` + `parse_data_category`

**Intentionally NOT replaced (by design):**
- `cli/commands/compose.rs:121-127` — `OkapiConfig` for `OkapiEmbedding` (embedding, not inference)
- `cli/commands/embed_corpus.rs:191-197` — `OkapiConfig` for `OkapiEmbedding` (embedding, not inference)
- MCP server call sites (P1 Prohibition — out of process)
- `cli/commands/ensemble.rs` improv/standing functions (divergent/surface-only, not extracted)
- `api/routes/ensemble.rs` improv/standing handlers (divergent/surface-only, not extracted)
- `cli/commands/pod.rs` PodManager construction (surface concern — `new_mock()` per invocation vs `PodManagerBuilder` for list)

### Constraint Forces (Key Decisions)

| Decision | Force | Rationale |
|----------|-------|-----------|
| InferenceService does NOT cache ports by model | Hypothesis | Needs verification — caching would improve perf but risks stale connections |
| MCP servers do NOT use InferenceService | Prohibition (P1) | Out-of-process servers can't share ServiceContext |
| resolve_port reuses shared port for default model | Guideline | Best practice, normalizes behavior across surfaces |
| list_models/search_models use direct Okapi (not MCP dispatch) | Prohibition | MCP is out-of-process; service layer must not depend on it |
| ModelInfo is a service-layer type, not OkapiModelEntry | Guideline | Surface adapters translate to their own types |
| InferenceContext is the surface-facing type (not ServiceContext) | Guideline | Surfaces shouldn't need to build full ServiceContext for inference calls; full composition deferred to Task 7b |
| ReplState stores ServiceConfig (not OkapiConfig) | Guideline | ServiceConfig provides all needed fields for InferenceContext construction |
| CuratorService resolves/dismisses with existence check | Guideline | Normalizes behavior — API checked, CLI didn't |
| CuratorContext cns_runtime/dispatch are Option | Guideline | Escalation-only ops don't need them; follows InferenceContext.shared_port pattern |
| CuratorAgent constructed fresh per run_metacognition call | Hypothesis | Matches CLI's current behavior; shared MetacognitionLoop is future work |
| EscalationStats re-exported from domain crate | Guideline | Clean domain type, no need for service-layer wrapper |
| MetacognitionSummary is a service-layer type | Guideline | HealthSnapshot.bot_status_reports is pub(crate) |
| From<&ServiceContext> for CuratorContext: escalation-only + async from_service_context | Guideline | Needs async read on RwLock for full context; From impl provides escalation-only |
| EnsembleService normalizes session-not-found | Guideline | 10+ call sites across CLI and API had ad-hoc string formatting |
| map_participant_role is a public helper | Guideline | Both surfaces had identical "orchestrator" → Curator mapping |
| Standing sessions NOT extracted | Divergent | CLI reads YAML; API takes JSON + MCP discovery + gas governance. Unifying would overcomplicate. |
| Improv operations NOT extracted | Divergent/Surface-only | improv_turn needs surface-specific inferencer; config/set ops are CLI-only |
| EnsembleContext has only session_manager | Guideline | Chat/deliberation ops only need SessionManager; standing ops need surface-specific state |
| list_deliberation_sessions stays direct | Pass-through | Thin delegation, no error normalization. Doesn't pass depth test. |
| PodContext holds only Arc<PodManager> | Guideline | Matches EnsembleContext pattern. Pod lifecycle ops need only the pod manager. |
| PodService::parse_pod_id centralizes UUID parsing | Guideline | Was duplicated in 6 call sites. Returns PodNotFound for invalid UUIDs. |
| PodService::deactivate_pod fixes CLI error swallowing | Guardrail | CLI did `let _ = manager.deactivate_pod(...)`. Service propagates errors consistently. |
| Auth/capability check stays in API for create_pod | Prohibition (P1) | OCAP capability gating is user sovereignty. Service doesn't decide who creates pods. |
| Persona parsing stays in surface | Guideline | CLI reads file; API receives JSON body. File I/O and deserialization are surface concerns. |
| CLI PodManager::new_mock() stays in surface | Hypothesis | CLI pod ops are stateless (transient PodManager). Service normalizes ops, not PodManager lifecycle. |
| PodNotFound(String) sentinel for UUID parse errors | Guideline | Follows EscalationNotFound/SessionNotFound pattern. |
| Pod(AgentPodError) catch-all for domain errors | Guideline | PodNotFound(PodID) maps to PodNotFound(String); all other variants pass through. |
| Memory domain skipped (fails depth test) | Evidence | Only 2 call sites for consolidation; episodic ops are P1 OCAP-gated; semantic ops are CLI-only; no duplication |
| SovereigntyContext holds only Arc<ConsentManager> | Guideline | Follows PodContext pattern. Consent/boundary ops need only the consent manager. Store construction is surface concern (Task 7). |
| parse_data_category centralized in service | Guideline | Both CLI and API had identical string-to-DataCategory mapping |
| check_access returns data + consent state; surface decides deny | Prohibition (P1) | OCAP capability gating is user sovereignty. Service doesn't decide who accesses what. |
| revoke_consent fixes CLI bug (spurious grant before revoke) | Guardrail | CLI did `consent_manager.grant_consent()` before `revoke_consent()`. Service only revokes. |
| SovereigntyBoundaryStore reads stay in CLI Status handler | Guideline | Per-user boundary data from persisted store; service returns default boundary. Surface merges both. |

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*