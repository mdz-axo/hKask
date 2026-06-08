# CONTINUATION.md — hKask Service Layer Extraction

**Sessions:** 12–18 | **Next:** Session 19+

## Context

You are continuing the hKask service layer extraction project. Sessions 12–17 built the infrastructure (ServiceContext/ServiceConfig, dependency injection wiring, dead code cleanup, ReplState deduplication). Session 18 extracted the first deep service module (ChatService) proving the pattern works.

**The project is NOT complete.** 22 of 27 CLI commands and several API routes still contain inline business logic that belongs in service modules. The infrastructure wiring is done; the actual business logic extraction is the remaining work.

**Read these files first:**
1. `HANDOFF.md` — session history, remaining work inventory, key decisions
2. `.agents/skills/refactor-service-layer/SKILL.md` — the strangler fig extraction methodology governing this refactoring

## Honest Assessment

Sessions 12–17 accomplished **infrastructure wiring** — making ServiceContext available in every surface. This was necessary but not sufficient. The service modules in `hkask-services` are mostly shallow delegates (41 of 57 public items). Only ChatService is a truly deep module that eliminates duplicated business logic.

**What's left:** Extracting inline business logic from 22 CLI files + 4 API routes into ~12 new or extended service modules, then deleting the legacy inline code. Estimated 28-40 hours of focused work.

## Extraction Priority

Follow the strangler fig pattern per the refactor-service-layer skill: **one domain per session, RED→GREEN→WIRED→DELETED per extraction.**

### P0 — Immediate (highest impact, clear duplication)

1. **`agent.rs` → `AgentService`** — Agent registration flow (ACP + definition + store insert) and agent listing via loader are inline. The CLI's `agent_register`, `agent_unregister`, and `bot_list` all contain business logic that should be in a service. ~2-3h.

2. **`consolidation.rs` CLI → delete inline code, delegate to existing `ConsolidationService`** — The CLI command **completely duplicates** the logic already in `hkask_services::ConsolidationService`. It opens DB, constructs TripleStore/EpisodicMemory/SemanticMemory/ConsolidationBridge, and verifies passphrases — all inline. Delete ~70 lines, delegate. ~1h.

3. **`user.rs` → `UserService`** — Registration, login, session management operations on `Arc<Mutex<UserStore>>` are business logic. Extract into a service with typed error normalization. ~2-3h.

### P1 — High impact

4. **`ensemble.rs` improv operations → Extend `EnsembleService`** — `ensemble_improv_turn`, `ensemble_improv_config`, `ensemble_standing_start` directly access `session_manager` and construct improv clients. Extend the existing service. ~2-3h.

5. **`onboarding.rs` → `OnboardingService`** — Secret derivation, keychain storage, DB cleanup, replicant registration, sign-in verification loop. This is the largest single extraction. ~3-4h.

6. **`compose.rs` → `ComposeService`** — DB + SemanticMemory construction, KNN search, exemplar pipeline, centroid validation, cosine distance. ~3-4h.

### P2 — Medium impact

7. **`cns.rs` → `CnsService`** — CNS runtime access, set-point config, queue depth. ~1-2h.
8. **`spec.rs` → `SpecService`** — Spec curation, MiniJinja rendering, Spec construction. ~2h.
9. **`git_archival.rs` → `ArchivalService`** — GitHub REST API calls, base64 encoding, registry serialization. ~2-3h.
10. **`embed_corpus.rs` → `EmbedService`** — HTTP download, corpus chunking, embedding batch loop. ~2-3h.

### P3 — Lower impact (CLI-only, no API duplication)

11. **`skill.rs` → `SkillService`** — Filesystem discovery, hash computation. ~2h.
12. **`keystore.rs` → `KeystoreService`** — Keychain CRUD, .env file parsing. ~1-2h.
13. **`magna_carta.rs` → `VerificationService`** — Manifest loading, structural audits. ~2h.
14. **`mcp.rs` / `models.rs` / `web_search.rs`** — MCP dispatcher invocation patterns. Centralize into an `McpService` or use existing `InferenceService::list_models()`. ~2-3h.

### API-specific

15. **`routes/ensemble.rs` standing_start + improv_turn** — Extend `EnsembleService` with `start_standing_session()` and `improv_turn()`. ~1-2h.
16. **`routes/episodic.rs`** — Fix stringly-typed OCAP error classification, centralize `serde_json::Value` → typed DTO mapping. Consider `MemoryService`. ~1-2h.

## Skills to Load

1. **`refactor-service-layer`** — **Required.** This is the governing methodology. Read it before starting any extraction. It defines the strangler fig sequence (P1), depth test (P2), dependency direction (P3), and surgical change principles (P5).
2. **`coding-guidelines`** — **Required.** Surgical changes: each extraction touches exactly one domain. No "while we're here" changes. No renaming. No comment additions.
3. **`constraint-forces`** — Recommended. Use to classify design decisions that arise during extraction (e.g., whether a module is too shallow to exist, whether a CLI-specific concern belongs in the service layer).
4. **`zoom-out`** — Recommended before starting each P0/P1 extraction to understand the full caller graph.

## Per-Extraction Checklist

Follow the strangler fig sequence from the refactor-service-layer skill:

```
[ ] RED: Write failing test for the service operation with // REQ: tag
[ ] GREEN: Implement the minimal service operation that passes the test
[ ] Wire CLI: Change CLI to call service, delete duplicate logic
[ ] Wire API: Change API to call service, delete duplicate logic (if applicable)
[ ] cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings
[ ] Deletion test: Service module is deep, not a shallow pass-through
[ ] Dependency direction verified: no circular deps
```

## Key Constraints

- **P3 (Dependency direction):** CLI → services → domain. No circular deps.
- **P5 (One domain per session):** Each extraction is a separate, atomic change.
- **Surgical changes:** Every changed line traces to the extraction. No style fixes in adjacent code.
- **Headless constraint:** No visual UI, no dashboards, no monitoring stacks.
- **P8 (Test quality):** Every `#[test]` verifies a stated behavioral property. Don't weaken tests.
- **Depth test (P2):** If deleting the proposed module makes complexity vanish, don't create it — merge or deepen instead.

## Build Commands

```bash
cargo check -p hkask-services          # Service layer check
cargo clippy -p hkask-services -- -D warnings
cargo check -p hkask-cli               # CLI check
cargo clippy -p hkask-cli -- -D warnings
cargo check -p hkask-api                # API check
cargo clippy -p hkask-api -- -D warnings
cargo check --workspace                 # Full workspace check
cargo clippy --workspace -- -D warnings # Full workspace lint
cargo test --workspace                  # Full workspace test
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*