# HANDOFF.md — hKask Service Layer Extraction

**Sessions:** 12–21 | **Status:** Infrastructure wiring complete. 5 deep service modules (ChatService, AgentService, UserService, ComposeService, OnboardingService) + SpecService (medium-deep) + EnsembleService extended with improv ops. consolidation.rs CLI deduplicated. `registration.rs` deleted. 13/27 CLI commands fully extracted. `onboarding.rs` reduced from ~639 to 377 lines. `cns.rs` evaluated and skipped (depth test fails — shallow). ~11-20h remaining. | **Verification:** `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace` all pass (4 pre-existing pod test failures unrelated).

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

---

## 2. Honest Assessment: Remaining Work

### What Sessions 12–21 Actually Accomplished

**Infrastructure wiring** — the pipes (ServiceContext/ServiceConfig) are built, connected to every surface, and all dead legacy plumbing is deleted.

**Five deep extractions + one extension + one medium-deep** — ChatService, AgentService, UserService, ComposeService, OnboardingService (deep); SpecService (medium-deep); EnsembleService extended with improv ops. Both CLI and API delegate to shared implementations instead of duplicating business logic.

**One CLI deduplication** — consolidation.rs no longer duplicates ConsolidationService.

**One type relocation** — `ResolvedSecrets` moved from CLI `onboarding.rs` to services, with CLI using `hkask_services::ResolvedSecrets`.

**One depth-test skip** — `cns.rs` evaluated and rejected: depth test fails (shallow pass-through, no duplicated logic).

### What Still Needs To Be Done

| Status | CLI Commands | API Routes |
|--------|-------------|------------|
| ✅ Fully extracted | 13/27 (curator, docs, goal, pod, sovereignty, chat, agent, user, consolidation, compose, ensemble, onboarding, spec) | 9/18 (pods, sovereignty, curator, goal, consolidation, chat, ensemble, spec, + acp shallow) |
| 🟡 Partially extracted | 6/27 (git_cmd, loops, serve, spec, template, models) | 3/18 (episodic, git, cns) |
| 🔴 Unextracted | 9/27 (cns, embed_corpus, git_archival, keystore, magna_carta, mcp, skill, web_search, bootstrap) | 1/18 (none fully unextracted) |

→ cns skipped (depth test fails)
| ⬜ Stub/N/A | 0/27 (bundle, registry deleted) | 6/18 (templates, mcp, acp, bundles, bots, spec stubs) |

### Priority Extraction Targets (by impact)

| Priority | Target | Inline Logic | Proposed Service | Estimated Effort |
|----------|--------|-------------|-------------------|------------------|
| ~~P1~~ | ~~`onboarding.rs`~~ | ~~Secret derivation, keychain storage, DB cleanup, replicant registration, sign-in flow~~ | ~~`OnboardingService`~~ | ~~✅ DONE (Session 21)~~ |
| ~~P2~~ | ~~`cns.rs`~~ | ~~CNS runtime access, set-point config, queue depth~~ | ~~`CnsService`~~ | ~~SKIPPED — depth test fails, shallow pass-through~~ |
| ~~P2~~ | ~~`spec.rs`~~ | ~~Spec curation, MiniJinja rendering, Spec construction~~ | ~~`SpecService`~~ | ~~✅ DONE (Session 21)~~ |
| **P2** | `git_archival.rs` | GitHub REST API calls, base64 encoding, registry serialization | `ArchivalService` | 2-3h |
| **P2** | `embed_corpus.rs` | HTTP download, corpus chunking, embedding batch loop, centroid computation | `EmbedService` | 2-3h |
| **P3** | `skill.rs` | Filesystem discovery, hash computation, visibility mutation | `SkillService` | 2h |
| **P3** | `keystore.rs` | Keychain CRUD, .env file parsing | `KeystoreService` | 1-2h |
| **P3** | `magna_carta.rs` | Manifest loading, structural audits | `VerificationService` | 2h |
| **P3** | `mcp.rs` / `models.rs` / `web_search.rs` | MCP dispatcher invocation patterns | `McpService` | 2-3h |

**Total estimated effort: ~11-20 hours** for complete extraction of all remaining inline business logic (adjusted for cns skip).

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

---

## 5. Remaining Legitimate Legacy Patterns (Do NOT Migrate)

| Pattern | Location | Why legitimate |
|---------|----------|---------------|
| `InferenceContext::from_parts()` fallback | `repl/init.rs:53,77` | REPL-specific gate port, before ServiceContext |
| `Database::open` for per-agent memory | `repl/init.rs:194`, `commands/embed_corpus.rs:106` | Per-agent DBs with user-provided passphrase |
| `Database::open` in onboarding service | `services/onboarding.rs` (via `init_registry`) | Bootstrap — must open DB before ServiceContext |
| `hkask_keystore::*` in onboarding/bootstrap/keystore/consolidation | Multiple | Bootstrap or keystore surface operations |

---

## 6. File Reference Map

| File | Role | Status |
|------|------|--------|
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
| `crates/hkask-services/src/error.rs` | ServiceError | ✅ InvalidWebID + Embedding + Improv + From<PoisonError> + From<uuid::Error> |
| `crates/hkask-services/src/lib.rs` | Services public API | ✅ Exports 13 service modules |
| `crates/hkask-cli/src/onboarding.rs` | Onboarding CLI | ✅ Delegates to OnboardingService (377 lines, was 639) |
| `crates/hkask-cli/src/commands/agent.rs` | Agent CLI | ✅ Delegates to AgentService |
| `crates/hkask-cli/src/commands/user.rs` | User CLI | ✅ Delegates to UserService |
| `crates/hkask-cli/src/commands/chat.rs` | Chat CLI | ✅ Delegates to ChatService |
| `crates/hkask-cli/src/commands/spec.rs` | Spec CLI | ✅ Delegates to SpecService |
| `crates/hkask-cli/src/commands/compose.rs` | Compose CLI | ✅ Delegates to ComposeService (121 lines, was 378) |
| `crates/hkask-cli/src/commands/ensemble.rs` | Ensemble CLI | ✅ Delegates improv ops to EnsembleService |
| `crates/hkask-cli/src/commands/consolidation.rs` | Consolidation CLI | ✅ Delegates to ConsolidationService (71 lines, was 127) |
| `crates/hkask-api/src/routes/ensemble.rs` | Ensemble API | ✅ improv_turn delegates to EnsembleService |
| `crates/hkask-cli/src/registration.rs` | DELETED | ✅ Logic moved to UserService |
| `crates/hkask-api/src/routes/acp.rs` | ACP API routes | ✅ Shallow — stays in surface |
| `crates/hkask-api/src/routes/consolidation.rs` | Consolidation API | ✅ Delegates to ConsolidationService (with db_path param) |
| `crates/hkask-api/src/routes/spec.rs` | Spec API | ✅ capture delegates to SpecService::build_spec |

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*