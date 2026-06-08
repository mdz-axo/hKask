# Handoff — hKask Service Layer Extraction

## 1. Session Context

Two sessions have completed work on the 9-task service layer extraction plan. This handoff covers all work done so far and what remains.

**Session 1** (Tasks 1–3): Created the `hkask-services` crate skeleton, extracted `ServiceError`, `ServiceConfig`, and `ServiceContext`. Left 3 clippy errors and no tests.

**Session 2** (Re-audit + Fixes + Task 4 start): Activated all 5 mandatory skills (`refactor-service-layer`, `improve-codebase-architecture`, `coding-guidelines`, `constraint-forces`, `zoom-out`, `tdd`). Ran full Phase 0→1→2 re-audit, found 4 MUST FIX bugs. Fixed all 4 plus 4 SHOULD FIX items. Created `InferenceService` module with 3 public functions and 4 tests. Did NOT wire CLI or API surfaces yet — that's the next step.

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

**NOT yet done:** Wiring CLI and API surfaces to use InferenceService (Phases 4c–4f of the strangler fig).

## 3. Current Module Structure

```
hkask-services/src/
├── lib.rs           — re-exports ServiceConfig, ServiceContext, ServiceError, InferenceService, ModelInfo
├── error.rs         — 31 variants across 9 domain groups + Keystore
├── config.rs        — ServiceConfig with 3 constructors + 8 default constants + template_cache_path
├── context.rs       — ServiceContext::async build() with 18 Arc fields
└── inference.rs     — InferenceService (3 functions) + ModelInfo struct
```

## 4. Verification Status

```
cargo check --workspace  ✅
cargo clippy --workspace -- -D warnings  ✅
cargo test --workspace  ✅
cargo test -p hkask-services  ✅ (4 tests passing)
No todo!/unimplemented! in hkask-services  ✅
```

## 5. Key Decisions

1. **Flat error hierarchy, not nested.** `ServiceError` composes domain errors via `#[from]`. `Keystore(String)` for secret resolution failures.
2. **`ServiceContext::build()` is async.** No more `Runtime::new()` + `block_on()` + `drop(rt)`. Callers `.await` it.
3. **Strangler fig: build alongside, don't replace yet.** Neither `ReplState` nor `ApiState` compose `ServiceContext`.
4. **MCP servers do NOT depend on `hkask-services`.** They use `hkask-templates` primitives directly.
5. **`InferenceService` does NOT cache ports by model.** Each non-default model call creates a fresh `OkapiInference`. Caching is a future Hypothesis.
6. **`InferenceService::resolve_port()` reuses shared port for default model.** Falls back to fresh instance for other models.
7. **No `chat.rs` module.** Agent-specific chat logic is REPL-only. Raw inference is in `InferenceService`.
8. **No `cns.rs` module.** `CnsRuntime` methods are direct delegations. Surfaces call `ctx.cns_runtime` directly.
9. **Memory adapter and loops share the same database connection** via `Arc<Connection>`. Different object instances, same underlying SQLite DB.
10. **CNS event sink uses `primary_conn`** for production persistence, not `in_memory_db()`.
11. **Template cache path is configurable** via `HKASK_TEMPLATE_CACHE_PATH` or `ServiceConfig.template_cache_path`.
12. **Dependency direction: CLI/API → services → domain crates.** Never the reverse.

## 6. What Remains

### IMMEDIATE — Task 4 Completion (InferenceService Wiring)

**Phase 4c — Wire CLI to call InferenceService** (strangler fig: call service alongside existing code)

Replace these call sites with `InferenceService::resolve_port()`:

| File | Current Code | Replacement |
|------|-------------|-------------|
| `cli/repl/init.rs:51-63` | `OkapiConfig::local_dev()` + `OkapiInference::new()` | `InferenceService::resolve_port(&ctx, &config.default_model)` |
| `cli/repl/init.rs:77-81` | `OkapiInference::new()` for gate port | `InferenceService::resolve_port(&ctx, &config.gate_model)` |
| `cli/commands/chat.rs:210-220` | `OkapiConfig::local_dev()` + `OkapiInference::new()` fallback | `InferenceService::resolve_port(&ctx, model)` |
| `cli/commands/compose.rs:121-127, 275-284` | `OkapiConfig::local_dev()` + `OkapiInference::new()` | `InferenceService::resolve_port(&ctx, model)` |
| `cli/commands/embed_corpus.rs:191-197` | `OkapiConfig::local_dev()` + `OkapiInference::new()` | `InferenceService::resolve_port(&ctx, model)` |
| `cli/commands/ensemble.rs:130-140` | `OkapiInference::new()` for improv | `InferenceService::resolve_port(&ctx, model)` |
| `cli/repl/handlers/hhh.rs:55-65` | `OkapiInference::new()` for gate model switch | `InferenceService::resolve_port(&ctx, model)` |

