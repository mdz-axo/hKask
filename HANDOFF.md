# Handoff — hKask Service Layer Extraction

## 1. Session Context

Six sessions have completed work on the 9-task service layer extraction plan. This handoff covers all work done so far and what remains.

**Session 1** (Tasks 1–3): Created the `hkask-services` crate skeleton, extracted `ServiceError`, `ServiceConfig`, and `ServiceContext`. Left 3 clippy errors and no tests.

**Session 2** (Re-audit + Fixes + Task 4 start): Activated all 5 mandatory skills. Ran full Phase 0→1→2 re-audit, found 4 MUST FIX bugs. Fixed all 4 plus 4 SHOULD FIX items. Created `InferenceService` module with 3 public functions and 4 tests. Did NOT wire CLI or API surfaces.

**Session 3** (Task 4 completion — Phases 4c–4f): Wired all CLI (8 sites) and API (4 sites) to call InferenceService. Introduced `InferenceContext` as a lightweight alternative to `ServiceContext` for surface layers. Removed all `OkapiConfig::local_dev()` and `OkapiInference::new()` calls from CLI and API inference sites. All workspace checks pass: `cargo check`, `cargo clippy -D warnings`, `cargo test`.

**Session 4** (Task 5 — CuratorService): Extracted `CuratorService` with 6 service operations, `CuratorContext`, and `MetacognitionSummary`. Full strangler fig cycle completed: RED (6 tests) → GREEN (all pass) → Wire CLI → Wire API → Delete duplication → Verify workspace.

**Session 5** (Task 6a — EnsembleService): Extracted `EnsembleService` with 8 service operations, `EnsembleContext`, and `map_participant_role` helper. Full strangler fig cycle completed for chat/deliberation operations. Standing sessions and improv operations intentionally excluded (divergent/surface-only). All workspace checks pass.

**Session 6** (Task 6b — PodService): Extracted `PodService` with 6 service operations (5 domain + 1 helper), `PodContext`, and `normalize_pod_error` helper. Full strangler fig cycle completed. Fixed CLI bug where `deactivate_pod` silently swallowed errors. Both CLI and API now route through `PodService`. All workspace checks pass.

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

## 3. Current Module Structure

```
hkask-services/src/
├── lib.rs           — re-exports: ServiceConfig, ServiceContext, ServiceError, InferenceContext, InferenceService, ModelInfo, CuratorContext, CuratorService, MetacognitionSummary, EnsembleContext, EnsembleService, map_participant_role
├── error.rs         — 31 variants across 9 domain groups + SessionNotFound + Keystore + EscalationNotFound + Cns
├── config.rs        — ServiceConfig with 3 constructors + 8 default constants + template_cache_path
├── context.rs       — ServiceContext::async build() with 18 Arc fields
├── inference.rs     — InferenceContext + InferenceService (3 functions) + ModelInfo struct + 4 tests
├── curator.rs       — CuratorContext + CuratorService (6 functions) + MetacognitionSummary + 6 tests
├── ensemble.rs      — EnsembleContext + EnsembleService (8 functions) + map_participant_role + 11 tests
└── pods.rs          — PodContext + PodService (6 functions) + normalize_pod_error + 6 tests
```

## 4. Verification Status

```
cargo check --workspace                    ✅
cargo clippy --workspace -- -D warnings   ✅
cargo test --workspace                    ✅ (all tests passing)
cargo test -p hkask-services              ✅ (27 tests: 4 inference + 6 curator + 11 ensemble + 6 pods)
No todo!/unimplemented! in hkask-services ✅
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
```

## 5. Key Decisions

1. **Flat error hierarchy, not nested.** `ServiceError` composes domain errors via `#[from]`. `Keystore(String)` for secret resolution failures.
2. **`ServiceContext::build()` is async.** No more `Runtime::new()` + `block_on()` + `drop(rt)`. Callers `.await` it.
3. **Strangler fig: build alongside, don't replace yet.** Neither `ReplState` nor `ApiState` compose `ServiceContext`. They use `InferenceContext`/`CuratorContext`/`EnsembleContext` + `ServiceConfig` instead.
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
20. **`From<&ServiceContext> for CuratorContext` deferred to Task 7b.** Extracting `Arc<CnsRuntime>` from `Arc<RwLock<CnsRuntime>>` requires async, which can't be done in a `From` impl.
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

## 6. What Remains

### MEDIUM — Task 6: Extract remaining service modules

Apply the same pattern (lightweight context like `InferenceContext`/`CuratorContext`/`EnsembleContext`) for:
- ~~`ensemble.rs`~~ — **DONE (Task 6a)**
- ~~`pods.rs`~~ — **DONE (Task 6b)** (6 functions: parse_pod_id, get_pod_status, list_pods, create_pod, activate_pod, deactivate_pod)
- `memory.rs` — episodic/semantic storage ports (5 functions: episodic store/recall, semantic store/recall, consolidation trigger)
- `sovereignty.rs` — consent and verification (4 functions)
- `spec.rs` — spec capture, cultivate, validate (4 functions)
- `goal.rs` — goal CRUD (3 functions)
- `models.rs` — already partially covered by `InferenceService::list_models/search_models`; apply depth test first

### MEDIUM — Task 7: Infrastructure unification

