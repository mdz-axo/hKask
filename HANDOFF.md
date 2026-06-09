# HANDOFF.md — hKask Service Layer Extraction

**Sessions:** 12–23 | **Status:** ✅ COMPLETE. 10 deep service modules + 2 medium-deep extracted. 17/27 CLI commands fully extracted. 8 depth-test skips. 1 API route OCAP fix. All remaining files evaluated and documented as surface-only. | **Verification:** `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace` all pass (4 pre-existing pod test failures unrelated). See §7 for final metrics and follow-up tasks.

---

## 1. Session History

### Sessions 12–17
Infrastructure wiring: ServiceContext/ServiceConfig created, all surfaces wired to ServiceContext, dead code deleted, ReplState deduplicated. See prior HANDOFF versions for details.

### Session 18 (Dead Code + ReplState Dedup + ChatService Extraction)
- **Step 10:** Deleted `commands/config.rs` entirely (9 dead functions). Moved `ResolvedSecrets` into `onboarding.rs`.
- **Step 11:** Removed all 5 duplicated ReplState fields. All consumers now read from `state.service_context.<field>`.
- **Step 12:** Full workspace verification passed.
- **ChatService extraction:** Created `hkask_services::ChatService` — the first DEEP service module. Encapsulates the full chat turn pipeline (agent lookup → prompt composition → semantic recall → inference → episodic storage) that was previously ~380 lines of inline business logic in `commands/chat.rs`. Wired both CLI `chat_with_agent()` and API `routes/chat.rs` through `ChatService::chat()`. CLI `chat.rs` reduced from ~470 lines to ~140 lines.

### Session 19 (AgentService + ConsolidationService Dedup + UserService)
- **P0 #1 — AgentService:** Created `hkask_services::AgentService` with 4 operations (`register`, `list`, `status`, `unregister`). Moves the 6-step registration flow (WebID parse → AgentKind validate → ACP register → AgentDefinition construction → RegisteredAgent assembly → store insert) and the loader-boot + filtering pattern from CLI into the service layer. `AgentReceipt` moved from CLI to services. Added `ServiceError::InvalidWebID` variant and `From<uuid::Error>` impl. CLI `commands/agent.rs` reduced from ~250 lines to ~170 lines. API `acp.rs` routes left as-is (previously evaluated as shallow).
- **P0 #2 — consolidation.rs CLI deduplication:** Deleted ~70 lines of inline DB+pipeline+passphrase code from `commands/consolidation.rs`. Now delegates to `ConsolidationService::verify_passphrase()` and `ConsolidationService::consolidate()`. Added `db_path` parameter to `ConsolidationService::consolidate()` so CLI and API can use their own path conventions. Dropped pre-consolidation stats reporting. CLI `commands/consolidation.rs` reduced from ~127 lines to ~71 lines.
- **P0 #3 — UserService:** Created `hkask_services::UserService` with 8 operations (`validate_passphrase`, `validate_registration`, `register`, `login`, `get_replicant`, `list_replicants`, `list_sessions`, `revoke_session`). Moved passphrase and registration validation from `hkask-cli/src/registration.rs` into the service layer. Added `From<PoisonError<T>> for ServiceError` for lock acquisition. Deleted `hkask-cli/src/registration.rs`. CLI `commands/user.rs` now delegates all application functions to UserService.

### Session 20 (ComposeService + EnsembleService Improv Extension)
- **P1 #1 — ComposeService:** Created `hkask_services::ComposeService` with 1 operation (`compose`). Moves the ~200-line Hemingway style synthesizer pipeline (DB open → TripleStore → EmbeddingStore → SemanticMemory → OkapiEmbedding → KNN search → filter → deduped retrieval → prompt composition → inference → centroid validation) from CLI into the service layer. `CognitionConfig` and sub-types moved from CLI to services. `cosine_distance()` utility moved to services. Added `ComposeRequest`, `ComposeResult`, `CentroidValidation` types. Added `ServiceError::Embedding(#[from] EmbeddingGenerationError)` variant. Added `serde_yaml` dependency to services crate. CLI `commands/compose.rs` reduced from ~378 lines to ~121 lines. 11 service-layer tests.
- **P1 #2 — EnsembleService improv extension:** Extended `hkask_services::EnsembleService` with 5 new operations (`improv_turn`, `improv_config`, `set_improv_threshold`, `set_improv_mode`, `list_participants`). Moves the session-not-found + session-access pattern that was duplicated across 5 CLI functions and 1 API route into the service layer. `improv_turn` is the deep operation (session lookup + inference adapter + turn execution + message persistence). Added `ParticipantInfo` struct for decoupled participant data. Added `ServiceError::Improv(String)` variant. Wired CLI and API improv_turn through EnsembleService. 6 new service-layer tests. CLI `commands/ensemble.rs` reduced by ~60 lines of inline session management code.

### Session 21 (OnboardingService Extraction)
- **P1 #1 — OnboardingService:** Created `hkask_services::OnboardingService` with 8 operations (`derive_and_store_secrets`, `derive_secrets`, `init_registry`, `register_replicant`, `try_sign_in`, `try_list_existing_replicants`, `remove_orphaned_db`, `cleanup_failed_onboarding`). Moves the full multi-step bootstrap flow (secret derivation → keychain storage → DB init + ACP state restoration → replicant registration → sign-in verification → failure cleanup) from CLI into the service layer. `ResolvedSecrets` and `SignInOutcome` types moved from CLI to services. `RegistryHandle` type introduced for `init_registry` return. Added `From<ServiceError> for OnboardingError` impl in CLI. CLI `onboarding.rs` reduced from ~639 lines to 377 lines. 6 service-layer tests. Interactive I/O (`prompt_line`, `prompt_passphrase`, `prompt_choice`) stays in CLI surface. `Database::open` in onboarding remains a legitimate legacy pattern — service accepts caller-provided `ServiceConfig`. `commands/chat.rs` and `repl/mod.rs` updated to use `hkask_services::ResolvedSecrets` instead of `crate::onboarding::ResolvedSecrets`.
- **CnsService evaluated and SKIPPED.** Depth test fails — `cns.rs` is mostly `println!` formatting. The domain logic (CnsRuntime health/alerts/variety calls) is already well-encapsulated in `hkask_cns`. API routes already access CnsRuntime through ServiceContext. No duplicated business logic between surfaces.
- **P2 #3 — SpecService:** Created `hkask_services::SpecService` with 5 operations (`capture`, `build_spec`, `validate`, `cultivate`, `list_categories`). Moves the spec construction pipeline (parse category → parse domain → build goal → build spec → save) and the evaluation pipeline (load → curator evaluate) from CLI into the service layer. `CapturedSpec` and `EvaluatedSpec` types introduced. CLI `commands/spec.rs` Capture, Validate, Cultivate actions now delegate to SpecService. API `routes/spec.rs` capture route delegates to `SpecService::build_spec()`. 5 service-layer tests. Render action stays in CLI (MiniJinja template rendering is surface-specific).

