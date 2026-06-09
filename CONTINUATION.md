# CONTINUATION.md — hKask Service Layer Extraction

**Sessions:** 12–22 | **Next:** Session 23+

---

## Context

You are continuing the hKask service layer extraction project. Sessions 12–17 built the infrastructure (ServiceContext/ServiceConfig, dependency injection wiring, dead code cleanup, ReplState deduplication). Session 18 extracted ChatService. Session 19 extracted AgentService, deduplicated consolidation.rs CLI, and extracted UserService. Session 20 extracted ComposeService and extended EnsembleService with improv operations. Session 21 extracted OnboardingService (deep, 8 ops) and SpecService (medium-deep, 5 ops), and skipped CnsService (depth test fails). Session 22 extracted ArchivalService (deep, 4 ops), EmbedService (deep, 2 ops + config types), SkillService (deep, 7 ops), and VerificationService (deep, 3 ops), while skipping KeystoreService and McpService (both fail depth test).

**The project is NEAR COMPLETE.** 17 of 27 CLI commands are fully extracted. All extractable business logic is now in the service layer. The remaining 3 unextracted CLI files (keystore, mcp, web_search) failed the depth test and are correctly left in the surface. 5 partially extracted files remain for evaluation. 1 API route needs typed DTOs.

**Read these files first (in this order):**

