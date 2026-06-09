# CONTINUATION.md — hKask Service Layer Extraction

**Sessions:** 12–19 | **Next:** Session 20+

## Context

You are continuing the hKask service layer extraction project. Sessions 12–17 built the infrastructure (ServiceContext/ServiceConfig, dependency injection wiring, dead code cleanup, ReplState deduplication). Session 18 extracted the first deep service module (ChatService). Session 19 extracted AgentService, deduplicated consolidation.rs CLI, and extracted UserService.

**The project is NOT complete.** 20 of 27 CLI commands still contain inline business logic. The infrastructure wiring and 3 deep extractions are done; the remaining business logic extraction continues.

**Read these files first:**
1. `HANDOFF.md` — session history, remaining work inventory, key decisions (decisions #1–#36)
2. `.agents/skills/refactor-service-layer/SKILL.md` — the strangler fig extraction methodology governing this refactoring

## Honest Assessment

Sessions 12–18 accomplished **infrastructure wiring** + **1 deep extraction** (ChatService). Session 19 added **2 more deep extractions** (AgentService, UserService) and **1 CLI deduplication** (consolidation.rs). The service modules in `hkask-services` now include 3 deep modules, 2 medium modules, and 5 shallow/medium modules.

**What's left:** Extracting inline business logic from ~20 CLI files + 4 API routes into ~10 new or extended service modules, then deleting the legacy inline code. Estimated 22-32 hours of focused work.

## Extraction Priority

Follow the strangler fig pattern per the refactor-service-layer skill: **one domain per session, RED→GREEN→WIRED→DELETED per extraction.**

### P1 — High impact (next targets)

1. **`compose.rs` → `ComposeService`** — DB + SemanticMemory construction, KNN search, exemplar pipeline, centroid validation, cosine distance. The `run_compose()` function at line 84 is ~300 lines of inline pipeline construction (DB open → TripleStore → EmbeddingStore → SemanticMemory → OkapiEmbedding → KNN retrieval → system prompt → inference → centroid validation). This is CLI-only (no API route). ~3-4h.

2. **`ensemble.rs` improv operations → Extend `EnsembleService`** — `ensemble_improv_turn` (line 149), `ensemble_improv_config` (line 186), `ensemble_improv_set_threshold` (line 201), `ensemble_improv_set_mode` (line 218), `ensemble_participants` (line 235) all directly access `ctx.session_manager` and construct improv clients with session-not-found error handling. The existing `EnsembleService` already handles chat/deliberation ops; extend it with improv ops. Note: standing sessions are explicitly excluded per decision in `ensemble.rs` service module doc comment. ~2-3h.

3. **`onboarding.rs` → `OnboardingService`** — Secret derivation, keychain storage, DB cleanup, replicant registration, sign-in verification loop. `run_onboarding()` at line 112 orchestrates a multi-step bootstrap flow. This is the largest single extraction. ~3-4h.

### P2 — Medium impact

4. **`cns.rs` → `CnsService`** — CNS runtime access, set-point config, queue depth. ~1-2h.
5. **`spec.rs` → `SpecService`** — Spec curation, MiniJinja rendering, Spec construction. ~2h.
6. **`git_archival.rs` → `ArchivalService`** — GitHub REST API calls, base64 encoding, registry serialization. ~2-3h.
7. **`embed_corpus.rs` → `EmbedService`** — HTTP download, corpus chunking, embedding batch loop. ~2-3h.

### P3 — Lower impact (CLI-only, no API duplication)

8. **`skill.rs` → `SkillService`** — Filesystem discovery, hash computation. ~2h.
9. **`keystore.rs` → `KeystoreService`** — Keychain CRUD, .env file parsing. ~1-2h.
10. **`magna_carta.rs` → `VerificationService`** — Manifest loading, structural audits. ~2h.
11. **`mcp.rs` / `models.rs` / `web_search.rs`** — MCP dispatcher invocation patterns. Centralize into an `McpService` or use existing `InferenceService::list_models()`. ~2-3h.

### API-specific

12. **`routes/ensemble.rs` standing_start + improv_turn** — Extend `EnsembleService` with `start_standing_session()` and `improv_turn()`. ~1-2h.
13. **`routes/episodic.rs`** — Fix stringly-typed OCAP error classification, centralize `serde_json::Value` → typed DTO mapping. Consider `MemoryService`. ~1-2h.

## Skills to Load

1. **`refactor-service-layer`** — **Required.** This is the governing methodology. Read it before starting any extraction.
2. **`coding-guidelines`** — **Required.** Surgical changes: each extraction touches exactly one domain.
3. **`constraint-forces`** — Recommended. Use to classify design decisions.
4. **`zoom-out`** — Recommended before starting each P1 extraction.

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

## Extraction-Specific Notes

### compose.rs → ComposeService (P1 #1)

- **CLI-only** — no API route for compose. Service will serve only CLI.
- **DB path convention** — compose uses user-provided `db_path` and `passphrase` (not ServiceContext's). Like consolidation, accept `db_path` + `db_passphrase` as parameters.
- **CognitionConfig** — the YAML config deserialization (lines 24-61) is compose-specific. Evaluate whether it moves to the service (makes it available to future API routes) or stays in CLI (surface-specific config parsing).
- **InferenceContext::from_parts()** — listed as a "legitimate legacy pattern" in HANDOFF.md for compose. The compose command constructs its own `InferenceContext` because it uses user-provided DB credentials. This stays as a surface concern; the service takes a pre-resolved inference port or config.
- **Key operations to extract:** DB open + pipeline construction, KNN search with distance threshold, system prompt composition with exemplars, centroid validation with cosine distance check.

### ensemble.rs improv ops → Extend EnsembleService (P1 #2)

- **Standing sessions explicitly excluded** — per the existing doc comment in `hkask-services/src/ensemble.rs` lines 19-28, standing sessions are classified as Divergent and stay in surface code.
- **Session-not-found pattern** — 5 functions repeat `manager.read().await; manager_read.get_chat(session_id).await ... .ok_or_else(|| format!("Chat session '{}' not found", session_id))`. Centralize into `EnsembleService::get_chat_session()`.
- **Improv client access** — `improv_turn` needs the improv client from the chat session, then calls methods on it. The session manager lookup + client extraction is the duplicated pattern.
- **`ensemble_improv_turn` is the deepest** — it also needs an inference port, making it similar to ChatService. Evaluate whether the inference port should be a parameter or derived from ServiceContext.

### onboarding.rs → OnboardingService (P1 #3)

- **Largest extraction** — multi-step bootstrap flow with secret derivation, keychain storage, DB init, replicant registration, sign-in verification.
- **`Database::open` in onboarding** — listed as a "legitimate legacy pattern" in HANDOFF.md because onboarding must open DB before ServiceContext exists. The service may need to accept pre-opened DB connections or use a different pattern.
- **`ResolvedSecrets` struct** — listed as legitimate in `onboarding.rs`. The service should own this type.
- **Two paths in `run_onboarding()`** — fast path (keys already configured → init registry → sign in) and slow path (derive secrets → store in keychain → init registry → register replicant → sign in). Both paths need extraction.

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