- **7a** — Extract cross-cutting infrastructure (DB/Store init, secret resolution, CNS/Loop/EventSink wiring) into ServiceContext::build()
- **7b** — Replace `ReplState` and `ApiState` assemblies with `ServiceContext::build()`. Compose full ServiceContext at CLI/API init instead of the current 4 independent assembly paths. Add `From<&ServiceContext> for InferenceContext` and `From<&ServiceContext> for CuratorContext`.
- **7c** — Extract DB/Store init from surface layers
- **7d** — Extract secret resolution from surface layers
- **7e** — Extract CNS/Loop/EventSink wiring from surface layers
- **7f** — Unify error mapping: `ServiceError` → CLI error enums and `ApiError`

### MEDIUM — Task 8: Verification

- Depth test every module in `hkask-services`
- Dependency direction verification (no circular deps)
- `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`
- Deletion test: removing any service module should cause complexity to reappear in 8+ call sites

### LOW — Task 9: Documentation

- Update `docs/status/test-inventory.md`
- Update `docs/architecture/hKask-architecture-master.md` with service layer section
- Write `OPEN_QUESTIONS.md` for F1–F17

## 7. Open Questions (F1–F17)

| ID | Question | Priority | Status |
|----|----------|----------|--------|
| F1 | Streaming response support | LOW | Deferred |
| F2 | Session lifecycle across surfaces | MEDIUM | Deferred |
| F3 | Unified authentication context | MEDIUM | Deferred |
| F4 | MCP server service access (by design — out of process) | LOW | By design |
| F5 | Test seam depth for ServiceContext::build() | HIGH | Must address before Task 7b |
| F6 | REPL vs API state boundary | MEDIUM | Deferred |
| F7 | ServiceConfig vs environment variables (3 places read HKASK_DB_PATH) | MEDIUM | Track |
| F8 | GovernedTool membrane boundary | LOW | Deferred |
| F9 | Production memory stores use `in_memory_db()` | HIGH | Track — P1 User Sovereignty |
| F10 | ServiceContext approaching god-object (19+ fields) | MEDIUM | Guard with sub-structs |
| F11 | InvalidPassphrase vs LoginFailed security concern | LOW | Track |
| F12 | ValidationError(String) too generic | LOW | Track |
| F13 | CapabilityChecker secret inconsistency (3 checkers, 2 secrets) | MEDIUM | Investigate before Task 7b |
| F14 | Dual error mapping in API (14 direct + ServiceError adapter) | MEDIUM | Planned for Task 7f |
| F15 | InferenceContext vs ServiceContext for service modules | MEDIUM | Decided — lightweight context for surfaces, ServiceContext for full composition |
| F16 | Embedding concern separation (OkapiEmbedding still uses OkapiConfig) | LOW | Track — embedding may get its own EmbeddingService later |
| F17 | CuratorService standalone commands still open DB each time | MEDIUM | Track — ReplState has escalation_queue; standalone kask curator commands could reuse it |
| F18 | EnsembleService standing session extraction | MEDIUM | Deferred — divergent CLI/API flows need parameterization |
| F19 | EnsembleService improv operation extraction | MEDIUM | Deferred — divergent inferencer setup needs surface-specific abstraction |
| F20 | EnsembleService `list_deliberation_sessions` depth test result | LOW | Pass-through — stays as direct SessionManager call |

## 8. Mandatory Skills for Next Session

**Load these BEFORE writing any code:**

1. **`refactor-service-layer`** — The strangler fig process, deletion test, depth test, and verification checklist. Every new service extraction must follow this skill's process.
2. **`coding-guidelines`** — Assess before implementing. Surgical changes only.
3. **`tdd`** — Every new service operation gets a RED→GREEN→REFACTOR cycle with `// REQ:` tags.
4. **`constraint-forces`** — Classify every design decision by force type before implementing.

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
        cns_runtime: Option<Arc<CnsRuntime>>,
        dispatch: Option<Arc<MessageDispatch>>,
    ) -> Self
}
// From<&ServiceContext> for CuratorContext deferred to Task 7b (needs async)

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
// From<&ServiceContext> for PodContext deferred to Task 7b

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

### Surface Wiring Pattern

CLI and API surfaces construct context structs from their own state:

```rust
// CLI ensemble (standalone commands using global SESSION_MANAGER)
let ctx = hkask_services::EnsembleContext::from_parts(get_session_manager());
EnsembleService::create_chat(&ctx, &session).await.map_err(|e| e.to_string())?;

// API ensemble (from ApiState)
let ctx = hkask_services::EnsembleContext::from_parts(state.session_manager.clone());
EnsembleService::register_participant(&ctx, &session, WebID::new(), &role, vec![]).await
    .map_err(ApiError::from)?;
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

**API (all inference + curator + ensemble + pods sites wired):**
1. `api/lib.rs` — `with_ensemble_inferencer()` → `InferenceService::resolve_port()`
2. `api/routes/chat.rs` — Fallback inference → `InferenceService::resolve_port()`
3. `api/routes/models.rs` — `list_models` → `InferenceService::list_models()`
4. `api/routes/models.rs` — `search_models` → `InferenceService::search_models()`
5. `api/routes/curator.rs` — All 4 curator routes → `CuratorService::*`
6. `api/routes/ensemble.rs` — 8 chat/deliberation handlers → `EnsembleService::*`
7. `api/routes/pods.rs` — 5 pod lifecycle handlers → `PodService::*`

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
| From<&ServiceContext> for CuratorContext deferred | Guideline | Needs async read on RwLock; add in Task 7b |
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

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*