1. **`HANDOFF.md`** — Session history (Sessions 12–22), remaining work inventory, key decisions (#1–#66), deep service module inventory, file reference map, legitimate legacy patterns.
2. **`CONTINUATION.md`** (this file) — Priority-ordered extraction targets, per-extraction checklists, extraction-specific notes, key constraints.
3. **`.agents/skills/refactor-service-layer/SKILL.md`** — The strangler fig methodology governing all extractions (P1–P6 principles, Phase 0–8 process, anti-patterns, checklists).

---

## Honest Assessment

Sessions 12–22 accomplished:

- **Infrastructure wiring** — ServiceContext/ServiceConfig built and connected to every surface.
- **10 deep extractions** — ChatService, AgentService, UserService, ComposeService, OnboardingService, ArchivalService, EmbedService, SkillService, VerificationService, ConsolidationService.
- **2 medium-deep extractions** — SpecService, EnsembleService (extended with improv ops).
- **5 depth-test skips** — CnsService (shallow), KeystoreService (thin pass-through), McpService (surface adapters).
- **1 CLI deduplication** — consolidation.rs.
- **2 file deletions** — `registration.rs`, `git_archival.rs`.
- **1 type relocation** — `ResolvedSecrets` moved to services.
- **33 new service-layer tests** (from Session 22 alone; 70+ total).

**What's left:** Evaluate 5 partially extracted CLI files with depth test. Fix 1 API route (`routes/episodic.rs`) with typed DTOs. If all remaining files fail the depth test, the project is **effectively complete** — the service layer contains all extractable business logic, and remaining surface code is genuinely presentation-only. Estimated 4–8 hours.

---

## Extraction Priority

Follow the strangler fig pattern per the refactor-service-layer skill: **one domain per extraction, RED→GREEN→WIRED→DELETED per extraction.**

### ~~P1~~ — High impact (✅ DONE)

1. **~~`onboarding.rs` → `OnboardingService`~~ ✅ DONE (Session 21)**

### ~~P2~~ — Medium impact (✅ DONE or SKIPPED)

2. **~~`cns.rs` → `CnsService`~~ ⚠️ SKIPPED (Session 21)** — Depth test fails: `cns.rs` is mostly `println!` formatting.
3. **~~`spec.rs` → `SpecService`~~ ✅ DONE (Session 21)**
4. **~~`git_archival.rs` → `ArchivalService`~~ ✅ DONE (Session 22)** — File deleted entirely.
5. **~~`embed_corpus.rs` → `EmbedService`~~ ✅ DONE (Session 22)** — CLI reduced ~290→~60 lines.

### ~~P3~~ — Lower impact (✅ DONE or SKIPPED)

6. **~~`skill.rs` → `SkillService`~~ ✅ DONE (Session 22)** — 7 ops, CLI reduced ~453→~170 lines.
7. **~~`keystore.rs` → `KeystoreService`~~ ⚠️ SKIPPED (Session 22)** — Depth test fails: thin pass-through over `Keychain` API.
8. **~~`magna_carta.rs` → `VerificationService`~~ ✅ DONE (Session 22)** — 3 ops, CLI reduced ~556→~102 lines.
9. **~~`mcp.rs` / `models.rs` / `web_search.rs` → `McpService`~~ ⚠️ SKIPPED (Session 22)** — Depth test fails: surface adapters over `mcp_dispatcher.invoke()`.

### Remaining — Evaluate with depth test

| # | Target | Current Status | Action |
|---|--------|---------------|--------|
| 10 | `git_cmd.rs` | Partially extracted (archival → ArchivalService; CAS ops inline) | Depth test CAS operations |
| 11 | `loops.rs` | Partially extracted | Depth test remaining logic |
| 12 | `serve.rs` | Partially extracted | Depth test remaining logic |
| 13 | `template.rs` | Partially extracted | Depth test remaining logic |
| 14 | `models.rs` | Depth test already failed (MCP adapter) | Leave as-is |

### API-specific

| # | Target | Issue | Proposed Service | Estimated Effort |
|---|--------|-------|-----------------|------------------|
| 15 | `routes/episodic.rs` | Stringly-typed OCAP error classification; `serde_json::Value` → typed DTO mapping | Consider `MemoryService` | ~1–2h |

---

## Skills to Load

**Load these skills before starting any extraction:**

1. **`refactor-service-layer`** — **Required.** This is the governing methodology. Read the full SKILL.md before starting. It defines the strangler fig sequence, depth test, dependency direction rules, anti-patterns, and the per-extraction checklist you must follow.

2. **`coding-guidelines`** — **Required.** Surgical changes only: each extraction touches exactly one domain. No "while we're here" changes. No renaming. No comment additions. Every changed line traces directly to the extraction.

3. **`constraint-forces`** — **Recommended.** Use to classify design decisions by force type (Prohibition, Guardrail, Guideline, Evidence, Hypothesis). Particularly useful when deciding whether a CLI-specific concern belongs in the service layer, or whether a module is too shallow to extract.

4. **`zoom-out`** — **Recommended before evaluating each partially extracted file.** Produce the module map, caller graph, data flow "before picture."

5. **`diagnose`** — If you encounter unexpected compilation errors or test failures during extraction.

6. **`magna-carta-verifier`** — If working on `routes/episodic.rs` OCAP error classification, this skill provides context on sovereignty compliance structures.

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
- **Depth test (P2):** If deleting the proposed module makes complexity vanish, don't create it — merge or deepen instead. A module with 20 public functions and thin delegations is shallow. A module with 3 public functions that encapsulate 500 lines of domain logic is deep. **Session 22 demonstrated 2 more depth-test skips (KeystoreService, McpService). Apply this test rigorously before starting ANY extraction.**

---

## Extraction-Specific Notes

### ~~onboarding.rs → OnboardingService~~ (P1 #1) ✅ DONE (Session 21)

- 8 operations. `ResolvedSecrets` and `SignInOutcome` moved to services. CLI reduced ~639→377 lines.
- `Database::open` remains legitimate legacy pattern — service accepts caller-provided `ServiceConfig`.

### ~~cns.rs → CnsService~~ (P2 #2) ⚠️ SKIPPED (Session 21)

- Depth test fails. Domain logic already in `hkask_cns`. CLI is `println!` formatting.

### ~~spec.rs → SpecService~~ (P2 #3) ✅ DONE (Session 21)

- 5 operations. MiniJinja rendering stays in CLI surface (template loading is surface-specific).

### ~~git_archival.rs → ArchivalService~~ (P2 #4) ✅ DONE (Session 22)

- 4 operations. `git_archival.rs` deleted entirely. `reqwest`/`base64` moved to services.
- Dead `McpRuntime`/`CapabilityChecker` params dropped. ArchivalService resolves GitHub credentials internally.

### ~~embed_corpus.rs → EmbedService~~ (P2 #5) ✅ DONE (Session 22)

- 2 operations + config parsing. `CorpusConfig` and 6 sub-types moved to services.
- `Database::open` remains legitimate legacy pattern — service accepts caller-provided `db_path` + `db_passphrase`.

### ~~skill.rs → SkillService~~ (P3 #6) ✅ DONE (Session 22)

- 7 operations. BLAKE3 hashing, SKILL.md YAML mutation, zone-aware publishing.
- `SkillInfo` and `SkillPublishResult` types introduced. `hex` dep added to services.

### ~~keystore.rs → KeystoreService~~ (P3 #7) ⚠️ SKIPPED (Session 22)

- Depth test fails. `Keychain` API is already the deep module. `.env` parsing is CLI presentation.

### ~~magna_carta.rs → VerificationService~~ (P3 #8) ✅ DONE (Session 22)

- 3 operations. `Manifest`, `Assertion`, `AssertionResult`, `PrincipleResult`, `VerificationReport` moved to services.
- `verify_json` serves both CLI and MCP tools. CLI reduced ~556→~102 lines.

### ~~mcp.rs / models.rs / web_search.rs → McpService~~ (P3 #9) ⚠️ SKIPPED (Session 22)

- Depth test fails. Surface adapters over `mcp_dispatcher.invoke()`. Each just formats JSON results differently.

### Remaining: Partially extracted CLI files

**Evaluate each with the depth test. If it fails, mark as "surface-only" and move on.**

#### `git_cmd.rs`

- Archival operations already delegate to ArchivalService.
- CAS (content-addressable storage) operations still inline.
- **Depth test question:** If deleting a `GitCasService` would cause CAS operations to reappear in any caller, it passes. If CAS is only used by this one CLI command, it's surface-only.

#### `loops.rs`

- Partially extracted. Evaluate what inline logic remains.
- **Depth test question:** Is the remaining logic domain operations that would be needed by any future caller, or is it CLI presentation/interaction flow?

#### `serve.rs`

- Partially extracted. Evaluate what inline logic remains.
- **Depth test question:** Same — domain logic or surface orchestration?

#### `template.rs`

- Partially extracted. Evaluate what inline logic remains.
- **Depth test question:** Template operations may already be well-served by `hkask-templates`. Check if remaining code is surface formatting.

#### `models.rs`

- **Already evaluated — depth test fails.** MCP adapter, not business logic. Leave as-is.

### routes/episodic.rs (API-specific)

- Stringly-typed OCAP error classification needs typed error variants.
- `serde_json::Value` → typed DTO mapping should be centralized.
- Consider a `MemoryService` that owns the type mapping and OCAP error classification.
- **Depth test:** If the type mapping and error classification would be needed by any future API route or MCP tool that accesses episodic memory, it passes. If it's purely HTTP response formatting, it's surface-only.

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

1. **Load skills** — `refactor-service-layer` (required), `coding-guidelines` (required), `zoom-out` (for evaluation).
2. **Read** HANDOFF.md and this file in order.
3. **Zoom out** on the next target — produce module map, caller graph, data flow.
4. **Apply depth test** — If the proposed module would be a shallow pass-through, skip it. Record the decision.
5. **If depth test passes:** RED → GREEN → Wire CLI → Wire API → Verify → Depth test → Update docs.
6. **If depth test fails:** Mark as "surface-only" in CONTINUATION.md and HANDOFF.md. Move to next target.
7. **After all evaluations:** If all remaining files fail the depth test, declare the project complete.
8. **Update docs** — HANDOFF.md (key decision + file map), CONTINUATION.md (mark done/skipped).

Aim for 2–3 evaluations per session for remaining targets. Any actual extractions will likely take 1–2 hours each.

---

## Project Completion Criteria

The service layer extraction project is **complete** when:

1. All CLI files with extractable business logic have been evaluated with the depth test.
2. All files that pass the depth test have been extracted to service modules.
3. All files that fail the depth test are documented as "surface-only" in HANDOFF.md.
4. `routes/episodic.rs` OCAP error classification and typed DTO mapping is resolved (either extracted or documented as surface-only).
5. `cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings` all pass.
6. HANDOFF.md and CONTINUATION.md reflect final state.

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*