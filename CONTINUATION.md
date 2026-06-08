# Continuation Prompt — hKask Service Layer Extraction

## Session Context

You are continuing the hKask service layer extraction. This is **Session 8**. Sessions 2–7 completed Tasks 4, 5, 6a, 6b, 6c (skipped), and 6d. You must load the required skills BEFORE writing any code.

## Mandatory Skills (Load First)

1. **`refactor-service-layer`** — Strangler fig process, depth test, verification checklist
2. **`coding-guidelines`** — Surgical changes only. Think before coding.
3. **`tdd`** — RED→GREEN→REFACTOR per operation with `// REQ:` tags
4. **`constraint-forces`** — Classify every design decision by force type

## What Was Completed

| Task | Module | Status | Details |
|------|--------|--------|---------|
| 4 | `inference.rs` | ✅ DONE | 3 functions, 4 tests |
| 5 | `curator.rs` | ✅ DONE | 6 functions, 6 tests |
| 6a | `ensemble.rs` | ✅ DONE | 8 functions, 11 tests |
| 6b | `pods.rs` | ✅ DONE | 6 functions, 6 tests |
| 6c | `memory.rs` | ❌ SKIPPED | Fails depth test — 2 call sites, P1 OCAP-gated, CLI-only semantic, consolidation is Task 7 infrastructure |
| 6d | `sovereignty.rs` | ✅ DONE | 9 functions + 2 types, 13 tests |

**Current test count:** 40 service-layer tests (4 inference + 6 curator + 11 ensemble + 6 pods + 13 sovereignty)

## What Remains

### Task 6e — `spec.rs` (4 functions: capture, cultivate, validate, list)

Apply the **depth test** first. Find all call sites in CLI (`commands/spec.rs`) and API (`routes/spec.rs`). If deleting this service would cause complexity to reappear in fewer than 8 call sites, **skip it** and document why.

### Task 6f — `goal.rs` (3 functions: create, list, update)

Apply the **depth test** first. Find all call sites in CLI (`commands/goal.rs`) and API (`routes/goal.rs`). If fewer than 8 call sites, skip.

### Task 6g — `models.rs` (depth test first)

Already partially covered by `InferenceService::list_models/search_models`. If all call sites are already covered, **skip entirely**.

### After Task 6 — Task 7: Infrastructure Unification

- **7a** — Extract DB/Store init, secret resolution, CNS/Loop/EventSink wiring into `ServiceContext::build()`
- **7b** — Replace `ReplState` and `ApiState` assemblies. Add `From<&ServiceContext>` for each context type.
- **7c** — Extract DB/Store init from surfaces
- **7d** — Extract secret resolution from surfaces
- **7e** — Extract CNS/Loop/EventSink wiring from surfaces
- **7f** — Unify error mapping: `ServiceError` → CLI error enums and `ApiError`

### Task 8: Verification

- Depth test every module
- Dependency direction check
- Full workspace build + clippy + test

### Task 9: Documentation

- Update `docs/status/test-inventory.md`
- Update `docs/architecture/hKask-architecture-master.md`
- Write `OPEN_QUESTIONS.md` for F1–F22

## Established Patterns (Follow These)

Five service extractions have established the **lightweight context pattern**:

1. **`InferenceContext`** — `Option<Arc<dyn InferencePort>>`, `String`, `String`
2. **`CuratorContext`** — `Arc<EscalationQueue>`, `Option<Arc<CnsRuntime>>`, `Option<Arc<MessageDispatch>>`
3. **`EnsembleContext`** — `Arc<RwLock<SessionManager>>`
4. **`PodContext`** — `Arc<PodManager>`
5. **`SovereigntyContext`** — `Arc<ConsentManager>`

All follow: `from_parts()` for surfaces, `From<&ServiceContext>` deferred to Task 7b.

## Key Files to Read First

