# CONTINUATION PROMPT — hKask Service Layer Extraction (Session 23+)

**Load these skills before starting:**

1. **`refactor-service-layer`** — **Required.** Governing methodology: strangler fig sequence, depth test (P2), dependency direction (P3), surgical changes (P5). Every extraction follows its RED→GREEN→WIRED→DELETED cycle. **Pay special attention to the depth test** — Sessions 21 and 22 skipped 5 files total (CnsService, KeystoreService, McpService) because they would have been shallow pass-throughs. The depth test is the primary gate for all remaining work.

2. **`coding-guidelines`** — **Required.** Surgical changes only: each extraction touches exactly one domain. No "while we're here" changes. No renaming. No comment additions.

3. **`zoom-out`** — **Required before evaluating each partially extracted file.** Produce the module map, caller graph, and data flow "before picture" for the target file. This is critical for the remaining evaluations — you need to understand what logic is already in the service layer vs. what remains inline.

4. **`constraint-forces`** — **Recommended.** Classify design decisions by force type (Prohibition, Guardrail, Guideline, Evidence, Hypothesis). Use when deciding whether a CLI-specific concern belongs in the service layer, or when evaluating if a module is too shallow to extract.

5. **`magna-carta-verifier`** — **If working on `routes/episodic.rs`.** Provides context on sovereignty compliance structures and OCAP error classification patterns.

6. **`diagnose`** — **Available if needed.** For unexpected compilation errors or test failures during extraction.

**Read these files first (in this order):**