**Phase 4d — Wire API to call InferenceService**

| File | Current Code | Replacement |
|------|-------------|-------------|
| `api/lib.rs:298-308` | `OkapiInference::new()` in `ApiState` init | `InferenceService::resolve_port(&ctx, model)` |
| `api/routes/chat.rs:78-88` | `OkapiConfig::local_dev()` + `OkapiInference::new()` fallback | `InferenceService::resolve_port(&ctx, model)` |
| `api/routes/models.rs:81-91` | `OkapiConfig::local_dev()` + `list_okapi_models()` | `InferenceService::list_models(&ctx).await` |
| `api/routes/models.rs:124-134` | `OkapiConfig::local_dev()` + `search_okapi_models()` | `InferenceService::search_models(&ctx, query).await` |

**Phase 4e — Delete duplication from both surfaces**

After both CLI and API delegate to InferenceService, remove the now-unused `OkapiConfig::local_dev()` calls from the affected files. Keep `OkapiConfig::default()` for any remaining direct uses.

**Phase 4f — Verify**

```bash
cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace
```

**NOTE:** `ServiceContext` is not yet composed by `ReplState` or `ApiState`. The CLI/API wiring in 4c/4d will need to either:
- (a) Create a `ServiceContext` in the CLI/API init and pass it through, OR
- (b) Create just the needed parts (inference port) inline and migrate to full `ServiceContext` composition in Task 7b.

Option (b) is more surgical and follows the strangler fig pattern. The CLI/API keep their existing init paths for now, but the `OkapiInference::new()` calls get replaced with `InferenceService` method calls. The `ServiceContext` composition happens later in Task 7b.

### HIGH — Task 5: Extract CuratorService (proof of concept)

Create `hkask-services/src/curator.rs` with `CuratorService` (6 functions):
- `list_escalations(ctx)` → `Result<Vec<EscalationEntry>, ServiceError>`
- `get_escalation(ctx, id)` → `Result<Option<EscalationEntry>, ServiceError>`
- `resolve_escalation(ctx, id, resolved_by)` → `Result<(), ServiceError>`
- `dismiss_escalation(ctx, id, dismissed_by)` → `Result<(), ServiceError>`
- `escalation_stats(ctx)` → `Result<EscalationStats, ServiceError>`
- `run_metacognition(ctx)` → `Result<MetacognitionSummary, ServiceError>`

Full strangler fig cycle: RED→GREEN→wire CLI→wire API→delete duplication→verify.

### MEDIUM — Tasks 6–9: Remaining services, infrastructure, verification, docs

See HANDOFF.md section 5 from previous session for full details.

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
| F10 | ServiceContext approaching god-object (19 fields) | MEDIUM | Guard with sub-structs |
| F11 | InvalidPassphrase vs LoginFailed security concern | LOW | Track |
| F12 | ValidationError(String) too generic | LOW | Track |
| F13 | CapabilityChecker secret inconsistency (3 checkers, 2 secrets) | MEDIUM | Investigate before Task 7b |
| F14 | Dual error mapping in API (14 direct + ServiceError adapter) | MEDIUM | Planned for Task 7f |

## 8. Mandatory Skills for Next Session

**Load these BEFORE writing any code:**

1. **`refactor-service-layer`** — The strangler fig process, deletion test, depth test, and verification checklist. Phase 4c–4f wiring and deletion must follow this skill's process.
2. **`coding-guidelines`** — Assess before implementing. Surgical changes only.
3. **`tdd`** — Every new service operation gets a RED→GREEN→REFACTOR cycle with `// REQ:` tags.

## 9. Architectural Context for Continuation Agent

### InferenceService Design (already implemented)