### Session 22 (ArchivalService + EmbedService + SkillService + VerificationService Extraction)
- **P2 #4 — ArchivalService:** Created `hkask_services::ArchivalService` with 4 operations (`archive_to_git`, `restore_from_git`, `list_archives`, `create_snapshot`). Moves the full GitHub REST API integration (credential resolution → authenticated client build → base64 encode/decode → conditional SHA handling → JSON payload construction → response parsing → registry serialization) from CLI into the service layer. `ArchiveResult` and `SnapshotResult` types introduced for structured returns. `ServiceError::Archival(String)` added as sentinel. Added `reqwest` and `base64` deps to services crate. CLI `commands/git_cmd.rs` Archive/Restore/List/Snapshot actions now delegate to ArchivalService. Dead `McpRuntime` and `CapabilityChecker` parameters dropped from CLI calls. CLI `git_archival.rs` (238 lines) deleted entirely — all callers now use ArchivalService. `reqwest` and `base64` removed from CLI Cargo.toml (orphaned). 7 service-layer tests.
- **P2 #5 — EmbedService:** Created `hkask_services::EmbedService` with 2 operations (`embed_corpus`, `parse_config`). Moves the full style corpus embedding pipeline (config parsing → DB open → purge → download/caching/chunking → batch embedding → centroid computation) from CLI into the service layer. `CorpusConfig` and 6 sub-types (`EmbeddingConfig`, `Work`, `FoundationalRule`, `ChunkingConfig`, `ValidationConfig`, `EmbedResult`) moved from CLI to services. `ServiceError::Embed(String)` added as sentinel. CLI `commands/embed_corpus.rs` reduced from ~290 lines to ~60 lines. `Database::open` in embed_corpus remains a legitimate legacy pattern — service accepts caller-provided `db_path` + `db_passphrase`. 9 service-layer tests.
- **P3 #6 — SkillService:** Created `hkask_services::SkillService` with 7 operations (`discover_skills`, `read_skill_visibility`, `read_skill_namespace`, `compute_content_hash`, `compute_file_hash`, `find_public_skill`, `publish_skill`). Moves the full skill visibility management and publishing pipeline (replicant name resolution → zone discovery → BLAKE3 hashing → SKILL.md YAML mutation → namespaced publishing) from CLI into the service layer. `SkillInfo` and `SkillPublishResult` types introduced. `ServiceError::Skill(String)` added as sentinel. Added `hex` dep to services crate. CLI `commands/skill.rs` reduced from ~453 lines to ~170 lines. 8 service-layer tests.
- **P3 #8 — VerificationService:** Created `hkask_services::VerificationService` with 3 operations (`verify`, `verify_json`, `load_manifests`). Moves the full Magna Carta verification pipeline (manifest loading → assertion dispatch → structural audit / resource verification / absence check → report building → JSON serialization) from CLI into the service layer. `Manifest`, `Assertion`, `AssertionResult`, `PrincipleResult`, `VerificationReport` types moved from CLI to services. `ServiceError::Verification(String)` added as sentinel. CLI `commands/magna_carta.rs` reduced from ~556 lines to ~102 lines. 9 service-layer tests.
- **KeystoreService SKIPPED.** Depth test fails — `keystore.rs` is thin pass-through over `Keychain` API (`.env` parsing is CLI presentation). The `hkask_keystore::Keychain` is already the deep module.
- **McpService SKIPPED.** Depth test fails — `mcp.rs`/`models.rs`/`web_search.rs` are surface adapters over `mcp_dispatcher.invoke()`. No business logic to extract.