| File | Purpose |
|------|---------|
| `HANDOFF.md` | Full context and status (Sections 5, 6, 9) |
| `crates/hkask-services/src/sovereignty.rs` | Most recent extraction — reference implementation |
| `crates/hkask-services/src/pods.rs` | Simplest context pattern reference |
| `crates/hkask-services/src/ensemble.rs` | Session manager context pattern reference |
| `crates/hkask-services/src/lib.rs` | Module re-exports |
| `crates/hkask-services/src/error.rs` | ServiceError variants (check if new variants needed) |
| `crates/hkask-cli/src/errors.rs` | CLI error adapters (existing `From<ServiceError>` impls) |
| `crates/hkask-api/src/error.rs` | API error adapters (existing `From<ServiceError>` impl) |

## Priority Order

1. **Task 6e** — `spec.rs` (depth test → extract or skip)
2. **Task 6f** — `goal.rs` (depth test → extract or skip)
3. **Task 6g** — `models.rs` (depth test → likely skip entirely)
4. **Task 7** — Infrastructure unification (after all 6x modules decided)
5. **Task 8** — Verification
6. **Task 9** — Documentation

## Constraint Forces to Preserve

- **P1 Prohibition**: MCP servers do NOT depend on `hkask-services`. Do NOT modify any `mcp-servers/` code.
- **P5 One Domain Per Commit**: One module at a time.
- **P3 Strangler Fig**: Both old and new paths must work before deleting old code.
- **No `todo!` or `unimplemented!`** in `hkask-services`.
- **Dependency direction**: CLI/API → services → domain. Never the reverse.
- **Surgical changes**: Only modify what's needed for the current module extraction.
- **Surface context pattern**: Each service module defines its own lightweight context struct. Surfaces construct it from their state.
- **Depth test**: Before extracting, verify 8+ call sites would benefit. If not, deepen or skip.

## Key Decisions from Previous Sessions (32 decisions)

Read `HANDOFF.md` Section 5 for the full table. Critical ones for upcoming work:

| # | Decision | Force |
|---|----------|-------|
| 6 | `InferenceContext` is surface-facing (not ServiceContext) | Guideline |
| 15 | `ReplState` stores `ServiceConfig` instead of `OkapiConfig` | Guideline |
| 30 | `ServiceError::Consent(ConsentError)` via `#[from]` | Guideline |
| F9 | Production memory stores use `in_memory_db()` | HIGH — P1 User Sovereignty |
| F10 | ServiceContext approaching god-object (19+ fields) | MEDIUM — guard with sub-structs |
| F13 | CapabilityChecker secret inconsistency (3 checkers, 2 secrets) | MEDIUM — investigate before Task 7b |
| F14 | Dual error mapping in API (14 direct + ServiceError adapter) | MEDIUM — planned for Task 7f |
| F21 | Memory domain skipped (depth test failed) | CLOSED |

## For Each Module Extraction, Follow the Strangler Fig Cycle

1. **RED**: Write one failing test per service operation with `// REQ:` tags
2. **GREEN**: Implement the service operation with minimal code
3. **Wire CLI**: Change CLI to call service alongside existing code
4. **Wire API**: Change API to call same service
5. **Delete duplication**: Remove duplicated logic from both surfaces
6. **Verify**: `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`

## Spec Domain Starting Points

- `crates/hkask-cli/src/commands/spec.rs` — CLI spec command
- `crates/hkask-api/src/routes/spec.rs` — API spec routes
- `crates/hkask-templates/src/` — Domain crate for specs

## Goal Domain Starting Points

- `crates/hkask-cli/src/commands/goal.rs` — CLI goal command
- `crates/hkask-api/src/routes/goal.rs` — API goal routes
- `crates/hkask-storage/src/goals.rs` — Domain crate for goals

## Verification Checklist Per Module

```
[ ] RED: Service operation test written with // REQ: tag
[ ] GREEN: Minimal implementation passes test
[ ] CLI wired: calls service, formats terminal output
[ ] API wired: calls service, serializes JSON
[ ] Both surfaces verified: cargo test -p hkask-cli && cargo test -p hkask-api
[ ] Duplicated logic deleted from both surfaces
[ ] Workspace verified: cargo check --workspace && cargo test --workspace
[ ] Depth test passed: service module is deep, not shallow
[ ] Dependency direction verified: no circular deps
[ ] No todo!/unimplemented!/#[deprecated] in service crate
[ ] clippy clean: cargo clippy -p hkask-services -- -D warnings
```

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*