# Handoff — hKask Service Layer Extraction

## 1. Session Context

Three sessions have completed work on the 9-task service layer extraction plan. This handoff covers all work done so far and what remains.

**Session 1** (Tasks 1–3): Created the `hkask-services` crate skeleton, extracted `ServiceError`, `ServiceConfig`, and `ServiceContext`. Left 3 clippy errors and no tests.

**Session 2** (Re-audit + Fixes + Task 4 start): Activated all 5 mandatory skills. Ran full Phase 0→1→2 re-audit, found 4 MUST FIX bugs. Fixed all 4 plus 4 SHOULD FIX items. Created `InferenceService` module with 3 public functions and 4 tests. Did NOT wire CLI or API surfaces.

**Session 3** (Task 4 completion — Phases 4c–4f): Wired all CLI (8 sites) and API (4 sites) to call InferenceService. Introduced `InferenceContext` as a lightweight alternative to `ServiceContext` for surface layers. Removed all `OkapiConfig::local_dev()` and `OkapiInference::new()` calls from CLI and API inference sites. All workspace checks pass: `cargo check`, `cargo clippy -D warnings`, `cargo test`.

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
| `ensemble.rs` | 7 (at limit) | Identical | PASS |
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

**Intentionally NOT replaced (by design):**
- `cli/commands/compose.rs:121-127` — `OkapiConfig` for `OkapiEmbedding` (embedding, not inference)
- `cli/commands/embed_corpus.rs:191-197` — `OkapiConfig` for `OkapiEmbedding` (embedding, not inference)
- `api/routes/models.rs` — `ApiState`'s own `OkapiConfig` usages removed
- MCP server call sites (P1 Prohibition — out of process)

## 3. Current Module Structure

```
hkask-services/src/
├── lib.rs           — re-exports ServiceConfig, ServiceContext, ServiceError, InferenceContext, InferenceService, ModelInfo
├── error.rs         — 31 variants across 9 domain groups + Keystore
├── config.rs        — ServiceConfig with 3 constructors + 8 default constants + template_cache_path
├── context.rs       — ServiceContext::async build() with 18 Arc fields
└── inference.rs     — InferenceContext + InferenceService (3 functions) + ModelInfo struct + 4 tests
```

## 4. Verification Status

```
cargo check --workspace                    ✅
cargo clippy --workspace -- -D warnings   ✅
cargo test --workspace                    ✅ (all 4 hkask-services tests passing)
cargo test -p hkask-services             ✅ (4 tests)
No todo!/unimplemented! in hkask-services ✅
No OkapiConfig::local_dev() in CLI or API ✅ (only in embedding uses + ServiceContext::build)
No OkapiInference::new() in CLI or API   ✅ (only in ServiceContext::build + MCP servers)
```

## 5. Key Decisions

1. **Flat error hierarchy, not nested.** `ServiceError` composes domain errors via `#[from]`. `Keystore(String)` for secret resolution failures.
2. **`ServiceContext::build()` is async.** No more `Runtime::new()` + `block_on()` + `drop(rt)`. Callers `.await` it.
3. **Strangler fig: build alongside, don't replace yet.** Neither `ReplState` nor `ApiState` compose `ServiceContext`. They use `InferenceContext` + `ServiceConfig` instead.
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

## 6. What Remains

### HIGH — Task 5: Extract CuratorService (proof of concept)

Create `hkask-services/src/curator.rs` with `CuratorService` (6 functions):
- `list_escalations(ctx)` → `Result<Vec<EscalationEntry>, ServiceError>`
- `get_escalation(ctx, id)` → `Result<Option<EscalationEntry>, ServiceError>`
- `resolve_escalation(ctx, id, resolved_by)` → `Result<(), ServiceError>`
- `dismiss_escalation(ctx, id, dismissed_by)` → `Result<(), ServiceError>`
- `escalation_stats(ctx)` → `Result<EscalationStats, ServiceError>`
- `run_metacognition(ctx)` → `Result<MetacognitionSummary, ServiceError>`

Full strangler fig cycle: RED→GREEN→wire CLI→wire API→delete duplication→verify.

### MEDIUM — Task 6: Extract remaining service modules

Apply the same pattern (InferenceContext-style lightweight context or ServiceContext) for:
- `models.rs` — already partially covered by InferenceService::list_models/search_models, but may need a ModelsService for richer queries
- `ensemble.rs` — ensemble session CRUD
- `pods.rs` — pod lifecycle
- `memory.rs` — episodic/semantic storage ports
- `sovereignty.rs` — consent and verification
- `spec.rs` — spec capture, cultivate, validate
- `goal.rs` — goal CRUD

### MEDIUM — Task 7: Infrastructure unification

- **7a** — Extract cross-cutting infrastructure (DB/Store init, secret resolution, CNS/Loop/EventSink wiring) into ServiceContext::build()
- **7b** — Replace `ReplState` and `ApiState` assemblies with `ServiceContext::build()`. Compose full ServiceContext at CLI/API init instead of the current 4 independent assembly paths.
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
- Write `OPEN_QUESTIONS.md` for F1–F14

