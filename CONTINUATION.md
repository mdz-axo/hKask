# CONTINUATION.md — hKask Service Layer Extraction

**Sessions:** 12–21 | **Next:** Session 22+

---

## Context

You are continuing the hKask service layer extraction project. Sessions 12–17 built the infrastructure (ServiceContext/ServiceConfig, dependency injection wiring, dead code cleanup, ReplState deduplication). Session 18 extracted ChatService. Session 19 extracted AgentService, deduplicated consolidation.rs CLI, and extracted UserService. Session 20 extracted ComposeService and extended EnsembleService with improv operations. Session 21 extracted OnboardingService (deep, 8 ops) and SpecService (medium-deep, 5 ops), and skipped CnsService (depth test fails).

**The project is NOT complete.** ~14 of 27 CLI commands still contain inline business logic. The infrastructure wiring and 5 deep + 1 medium-deep extractions + 1 extension are done; the remaining business logic extraction continues.

**Read these files first (in this order):**

1. **`HANDOFF.md`** — Session history (Sessions 12–21), remaining work inventory, key decisions (#1–#57), deep service module inventory, file reference map, legitimate legacy patterns.
2. **`CONTINUATION.md`** (this file) — Priority-ordered extraction targets, per-extraction checklists, extraction-specific notes, key constraints.
3. **`.agents/skills/refactor-service-layer/SKILL.md`** — The strangler fig methodology governing all extractions (P1–P6 principles, Phase 0–8 process, anti-patterns, checklists).

---

## Honest Assessment

Sessions 12–19 accomplished **infrastructure wiring** + **3 deep extractions** (ChatService, AgentService, UserService) + **1 CLI deduplication** (consolidation.rs). Session 20 added **1 deep extraction** (ComposeService) and **1 extension** (EnsembleService improv ops). Session 21 added **1 deep extraction** (OnboardingService, 8 ops) and **1 medium-deep extraction** (SpecService, 5 ops), and **1 depth-test skip** (CnsService — shallow pass-through). The service modules in `hkask-services` now include 5 deep modules, 1 medium-deep module, 2 medium modules, and 4 shallow/medium modules.

**What's left:** Extracting inline business logic from ~14 CLI files + 2 API routes into ~7 new or extended service modules, then deleting the legacy inline code. Estimated 11–20 hours of focused work.

---

## Extraction Priority

Follow the strangler fig pattern per the refactor-service-layer skill: **one domain per extraction, RED→GREEN→WIRED→DELETED per extraction.**

### ~~P1~~ — High impact (✅ DONE)

1. **~~`onboarding.rs` → `OnboardingService`~~ ✅ DONE (Session 21)** — Secret derivation, keychain storage, DB cleanup, replicant registration, sign-in verification loop. ~3-4h.

### P2 — Medium impact (next targets)

2. **~~`cns.rs` → `CnsService`~~ ⚠️ SKIPPED (Session 21)** — Depth test fails: `cns.rs` is mostly `println!` formatting, domain logic already well-encapsulated in `hkask_cns`.
3. **~~`spec.rs` → `SpecService`~~ ✅ DONE (Session 21)** — Spec construction pipeline + curator evaluation. ~2h.
4. **`git_archival.rs` → `ArchivalService`** — GitHub REST API calls, base64 encoding, registry serialization. ~2-3h.
5. **`embed_corpus.rs` → `EmbedService`** — HTTP download, corpus chunking, embedding batch loop, centroid computation. ~2-3h.

### P3 — Lower impact (CLI-only, no API duplication)

6. **`skill.rs` → `SkillService`** — Filesystem discovery, hash computation. ~2h.
7. **`keystore.rs` → `KeystoreService`** — Keychain CRUD, .env file parsing. ~1-2h.
8. **`magna_carta.rs` → `VerificationService`** — Manifest loading, structural audits. ~2h.
9. **`mcp.rs` / `models.rs` / `web_search.rs`** — MCP dispatcher invocation patterns. Centralize into an `McpService` or use existing `InferenceService::list_models()`. ~2-3h.

### API-specific

10. **`routes/episodic.rs`** — Fix stringly-typed OCAP error classification, centralize `serde_json::Value` → typed DTO mapping. Consider `MemoryService`. ~1-2h.

---

## Skills to Load

**Load these skills before starting any extraction:**

1. **`refactor-service-layer`** — **Required.** This is the governing methodology. Read the full SKILL.md before starting. It defines the strangler fig sequence, depth test, dependency direction rules, anti-patterns, and the per-extraction checklist you must follow.

2. **`coding-guidelines`** — **Required.** Surgical changes only: each extraction touches exactly one domain. No "while we're here" changes. No renaming. No comment additions. Every changed line traces directly to the extraction.

3. **`constraint-forces`** — Recommended. Use to classify design decisions by force type (Prohibition, Guardrail, Guideline, Evidence, Hypothesis). Particularly useful when deciding whether a CLI-specific concern belongs in the service layer, or whether a module is too shallow to extract.

4. **`zoom-out`** — Recommended before starting each P1 extraction. Produce the module map, caller graph, data flow "before picture." This is especially important for `onboarding.rs` which has complex two-path flow and multiple domain crossings.

5. **`diagnose`** — If you encounter unexpected compilation errors or test failures during extraction. Follow the disciplined diagnosis loop: reproduce → anchor → hypothesize → instrument → fix → regression-test.

---

## Per-Extraction Checklist

Follow the strangler fig sequence from the refactor-service-layer skill. Complete every step before moving to the next extraction:

```
[ ] RED:    Write failing test for the service operation with // REQ: tag
[ ] GREEN:  Implement the minimal service operation that passes the test
[ ] Wire CLI: Change CLI to call service, delete duplicate logic
[ ] Wire API: Change API to call service, delete duplicate logic (if applicable)
[ ] Verify:  cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings
[ ] Deletion test: Service module is deep, not a shallow pass-through
[ ] Dependency direction verified: no circular deps
```

After completing each extraction, update `HANDOFF.md` (add key decision, update file reference map, update completion counts) and `CONTINUATION.md` (mark extraction done, update priority list).

---

## Key Constraints

- **P3 (Dependency direction):** CLI → services → domain. No circular deps. If you find a service needing something from CLI, stop and redesign.
- **P5 (One domain per extraction):** Each extraction is a separate, atomic change. No cross-domain refactors.
- **Surgical changes:** Every changed line traces to the extraction. No style fixes, no renaming, no comment additions in adjacent code.
- **Headless constraint:** No visual UI, no dashboards, no monitoring stacks. The service layer never produces terminal output or HTTP responses.
- **P8 (Test quality):** Every `#[test]` verifies a stated behavioral property. Don't weaken tests. Each test carries a `// REQ:` tag.
- **Depth test (P2):** If deleting the proposed module makes complexity vanish, don't create it — merge or deepen instead. A module with 20 public functions and thin delegations is shallow. A module with 3 public functions that encapsulate 500 lines of domain logic is deep.

---

## Extraction-Specific Notes

### ~~onboarding.rs → OnboardingService~~ (P1 #1) ✅ DONE (Session 21)

**File:** `crates/hkask-cli/src/onboarding.rs` (not in `commands/` — it's a top-level module)

**Key structures:**
- `OnboardingError` (L25-38) — error enum with Cancelled, Registry, Keychain, Database, Io, InvalidPassphrase
- `OnboardingOutcome` (L41-51) — returns signed-in agent + resolved secrets
- `ResolvedSecrets` (L55-58) — acp_secret + db_passphrase (currently in onboarding.rs, listed as legitimate in HANDOFF.md §5)

**Key functions to extract:**
- `init_registry_from_config()` (L66-105) — opens registry DB, initializes ACP, bootstraps agent registry
- `run_onboarding()` (L112-136) — orchestrates the two-path flow
- `interactive_onboarding()` (L139-179) — interactive flow choosing fast vs slow path
- `create_first_replicant_flow()` (L182-266) — slow path: derive secrets → keychain → DB → register replicant
- `sign_in_flow()` (L269-334) — sign in to existing replicant
- `register_replicant()` (L337-396) — ACP register + AgentService.register + store
- `store_secrets_in_keychain()` (L403-419) — keychain write
- `try_list_existing_replicants()` (L422-456) — list replicants from store
- `cleanup_keychain()` (L549-558) — keychain cleanup on failure
- `cleanup_db()` (L565-580) — DB cleanup on failure

**Two-path flow:**
1. **Fast path:** Keys already in keychain → derive secrets → init registry → list replicants → sign in
2. **Slow path:** Prompt for passphrase → derive secrets → store in keychain → init registry → create first replicant → sign in

**Interactive functions to leave in surface (CLI-only):**
- `read_line()`, `prompt_line()`, `prompt_passphrase()`, `prompt_passphrase_with_confirm()`, `prompt_choice()` — these are CLI I/O and stay in the surface
- `list_replicants()`, `pick_or_default_replicant()` — terminal presentation, stays in CLI

**Design challenges:**
- **`Database::open` in onboarding** — listed as "legitimate legacy pattern" in HANDOFF.md because onboarding must open DB before ServiceContext exists. The service should accept a pre-opened DB connection or construct its own from caller-provided path+passphrase.
- **`ResolvedSecrets`** — the service should own this type (move from CLI to services).
- **`hkask_keystore::*` calls** — `store_secrets_in_keychain()` and `cleanup_keychain()` use keychain APIs. The service can own these operations.
- **`init_registry_from_config()`** — this crosses 3 domain boundaries (storage → ACP → agents). It's a composite operation that justifies the depth test.
- **No API counterpart** — onboarding is CLI-only, but the service layer makes it available for future API use.

**Recommended approach:**
1. **Zoom out** first — produce module map, caller graph, data flow for the onboarding domain.
2. Design `OnboardingService` with operations: `derive_and_store_secrets()`, `init_registry()`, `register_replicant()`, `sign_in()`, `cleanup_keychain()`, `cleanup_db()`. The service does NOT own the interactive flow — `run_onboarding()` stays in CLI as an orchestrator that calls service operations.
3. Move `ResolvedSecrets`, `OnboardingError`, `OnboardingOutcome` to services.
4. Apply the depth test: if deleting OnboardingService would cause the multi-step secret derivation + keychain + DB + registry flow to reappear in any caller, it passes.

### ~~cns.rs → CnsService~~ (P2 #2) ⚠️ SKIPPED (Session 21)

- **Depth test fails.** `cns.rs` CLI is ~140 lines, mostly `println!` formatting. The domain operations (CnsRuntime health/alerts/variety) are already well-encapsulated in `hkask_cns`. API routes already access CnsRuntime through ServiceContext. No duplicated business logic between surfaces. Creating a CnsService would be a shallow pass-through.
- **Lesson:** Always apply the depth test before creating a service module. If deleting the proposed module would make complexity vanish (because the domain crate already handles it), don't create it — the surface can call the domain crate directly.

### ~~spec.rs → SpecService~~ (P2 #3) ✅ DONE (Session 21)

- Spec curation pipeline (capture → cultivate → validate).
- MiniJinja rendering for spec templates.
- Check if `hkask-mcp-spec` MCP server duplicates this logic.

### git_archival.rs → ArchivalService (P2 #4)

- GitHub REST API calls, base64 encoding, registry serialization.
- Uses `reqwest` for HTTP — service crate needs the dep or takes a pre-built client.
- CLI-only currently.

### embed_corpus.rs → EmbedService (P2 #5)

- HTTP download, corpus chunking, embedding batch loop, centroid computation.
- Similar to ComposeService in DB + SemanticMemory construction.
- Uses user-provided DB credentials (like ComposeService and ConsolidationService).

### P3 extractions

- All CLI-only, no API duplication to remove.
- **skill.rs** — filesystem discovery, SHA256 hashing, visibility mutation.
- **keystore.rs** — keychain CRUD, `.env` file parsing.
- **magna_carta.rs** — YAML manifest loading, structural audits against sovereignty principles.
- **mcp.rs / models.rs / web_search.rs** — evaluate whether these share a common MCP dispatcher invocation pattern. May consolidate into one service.

### routes/episodic.rs (API-specific)

- Stringly-typed OCAP error classification needs fixing.
- `serde_json::Value` → typed DTO mapping should be centralized.
- Consider a `MemoryService` that owns the type mapping.

---

## Build Commands

```bash
# Per-crate verification
cargo check -p hkask-services && cargo clippy -p hkask-services -- -D warnings
cargo check -p hkask-cli && cargo clippy -p hkask-cli -- -D warnings
cargo check -p hkask-api && cargo clippy -p hkask-api -- -D warnings

# Full workspace verification (run after every extraction)
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

## Recommended Session Strategy

A productive session follows this cadence:

1. **Load skills** — `refactor-service-layer` (required), `coding-guidelines` (required), `zoom-out` (for P1).
2. **Read** HANDOFF.md and this file in order.
3. **Zoom out** on the next target (especially for P1 onboarding).
4. **RED** — Write failing service test with `// REQ:` tag.
5. **GREEN** — Implement minimal service operation.
6. **Wire CLI** — Change CLI to delegate, delete inline logic.
7. **Wire API** — If applicable, change API route to delegate.
8. **Verify** — `cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`.
9. **Depth test** — Confirm the service module is deep, not shallow.
10. **Update docs** — HANDOFF.md (key decision + file map), CONTINUATION.md (mark done).
11. **Next extraction** — Repeat from step 3.

Aim for 2–3 extractions per session for P2/P3 targets, 1 extraction per session for P1 onboarding.

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*