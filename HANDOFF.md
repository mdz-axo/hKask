# HANDOFF.md — hKask Service Layer Extraction

**Sessions:** 12–19 | **Status:** Infrastructure wiring complete. ChatService + AgentService + UserService extracted (3 deep service modules). consolidation.rs CLI deduplicated. `registration.rs` deleted (logic moved to UserService). 20 of 27 CLI commands still contain inline business logic. | **Verification:** `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace` all pass.

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
- **P0 #2 — consolidation.rs CLI deduplication:** Deleted ~70 lines of inline DB+pipeline+passphrase code from `commands/consolidation.rs`. Now delegates to `ConsolidationService::verify_passphrase()` and `ConsolidationService::consolidate()`. Added `db_path` parameter to `ConsolidationService::consolidate()` so CLI and API can use their own path conventions (CLI: `hkask-memory-{agent_name}.db`, API: `hkask-memory-agent-{webid}.db`). Dropped pre-consolidation stats reporting (service doesn't expose domain-level stats methods). CLI `commands/consolidation.rs` reduced from ~127 lines to ~71 lines.
- **P0 #3 — UserService:** Created `hkask_services::UserService` with 8 operations (`validate_passphrase`, `validate_registration`, `register`, `login`, `get_replicant`, `list_replicants`, `list_sessions`, `revoke_session`). Moved passphrase and registration validation from `hkask-cli/src/registration.rs` into the service layer. CLI `commands/user.rs` now delegates all application functions to UserService. Added `From<PoisonError<T>> for ServiceError` for lock acquisition. Deleted `hkask-cli/src/registration.rs` (dead code — all validation moved to UserService). CLI `commands/user.rs` reduced from ~327 lines to ~350 lines (but the application functions are now thin delegates, not inline business logic).

---

## 2. Honest Assessment: Remaining Work

### What Sessions 12–19 Actually Accomplished

**Infrastructure wiring** — the pipes (ServiceContext/ServiceConfig) are built, connected to every surface, and all dead legacy plumbing is deleted.

**Three deep extractions** — ChatService, AgentService, and UserService prove the pattern works. Both CLI and API delegate to shared implementations instead of duplicating business logic.

**One CLI deduplication** — consolidation.rs no longer duplicates ConsolidationService.

### What Still Needs To Be Done

| Status | CLI Commands | API Routes |
|--------|-------------|------------|
| ✅ Fully extracted | 9/27 (curator, docs, goal, pod, sovereignty, chat, agent, user, consolidation) | 7/18 (pods, sovereignty, curator, goal, consolidation, chat, + acp shallow) |
| 🟡 Partially extracted | 8/27 (ensemble, git_cmd, loops, serve, spec, template, models, compose) | 4/18 (ensemble, episodic, git, cns) |
| 🔴 Unextracted | 10/27 (cns, embed_corpus, git_archival, keystore, magna_carta, mcp, skill, web_search, onboarding, bootstrap) | 1/18 (none fully unextracted) |
| ⬜ Stub/N/A | 0/27 (bundle, registry deleted) | 6/18 (templates, mcp, acp, bundles, bots, spec stubs) |

### Priority Extraction Targets (by impact)