## 7. Open Questions (F1–F14)

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
| F15 | InferenceContext vs ServiceContext for service modules | MEDIUM | Decided — InferenceContext for surfaces, ServiceContext for full composition |
| F16 | Embedding concern separation (OkapiEmbedding still uses OkapiConfig) | LOW | Track — embedding may get its own EmbeddingService later |

## 8. Mandatory Skills for Next Session

**Load these BEFORE writing any code:**

1. **`refactor-service-layer`** — The strangler fig process, deletion test, depth test, and verification checklist. Every new service extraction must follow this skill's process.
2. **`coding-guidelines`** — Assess before implementing. Surgical changes only.
3. **`tdd`** — Every new service operation gets a RED→GREEN→REFACTOR cycle with `// REQ:` tags.

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

### ServiceContext Key Fields for InferenceService

```rust
pub struct ServiceContext {
    pub inference_port: Option<Arc<dyn InferencePort>>,  // shared port for default model
    pub config: ServiceConfig,                            // has default_model, okapi_base_url
    // ... 16 other Arc fields
}
```

### ServiceConfig Key Fields

```rust
pub struct ServiceConfig {
    pub db_path: String,
    pub db_passphrase: String,
    pub acp_secret: Vec<u8>,
    pub mcp_secret: Vec<u8>,
    pub okapi_base_url: String,      // <-- InferenceService uses this
    pub cns_threshold: u64,
    pub gas_budget_cap: u64,
    pub gas_replenish_rate: u64,
    pub in_memory: bool,
    pub default_model: String,        // <-- InferenceService uses this
    pub gate_model: String,           // <-- InferenceService uses this
    pub agent_name: String,
    pub template_cache_path: String,
}
```

### Surface Wiring Pattern

CLI and API surfaces construct an `InferenceContext` from their own state:

```rust
// CLI (from ReplState)
let ctx = InferenceContext::from_parts(
    Some(state.inference_port.clone()),
    &state.service_config.default_model,
    &state.service_config.okapi_base_url,
);

// API (from ApiState)
let ctx = InferenceContext::from_parts(
    state.inference_port.clone(),
    &state.service_config.default_model,
    &state.service_config.okapi_base_url,
);

// Standalone commands (from env or args)
let ctx = InferenceContext::from_parts(
    None,
    model_name,
    okapi_base_url,
);
```

This pattern will extend to future service modules. Each service module that needs context from the surface will define its own lightweight context struct (e.g., `CuratorContext`), and the surface will construct it from its state.

### Completed Call Site Replacements

**CLI (all inference port sites wired):**
1. `cli/repl/init.rs` — Default + gate inference ports → `InferenceService::resolve_port()`
2. `cli/repl/handlers/hhh.rs` — Gate model switch → `InferenceService::resolve_port()`
3. `cli/repl/handlers/model.rs` — Model listing/search → `InferenceService::search_models()`
4. `cli/commands/chat.rs` — Fallback inference port → `InferenceService::resolve_port()`
5. `cli/commands/compose.rs:275-284` — Generation inference → `InferenceService::resolve_port()`
6. `cli/commands/ensemble.rs:130-140` — Ensemble improv → `InferenceService::resolve_port()`

**API (all inference/model sites wired):**
1. `api/lib.rs` — `with_ensemble_inferencer()` → `InferenceService::resolve_port()`
2. `api/routes/chat.rs` — Fallback inference → `InferenceService::resolve_port()`
3. `api/routes/models.rs` — `list_models` → `InferenceService::list_models()`
4. `api/routes/models.rs` — `search_models` → `InferenceService::search_models()`

**Intentionally NOT replaced (by design):**
- `cli/commands/compose.rs:121-127` — `OkapiConfig` for `OkapiEmbedding` (embedding, not inference)
- `cli/commands/embed_corpus.rs:191-197` — `OkapiConfig` for `OkapiEmbedding` (embedding, not inference)
- MCP server call sites (P1 Prohibition — out of process)

### Constraint Forces (Key Decisions for InferenceService)

| Decision | Force | Rationale |
|----------|-------|-----------|
| InferenceService does NOT cache ports by model | Hypothesis | Needs verification — caching would improve perf but risks stale connections |
| MCP servers do NOT use InferenceService | Prohibition (P1) | Out-of-process servers can't share ServiceContext |
| resolve_port reuses shared port for default model | Guideline | Best practice, normalizes behavior across surfaces |
| list_models/search_models use direct Okapi (not MCP dispatch) | Prohibition | MCP is out-of-process; service layer must not depend on it |
| ModelInfo is a service-layer type, not OkapiModelEntry | Guideline | Surface adapters translate to their own types |
| InferenceContext is the surface-facing type (not ServiceContext) | Guideline | Surfaces shouldn't need to build full ServiceContext for inference calls; full composition deferred to Task 7b |
| ReplState stores ServiceConfig (not OkapiConfig) | Guideline | ServiceConfig provides all needed fields for InferenceContext construction |

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*