# Handoff — hKask Service Layer Extraction

## 1. Session Context

This session continued the 9-task service layer extraction plan. Before proceeding to Task 4, a mandatory re-audit was performed using the `refactor-service-layer`, `improve-codebase-architecture`, `coding-guidelines`, `constraint-forces`, `zoom-out`, and `tdd` skills. The re-audit surfaced 4 MUST FIX bugs and 5 SHOULD FIX items. All MUST FIX items have been addressed.

## 2. What Was Done This Session

### Re-Audit (Phase 0→1→2 + Coding Guidelines)
- **Phase 0 (Zoom-Out):** Produced module map, caller graph, data flow, boundary summary, key invariants, depth/deletion tests for all 3 existing modules.
- **Phase 1 (Architecture Audit):** Surfaced 10 candidates: 4 MUST FIX, 5 SHOULD FIX, 2 MAY FIX.
- **Phase 2 (Constraint Forces):** Classified all 10 design decisions. Found Decision #6 (in-memory memory stores) is a **P1 User Sovereignty Guardrail** and Decision #2 (sync build with dropped runtime) is a **Hypothesis requiring verification** (now promoted to Evidence: confirmed bug).
- **Coding Guidelines Assessment:** 4 MUST FIX, 5 SHOULD FIX, 2 MAY FIX. All MUST FIX addressed.

### Fixes Applied

| # | Fix | Category |
|---|-----|----------|
| F1 | Added `ServiceError::Keystore(String)` variant; moved keystore errors from `Cns(String)` | Semantic correctness |
| F2 | Converted `ServiceContext::build()` from sync (`Runtime::new()` + `block_on` + `drop`) to `async fn build()` | Correctness — dangling Handle bug |
| F3 | Added `ApiError::ServiceUnavailable { reason }` variant (503) for keystore errors | Surface adapter |
| F4 | Fixed memory instance sharing: adapter now uses same `mem_conn` as loops (documented sharing pattern) | Data consistency |
| F5 | Added `template_cache_path` field to `ServiceConfig`; removed hardcoded `/tmp/hkask-templates` | Config hygiene |
| F6 | Extracted default constants (`DEFAULT_DB_PATH`, `DEFAULT_OKAPI_BASE_URL`, etc.) to module level | DRY |
| F7 | CNS event sink now uses `primary_conn` instead of `in_memory_db()` | Data loss prevention |
| F8 | Fixed 3 clippy errors: redundant closures, unnecessary_to_owned | P6/P7 compliance |

### Key Design Change: `async fn build()`

`ServiceContext::build()` is now async. This means:
- **CLI callers** need: `rt.block_on(async { ServiceContext::build(config).await })`
- **API callers** need: `ServiceContext::build(config).await`
- **No more dangling `Handle`** — `tokio::runtime::Handle::current()` gets the caller's runtime

## 3. Current State

### Module Structure
```
hkask-services/src/
├── lib.rs           — re-exports ServiceConfig, ServiceContext, ServiceError
├── error.rs         — 31 variants across 9 domain groups + Keystore
├── config.rs        — ServiceConfig with 3 constructors + 8 default constants
└── context.rs       — ServiceContext::async build() with 18 Arc fields
```

### Verification
```
cargo check --workspace  ✅
cargo clippy --workspace -- -D warnings  ✅
cargo test --workspace  ✅
```

### What Has NOT Been Done Yet
- No domain service modules exist yet (inference.rs, curator.rs, etc.)
- Neither ReplState nor ApiState composed with ServiceContext yet (Task 7b)
- No tests in hkask-services (0 `#[test]` functions)
- Memory stores still use `in_memory_db()` for production (see open questions)

## 4. Open Questions (Updated)

