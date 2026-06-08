# Continuation Prompt — hKask Service Layer Extraction, Session 7

## Context

You are continuing the hKask service layer extraction. **Before writing any code, you MUST activate the following skills in order:**

1. **`refactor-service-layer`** — Load it, read it, follow its Phase 5 process for the next domain extraction
2. **`coding-guidelines`** — Assess before implementing. Surgical changes only.
3. **`tdd`** — Every new service operation gets a RED→GREEN→REFACTOR cycle with `// REQ:` tags.
4. **`constraint-forces`** — Classify every design decision by force type (Prohibition, Guardrail, Guideline, Evidence, Hypothesis) before implementing.

Read `HANDOFF.md` at the project root for the full status. The critical sections are:
- **Section 2**: What was done (6 sessions, Tasks 4, 5, 6a, and 6b complete)
- **Section 5**: Key decisions (32 decisions that constrain all future work)
- **Section 6**: What remains (Tasks 6c–6g, 7, 8, 9)
- **Section 9**: Architectural context (InferenceContext, CuratorContext, EnsembleContext, PodContext patterns, surface wiring pattern, constraint forces)

### What Was Completed

**Task 4 (InferenceService)** — fully complete (4 phases done, 3 public functions, 4 tests).
**Task 5 (CuratorService)** — fully complete (6 phases done, 6 public functions, 6 tests).
**Task 6a (EnsembleService)** — fully complete (6 phases done, 8 public functions, 11 tests).
**Task 6b (PodService)** — fully complete (6 phases done, 6 public functions, 6 tests).

Key achievements for 6b:
- `PodContext` with `pod_manager: Arc<PodManager>` — the simplest context yet (single field, matches EnsembleContext pattern)
- `PodService::parse_pod_id()` — centralizes UUID parsing from 6 call sites into one service method
- **Fixed CLI bug**: `deactivate_pod` was silently swallowing errors (`let _ = ...`). Service layer now propagates errors consistently.
- Auth/capability check for `create_pod` stays in API surface (P1 Prohibition: OCAP capability gating is user sovereignty)
- Persona YAML parsing stays in surface (CLI reads file, API receives JSON body)
- `ServiceError::PodNotFound(String)` and `ServiceError::Pod(#[from] AgentPodError)` added with proper mapping in both CLI and API error adapters
- All 27 service-layer tests passing, workspace compiles clean with clippy `-D warnings`

### Established Patterns (Follow These)

Four service extractions have established the **lightweight context pattern**:

1. **`InferenceContext`** — `Option<Arc<dyn InferencePort>>`, `String`, `String` — surfaces construct from their own state
2. **`CuratorContext`** — `Arc<EscalationQueue>`, `Option<Arc<CnsRuntime>>`, `Option<Arc<MessageDispatch>>` — escalation-only needs just the queue; metacognition needs all three
3. **`EnsembleContext`** — `Arc<RwLock<SessionManager>>` — chat/deliberation ops need only the session manager
4. **`PodContext`** — `Arc<PodManager>` — pod lifecycle ops need only the pod manager

All follow the same pattern: `from_parts()` for surfaces, `From<&ServiceContext>` deferred to Task 7b.

### What You Should Do Next: Task 6c — memory.rs

The next module to extract is **memory.rs** (episodic/semantic storage). Before implementing, you must:

1. **Zoom out** on the memory domain — find all call sites in CLI and API
2. **Apply the depth test** — does deleting this service cause complexity to reappear in 8+ call sites? If not, deepen or merge.
3. **Classify operations** as Identical, Divergent, Surface-only, or Pass-through using the audit framework from `refactor-service-layer`
4. **Define the context struct** with only the fields the service needs
5. **Write tests first (RED)**, then implement (GREEN), then wire surfaces, then delete duplication, then verify

### Key Files to Read First

- `HANDOFF.md` — Full context and status (especially Sections 5, 6, 9)
- `crates/hkask-services/src/pods.rs` — Reference implementation showing the PodContext pattern (simplest context)
- `crates/hkask-services/src/ensemble.rs` — Reference implementation showing the EnsembleContext pattern
- `crates/hkask-services/src/curator.rs` — Reference implementation showing the CuratorContext pattern (optional fields)
- `crates/hkask-services/src/inference.rs` — Reference implementation showing the InferenceContext pattern
- `crates/hkask-services/src/lib.rs` — Module re-exports
- `crates/hkask-services/src/error.rs` — ServiceError variants (check if new variants needed for next domains)
- `crates/hkask-cli/src/errors.rs` — CLI error adapters (existing `From<ServiceError>` impls)
- `crates/hkask-api/src/error.rs` — API error adapters (existing `From<ServiceError>` impls)

### Memory Domain Starting Points

Look at these files to understand the memory domain:
- `crates/hkask-memory/` — Domain crate for episodic and semantic memory
- `crates/hkask-cli/src/commands/consolidation.rs` — CLI consolidation command
- `crates/hkask-api/src/routes/consolidation.rs` — API consolidation routes
- `crates/hkask-api/src/routes/episodic.rs` — API episodic memory routes

### The Strangler Fig Cycle (Follow For Each Module)

1. **RED**: Write one failing test per service operation with `// REQ:` tags
2. **GREEN**: Implement the service operation with minimal code
3. **Wire CLI**: Change CLI to call service alongside existing code
4. **Wire API**: Change API to call same service
5. **Delete duplication**: Remove duplicated logic from both surfaces
6. **Verify**: `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`

### Priority Order for Remaining Modules

- **6c** — `memory.rs` (5 functions: episodic store/recall, semantic store/recall, consolidation trigger)
- **6d** — `sovereignty.rs` (4 functions: consent grant/check/verify, audit)
- **6e** — `spec.rs` (4 functions: capture, cultivate, validate, list)
- **6f** — `goal.rs` (3 functions: create, list, update)
- **6g** — `models.rs` — **Apply depth test first.** `InferenceService` already provides `list_models`/`search_models`. If all call sites are already covered, skip this module entirely.

### Constraints to Preserve

- **P1 Prohibition**: MCP servers do NOT depend on `hkask-services`. Do NOT modify any `mcp-servers/` code.
- **P5 One Domain Per Commit**: This is still Task 6 (one module at a time). Do not start Task 7+ in the same commit.
- **P3 Strangler Fig**: Both old and new paths must work before deleting old code.
- **No `todo!` or `unimplemented!`** in `hkask-services`.
- **Dependency direction**: CLI/API → services → domain. Never the reverse.
- **Surgical changes**: Only modify what's needed for the current module extraction. Don't refactor adjacent code.
- **Surface context pattern**: Each service module defines its own lightweight context struct. Surfaces construct it from their state.
- **Depth test**: Apply before extracting. If deletion wouldn't cause complexity to reappear in 8+ call sites, deepen or merge.

### Recommended Strategy

1. Start with **`zoom-out`** on the memory domain to identify all call sites in CLI and API.
2. Apply the depth test: does deleting this service cause complexity to reappear in 8+ call sites?
3. Classify each operation as Identical, Divergent, Surface-only, or Pass-through.
4. Define the context struct with only the fields the service needs.
5. Write tests first (RED), then implement (GREEN), then wire surfaces, then delete duplication, then verify.
6. After memory, proceed to sovereignty, spec, goal, then models (depth test first for models).