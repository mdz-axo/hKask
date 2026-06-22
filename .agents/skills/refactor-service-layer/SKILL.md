---
name: refactor-service-layer
visibility: public
description: >
  Extract a shared service layer from duplicated surface logic (CLI, API, MCP servers)
  using the strangler fig pattern, deep-module discipline, and vertical tracer-bullet TDD.
  Activate when the user says "refactor service layer", "extract shared logic",
  "deduplicate surfaces", or when architectural analysis reveals business logic
  embedded in presentation layers.
composes_skills: [improve-codebase-architecture, tdd, coding-guidelines, constraint-forces, zoom-out, pragmatic-semantics, strangler-fig, deep-module]
---

# Refactor Service Layer

Extract duplicated business logic from surface layers (`hkask-cli`, `hkask-api`, MCP servers) into a shared `hkask-services` crate. This skill **orchestrates** eight other skills in a specific hKask architecture sequence. It does not restate their methodologies — delegation is the point.

## When to Activate

- User says "refactor service layer", "extract shared logic", "deduplicate surfaces"
- Architectural analysis reveals business logic in `hkask-cli/commands/` that duplicates logic in `hkask-api/routes/` or MCP servers
- A domain operation (chat, curator, CNS, ensemble, etc.) exists in two or more surfaces with divergent return types, error handling, or state construction

## Do NOT Activate For

- Adding new features (use `tdd`)
- One-off bug fixes (use `diagnose`)
- Pure presentation changes (terminal formatting, HTTP serialization)
- Refactoring that doesn't extract business logic to a shared crate

## Core Principles

### P1 — Dependency Direction (hKask Architecture)

```
hkask-cli ──→ hkask-services ──→ hkask-agents
hkask-api  ──→ hkask-services ──→ hkask-cns
                                ──→ hkask-memory
                                ──→ hkask-templates
                                ──→ hkask-types
                                ──→ hkask-storage
```

**Rule**: Domain crates NEVER depend on `hkask-services`. Neither `hkask-cli` nor `hkask-api` directly depend on domain crates for business operations. MCP servers do NOT depend on `hkask-services` — they use primitives. Circular dependency = wrong extraction boundary → stop and redesign.

### P2 — hKask Constraint Enforcement

Use `constraint-forces` to classify every design decision. These are the hKask-specific rules:

| Decision | Force | Rationale |
|----------|-------|-----------|
| OCAP gates stay in domain crates | Prohibition (P1) | User Sovereignty is inviolable |
| Service layer is headless | Prohibition (P1.6) | No visual UI ever |
| CNS thresholds are Guardrails | Guardrail | Measured boundary, user-overridable |
| ServiceContext owns dependency graph | Guideline | Best practice, relax with reason |
| InferenceService caching | Hypothesis | Needs verification |
| Error type unification | Evidence | 7 CLI enums, 1 API enum, measured |

When constraints conflict, state the conflict and resolution explicitly. Never silently relax a Prohibition or Guardrail.

## Process

### Phase 0 — Zoom Out

Delegate to `zoom-out`. Produce a map of crates involved, their ownership, caller graph, data flow, boundary summary, and key invariants. This is the **before picture** referenced by every subsequent phase.

### Phase 1 — Audit and Classify

Delegate to `improve-codebase-architecture`. For each domain operation appearing in multiple surfaces, classify as Identical, Divergent, Surface-only, or Pass-through. Apply the `deep-module` deletion test to each candidate before extraction.

### Phase 2 — Classify Constraint Forces

Delegate to `constraint-forces`. Classify every design decision by force type. Document in the plan using the `(Decision) → (Force) → (Rationale)` format. Use the table in P2 above as the starting point.

### Phase 3 — Design the Service Crate

Delegate to `deep-module`. Design `hkask-services` with modules per bounded context:

```
hkask-services/
├── src/
│   ├── lib.rs           — public API, re-exports
│   ├── context.rs        — ServiceContext (shared dependency graph)
│   ├── config.rs         — ServiceConfig (DB path, secrets, thresholds)
│   ├── error.rs          — ServiceError (unified domain error hierarchy)
│   ├── inference.rs      — InferenceService (port factory + model resolution)
│   ├── chat.rs           — ChatService
│   ├── curator.rs        — CuratorService
│   ├── cns.rs            — CNSService
│   ├── ensemble.rs       — EnsembleService
│   ├── pods.rs           — PodService
│   ├── models.rs         — ModelService
│   ├── memory.rs         — MemoryService
│   ├── sovereignty.rs    — SovereigntyService
│   ├── spec.rs           — SpecService
│   └── goal.rs           — GoalService
```

