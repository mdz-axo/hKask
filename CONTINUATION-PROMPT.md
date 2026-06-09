# CONTINUATION PROMPT — hKask Post-Extraction Follow-Up (Session 25+)

**Status:** Service layer extraction ✅ COMPLETE (Session 23). Follow-ups F9 and F5
✅ COMPLETE (Session 24). This document covers remaining follow-up work identified
during Sessions 12–24.

---

## Skills to Load (Required Before Starting)

Load these skills **before** any follow-up work. They provide the methodology and
discipline that govern all changes to `hkask-services` and related crates:

1. **`refactor-service-layer`** — **Required.** Governing methodology for all service-layer
   changes. Defines the strangler fig sequence (P1), depth test (P2), dependency
   direction (P3), surgical changes (P5), and the per-extraction checklist. **Any change
   that touches `hkask-services` MUST follow this skill's principles**, even if it's
   not a full extraction. The depth test applies to any new module or type introduction.
   The dependency direction rule (CLI → services → domain) is inviolable.

2. **`coding-guidelines`** — **Required.** Surgical changes only: every changed line
   traces to the task. No "while we're here" refactors. No renaming. No comment additions.
   Match existing style. Think before coding — state assumptions explicitly.

3. **`constraint-forces`** — **Required.** Classify every design decision by force type.
   Use when deciding whether a change belongs in the service layer vs. domain crate
   vs. surface. Particularly critical for any OCAP-related change — the OCAP gates
   that port traits enforce are Guardrails.

4. **`zoom-out`** — **Required before starting any follow-up task.** Produce the module
   map, caller graph, and data flow "before picture" for the target area. This ensures
   you understand what the service layer currently owns vs. what the domain crates own
   before making changes.

5. **`diagnose`** — **Recommended.** If any follow-up change breaks the workspace build
   or tests, use this skill to trace the root cause before making speculative fixes.

6. **`magna-carta-verifier`** — **If working on any OCAP/sovereignty change.** Provides
   context on sovereignty compliance structures. Any change to port traits or
   `MemoryError` affects OCAP enforcement and must be verified against P1 (User
   Sovereignty) and P2 (Affirmative Consent).

---

## Read These Files First (In This Order)