1. `HANDOFF.md` — Session history (Sessions 12–22), remaining work inventory with effort estimates, key decisions (#1–#66), deep service module inventory, file reference map, legitimate legacy patterns.
2. `CONTINUATION.md` — Priority-ordered extraction targets (remaining evaluations + API-specific), per-extraction checklist, extraction-specific notes for remaining targets, key constraints, build commands, recommended session strategy, project completion criteria.
3. `.agents/skills/refactor-service-layer/SKILL.md` — The strangler fig methodology governing all extractions (P1–P6 principles, Phase 0–8 process, anti-patterns, checklists).

---

## Current State (End of Session 22)

- **Infrastructure wiring: DONE.** ServiceContext/ServiceConfig are built and wired to every surface. ReplState has zero duplicated fields. Dead code is deleted. Workspace passes check+clippy+test.
- **10 deep extractions: COMPLETE** — ChatService (6+ step chat turn), AgentService (6-step registration), UserService (8 ops, validation + lock + opaque errors), ComposeService (8+ step style synthesizer), OnboardingService (8 ops, full bootstrap flow), ArchivalService (4 ops, GitHub REST API + credential resolution + base64), EmbedService (2 ops + config, full embedding pipeline), SkillService (7 ops, BLAKE3 + visibility + publishing), VerificationService (3 ops, Magna Carta verification pipeline), ConsolidationService (keystore + key derivation + DB + pipeline).
- **2 medium-deep: COMPLETE** — SpecService (5 ops, spec construction + evaluation), EnsembleService extended with 5 improv ops.
- **5 depth-test skips: DOCUMENTED** — CnsService (shallow, `println!` formatting), KeystoreService (thin pass-through over `Keychain`), McpService (surface adapters over `mcp_dispatcher.invoke()`).
- **17/27 CLI commands fully extracted.** 2 files deleted (`registration.rs`, `git_archival.rs`).
- **33 new service-layer tests** from Session 22 alone; 70+ total across all services.
- **Verification:** `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace` all pass (4 pre-existing pod test failures unrelated to service layer).

---

## Session 23 Task Plan

### Phase 1: Evaluate partially extracted CLI files (depth test gate)

**For each file, zoom out first, then apply the depth test. If it fails, document the skip and move on.**

#### 1a. `git_cmd.rs` — CAS operations evaluation

- Archival operations already delegate to `ArchivalService`.
- CAS (content-addressable storage) operations still inline.
- **Depth test:** Would CAS operations (blob hashing, tree construction, commit creation, object storage) reappear in any future caller? If CAS is only used by this CLI command and `hkask-storage` or `hkask-templates` already handle it, it's surface-only.
- **Strategy:** Read `git_cmd.rs`, identify remaining inline logic beyond ArchivalService delegation. Check if CAS operations call into existing domain crates.

#### 1b. `loops.rs` — Remaining logic evaluation

- **Depth test:** Is the remaining logic domain operations (loop system construction, health check orchestration, variety computation) that any future caller would need? Or is it CLI presentation (status printing, formatted output)?
- **Strategy:** Read `loops.rs`, separate domain logic from presentation. Check if `hkask_cns` already encapsulates the domain operations.

#### 1c. `serve.rs` — Remaining logic evaluation

- **Depth test:** Same question — domain logic or surface orchestration?
- **Strategy:** Read `serve.rs`. Server startup is typically surface orchestration. Check if any business logic (auth, routing decisions, middleware) exists beyond Axum route wiring.

#### 1d. `template.rs` — Remaining logic evaluation

- **Depth test:** Template operations may already be well-served by `hkask-templates`. Check if remaining code is surface formatting (CLI arg parsing, output formatting).
- **Strategy:** Read `template.rs`. Compare remaining inline logic against `hkask_templates` crate API.

#### 1e. `models.rs` — Already evaluated

- **Depth test already failed** (MCP adapter). Leave as-is.

### Phase 2: API route fix

#### 2a. `routes/episodic.rs` — Typed DTOs and OCAP error classification

- **Problem:** Stringly-typed OCAP error classification (matching on error message strings instead of typed variants).
- **Problem:** `serde_json::Value` used where typed DTOs would provide type safety.
- **Depth test for MemoryService:** Would the type mapping and OCAP error classification be needed by any future API route or MCP tool that accesses episodic memory? If yes, create `MemoryService`. If it's purely HTTP response formatting, leave in surface.
- **Strategy:** Read `routes/episodic.rs`. Identify stringly-typed error matches. Design typed error variants. Replace `serde_json::Value` with typed structs. Consider whether this warrants a `MemoryService` or if it's surface-only.

### Phase 3: Project completion assessment

After all evaluations:

1. If all remaining files fail the depth test AND `routes/episodic.rs` is surface-only → **Project is complete.** Update HANDOFF.md with final status, mark all remaining files as "surface-only."
2. If any file passes the depth test → Extract it following the strangler fig RED→GREEN→WIRED→DELETED cycle.
3. After any extractions OR all skips documented → Final workspace verification.

---

## Per-Extraction Discipline (from `refactor-service-layer` skill)

```
[ ] RED:    Write failing test for the service operation with // REQ: tag
[ ] GREEN:  Implement the minimal service operation that passes the test
[ ] Wire CLI: Change CLI to call service, delete duplicate logic
[ ] Wire API: Change API to call service, delete duplicate logic (if applicable)
[ ] Verify:  cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings
[ ] Deletion test: Service module is deep, not a shallow pass-through
[ ] Dependency direction verified: no circular deps
```

---

## Key Constraints to Preserve

- **P3:** CLI → services → domain. No circular deps.
- **P5:** One domain per extraction. Every changed line traces to it.
- **Headless:** No visual UI, no dashboards, no monitoring stacks.
- **P8:** Every `#[test]` verifies a stated behavioral property. Don't weaken tests.
- **Depth test (P2):** If deleting the proposed module makes complexity vanish, don't create it — merge or deepen instead. **Always apply this test before starting an extraction.** Sessions 21–22 demonstrated 5 depth-test skips. This is the norm, not the exception — most remaining files will likely be surface-only.
- **Surgical changes:** No style fixes, no renaming, no comment additions in adjacent code.

---

## Known Patterns from Prior Extractions

- Services that open DB before ServiceContext exists (onboarding, consolidation, compose, embed) accept `db_path` + `db_passphrase` as parameters — the service doesn't impose path conventions.
- `SpecStore` is a trait; `ServiceContext` stores `SqliteSpecStore`. When a service needs a spec store, accept `&SqliteSpecStore` (concrete type).
- `Keychain::default()` creates a keychain with service name "hkask". Services that interact with the keychain use this default.
- Error mapping: service uses `ServiceError` variants with `#[from]` where possible. CLI surfaces add `From<ServiceError> for TheirError` impls.
- `Database::open` is a legitimate legacy pattern in onboarding and consolidation — these must open DB before ServiceContext exists.
- Services that resolve credentials internally (ArchivalService, OnboardingService) use `resolve_credential()` from `hkask_mcp::server`.
- Type relocations: domain types (config structs, result types, error enums) move from CLI to services. Surface presentation types (CLI error enums, prompt helpers) stay in CLI.
- `ServiceError` sentinel variants: each new service adds a `ServiceError::<Domain>(String)` variant. Current count: 29 variants.

---

## Build Commands

```bash
# Per-crate verification
cargo check -p hkask-services && cargo clippy -p hkask-services -- -D warnings
cargo check -p hkask-cli && cargo clippy -p hkask-cli -- -D warnings
cargo check -p hkask-api && cargo clippy -p hkask-api -- -D warnings

# Full workspace verification (run after every extraction or evaluation)
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

## When Finished with an Evaluation or Extraction

Update `HANDOFF.md`:
- Add key decision (e.g., "#67 — `git_cmd.rs` CAS operations: depth test fails, surface-only")
- Update file reference map if any file was extracted
- Update completion counts

Update `CONTINUATION.md`:
- Mark extraction done or evaluation skipped
- Update priority list status
- If all targets are resolved, update "Honest Assessment" to reflect project completion

If the project is complete, add a **Project Complete** section to HANDOFF.md with:
- Final metrics (commands extracted, services created, depth-test skips, tests)
- Completion date
- Summary of what the service layer now provides

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*