| Priority | Target | Inline Logic | Proposed Service | Estimated Effort |
|----------|--------|-------------|-------------------|-----------------|
| **P1** | `compose.rs` | DB + SemanticMemory construction, KNN search, exemplar pipeline, centroid validation | `ComposeService` | 3-4h |
| **P1** | `ensemble.rs` (improv) | Improv client construction, session access, message persistence | Extend `EnsembleService` | 2-3h |
| **P1** | `onboarding.rs` | Secret derivation, keychain storage, DB cleanup, replicant registration, sign-in flow | `OnboardingService` | 3-4h |
| **P2** | `spec.rs` | Spec curation, MiniJinja rendering, Spec construction | `SpecService` | 2h |
| **P2** | `git_archival.rs` | GitHub REST API calls, base64 encoding, registry serialization | `ArchivalService` | 2-3h |
| **P2** | `embed_corpus.rs` | HTTP download, corpus chunking, embedding batch loop, centroid computation | `EmbedService` | 2-3h |
| **P2** | `cns.rs` | CNS runtime access, set-point config, queue depth | `CnsService` | 1-2h |
| **P3** | `skill.rs` | Filesystem discovery, hash computation, visibility mutation | `SkillService` | 2h |
| **P3** | `keystore.rs` | Keychain CRUD, .env file parsing | `KeystoreService` | 1-2h |
| **P3** | `magna_carta.rs` | Manifest loading, structural audits | `VerificationService` | 2h |
| **P3** | `mcp.rs` / `models.rs` / `web_search.rs` | MCP dispatcher invocation patterns | `McpService` | 2-3h |

**Total estimated effort: ~22-32 hours** for complete extraction of all remaining inline business logic.

---

## 3. Deep Service Module Inventory

| Module | Operations | Depth | Surfaces Served |
|--------|-----------|-------|----------------|
| `ChatService` | `chat()` | DEEP — full chat turn pipeline (6+ steps) | CLI, API |
| `AgentService` | `register`, `list`, `status`, `unregister` | DEEP — 6-step registration + loader boot + filtering | CLI (API ACP routes shallow) |
| `UserService` | `validate_passphrase`, `validate_registration`, `register`, `login`, `get_replicant`, `list_replicants`, `list_sessions`, `revoke_session` | DEEP — validation + lock + opaque errors + composite ops | CLI |
| `ConsolidationService` | `verify_passphrase`, `consolidate` | DEEP — keystore + key derivation + DB + pipeline assembly | CLI, API |
| `PodService` | `parse_pod_id`, `get_pod_status`, `list_pods`, `create_pod`, `activate_pod`, `deactivate_pod` | MEDIUM — UUID normalization + error mapping | CLI, API |
| `CuratorService` | `escalation_*`, `metacognition` | MEDIUM — escalation normalization + metacognition | CLI |
| `SovereigntyService` | `check_*`, `grant_*`, `revoke_*` | MEDIUM — consent normalization | CLI, API |
| `EnsembleService` | `create_session`, `list_sessions`, `add_participant` | SHALLOW-MEDIUM — thin delegates | CLI, API |
| `GoalService` | `create`, `list`, `update`, `delete` | SHALLOW — thin CRUD delegates | CLI, API |
| `InferenceService` | `resolve_port`, `list_models` | SHALLOW — thin port factory | CLI, API |

---

## 4. Key Decisions to Preserve

1–24. **All prior decisions still hold.**

25. **`commands/config.rs` is deleted.** All 9 dead functions removed. `ResolvedSecrets` moved to `onboarding.rs`.

26. **ReplState has zero duplicated ServiceContext fields.** All 5 removed.

27. **`ChatService::chat()` is the canonical chat turn implementation.** Both CLI and API delegate to it. `ChatRequest` carries port overrides for REPL-specific ports.

28. **`ChatResponse` and `TokenUsage` live in `hkask-services`.** CLI re-exports them as `type` aliases.

29. **`AgentService` encapsulates the composite registration flow.** The 6-step registration (WebID parse → AgentKind validate → ACP register → AgentDefinition → RegisteredAgent → store insert) and the loader-boot + filtering pattern are service-layer operations. `AgentReceipt` lives in services.

30. **API ACP routes stay in the surface layer.** Previously evaluated as shallow (pure delegation to `AcpRuntime` with HTTP mapping). AgentService does NOT wrap ACP-only operations.

31. **`ConsolidationService::consolidate()` accepts a `db_path` parameter.** CLI uses `hkask-memory-{agent_name}.db`; API uses `hkask-memory-agent-{webid}.db`. The service doesn't impose a path convention.