```rust
// inference.rs — 3 public functions, all take &ServiceContext
pub struct InferenceService;

impl InferenceService {
    pub fn resolve_port(ctx: &ServiceContext, model: &str) -> Result<Arc<dyn InferencePort>, ServiceError>
    pub async fn list_models(ctx: &ServiceContext) -> Result<Vec<ModelInfo>, ServiceError>
    pub async fn search_models(ctx: &ServiceContext, query: &str) -> Result<Vec<ModelInfo>, ServiceError>
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

### Strangler Fig Wiring Strategy

The wiring approach for Phases 4c/4d is:
- CLI and API do NOT yet compose `ServiceContext`. They keep their existing init paths.
- In CLI call sites: replace `OkapiConfig::local_dev()` + `OkapiInference::new(model, config)` with `InferenceService::resolve_port(&ctx, model)` where `ctx` is a `ServiceContext` constructed via `ServiceContext::build(config).await`.
- In API call sites: same pattern, but using the existing `ApiState` infrastructure to access a `ServiceContext`.
- This means the CLI and API will need a `ServiceContext` instance. The simplest approach is to construct one alongside the existing state and use it only for `InferenceService` calls.
- The full `ServiceContext` composition (replacing `ReplState` and `ApiState`) happens in Task 7b.

### Call Site Inventory (CLI, 7 sites)

1. **`cli/repl/init.rs:51-63`** — Initial inference port creation for default model
2. **`cli/repl/init.rs:77-81`** — Gate model inference port creation
3. **`cli/commands/chat.rs:210-220`** — Fallback inference port for chat
4. **`cli/commands/compose.rs:121-127`** — Compose command inference port (config)
5. **`cli/commands/compose.rs:275-284`** — Compose command inference port (creation)
6. **`cli/commands/embed_corpus.rs:191-197`** — Embed command inference port
7. **`cli/commands/ensemble.rs:130-140`** — Ensemble improv inference port
8. **`cli/repl/handlers/hhh.rs:55-65`** — HHH gate model switch (uses existing `state.okapi_config`)

### Call Site Inventory (API, 4 sites)

1. **`api/lib.rs:298-308`** — `ApiState::with_ensemble_inferencer()` creates inference port
2. **`api/routes/chat.rs:78-88`** — Fallback inference port in chat handler
3. **`api/routes/models.rs:81-91`** — `list_models` route
4. **`api/routes/models.rs:124-134`** — `search_models` route

### MCP Server Call Sites (DO NOT REPLACE — by design)

1. **`hkask-mcp-inference/src/tools.rs:113-117`** — Uses `OkapiConfig::default()` directly
2. **`hkask-mcp-markitdown/src/tools.rs:114-124`** — Uses `self.okapi_config` directly
3. **`hkask-mcp-replicant/src/tools.rs:129-138`** — Uses `self.okapi_base_url` directly

### Phase 1 RDF Triples (Key Duplicated Operations)

For reference, the Phase 1 audit found these 17 duplicated operations. The InferenceService addresses operation #17 (inference_port_creation).

```
(17 inference_port_creation) (duplicates-in) [cli/repl/init.rs, cli/commands/chat.rs, cli/commands/compose.rs, cli/commands/embed_corpus.rs, cli/commands/ensemble.rs, cli/repl/handlers/hhh.rs, api/lib.rs, api/routes/chat.rs, api/routes/models.rs×2, hkask-services/context.rs])
(17 inference_port_creation) (returns) (Arc<dyn InferencePort> × Arc<dyn InferencePort> × Arc<dyn InferencePort>)
(17 inference_port_creation) (divergence) (divergent)
(17 inference_port_creation) (owns) (11 call sites → hkask-services/inference)
```

### Constraint Forces (Key Decisions for InferenceService)

| Decision | Force | Rationale |
|----------|-------|-----------|
| InferenceService does NOT cache ports by model | Hypothesis | Needs verification — caching would improve perf but risks stale connections |
| MCP servers do NOT use InferenceService | Prohibition (P1) | Out-of-process servers can't share ServiceContext |
| resolve_port reuses shared port for default model | Guideline | Best practice, normalizes behavior across surfaces |
| list_models/search_models use direct Okapi (not MCP dispatch) | Prohibition | MCP is out-of-process; service layer must not depend on it |
| ModelInfo is a service-layer type, not OkapiModelEntry | Guideline | Surface adapters translate to their own types |