| ID | Question | Priority | Status |
|----|----------|----------|--------|
| F1 | Streaming response support | LOW | Deferred |
| F2 | Session lifecycle across surfaces | MEDIUM | Deferred |
| F3 | Unified authentication context | MEDIUM | Deferred |
| F4 | MCP server service access (MCP servers use primitives, not ServiceContext) | LOW | By design |
| F5 | Test seam depth for ServiceContext::build() | HIGH | Must address before Task 7b |
| F6 | REPL vs API state boundary | MEDIUM | Deferred |
| F7 | ServiceConfig vs environment variables (3 places read HKASK_DB_PATH) | MEDIUM | Track |
| F8 | GovernedTool membrane boundary | LOW | Deferred |
| F9 | Production memory stores still use `in_memory_db()` — episodic/semantic memories are ephemeral. Need `memory_db_path`/`memory_passphrase` in ServiceConfig for persistent memory. | HIGH | Track |
| F10 | `ServiceContext` approaching god-object (19 fields). Guard with sub-structs or `#[non_exhaustive]` before further growth. | MEDIUM | Track |
| F11 | `InvalidPassphrase(String)` vs `LoginFailed(String)` — `InvalidPassphrase` leaks whether passphrase was wrong. Should unify for security. | LOW | Track |
| F12 | `ValidationError(String)` is too generic. Consider `ConfigValidation(String)`. | LOW | Track |
| F13 | Three `CapabilityChecker` instances with two different secrets. Verify which secret should be used where. | MEDIUM | Investigate before Task 7b |
| F14 | Dual error mapping in API (14 direct `From<DomainError>` + `From<ServiceError>` adapter). Both paths coexist during strangler fig migration. Delete direct paths in Task 7f. | MEDIUM | Planned for Task 7f |

## 5. What Remains

### Task 4: Extract `InferenceService`
- Create `hkask-services/src/inference.rs` with `InferenceService` struct
- Methods: `get_port()`, `get_or_create()`
- Replace 11 `OkapiConfig::local_dev()` + `OkapiInference::new()` call sites
- TDD: RED→GREEN per behavior, `// REQ:` tags

### Task 5: Extract `CuratorService` (proof of concept)
- Create `hkask-services/src/curator.rs`
- Operations: `list_escalations`, `resolve_escalation`, `dismiss_escalation`, `run_metacognition`
- Full strangler fig cycle: RED→GREEN→wire CLI→wire API→delete duplication
- This validates the entire approach

### Task 6: Extract remaining domain services
### Task 7: Extract cross-cutting infrastructure (wiring)
### Task 8: End-to-end verification
### Task 9: Update documentation

## 6. Key Decisions to Preserve

1. **Flat error hierarchy, not nested.** `ServiceError` composes domain errors directly via `#[from]`. `Keystore(String)` added for secret resolution failures.
2. **`ServiceContext::build()` is now async.** Eliminates the create-and-drop runtime pattern that caused a dangling `Handle` bug. CLI calls `rt.block_on(async { build().await })`, API calls `build().await`.
3. **Strangler fig: build alongside, don't replace yet.** `ServiceContext::build()` exists but neither `ReplState` nor `ApiState` compose it yet.
4. **MCP servers do NOT depend on `hkask-services`.** Out-of-process servers use `hkask-templates` primitives.
5. **`ServiceConfig` has `from_secrets()` for REPL onboarding.** Also has `from_env()` and `in_memory()`.
6. **Memory adapter and loops share the same database connection** via `Arc<Connection>`. Different object instances, same underlying SQLite DB.
7. **CNS event sink uses `primary_conn`** for production persistence, not `in_memory_db()`.
8. **Template cache path is configurable** via `HKASK_TEMPLATE_CACHE_PATH` env var or `ServiceConfig.template_cache_path`.
9. **Default values are centralized** as module-level constants in `config.rs`.
10. **No `todo!` or `unimplemented!`** in `hkask-services`.
11. **Dependency direction: CLI/API → services → domain crates.** Never the reverse.

## 7. Mandatory Skills for Next Session

Same as this session — activate before any code:
1. `refactor-service-layer` — Phase 0→1→2→3 per domain
2. `coding-guidelines` — Assess before implementing
3. `tdd` — RED→GREEN→REFACTOR per behavior
4. `zoom-out` — Before each new extraction
5. `constraint-forces` — Classify each new design decision