32. **Pre-consolidation stats dropped from CLI.** The domain `ConsolidationService` (hkask-memory) exposes stats methods that the service layer wrapper doesn't surface. The CLI previously reported pre-consolidation candidate counts; this is now omitted.

33. **`UserService` owns passphrase and registration validation.** Moved from `hkask-cli/src/registration.rs` to `hkask-services/src/user.rs`. Both CLI and future API routes share the same validation logic.

34. **`registration.rs` is deleted.** All validation moved to `UserService`; the CLI module is dead code.

35. **`ServiceError::InvalidWebID` variant added.** Maps `uuid::Error` from `WebID::from_str()` in AgentService.

36. **`From<PoisonError<T>> for ServiceError` added.** Needed by UserService lock acquisition. Maps to `ServiceError::Infra(InfrastructureError::LockPoisoned)`.

---

## 5. Remaining Legitimate Legacy Patterns (Do NOT Migrate)

| Pattern | Location | Why legitimate |
|---------|----------|---------------|
| `InferenceContext::from_parts()` fallback | `repl/init.rs:53,77` | REPL-specific gate port, before ServiceContext |
| `InferenceContext::from_parts()` for compose | `commands/compose.rs:275` | Standalone command, uses user-provided DB |
| `Database::open` for per-agent memory | `repl/init.rs:194`, `commands/compose.rs:110`, `commands/embed_corpus.rs:106` | Per-agent DBs with user-provided passphrase |
| `Database::open` in onboarding | `onboarding.rs` (5 sites) | Bootstrap — must open DB before ServiceContext |
| `hkask_keystore::*` in onboarding/bootstrap/keystore/consolidation | Multiple | Bootstrap or keystore surface operations |
| `ResolvedSecrets` struct | `onboarding.rs` | Natural home — onboarding creates and consumes it |
| `AuthService::new()` | `middleware/auth.rs:40` | Legacy path kept for tests/standalone |

---

## 6. File Reference Map

| File | Role | Status |
|------|------|--------|
| `crates/hkask-services/src/agent.rs` | AgentService | ✅ DEEP: 6-step registration + loader boot + filtering |
| `crates/hkask-services/src/user.rs` | UserService | ✅ DEEP: validation + lock + opaque errors + composite ops |
| `crates/hkask-services/src/chat.rs` | ChatService | ✅ DEEP: full chat turn pipeline |
| `crates/hkask-services/src/consolidation.rs` | ConsolidationService | ✅ DEEP: keystore + key derivation + DB + pipeline assembly |
| `crates/hkask-services/src/context.rs` | ServiceContext | ✅ All fields populated |
| `crates/hkask-services/src/config.rs` | ServiceConfig | ✅ |
| `crates/hkask-services/src/error.rs` | ServiceError | ✅ Added InvalidWebID + From<PoisonError> + From<uuid::Error> |
| `crates/hkask-services/src/lib.rs` | Services public API | ✅ Exports 10 service modules (was 8) |
| `crates/hkask-cli/src/commands/agent.rs` | Agent CLI | ✅ Delegates to AgentService (170 lines, was 250) |
| `crates/hkask-cli/src/commands/user.rs` | User CLI | ✅ Delegates to UserService |
| `crates/hkask-cli/src/commands/chat.rs` | Chat CLI | ✅ Delegates to ChatService |
| `crates/hkask-cli/src/commands/consolidation.rs` | Consolidation CLI | ✅ Delegates to ConsolidationService (71 lines, was 127) |
| `crates/hkask-cli/src/registration.rs` | DELETED | ✅ Logic moved to UserService |
| `crates/hkask-api/src/routes/acp.rs` | ACP API routes | ✅ Shallow — stays in surface |
| `crates/hkask-api/src/routes/consolidation.rs` | Consolidation API | ✅ Delegates to ConsolidationService (with db_path param) |
| All other extracted files | See prior HANDOFF versions | ✅ No regression |

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*