1. **`HANDOFF.md`** — Full session history (Sessions 12–24), key decisions (#1–#74),
   deep service module inventory (§3), file reference map (§6), completion metrics
   and open questions (§7).

2. **`CONTINUATION.md`** — Session 24 summary with F9/F5 completion, task status
   matrix, and verification results.

3. **`.agents/skills/refactor-service-layer/SKILL.md`** — The governing methodology.
   Re-read the depth test (P2), dependency direction (P3), and anti-patterns sections
   before starting any task that touches the service layer.

---

## Context

The hKask service layer extraction project (Sessions 12–23) produced:

- **10 deep service modules** — ChatService, AgentService, UserService, ComposeService,
  OnboardingService, ArchivalService, EmbedService, SkillService, VerificationService,
  ConsolidationService.
- **2 medium-deep modules** — SpecService, EnsembleService (with improv ops).
- **4 shallow modules** (pre-existing) — PodService, CuratorService, SovereigntyService,
  GoalService.
- **8 depth-test skips** — CnsService, KeystoreService, McpService, GitCasService,
  LoopSystemService, ServeService, TemplateService, MemoryService.
- **1 OCAP fix** — `routes/episodic.rs` stringly-typed errors replaced with typed
  `MemoryError::CapabilityDenied` matching (Session 23, #72).
- **1 typed DTO** — `RecalledEpisode` replaces `Vec<serde_json::Value>` from
  `EpisodicStoragePort::recall_episodic` (Session 24, #73).
- **1 test fixture fix** — `PodManager::new_mock()` uses deterministic test ACP secret
  so 4 pod tests pass without `HKASK_ACP_SECRET_KEY` (Session 24, #74).
- **138 service-layer tests** across all modules.
- **17/27 CLI commands** fully extracted to service layer.
- **10/27 CLI commands** documented as surface-only.

**Verification baseline:**
```
cargo check --workspace    ✅
cargo clippy -p hkask-agents -p hkask-services -p hkask-api -- -D warnings  ✅
cargo test --workspace --exclude hkask-mcp-condenser  ✅ (138 passed in hkask-services, 0 failed)
```

Note: `hkask-mcp-condenser` has a pre-existing build failure (uses renamed
`McpToolError::internal_error` → `McpToolError::internal`). This is unrelated to
service-layer or follow-up work.

---

## Follow-Up Tasks (Priority-Ordered)

### Task 1: F10 — Typed DTOs for SemanticStoragePort (Medium Priority)

**Problem:** `SemanticStoragePort::recall_semantic()` still returns
`Vec<serde_json::Value>`. `ChatService::recall_semantic` destructures it with
`t.get("value").and_then(|v| v.as_str())` — the same fragile pattern that F9 fixed
for episodic recall. If storage field names change, the chat service silently produces
`None` instead of failing at compile time.

**Goal:** Add a `RecalledSemantic` struct and change `recall_semantic` to return
`Vec<RecalledSemantic>` instead of `Vec<serde_json::Value>`.

**Scope:**
- `hkask-agents/src/ports/memory_storage.rs` — Define `RecalledSemantic` struct;
  change `SemanticStoragePort::recall_semantic` return type.
- `hkask-agents/src/adapters/memory_loop_adapter.rs` — Update implementation to
  construct `RecalledSemantic` from `Triple`. Replace `triple_to_json` with
  `triple_to_recalled_semantic` (or remove `triple_to_json` if no longer used).
- `hkask-services/src/chat.rs` — Simplify `ChatService::recall_semantic` to use
  typed field access instead of `.get("value").and_then()`.
- `hkask-agents/src/pod/context.rs` — Update `PodContext::recall_semantic` return type.
- Any MCP semantic server that calls `recall_semantic` — update return type handling.

**Depth test before starting:** `RecalledSemantic` lives in `hkask-agents` (same as
`RecalledEpisode`). Deleting it would force N callers (ChatService + MCP semantic
server + PodContext) to duplicate the field mapping → passes. Semantic triples have
no `perspective` field (always `None`), so the struct differs from `RecalledEpisode`
— it should be a separate type, not a shared generic.

**Constraint classification:**
- Port trait return types belong in domain crates → Guideline (best practice)
- OCAP visibility enforcement must not be weakened → Guardrail (P2)
- Current `.get("value").and_then()` is fragile → Evidence (measured by silent-None failures)

**Strategy:**
1. Zoom out on `SemanticStoragePort`, `MemoryLoopAdapter`, `PodContext::recall_semantic`,
   and `ChatService::recall_semantic` to produce the before picture.
2. Define `RecalledSemantic` in `hkask-agents/src/ports/memory_storage.rs`. Semantic
   triples have no perspective, so the struct omits that field (or uses
   `perspective: Option<WebID>` with `None` — match the storage representation).
3. Change `SemanticStoragePort::recall_semantic` return type from
   `Result<Vec<Value>, MemoryError>` to `Result<Vec<RecalledSemantic>, MemoryError>`.
4. Update `MemoryLoopAdapter::recall_semantic` to construct `RecalledSemantic`
   instead of calling `triple_to_json`. If `triple_to_json` has no remaining callers,
   delete it.
5. Update `PodContext::recall_semantic` return type.
6. Simplify `ChatService::recall_semantic` to use typed field access.
7. Check for any MCP semantic server callers and update.
8. Verify: `cargo check --workspace && cargo clippy -p hkask-agents -p hkask-services -p hkask-api -- -D warnings && cargo test --workspace --exclude hkask-mcp-condenser`.

**Estimated effort:** ~1–2 hours.

---

### Task 2: OPEN_QUESTIONS.md — Document F1–F10 (Medium Priority)

**Problem:** The refactor-service-layer skill (Phase 8) specifies creating
`OPEN_QUESTIONS.md` with open question items. This file doesn't exist yet. The
current count is F1–F10 (F9 and F10 are the typed DTO items; F5 and F9 are now
resolved).

**Goal:** Create `OPEN_QUESTIONS.md` at the project root with structured entries
for each open question, including topic, status, force type, affected crates, and
recommended resolution approach.

**Strategy:**
1. Use the F1–F10 items from `HANDOFF.md` §7 as the source.
2. For each item, add: topic, status (deferred/resolved), force type (from
   `constraint-forces`), affected crates, and a brief recommendation.
3. Mark F5 and F9 as Resolved with session references.
4. Keep the format consistent with existing hKask documentation style.

**Estimated effort:** ~30 minutes.

---

### Task 3: Test Inventory Update (Medium Priority)

**Problem:** `docs/status/test-inventory.md` may not reflect the 138 service-layer
tests (was 70+ at last update; 4 pod tests now pass after F5 fix) or the domain-layer
typed DTO changes from F9.

**Goal:** Update the test inventory with all service-layer test seams and counts,
plus the domain-layer `RecalledEpisode` changes.

**Strategy:**
1. Count tests per service module: `cargo test -p hkask-services -- --list 2>&1 | grep 'test$' | wc -l`.
2. Count domain-layer tests: `cargo test -p hkask-agents -- --list 2>&1 | grep 'test$' | wc -l`.
3. Compare against `docs/status/test-inventory.md`.
4. Add or update the `hkask-services` section with per-module test counts and
   seam descriptions.
5. Add `hkask-agents` changes: `RecalledEpisode` type, `PodManager::new_mock` fixture.
6. Reference the `// REQ:` tags that anchor tests to spec requirements.

**Estimated effort:** ~1 hour.

---

### Task 4: hkask-mcp-condenser Build Fix (Medium Priority)

**Problem:** `hkask-mcp-condenser` fails to compile because it uses
`McpToolError::internal_error` and `McpErrorKind::InternalError` which have been
renamed to `McpToolError::internal` and `McpErrorKind::Internal` in `hkask-mcp`.

**Goal:** Update the condenser to use the current API names so it compiles and
its tests pass.

**Scope:**
- `mcp-servers/hkask-mcp-condenser/src/main.rs` — Replace `internal_error` with
  `internal`, `InternalError` with `Internal`.
- `mcp-servers/hkask-mcp-condenser/src/types.rs` — Remove or `#[allow(dead_code)]`
  the unused `ThreadSummaryRequest` and `ThreadSummaryOutput` structs.

**Strategy:**
1. Build `hkask-mcp-condenser` to see all errors.
2. Apply renames surgically.
3. Address dead_code warnings for the two unused structs (either remove them if
   truly unused, or add `#[allow(dead_code)]` with a comment explaining they're
   reserved for future use).
4. Verify: `cargo build -p hkask-mcp-condenser && cargo test -p hkask-mcp-condenser`.

**Constraint classification:**
- Renaming to match upstream API → Evidence (the API was renamed, measured by
  compile errors)
- Dead code in types.rs → Guideline (P6/P7 constraints say no `todo!`, no
  `#[deprecated]`, but `#[allow(dead_code)]` with justification is acceptable)

**Estimated effort:** ~15–30 minutes.

---

### Task 5: F3 — Unified Authentication Context (Low Priority / Speculative)

**Problem:** API uses HTTP auth middleware (`AuthContext`), CLI uses keystore
(`ServiceConfig::acp_secret`). The two paths produce `DelegationToken` differently
and there's no shared "who am I and what can I do" context.

**Goal:** Investigate whether a unified `AuthContext` type in the service layer
could reduce the per-surface auth wiring. Currently deferred — the two surfaces
produce valid tokens through different mechanisms, and unifying them is a design
question, not a bug.

**Constraint classification:** Hypothesis — needs verification that unification
actually reduces complexity. Current two-path approach is Evidence (it works).

**Estimated effort:** Deferred until requested.

---

### Task 6: F4 — MCP Server Service Access (Low Priority / Speculative)

**Problem:** MCP servers use domain primitives (`TripleStore`, `AcpRuntime`),
not `hkask-services`. This is correct (anti-pattern from refactor-service-layer skill:
MCP servers must NOT depend on `hkask-services`). But some MCP servers duplicate
logic that services own (e.g., ACP registration, spec verification).

**Goal:** Investigate whether MCP servers could call service operations through
a lightweight service-client interface, or whether the current domain-primitive
approach is preferable. Currently deferred — no duplication is causing bugs.

**Constraint classification:** Hypothesis — needs verification. Current architecture
is Evidence (MCP servers compile and work with domain primitives).

**Estimated effort:** Deferred until requested.

---

### Task 7: F1 — Streaming Responses (Low Priority / Speculative)

**Problem:** The service layer returns complete results. For long-running
operations (chat, compose, embed), streaming would improve responsiveness.

**Goal:** Investigate whether service operations should return
`impl Stream<Item = Result<Chunk>>` or whether streaming is purely a surface
concern (Axum SSE, terminal line-by-line).

**Constraint classification:** Hypothesis — the service layer currently doesn't
stream, and this is fine for correctness. Streaming is a performance/UX concern.

**Estimated effort:** Deferred until requested.

---

### Task 8: F8 — GovernedTool Membrane Boundary (Low Priority / Speculative)

**Problem:** `GovernedTool` in `hkask-cns` routes tool invocations through CNS
governance (gas budget, variety sensing). The boundary between the service layer
and `GovernedTool` is unclear — should services own the membrane, or should it
stay in the CNS crate?

**Goal:** Investigate the correct boundary. Currently deferred — `GovernedTool`
works correctly in `hkask-cns` and the service layer calls it through `PodContext`.

**Constraint classification:** Hypothesis — current architecture is Evidence
(it works). The boundary question is about future extensibility.

**Estimated effort:** Deferred until requested.

---

## Per-Task Discipline

For every task that touches code:

```
[ ] Zoom out on the target area (module map, caller graph, data flow)
[ ] Apply depth test — is the proposed change deep or shallow?
[ ] Classify constraints with constraint-forces skill
[ ] State assumptions explicitly before implementing
[ ] Implement with surgical changes — every line traces to the task
[ ] Verify: cargo check --workspace
[ ] Run clippy: cargo clippy -p hkask-agents -p hkask-services -p hkask-api -- -D warnings
[ ] Run tests: cargo test --workspace --exclude hkask-mcp-condenser
[ ] Update HANDOFF.md (add key decision, update file map if needed)
[ ] Update CONTINUATION.md (mark task done or document new findings)
```

For tasks that introduce new types to `hkask-agents` (domain crate):

```
[ ] Apply depth test — would the type be needed by N callers or just 1?
[ ] Check dependency direction — does the type belong in domain crate or services?
[ ] Add MemoryError variant if the operation can fail
[ ] Write test with // REQ: tag
[ ] Verify both CLI and API surfaces still work
```

---

## Key Constraints (From Extraction Project — Still Apply)

- **P3 (Dependency direction):** CLI → services → domain. No circular deps.
- **P5 (One domain per change):** Each change touches exactly one concern.
- **Headless:** No visual UI, no dashboards, no monitoring stacks.
- **P8 (Test quality):** Every `#[test]` verifies a stated behavioral property.
- **Depth test (P2):** If deleting the proposed module/type makes complexity
  vanish, don't create it.
- **Surgical changes:** No style fixes, no renaming, no comment additions.
- **MCP servers must NOT depend on `hkask-services`.** (Anti-pattern from
  refactor-service-layer skill.)

---

## Known Patterns (From 13 Sessions of Extractions + Follow-Ups)

- Services that open DB before ServiceContext exists (onboarding, consolidation,
  compose, embed) accept `db_path` + `db_passphrase` as parameters.
- `SpecStore` is a trait; `ServiceContext` stores `SqliteSpecStore`. When a service
  needs a spec store, accept `&SqliteSpecStore` (concrete type).
- `Keychain::default()` creates a keychain with service name "hkask".
- Error mapping: service uses `ServiceError` variants with `#[from]` where possible.
  CLI surfaces add `From<ServiceError> for TheirError` impls.
- `Database::open` in onboarding/consolidation/compose/embed is a legitimate legacy
  pattern — services accept caller-provided path+passphrase.
- Services that resolve credentials internally (ArchivalService, OnboardingService)
  use `resolve_credential()` from `hkask_mcp::server`.
- Type relocations: domain types move from CLI to services. Surface presentation
  types stay in CLI.
- `ServiceError` sentinel variants: each service adds a `ServiceError::<Domain>(String)`
  variant. Current count: 29 variants (plus `InfrastructureError` sub-variants).
- `MemoryError::CapabilityDenied { resource, action }` is the typed OCAP denial
  variant in `hkask-agents`. The API route maps this to 403. The MCP server would
  map it differently.
- `RecalledEpisode` uses domain types (`Confidence`, `Visibility`, `Option<WebID>`)
  for compile-time safety. Port trait return types belong in the domain crate, not
  the service layer. (#73)
- `PodManager::new_mock()` uses a deterministic test ACP secret
  (`b"hkask-mock-acp-secret-32-bytes!!"`) so tests pass without environment
  variables. Test-only secrets are Guardrails — acceptable with documentation. (#74)
- `triple_to_json` is still used by `recall_semantic`. `triple_to_recalled_episode`
  is used by `recall_episodic`. If F10 is done, `triple_to_json` may become unused
  and should be deleted.

---

## Build Commands

```bash
# Per-crate verification
cargo check -p hkask-services && cargo clippy -p hkask-services -- -D warnings
cargo check -p hkask-cli && cargo clippy -p hkask-cli -- -D warnings
cargo check -p hkask-api && cargo clippy -p hkask-api -- -D warnings
cargo check -p hkask-agents && cargo clippy -p hkask-agents -- -D warnings

# Full workspace verification (run after every change)
cargo check --workspace
cargo test --workspace --exclude hkask-mcp-condenser
cargo clippy --workspace -- -D warnings  # Note: condenser will fail; fix separately
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*