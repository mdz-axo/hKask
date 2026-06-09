# CONTINUATION PROMPT — hKask Service Layer Extraction (Session 22+)

**Load these skills before starting:**

1. `refactor-service-layer` — **Required.** Governing methodology: strangler fig sequence, depth test (P2), dependency direction (P3), surgical changes (P5). Every extraction follows its RED→GREEN→WIRED→DELETED cycle. **Pay special attention to the depth test** — Session 21 skipped `cns.rs` because it would have been a shallow pass-through.
2. `coding-guidelines` — **Required.** Surgical changes only: each extraction touches exactly one domain. No "while we're here" changes. No renaming. No comment additions.
3. `zoom-out` — **Required before each new P2 extraction.** Produce the module map, caller graph, and data flow "before picture" for the target file.
4. `constraint-forces` — **Recommended.** Classify design decisions by force type (Prohibition, Guardrail, Guideline, Evidence, Hypothesis). Use when deciding whether a CLI-specific concern belongs in the service layer, or when evaluating if a module is too shallow to extract.
5. `diagnose` — **Available if needed.** For unexpected compilation errors or test failures during extraction.

**Read these files first (in this order):**

1. `HANDOFF.md` — Session history (Sessions 12–21), remaining work inventory with effort estimates, key decisions (#1–#57), deep service module inventory, file reference map, legitimate legacy patterns.
2. `CONTINUATION.md` — Priority-ordered extraction targets (P2–P3 + API-specific), per-extraction checklist, extraction-specific notes for remaining targets, key constraints, build commands, recommended session strategy.
3. `.agents/skills/refactor-service-layer/SKILL.md` — The strangler fig methodology governing all extractions (P1–P6 principles, Phase 0–8 process, anti-patterns, checklists).

**Current state:**

- **Infrastructure wiring: DONE.** ServiceContext/ServiceConfig are built and wired to every surface. ReplState has zero duplicated fields. Dead code is deleted. Workspace passes check+clippy+test.
- **Deep extractions: 5 complete** (ChatService, AgentService, UserService, ComposeService, OnboardingService) + SpecService (medium-deep) + EnsembleService extended with 5 improv ops + consolidation.rs CLI deduplicated + `registration.rs` deleted. **13 of 27** CLI commands fully extracted.
- **Depth-test skip: 1** (CnsService — `cns.rs` is mostly `println!` formatting, domain logic already in `hkask_cns`).
- **Remaining work:** ~14 CLI commands + 2 API routes contain inline business logic needing extraction into ~7 new or extended service modules. Estimated 11–20 hours.

**Start with P2 targets in order:**

1. **`git_archival.rs` → `ArchivalService`** (~2-3h) — GitHub REST API calls via `reqwest`, base64 encoding, registry serialization. CLI-only currently. Uses `hkask_templates::SqliteRegistry` for reading template metadata and `reqwest` for HTTP calls to GitHub API. Service will need `reqwest` dep or accept a caller-provided client. Apply depth test: the file constructs GitHub API URLs, encodes payloads as base64, parses JSON responses, and serializes registry data — this is real domain logic that would reappear in any API caller.

2. **`embed_corpus.rs` → `EmbedService`** (~2-3h) — HTTP download via `reqwest`, corpus chunking, embedding batch loop with `OkapiEmbedding`, centroid computation. Similar to ComposeService in DB + SemanticMemory construction. Uses user-provided DB credentials (like ComposeService and ConsolidationService). Service accepts caller-provided `db_path` + `db_passphrase`.

**After P2, continue with P3 targets:** `skill.rs` → `SkillService` (~2h), `keystore.rs` → `KeystoreService` (~1-2h), `magna_carta.rs` → `VerificationService` (~2h), `mcp.rs`/`models.rs`/`web_search.rs` → evaluate for consolidation (~2-3h).

**Then API-specific:** `routes/episodic.rs` — fix stringly-typed OCAP error classification, centralize `serde_json::Value` → typed DTO mapping. Consider `MemoryService`. (~1-2h).

**Per-extraction discipline (from `refactor-service-layer` skill):**

```
[ ] RED:    Write failing test for the service operation with // REQ: tag
[ ] GREEN:  Implement the minimal service operation that passes the test
[ ] Wire CLI: Change CLI to call service, delete duplicate logic
[ ] Wire API: Change API to call service, delete duplicate logic (if applicable)
[ ] Verify:  cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings
[ ] Deletion test: Service module is deep, not a shallow pass-through
[ ] Dependency direction verified: no circular deps
```

**Key constraints to preserve:**

- **P3:** CLI → services → domain. No circular deps.
- **P5:** One domain per extraction. Every changed line traces to it.
- **Headless:** No visual UI, no dashboards, no monitoring stacks.
- **P8:** Every `#[test]` verifies a stated behavioral property. Don't weaken tests.
- **Depth test (P2):** If deleting the proposed module makes complexity vanish, don't create it — merge or deepen instead. **Always apply this test before starting an extraction.** Session 21 demonstrated that `cns.rs` fails this test — it was correctly skipped.
- **Surgical changes:** No style fixes, no renaming, no comment additions in adjacent code.

**When finished with an extraction,** update `HANDOFF.md` (add key decision, update file reference map, update completion counts) and `CONTINUATION.md` (mark extraction done, update priority list).

**Known patterns from prior extractions:**

- Services that open DB before ServiceContext exists (onboarding, consolidation, compose, embed_corpus) accept `db_path` + `db_passphrase` as parameters — the service doesn't impose path conventions.
- `SpecStore` is a trait; `ServiceContext` stores `SqliteSpecStore`. When a service needs a spec store, accept `&SqliteSpecStore` (concrete type) to avoid generic constraints in callers.
- `Keychain::default()` creates a keychain with service name "hkask". Services that interact with the keychain (OnboardingService, ConsolidationService) use this default.
- Error mapping: service uses `ServiceError` variants with `#[from]` where possible. CLI surfaces add `From<ServiceError> for TheirError` impls. Use `.inspect_err()` for side effects (cleanup), not `.map_err(|e| { cleanup(); e })`.
- `Database::open` is a legitimate legacy pattern in onboarding and consolidation — these must open DB before ServiceContext exists.