**hKask-specific rules** (on top of `deep-module`'s generic rules):
- Every function takes `&ServiceContext` + domain input, returns `Result<DomainType, ServiceError>`
- No surface types in signatures: no Axum `Json<>`, no CLI `println!` formatting
- Surface-varying configuration goes in `ServiceConfig`, not function signatures

### Phase 4 — Migrate One Domain (Proof of Concept)

Delegate to `strangler-fig` + `tdd`. Select the smallest, most self-contained domain — typically `curator`. Execute the CREATE → WIRE CLI → WIRE API → DELETE sequence. One RED→GREEN→REFACTOR tracer bullet per behavior. Each test anchored on a contract via `expect:` + `[P{N}]`.

### Phase 5 — Migrate Remaining Domains

Delegate to `strangler-fig` + `tdd`. Migrate domains in dependency order, one per commit:

1. `cns` — thin extraction, well-separated in domain crate
2. `chat` — largest; `ChatService::chat_turn` unifies CLI and API
3. `ensemble` — sessions, improv, standing
4. `pods` — CRUD
5. `models` — listing, search
6. `memory` — consolidation, recall, store
7. `sovereignty` — verify, consent
8. `spec` — capture, cultivate, validate
9. `goal` — CRUD

### Phase 6 — Extract Cross-Cutting Infrastructure

After all domain migrations, unify shared infrastructure:

| Step | Action | Target |
|------|--------|--------|
| 6a | Replace all direct inference construction with `InferenceService` | → `InferenceService` |
| 6b | Replace `ReplState`, `ApiState`, `build_loop_system()`, `commands/loops.rs` | → `ServiceContext` |
| 6c | Replace `open_registry_db()`, `Stores::init()`, `ServerContext::open_database()` | → `ServiceContext::build()` |
| 6d | Replace `resolve_acp_secret()` / `CapabilityChecker::new()` | → `ServiceContext::build()` |
| 6e | Replace 4 independent CNS/Loop/EventSink assemblies | → `ServiceContext::build()` |
| 6f | Unify CLI error enums + API `ApiError` | → `ServiceError` |

### Phase 7 — Verify Surgical Completeness

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Verify dependency direction (P1 above). Apply `deep-module` deletion test to every module in `hkask-services`. Verify P6/P7/P8 compliance: no `todo!()`, no `unimplemented!()`, every function carries `expect:` + `[P{N}]` contract annotations.

### Phase 8 — Document Open Questions

Record in `OPEN_QUESTIONS.md`:
- F1: Streaming responses
- F2: Session lifecycle across surfaces
- F3: Unified authentication context
- F4: MCP server service access
- F5: Test seam depth (C8)
- F6: REPL vs API state boundary
- F7: ServiceConfig vs environment variables
- F8: GovernedTool membrane boundary

## Anti-Patterns (hKask-Specific)

These go beyond what `strangler-fig` and `deep-module` already cover:

1. **Surface types leaking into services** — `Json<T>`, `println!`, HTTP status codes in service signatures
2. **Surface context leaking** — `ReplState` or `ApiState` passed into service functions
3. **MCP servers depending on `hkask-services`** — out-of-process servers use primitives, not the service crate
4. **Missing contract annotations** — functions without `expect:` + `[P{N}]` annotations violate P8
5. **OCAP gate extraction** — authorization stays in domain crates, never moves to services

## Checklist Per Domain Migration

```
[ ] RED: Service operation test written                      (→ tdd)
[ ] GREEN: Minimal implementation passes test                   (→ tdd)
[ ] CLI wired: calls service, formats terminal output           (→ strangler-fig)
[ ] API wired: calls service, serializes JSON                   (→ strangler-fig)
[ ] Both surfaces verified: cargo test -p hkask-cli && api      (→ strangler-fig)
[ ] Duplicated logic deleted from both surfaces                 (→ strangler-fig)
[ ] Workspace verified: cargo check --workspace && test         (→ strangler-fig)
[ ] Deletion test passed: service module is deep                (→ deep-module)
[ ] Dependency direction verified: no circular deps             (→ P1 above)
[ ] No todo!/unimplemented!/#[deprecated] in service crate      (→ coding-guidelines)
[ ] clippy clean: cargo clippy -p hkask-services -- -D warnings
```

## End-of-Migration Checklist

```
[ ] Every domain service extracted and both surfaces delegating
[ ] ServiceContext owns all shared state assembly
[ ] InferenceService constructs InferenceRouter from InferenceConfig
[ ] ServiceError unified; surface adapters translate to presentation format
[ ] CNS/Loop/EventSink wiring unified in ServiceContext::build()
[ ] Secret resolution and ACP bootstrap unified in ServiceContext::build()
[ ] DB/Store initialization unified in ServiceContext::build()
[ ] Dependency direction verified (no circular deps)
[ ] cargo check --workspace passes
[ ] cargo test --workspace passes
[ ] cargo clippy --workspace -- -D warnings passes
[ ] Deletion test applied to every module in hkask-services
[ ] Every contract carries `expect:` + `[P{N}]` annotations
[ ] docs/status/corpus_inventory.yaml updated if new test surface was added
[ ] OPEN_QUESTIONS.md updated with F1–F8
```

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `rsl-audit.j2` | KnowAct | Audit and classify duplicated operations across surfaces |
| `rsl-strangle.j2` | KnowAct | Plan strangler fig migration for a selected domain |
| `rsl-verify.j2` | KnowAct | Verify surgical completeness after domain migration |