### Session 23 (Final Evaluation Sweep + OCAP Error Fix)
- **Phase 1 — Depth-test evaluations:** All 4 remaining partially extracted CLI files failed the depth test. `git_cmd.rs` CAS ops: shallow pass-through over `GitCASPort` (#67). `loops.rs`: pure CLI orchestration, 43 lines (#68). `serve.rs`: pure server startup orchestration, 109 lines (#69). `template.rs`: thin pass-throughs over `SqliteRegistry` + `McpRuntime` (#70). `models.rs` was already evaluated (MCP adapter).
- **Phase 2 — OCAP error fix:** Fixed stringly-typed `MemoryError` matching in `routes/episodic.rs`. Replaced `.to_string().contains("denied")` / `.contains("read-only")` with typed `match &e { MemoryError::CapabilityDenied { .. } => 403, _ => 500 }`. MemoryService extraction SKIPPED — depth test fails (#71): OCAP error classification is HTTP-specific, `serde_json::Value` mapping is API-specific.
- **Phase 3 — Project declared complete.** All 6 completion criteria met. 72 key decisions recorded. 8 depth-test skips documented. 9 open questions identified (F1–F9).

### Session 24 (F9 Typed DTOs + F5 Pod Test Fixture)
- **F9 — Typed DTOs for EpisodicStoragePort (#73):** Added `RecalledEpisode` struct with domain-typed fields (`Confidence`, `Visibility`, `Option<WebID>`) in `hkask-agents/src/ports/memory_storage.rs`. Changed `EpisodicStoragePort::recall_episodic` return type from `Vec<serde_json::Value>` to `Vec<RecalledEpisode>`. Updated `MemoryLoopAdapter` (new `triple_to_recalled_episode` helper), `PodContext`, `PodManager`. Simplified `routes/episodic.rs::query_episodes` — replaced fragile `.get("field").and_then(|v| v.as_str()).unwrap_or_default()` destructuring with direct field mapping. Left `recall_semantic` unchanged (separate concern). Depth test passes: deleting `RecalledEpisode` would force N callers to duplicate the field mapping.
- **F5 — Pod Test ACP Secret Fixture (#74):** Replaced `AcpRuntime::default()` (panics without `HKASK_ACP_SECRET_KEY`) in `PodManager::new_mock()` with `AcpRuntime::new(MOCK_ACP_SECRET)` using a deterministic 32-byte test secret. Both `AcpRuntime` and `CapabilityChecker` share the same secret so tokens signed by the runtime are verifiable by the checker. 4 previously-failing pod tests (`activate_pod_returns_not_found`, `deactivate_pod_returns_not_found`, `get_pod_status_returns_not_found`, `list_pods_returns_empty`) now pass. Test-only secret documented as Guardrail (security concern) — acceptable for test fixtures with explicit "Never use in production" annotation.
- **Verification:** `cargo check --workspace` ✅. `cargo clippy -p hkask-agents -p hkask-services -p hkask-api -- -D warnings` ✅. `cargo test --workspace` ✅ (138 passed in hkask-services, 51 passed in condenser, 0 failed).

### Session 25 (F10 Typed DTOs + OPEN_QUESTIONS.md)
- **F10 — Typed DTOs for SemanticStoragePort (#75):** Added `RecalledSemantic` struct (no `perspective` field — semantic triples are perspective-free by definition) in `hkask-agents/src/ports/memory_storage.rs`. Changed `SemanticStoragePort::recall_semantic` return type from `Vec<serde_json::Value>` to `Vec<RecalledSemantic>`. Replaced `triple_to_json` with `triple_to_recalled_semantic` in `MemoryLoopAdapter`. Updated `PodContext::recall_semantic` return type. Simplified `ChatService::recall_semantic` — replaced `t.get("value").and_then(|v| v.as_str())` with `t.value.as_str()`. Deleted `triple_to_json` (no remaining callers). Depth test passes: deleting `RecalledSemantic` would force N callers to duplicate the field mapping.
- **OPEN_QUESTIONS.md:** Created at project root with structured F1–F10 entries (5 resolved, 5 deferred) including constraint force classifications, affected crates, and recommended resolution approaches.
- **Condenser build fix:** Already resolved — `cargo build -p hkask-mcp-condenser` and `cargo clippy -p hkask-mcp-condenser` both pass. `ToolSpanGuard::internal_error` was not renamed.
- **Verification:** `cargo check --workspace` ✅. `cargo clippy --workspace -- -D warnings` ✅. `cargo test --workspace` ✅ (all 0 failures, 138 hkask-services tests, 51 condenser tests).

### Session 27 (Auth & Streaming Completion)

- **F3 — AuthContext completion:** Unified `ChatService::chat()` to use `ctx.capability_checker.grant_registry()` for both authenticated (API) and anonymous (CLI) paths. Previously, the legacy path minted tokens with `config.acp_secret` directly; now both paths derive tokens through the same `mcp_secret`-backed checker. When `AuthContext` is provided, the caller's WebID is the delegator; when absent, `ctx.system_webid` is used. Removed `DelegationResource` import from `chat.rs` (no longer needed). Documented the `mcp_secret`/`acp_secret` split in `ServiceConfig` as a valid Guardrail (defense in depth — in-process vs inter-process HMAC keys serve different trust boundaries). Added doc comments to `ServiceContext::capability_checker` clarifying it uses `mcp_secret`. (#79)

  **Audit finding:** Only `ChatService::chat()` in `hkask-services` creates `DelegationToken` directly. All other `DelegationToken::new` calls are in CLI surfaces (`invoke.rs`, `tool_augmented.rs`), domain crates (`AgentPod::new`, `CapabilityChecker::grant_*` factories), template executor, or MCP servers. Service-layer AuthContext threading is complete.

  **Secret split resolution:** `acp_secret` (in-process ACP: `AcpRuntime`, `PodManager`, `FullMcpAdapter`, `ManifestExecutor`, CLI tool invocation) vs `mcp_secret` (inter-process: API auth middleware, `ServiceContext::capability_checker`, `McpDispatcher`). These serve different trust boundaries. Collapsing them would weaken defense in depth. Classified as **Guardrail** — measured boundary, user-overridable with informed consent.

- **F1 — Streaming override + API SSE endpoint:** Overrode `generate_stream()` in `OkapiInference` to send `stream: true` and parse SSE/NDJSON responses into `InferenceStreamChunk` items. Added `generate_stream_with_model()` to `InferencePort` trait (with default fallback to single-chunk from `generate_with_model`) and blanket `Arc<dyn InferencePort>` impl. Added `POST /api/chat/stream` SSE endpoint that calls `generate_stream_with_model()` and yields `InferenceStreamChunk` items as SSE `data:` events via a `tokio::sync::mpsc` channel bridge (satisfies `'static` bound for the SSE response). Added `stream: Option<bool>` to `OkapiRequest`. Added SSE streaming response types (`StreamChunk`, `StreamChoice`, `StreamDelta`). Added `tokio-stream` dependency to `hkask-api`. Remaining: CLI incremental printing. (#80)

- **Verification:** `cargo check --workspace` ✅. `cargo clippy --workspace -- -D warnings` ✅. `cargo test -p hkask-services -p hkask-api -p hkask-types -p hkask-templates` ✅ (0 failures). Pre-existing `hkask-cns` test compile error (mutable gate variable) — unrelated.

### Session 26 (Condenser Test+Debug+Gap Closure)
- **Test suite created:** 53 tests (was 0) covering types (13), algorithms (23), engine (12). Every test has `// REQ:` tag per P8.
- **Bug fix — classify_tool priority inversion:** `classify_tool()` checked ShellCommand substrings first; "run" in "pytest_run" caused wrong classification. Fixed: more-specific categories (TestOutput, BuildOutput) checked before ShellCommand catch-all. Then refactored to two-phase: token-split exact match → substring heuristic fallback.
- **Bug fix — float truncation in target_lines:** All three algorithms used `as usize` (truncating) instead of `.round() as usize`. Light profile with 2 lines: `1.9 → 1` caused unnecessary compression. Fixed across RtkStyleAlgorithm, SaliencyRankAlgorithm, FlashrankAlgorithm.
- **API surface fix — ThreadSummaryRequest.messages:** Changed from `String` (JSON-in-string) to `Vec<serde_json::Value>`. Removed 14-line parse block from `condenser_thread_summary`.
- **Schema fix — JsonSchema on response types:** Added `JsonSchema` derive to `Profile`, `ContextCategory`, `CompressedOutput`, `CondenserStats`, `ThreadSummaryOutput` for proper MCP schema fidelity.
- **Gas cost differentiation:** Added per-tool gas cost for `condenser_thread_summary` (25) vs server default (10) in `TableGasEstimator`. Implemented `tool_costs` HashMap population (was empty despite existing field).
- **Documentation — 12 items fixed:** Removed all `--exclude hkask-mcp-condenser` from CONTINUATION-PROMPT.md, CONTINUATION.md, HANDOFF.md; marked Task 4 ✅; updated README checklist; fixed ERD (TEMPLATE_ABSTRACTION→ALGORITHM_REGISTRY); updated skill doc (6→7 tools); created per-crate README; updated LOC counts; added gas tier to loop-architecture.md.
- **Status docs created:** `docs/status/mcp-tools-inventory.md` (21 servers, 119 tools); `docs/status/test-inventory.md` (12 crates, 42 seams, 192 tests, gap analysis).
- **DDMVSS skill mapping:** Added condenser-continuation to §11 skill-to-DDMVSS table in test-program.md.
- **TODO.md:** Marked P2-12 and P2-13 as ✅ Complete.
- **Verification:** `cargo check --workspace` ✅. `cargo clippy --workspace -- -D warnings` ✅. `cargo test --workspace` ✅ (53 condenser, 138 services, 16 templates, 3 API, 0 failures).

---

## 2. Honest Assessment: Remaining Work

### What Sessions 12–21 Actually Accomplished

**Infrastructure wiring** — the pipes (ServiceContext/ServiceConfig) are built, connected to every surface, and all dead legacy plumbing is deleted.

**Five deep extractions + one extension + one medium-deep** — ChatService, AgentService, UserService, ComposeService, OnboardingService (deep); SpecService (medium-deep); EnsembleService extended with improv ops. Both CLI and API delegate to shared implementations instead of duplicating business logic.

**One CLI deduplication** — consolidation.rs no longer duplicates ConsolidationService.

**One type relocation** — `ResolvedSecrets` moved from CLI `onboarding.rs` to services, with CLI using `hkask_services::ResolvedSecrets`.

**One depth-test skip** — `cns.rs` evaluated and rejected: depth test fails (shallow pass-through, no duplicated logic).

### What Still Needs To Be Done

**Nothing.** All CLI files with extractable business logic have been evaluated.
All files that pass the depth test have been extracted. All files that fail are
documented as surface-only. See §7 (Project Complete) for final metrics.

| Status | CLI Commands | API Routes |
|--------|-------------|------------|
| ✅ Fully extracted | 17/27 | 9/18 |
| ✅ Surface-only (depth-test skip) | 10/27 | 9/18 (incl. episodic typed OCAP fix) |
| ⬜ Stub/N/A | 0/27 (bundle, registry deleted) | 6/18 (templates, mcp, acp, bundles, bots, spec stubs) |

### Priority Extraction Targets (by impact)

| Priority | Target | Inline Logic | Proposed Service | Estimated Effort |
|----------|--------|-------------|-------------------|------------------|
| ~~P1~~ | ~~`onboarding.rs`~~ | ~~Secret derivation, keychain storage, DB cleanup, replicant registration, sign-in flow~~ | ~~`OnboardingService`~~ | ~~✅ DONE (Session 21)~~ |
| ~~P2~~ | ~~`cns.rs`~~ | ~~CNS runtime access, set-point config, queue depth~~ | ~~`CnsService`~~ | ~~SKIPPED — depth test fails, shallow pass-through~~ |
| ~~P2~~ | ~~`spec.rs`~~ | ~~Spec curation, MiniJinja rendering, Spec construction~~ | ~~`SpecService`~~ | ~~✅ DONE (Session 21)~~ |
| ~~P2~~ | ~~`git_archival.rs`~~ | ~~GitHub REST API calls, base64 encoding, registry serialization~~ | ~~`ArchivalService`~~ | ~~✅ DONE (Session 22)~~ |
| ~~P2~~ | ~~`embed_corpus.rs`~~ | ~~HTTP download, corpus chunking, embedding batch loop, centroid computation~~ | ~~`EmbedService`~~ | ~~✅ DONE (Session 22)~~ |
| **P3** | ~~`skill.rs`~~ | ~~Filesystem discovery, hash computation, visibility mutation~~ | ~~`SkillService`~~ | ~~✅ DONE (Session 22)~~ |
| **P3** | ~~`keystore.rs`~~ | ~~Keychain CRUD, .env file parsing~~ | ~~`KeystoreService`~~ | ~~⚠️ SKIPPED — depth test fails, shallow pass-through over Keychain~~ |
| **P3** | ~~`magna_carta.rs`~~ | ~~Manifest loading, structural audits~~ | ~~`VerificationService`~~ | ~~✅ DONE (Session 22)~~ |
| **P3** | ~~`mcp.rs` / `models.rs` / `web_search.rs`~~ | ~~MCP dispatcher invocation patterns~~ | ~~`McpService`~~ | ~~⚠️ SKIPPED — depth test fails, surface adapters over MCP dispatcher~~ |
| **Remaining** | ~~`git_cmd.rs` CAS ops~~ | ~~CAS port calls + println formatting~~ | ~~`GitCasService`~~ | ~~⚠️ SKIPPED — depth test fails, shallow pass-through over GitCASPort (#67)~~ |
| **Remaining** | ~~`loops.rs`~~ | ~~Config → ServiceContext → start → wait → shutdown~~ | ~~`LoopSystemService`~~ | ~~⚠️ SKIPPED — depth test fails, pure surface orchestration (#68)~~ |
| **Remaining** | ~~`serve.rs`~~ | ~~Server startup orchestration~~ | ~~`ServeService`~~ | ~~⚠️ SKIPPED — depth test fails, pure surface orchestration (#69)~~ |
| **Remaining** | ~~`template.rs`~~ | ~~SqliteRegistry + McpRuntime pass-throughs~~ | ~~`TemplateService`~~ | ~~⚠️ SKIPPED — depth test fails, shallow adapter (#70)~~ |
| **API** | ~~`routes/episodic.rs`~~ | ~~Stringly-typed OCAP errors~~ | ~~`MemoryService`~~ | ~~⚠️ SKIPPED (depth test fails #71); typed matching fixed (#72)~~ |

---

## 3. Deep Service Module Inventory

| Module | Operations | Depth | Surfaces Served |
|--------|-----------|-------|----------------|
| `ChatService` | `chat()` | DEEP — full chat turn pipeline (6+ steps) | CLI, API |
| `AgentService` | `register`, `list`, `status`, `unregister` | DEEP — 6-step registration + loader boot + filtering | CLI (API ACP routes shallow) |
| `UserService` | `validate_passphrase`, `validate_registration`, `register`, `login`, `get_replicant`, `list_replicants`, `list_sessions`, `revoke_session` | DEEP — validation + lock + opaque errors + composite ops | CLI |
| `ComposeService` | `compose()` | DEEP — full style synthesizer pipeline (8+ steps) | CLI |
| `OnboardingService` | `derive_and_store_secrets`, `derive_secrets`, `init_registry`, `register_replicant`, `try_sign_in`, `try_list_existing_replicants`, `remove_orphaned_db`, `cleanup_failed_onboarding` | DEEP — secret derivation + keychain + DB init + ACP restoration + replicant registration + sign-in + cleanup | CLI |
| `SpecService` | `capture`, `build_spec`, `validate`, `cultivate`, `list_categories` | MEDIUM-DEEP — spec construction pipeline + curator evaluation | CLI, API |
| `ArchivalService` | `archive_to_git`, `restore_from_git`, `list_archives`, `create_snapshot` | DEEP — GitHub REST API + credential resolution + base64 + conditional SHA + registry serialization | CLI |
| `EmbedService` | `embed_corpus`, `parse_config` | DEEP — full style embedding pipeline (config → DB → purge → download/cache/chunk → batch embed → centroid) | CLI |
| `SkillService` | `discover_skills`, `read_skill_visibility`, `read_skill_namespace`, `compute_content_hash`, `compute_file_hash`, `find_public_skill`, `publish_skill` | DEEP — replicant name resolution + zone discovery + BLAKE3 hashing + SKILL.md YAML mutation + namespaced publishing | CLI |
| `VerificationService` | `verify`, `verify_json`, `load_manifests` | DEEP — manifest loading + assertion dispatch + structural audit + resource verification + absence check + report building | CLI, MCP |
| `ConsolidationService` | `verify_passphrase`, `consolidate` | DEEP — keystore + key derivation + DB + pipeline assembly | CLI, API |
| `EnsembleService` | `create_session`, `list_sessions`, `add_participant`, `improv_turn`, `improv_config`, `set_improv_threshold`, `set_improv_mode`, `list_participants` | MEDIUM-DEEP — thin delegates + deep improv_turn (session + inference + persistence) | CLI, API |
| `PodService` | `parse_pod_id`, `get_pod_status`, `list_pods`, `create_pod`, `activate_pod`, `deactivate_pod` | MEDIUM — UUID normalization + error mapping | CLI, API |
| `CuratorService` | `escalation_*`, `metacognition` | MEDIUM — escalation normalization + metacognition | CLI |
| `SovereigntyService` | `check_*`, `grant_*`, `revoke_*` | MEDIUM — consent normalization | CLI, API |
| `GoalService` | `create`, `list`, `update`, `delete` | SHALLOW — thin CRUD delegates | CLI, API |
| `InferenceService` | `resolve_port`, `list_models` | SHALLOW — thin port factory | CLI, API |

---

## 4. Key Decisions to Preserve

1–24. **All prior decisions still hold.**

25. **`commands/config.rs` is deleted.** All 9 dead functions removed. `ResolvedSecrets` moved to `onboarding.rs`.

26. **ReplState has zero duplicated ServiceContext fields.** All 5 removed.

27. **`ChatService::chat()` is the canonical chat turn implementation.** Both CLI and API delegate to it. `ChatRequest` carries port overrides for REPL-specific ports.

28. **`ChatResponse` and `TokenUsage` live in `hkask-services`.** CLI re-exports them as `type` aliases.

29. **`AgentService` encapsulates the composite registration flow.** The 6-step registration and the loader-boot + filtering pattern are service-layer operations. `AgentReceipt` lives in services.

30. **API ACP routes stay in the surface layer.** Previously evaluated as shallow. AgentService does NOT wrap ACP-only operations.

31. **`ConsolidationService::consolidate()` accepts a `db_path` parameter.** Surfaces derive this differently; the service doesn't impose a path convention.

32. **Pre-consolidation stats dropped from CLI.** The domain `ConsolidationService` (hkask-memory) exposes stats methods that the service layer wrapper doesn't surface.

33. **`UserService` owns passphrase and registration validation.** Moved from `hkask-cli/src/registration.rs` to `hkask-services/src/user.rs`.

34. **`registration.rs` is deleted.** All validation moved to `UserService`.

35. **`ServiceError::InvalidWebID` variant added.** Maps `uuid::Error` from `WebID::from_str()` in AgentService.

36. **`From<PoisonError<T>> for ServiceError` added.** Needed by UserService lock acquisition. Maps to `ServiceError::Infra(InfrastructureError::LockPoisoned)`.

37. **`ComposeService::compose()` is the canonical style synthesizer implementation.** `ComposeRequest` carries DB path + passphrase + cognition config + inference context.

38. **`CognitionConfig` lives in the service layer.** It's domain configuration for the compose operation, not surface-specific CLI arg parsing. Future API routes would use the same type.

39. **`ComposeService` accepts caller-provided `db_path` + `db_passphrase`.** Like ConsolidationService, the service doesn't impose a path convention.

40. **`ServiceError::Embedding(#[from] EmbeddingGenerationError)` added.** Maps Okapi embedding failures.

41. **`cosine_distance()` moved to `hkask_services::compose`.** Domain utility used by centroid validation.

42. **`EnsembleService::improv_turn()` accepts `&Arc<CircuitBreakerInferenceAdapter>`.** The `improv_turn` method on `EnsembleChat` requires `&Arc<C: InferenceClient>`, so the service mirrors this signature. The inference adapter is caller-provided.

43. **`ParticipantInfo` decoupled from `ChatParticipant`.** The service returns `Vec<ParticipantInfo>` (name, role, capabilities) instead of exposing `ChatParticipant` which includes `WebID` and `pod_id`.

44. **`ServiceError::Improv(String)` added.** Maps `ImprovError<C::Error>` from `improv_turn()` which is generic and can't have a `#[from]` impl.

45. **Standing sessions remain excluded from EnsembleService.** Marked as Divergent (CLI: YAML bootstrap, API: JSON body + MCP discovery + gas governance).

46. **`OnboardingService` encapsulates the full multi-step bootstrap flow.** 8 operations: derive_and_store_secrets, derive_secrets, init_registry, register_replicant, try_sign_in, try_list_existing_replicants, remove_orphaned_db, cleanup_failed_onboarding. CLI `onboarding.rs` reduced from ~639 to 377 lines.

47. **`ResolvedSecrets` and `SignInOutcome` live in `hkask-services`.** Moved from CLI `onboarding.rs`. CLI uses `hkask_services::ResolvedSecrets`. `OnboardingError` and `OnboardingOutcome` stay in CLI (surface presentation types).

48. **`RegistryHandle` is the return type for `init_registry`.** Contains `Arc<AcpRuntime>` and `AgentRegistryStore`. The CLI surface accesses `handle.acp` and `handle.store` for subsequent operations.

49. **`OnboardingService::try_sign_in()` stores resolved secrets directly in keychain.** Receives `ResolvedSecrets` (already derived from master passphrase), stores `acp_secret` and `db_passphrase` directly. Does NOT re-derive from `db_passphrase` (which would produce different secrets).

50. **`OnboardingService::cleanup_failed_onboarding()` replaces `cleanup_keychain()` + `cleanup_db()`.** Combined rollback operation that removes keychain entries + DB file + salt file. Accepts `ServiceConfig` for DB path resolution.

51. **`From<ServiceError> for OnboardingError` added in CLI.** Maps storage/registry/keystore errors to the CLI's onboarding error presentation. The `ServiceError::Keystore` variant maps to `OnboardingError::Database` (not `KeychainError`) because the service layer wraps keychain errors as string messages.

52. **`Database::open` in onboarding remains a legitimate legacy pattern.** OnboardingService accepts `ServiceConfig` (not `ServiceContext`) because onboarding runs before ServiceContext exists. The service opens DB from config path+passphrase.

53. **CnsService extraction SKIPPED — depth test fails.** `cns.rs` CLI is mostly `println!` formatting. The domain operations (CnsRuntime health/alerts/variety) are already well-encapsulated in `hkask_cns`. API routes access CnsRuntime through ServiceContext. No duplicated business logic between surfaces. Creating a CnsService would be a shallow pass-through.

54. **SpecService encapsulates the spec construction and evaluation pipelines.** 5 operations: capture (parse + build + save), build_spec (parse + build, no save), validate (load + evaluate), cultivate (alias for validate), list_categories. CLI and API capture routes now delegate to SpecService.

55. **`SpecService::capture` accepts `SqliteSpecStore` (concrete type).** `SpecStore` is a trait; `ServiceContext` stores `SqliteSpecStore`. Using the concrete type avoids generic constraints in callers.

56. **`SpecService::build_spec` is for API routes that don't persist.** The API capture route constructs a spec for JSON response without saving. `build_spec` returns a `Spec` directly (no `Result` — it doesn't touch the store).

57. **MiniJinja rendering stays in CLI surface.** The Render action uses `minijinja::Environment`, filesystem template loading, and spec-to-context mapping. This is surface-specific (CLI reads templates from `registry/templates/`, API would need a different template resolution). Not extracted to SpecService.

58. **ArchivalService resolves GitHub credentials internally.** `build_github_client()` calls `resolve_credential("HKASK_GITHUB_TOKEN")` from `hkask_mcp::server` which resolves from OS keychain. Callers don't provide a client or token. This matches the pattern where the service owns credential resolution for its domain.

59. **`McpRuntime` and `CapabilityChecker` params were dead in git_archival.** The original `git_archival.rs` functions took `_runtime: &McpRuntime` and `_checker: &CapabilityChecker` but never used them. ArchivalService drops these parameters entirely.

60. **`git_archival.rs` deleted entirely.** All 4 archival operations now live in `ArchivalService`. The CLI `git_cmd.rs` calls `ArchivalService` directly. No code in the CLI crate needs `reqwest` or `base64` directly anymore.

61. **EmbedService accepts caller-provided `db_path` + `db_passphrase`.** Like ConsolidationService and ComposeService, the service doesn't impose a path convention. `Database::open` in embed_corpus remains a legitimate legacy pattern.

62. **`CorpusConfig` and sub-types moved from CLI to services.** These are domain configuration types for the embedding pipeline, not CLI arg parsing. Future API routes would use the same types. `ValidationConfig` derives `Clone` for inclusion in `EmbedResult`.

63. **SkillService encapsulates the full skill visibility and publishing pipeline.** 7 operations covering discovery, front matter parsing, BLAKE3 hashing, zone-aware publishing. CLI `skill.rs` reduced from ~453 to ~170 lines. `SkillInfo` and `SkillPublishResult` types introduced for structured returns.

64. **VerificationService encapsulates the full Magna Carta verification pipeline.** 3 operations (verify, verify_json, load_manifests). `Manifest`, `Assertion`, `AssertionResult`, `PrincipleResult`, `VerificationReport` types moved from CLI to services. CLI `magna_carta.rs` reduced from ~556 to ~102 lines. The `verify_json` operation serves both CLI and MCP tool.

65. **KeystoreService extraction SKIPPED — depth test fails.** `keystore.rs` is thin pass-through over `Keychain` API. The `.env` parsing logic is CLI presentation (per-key feedback like "skipped", "stored", "failed"). `hkask_keystore::Keychain` is already the deep module.

66. **McpService extraction SKIPPED — depth test fails.** `mcp.rs`/`models.rs`/`web_search.rs` are surface adapters over `mcp_dispatcher.invoke()`. All three share `build_service_context()` + `issue_capability()` + `shutdown_all()` lifecycle patterns, but this is MCP dispatch orchestration, not business logic. No duplication between surfaces — each just formats JSON results differently.

67. **GitCasService extraction SKIPPED — depth test fails.** `git_cmd.rs` CAS operations (CasVerify, CasDiff, CasLog, CasSnapshot, CasRestore) call `GitCASPort` methods directly and format results with `println!`. `GitCASPort` implementations in `hkask-mcp` are already the deep modules. `resolve_git_cas_port()` is environment-specific; `parse_repo_id()` is CLI arg parsing. A GitCasService would be a shallow pass-through over the port trait.

68. **LoopSystemService extraction SKIPPED — depth test fails.** `loops.rs` (43 lines) is pure CLI orchestration: resolve config → build ServiceContext → print loop IDs → start system → wait for Ctrl+C → shutdown. All domain logic (loop system construction, health checks, variety computation) is in `ServiceContext::build()` and `hkask_cns`. No business logic to extract.

69. **ServeService extraction SKIPPED — depth test fails.** `serve.rs` (109 lines) is pure server startup orchestration: resolve config → build ServiceContext → start MCP servers → build ApiState → create router → bind + serve. The `API_SERVERS` const and `start_api_servers()` are infrastructure configuration, not business logic. No domain operations to extract.

70. **TemplateService extraction SKIPPED — depth test fails.** `template.rs` (188 lines) contains 9 functions, all thin pass-throughs over `SqliteRegistry` (list, register, get, search_by_lexicon) and `McpRuntime` (list_servers, discover_tools, get_tool, register_server). `hkask_templates::SqliteRegistry` is already the deep module. A TemplateService would be a shallow adapter.

71. **MemoryService extraction SKIPPED — depth test fails.** `routes/episodic.rs` OCAP error classification is HTTP-specific (mapping `MemoryError::CapabilityDenied` → 403, `MemoryError::Infra` → 500). The MCP episodic server handles OCAP errors differently. The `serde_json::Value` → `EpisodeResponse` mapping is API-specific serialization. No shared business logic between surfaces — creating a MemoryService would be a shallow adapter.

72. **Stringly-typed OCAP errors in `routes/episodic.rs` fixed.** Replaced `.to_string().contains("denied")` / `.contains("read-only")` with typed `match &e { MemoryError::CapabilityDenied { .. } => 403, _ => 500 }`. The `MemoryError::CapabilityDenied` variant already existed in `hkask-agents` — the string matching was redundant and fragile.

73. **`RecalledEpisode` typed DTO replaces `Vec<serde_json::Value>` from `EpisodicStoragePort::recall_episodic`.** The port trait now returns `Result<Vec<RecalledEpisode>, MemoryError>`. `RecalledEpisode` uses domain types (`Confidence`, `Visibility`, `Option<WebID>`) instead of raw strings. This eliminates the fragile `.get("field").and_then(|v| v.as_str()).unwrap_or_default()` destructuring that would silently produce empty strings on schema changes. The type lives in `hkask-agents/src/ports/memory_storage.rs` (domain crate, not services) because it's the return type of a port trait. Depth test passes: deleting `RecalledEpisode` would force N callers to duplicate the field mapping. `recall_semantic` left unchanged (separate concern, F10 candidate).

74. **`PodManager::new_mock()` uses a deterministic test ACP secret.** Replaced `AcpRuntime::default()` (which panics without `HKASK_ACP_SECRET_KEY`) with `AcpRuntime::new(MOCK_ACP_SECRET)` where `MOCK_ACP_SECRET = b"hkask-mock-acp-secret-32-bytes!!"`. Both `AcpRuntime` and `CapabilityChecker` share the same secret so tokens signed by the runtime are verifiable by the checker. 4 previously-failing pod tests now pass. Test-only secret is a Guardrail — acceptable for test fixtures with explicit "Never use in production" annotation. `resolve_acp_secret_for_checker()` still exists for `PodManagerBuilder::build()` (production path).

75. **`RecalledSemantic` typed DTO replaces `Vec<serde_json::Value>` from `SemanticStoragePort::recall_semantic`.** The port trait now returns `Result<Vec<RecalledSemantic>, MemoryError>`. `RecalledSemantic` uses domain types (`Confidence`, `Visibility`) and omits the `perspective` field because semantic triples are perspective-free by definition (consolidated from episodic, shared/public knowledge). This eliminates the fragile `.get("value").and_then(|v| v.as_str())` destructuring in `ChatService::recall_semantic`. The type lives in `hkask-agents/src/ports/memory_storage.rs` (domain crate, not services) because it's the return type of a port trait. `triple_to_json` deleted — no remaining callers after F10. Depth test passes: deleting `RecalledSemantic` would force N callers to duplicate the field mapping.

76. **GovernedTool wired to PodManager in ServiceContext::build().** The `GovernedTool` membrane (gas budget, variety tracking, CNS spans) was created at L386-393 and wired to `McpDispatcher` at L394-398, but NOT to `PodManager`. This meant `PodContext::invoke_tool()` fell through to the raw `mcp_runtime` path, bypassing CNS governance for pod-initiated tool calls. Fixed by adding `.with_governed_tool(governed_tool.clone())` to the `PodManager::new(...)` chain. F8 is resolved — the service layer never touches `GovernedTool` directly; governance is mediated through `PodContext` and `McpDispatcher`. (#76)

77. **`AuthContext` unified across API and service layer.** The API's `AuthContext` (trapped in `hkask-api/src/middleware/auth.rs`) is now a type alias for the domain-level `AuthContext` in `hkask-types/src/capability/mod.rs`. `AuthContext` carries `token: DelegationToken` and `webid: WebID`. `ChatRequest` now accepts `auth_context: Option<AuthContext>`. When provided, `ChatService::chat()` uses `CapabilityChecker::grant_registry()` to derive operation-specific tokens from the caller's verified identity. When absent (CLI), falls back to the legacy system-level token from config secrets. The API chat route extracts `AuthContext` from middleware-verified request extensions and passes it through. F3 partially resolved — the unified type exists and is threaded through the primary service; remaining work is to thread it through all service operations and collapse the `mcp_secret`/`acp_secret` split. (#77)

78. **`generate_stream()` added to `InferencePort` with default implementation.** The trait now has a streaming method that returns `Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>>`. `InferenceStreamChunk` carries `text_delta`, `model`, `finish_reason: Option<String>`, `usage: Option<InferenceUsage>`, and `tool_calls`. The default implementation yields a single chunk from `generate()`. Implementors override this when the backend supports SSE. The blanket `Arc<dyn InferencePort>` impl delegates to the inner type. F1 foundation is laid — surfaces can now call `generate_stream()` for incremental output; the `OkapiInference` override and surface-specific streaming endpoints are the remaining work. (#78)

79. **`EnsembleService::get_chat` + `list_deliberations` replace direct `session_manager` access in API routes.** Previously `ensemble.rs` routes `get_chat` and `list_deliberations` read `state.service_context.session_manager` directly, bypassing `EnsembleService`. Now both go through `EnsembleService` with consistent `ServiceError::SessionNotFound` error handling.

80. **`SovereigntyService::grant_consent_and_fetch` combines grant + re-fetch.** The API route `sovereignty_grant_consent` previously called `grant_consent` then `get_granted_categories` separately. Combined into single service method `grant_consent_and_fetch` that atomically grants and returns updated categories.

81. **`ConsolidationService::check_rate_limit` + `db_path_for_agent` extracted from API route.** Rate limiter (AtomicU64 epoch seconds, 30s minimum interval) and per-agent DB path template (`hkask-memory-agent-{webid}.db`) moved from `crates/hkask-api/src/routes/consolidation.rs` to `ConsolidationService`. Both CLI and API now share the same rate limit and path convention. `ServiceError::RateLimited` variant added for rate-limit errors.

82. **`ChatService::prepare_chat()` extracted for streaming support.** The chat pipeline was split into `prepare_chat()` (agent lookup, prompt composition, semantic recall, capability token, inference port resolution) and the inference step. `ChatService::chat()` now delegates to `prepare_chat()` internally. This allows CLI and API surfaces to stream inference output by calling `prepare_chat()` and then `generate_stream_with_model()` directly on the inference port. Streaming is a surface concern — no `ChatService::chat_stream()` method was added. `recall_semantic()` and `store_episodic()` made public for surface consumption. `PreparedChat` struct carries prompt, model, inference port, episodic port, agent WebID, capability token, and agent name.

83. **MCP server duplication classified as parity-test candidates (option c).** Zoom-out analysis of goal, replicant, and spec MCP servers revealed that all three fall under parity tests, not domain-crate extraction. Goal: both delegate to `SqliteGoalRepository`; duplication is surface-specific validation. Replicant: P1 Prohibition against `PodService`/`InferenceService`; duplication is intentional per architecture. Spec: 8 of 11 tools are MCP-only (OCAP, Writing Excellence, test traceability); 3 partially-duplicated tools use same domain types. F4 resolved — no domain-crate extraction needed.

---

## 5. Remaining Legitimate Legacy Patterns (Do NOT Migrate)

| Pattern | Location | Why legitimate |
|---------|----------|---------------|
| `InferenceContext::from_parts()` fallback | `repl/init.rs:53,77` | REPL-specific gate port, before ServiceContext |
| `Database::open` for per-agent memory | `repl/init.rs:194`, `commands/embed_corpus.rs:106` | Per-agent DBs with user-provided passphrase — now via EmbedService |
| `Database::open` in onboarding service | `services/onboarding.rs` (via `init_registry`) | Bootstrap — must open DB before ServiceContext |
| `hkask_keystore::*` in onboarding/bootstrap/keystore/consolidation | Multiple | Bootstrap or keystore surface operations |

---

## 6. File Reference Map

| File | Role | Status |
|------|------|--------|
| `crates/hkask-services/src/archival.rs` | ArchivalService | ✅ DEEP: GitHub REST API + credential resolution + base64 + conditional SHA + registry serialization |
| `crates/hkask-services/src/embed.rs` | EmbedService | ✅ DEEP: full style embedding pipeline (config → DB → purge → download/cache/chunk → batch embed → centroid) |
| `crates/hkask-services/src/agent.rs` | AgentService | ✅ DEEP: 6-step registration + loader boot + filtering |
| `crates/hkask-services/src/user.rs` | UserService | ✅ DEEP: validation + lock + opaque errors + composite ops |
| `crates/hkask-services/src/chat.rs` | ChatService | ✅ DEEP: full chat turn pipeline |
| `crates/hkask-services/src/compose.rs` | ComposeService | ✅ DEEP: full style synthesizer pipeline (8+ steps) |
| `crates/hkask-services/src/ensemble.rs` | EnsembleService | ✅ MEDIUM-DEEP: chat/deliberation delegates + deep improv_turn |
| `crates/hkask-services/src/onboarding.rs` | OnboardingService | ✅ DEEP: secret derivation + keychain + DB init + ACP restoration + replicant registration + sign-in + cleanup |
| `crates/hkask-services/src/spec.rs` | SpecService | ✅ MEDIUM-DEEP: spec construction pipeline + curator evaluation |
| `crates/hkask-services/src/consolidation.rs` | ConsolidationService | ✅ DEEP: keystore + key derivation + DB + pipeline assembly |
| `crates/hkask-services/src/context.rs` | ServiceContext | ✅ All fields populated |
| `crates/hkask-services/src/config.rs` | ServiceConfig | ✅ |
| `crates/hkask-services/src/error.rs` | ServiceError | ✅ InvalidWebID + Embedding + Improv + Archival + Embed + Skill + Verification + From<PoisonError> + From<uuid::Error> |
| `crates/hkask-services/src/lib.rs` | Services public API | ✅ Exports 17 service modules |
| `crates/hkask-services/src/skill.rs` | SkillService | ✅ DEEP: replicant name resolution + zone discovery + BLAKE3 hashing + SKILL.md YAML mutation + namespaced publishing |
| `crates/hkask-services/src/verification.rs` | VerificationService | ✅ DEEP: manifest loading + assertion dispatch + structural audit + resource verification + absence check + report building |
| `crates/hkask-agents/src/ports/memory_storage.rs` | EpisodicStoragePort + RecalledEpisode + SemanticStoragePort + RecalledSemantic | ✅ Typed DTOs for recall_episodic (F9, #73) and recall_semantic (F10, #75) |
| `crates/hkask-agents/src/adapters/memory_loop_adapter.rs` | MemoryLoopAdapter | ✅ triple_to_recalled_episode + triple_to_recalled_semantic helpers; triple_to_json deleted (F10) |
| `crates/hkask-agents/src/pod/context.rs` | PodContext | ✅ recall_episodic returns Vec<RecalledEpisode>; recall_semantic returns Vec<RecalledSemantic> |
| `crates/hkask-agents/src/pod/manager.rs` | PodManager | ✅ new_mock uses deterministic ACP secret (F5, #74); recall_pod_events returns Vec<RecalledEpisode> |
| `crates/hkask-cli/src/onboarding.rs` | Onboarding CLI | ✅ Delegates to OnboardingService (377 lines, was 639) |
| `crates/hkask-cli/src/commands/agent.rs` | Agent CLI | ✅ Delegates to AgentService |
| `crates/hkask-cli/src/commands/user.rs` | User CLI | ✅ Delegates to UserService |
| `crates/hkask-cli/src/commands/chat.rs` | Chat CLI | ✅ Delegates to ChatService; streaming via `chat_with_agent_streaming()` + `ChatService::prepare_chat()` |
| `crates/hkask-cli/src/commands/spec.rs` | Spec CLI | ✅ Delegates to SpecService |
| `crates/hkask-cli/src/commands/compose.rs` | Compose CLI | ✅ Delegates to ComposeService (121 lines, was 378) |
| `crates/hkask-cli/src/commands/ensemble.rs` | Ensemble CLI | ✅ Delegates improv ops to EnsembleService |
| `crates/hkask-cli/src/commands/consolidation.rs` | Consolidation CLI | ✅ Delegates to ConsolidationService (71 lines, was 127) |
| `crates/hkask-cli/src/commands/embed_corpus.rs` | Embed CLI | ✅ Delegates to EmbedService (~60 lines, was 290) |
| `crates/hkask-cli/src/commands/git_cmd.rs` | Git CLI | ✅ Archive/Restore/List/Snapshot delegate to ArchivalService; CAS ops surface-only (depth-test skip #67) |
| `crates/hkask-cli/src/commands/loops.rs` | Loops CLI | ✅ Surface-only orchestration (depth-test skip #68) |
| `crates/hkask-cli/src/commands/serve.rs` | Serve CLI | ✅ Surface-only server startup (depth-test skip #69) |
| `crates/hkask-cli/src/commands/template.rs` | Template CLI | ✅ Surface-only pass-throughs over SqliteRegistry+McpRuntime (depth-test skip #70) |
| `crates/hkask-api/src/routes/episodic.rs` | Episodic API | ✅ Typed OCAP error matching; RecalledEpisode field mapping; MemoryService depth-test skip #71 |
| `crates/hkask-cli/src/git_archival.rs` | DELETED | ✅ Logic moved to ArchivalService |
| `crates/hkask-cli/src/commands/skill.rs` | Skill CLI | ✅ Delegates to SkillService (~170 lines, was 453) |
| `crates/hkask-cli/src/commands/magna_carta.rs` | Magna Carta CLI | ✅ Delegates to VerificationService (~102 lines, was 556) |
| `crates/hkask-api/src/routes/ensemble.rs` | Ensemble API | ✅ improv_turn delegates to EnsembleService |
| `crates/hkask-cli/src/registration.rs` | DELETED | ✅ Logic moved to UserService |
| `crates/hkask-api/src/routes/acp.rs` | ACP API routes | ✅ Shallow — stays in surface |
| `crates/hkask-api/src/routes/consolidation.rs` | Consolidation API | ✅ Delegates to ConsolidationService (with db_path param) |
| `crates/hkask-api/src/routes/spec.rs` | Spec API | ✅ capture delegates to SpecService::build_spec |

---

## 7. Project Complete

**The hKask service layer extraction project is complete.** All CLI files with extractable
business logic have been evaluated with the depth test. All files that pass the depth test
have been extracted to service modules. All files that fail the depth test are documented
above as surface-only.

### Final Metrics

| Metric | Value |
|--------|-------|
| CLI commands fully extracted | 17/27 |
| CLI commands surface-only (depth-test skip) | 10/27 (git_cmd CAS, loops, serve, template, models, cns, keystore, mcp, web_search, bootstrap) |
| Deep service modules | 10 (ChatService, AgentService, UserService, ComposeService, OnboardingService, ArchivalService, EmbedService, SkillService, VerificationService, ConsolidationService) |
| Medium-deep service modules | 2 (SpecService, EnsembleService) |
| Shallow service modules | 4 (PodService, CuratorService, SovereigntyService, GoalService) — pre-existing, not from this project |
| Depth-test skips | 7 (CnsService, KeystoreService, McpService, GitCasService, LoopSystemService, ServeService, TemplateService) + MemoryService |
| API routes with typed OCAP matching | 1 (episodic.rs — fixed Session 23) |
| Service-layer tests | 138 (was 70+; 4 pod tests now pass after F5 fix) |
| Port traits with typed return DTOs | 2 (EpisodicStoragePort — RecalledEpisode, SemanticStoragePort — RecalledSemantic) |
| Unified domain types | AuthContext (hkask-types), InferenceStreamChunk (hkask-types) |
| GovernedTool wiring fix | PodManager now routes through CNS governance (#76) |
| Files deleted | 2 (registration.rs, git_archival.rs) |
| Type relocations | ResolvedSecrets, SignInOutcome, CorpusConfig, Manifest/Assertion/VerificationReport, SkillInfo/SkillPublishResult |
| Completion date | 2026-06-08 (Session 23) |

### What the Service Layer Now Provides

The `hkask-services` crate is the single source of truth for all shared business logic
between CLI, API, and MCP surfaces. Key capabilities:

- **ServiceContext** — Unified dependency graph for all surfaces (replaces per-surface state assemblies)
- **ServiceConfig** — Configuration resolution from keystore/environment
- **ServiceError** — Unified domain error hierarchy with surface adapters
- **10 deep service modules** — Each encapsulating a domain operation pipeline with multiple steps
- **2 medium-deep modules** — SpecService (spec construction + evaluation), EnsembleService (improv)
- **Depth-test discipline** — Every candidate module was evaluated; 7+ candidates correctly rejected

### Open Questions

| ID | Topic | Status |
|----|-------|--------|
| F1 | Streaming responses | ✅ Resolved (Sessions 27–28, #80–#81) — CLI incremental printing via `ChatService::prepare_chat()` + `generate_stream_with_model()` |
| F2 | Session lifecycle across surfaces | Deferred — sessions are CLI-local currently |
| F3 | Unified authentication context | ✅ Resolved (Session 27, #79) — `ChatService` uses unified `capability_checker`; `mcp_secret`/`acp_secret` split documented as Guardrail |
| F4 | MCP server duplication | ✅ Resolved (Session 28) — All three servers classified as parity-test candidates; no domain-crate extraction needed |
| F5 | Test seam depth (C8) | ✅ Resolved (Session 24) — PodManager::new_mock() uses deterministic test ACP secret |
| F6 | REPL vs API state boundary | Resolved — ServiceContext bridges both |
| F7 | ServiceConfig vs environment variables | Resolved — ServiceConfig::from_env() resolves from both |
| F8 | GovernedTool membrane boundary | ✅ Resolved (Session 26, #76) — GovernedTool wired to PodManager; service layer never touches it |
| F9 | `serde_json::Value` from EpisodicStoragePort.recall | ✅ Resolved (Session 24) — RecalledEpisode typed DTO replaces untyped Values |
| F10 | `serde_json::Value` from SemanticStoragePort.recall | ✅ Resolved (Session 25) — RecalledSemantic typed DTO replaces untyped Values; triple_to_json deleted |

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*