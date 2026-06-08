# HANDOFF.md — hKask Service Layer Extraction

**Sessions:** 12–18 | **Status:** Infrastructure wiring complete. ChatService extracted (first deep service module). 22 of 27 CLI commands and 5 of 18 API routes still contain inline business logic. | **Verification:** `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace` all pass.

---

## 1. Session History

### Sessions 12–17
Infrastructure wiring: ServiceContext/ServiceConfig created, all surfaces wired to ServiceContext, dead code deleted, ReplState deduplicated. See prior HANDOFF versions for details.

### Session 18 (Dead Code + ReplState Dedup + ChatService Extraction)
- **Step 10:** Deleted `commands/config.rs` entirely (9 dead functions). Moved `ResolvedSecrets` into `onboarding.rs`.
- **Step 11:** Removed all 5 duplicated ReplState fields. All consumers now read from `state.service_context.<field>`.
- **Step 12:** Full workspace verification passed.
- **ChatService extraction:** Created `hkask_services::ChatService` — the first DEEP service module. Encapsulates the full chat turn pipeline (agent lookup → prompt composition → semantic recall → inference → episodic storage) that was previously ~380 lines of inline business logic in `commands/chat.rs`. Wired both CLI `chat_with_agent()` and API `routes/chat.rs` through `ChatService::chat()`. CLI `chat.rs` reduced from ~470 lines to ~140 lines.

---

## 2. Honest Assessment: Remaining Work

### What Sessions 12–18 Actually Accomplished

**Infrastructure wiring** — the pipes (ServiceContext/ServiceConfig) are built, connected to every surface, and all dead legacy plumbing is deleted. This was necessary foundation work.

**One deep extraction** — ChatService proves the pattern works. Both CLI and API now delegate to a single implementation instead of duplicating ~400 lines of business logic.

### What Still Needs To Be Done

| Status | CLI Commands | API Routes |
|--------|-------------|------------|
| ✅ Fully extracted | 6/27 (curator, docs, goal, pod, sovereignty, chat) | 6/18 (pods, sovereignty, curator, goal, consolidation, chat) |
| 🟡 Partially extracted | 9/27 (consolidation, ensemble, git_cmd, loops, serve, spec, template, user, models) | 4/18 (ensemble, episodic, git, cns) |
| 🔴 Unextracted | 12/27 (agent, cns, compose, embed_corpus, git_archival, keystore, magna_carta, mcp, skill, web_search, onboarding, bootstrap) | 1/18 (none fully unextracted after chat) |
| ⬜ Stub/N/A | 2/27 (bundle, registry) | 7/18 (templates, mcp, acp, bundles, bots, spec stubs) |

### Priority Extraction Targets (by impact)

| Priority | Target | Inline Logic | Proposed Service | Estimated Effort |
|----------|--------|-------------|-------------------|-----------------|
| **P0** | `onboarding.rs` | Secret derivation, keychain storage, DB cleanup, replicant registration, sign-in flow | `OnboardingService` | 3-4h |
| **P0** | `agent.rs` | Agent registration (ACP + definition + store insert), agent listing via loader | `AgentService` | 2-3h |
| **P1** | `consolidation.rs` CLI | **Duplicates** `ConsolidationService` — delete inline code, delegate | Use existing `ConsolidationService` | 1h |
| **P1** | `compose.rs` | DB + SemanticMemory construction, KNN search, exemplar pipeline, centroid validation | `ComposeService` | 3-4h |
| **P1** | `user.rs` | Registration + login + session operations on `Arc<Mutex<UserStore>>` | `UserService` | 2-3h |
| **P1** | `ensemble.rs` (improv) | Improv client construction, session access, message persistence | Extend `EnsembleService` | 2-3h |
| **P2** | `spec.rs` | Spec curation, MiniJinja rendering, Spec construction | `SpecService` | 2h |
| **P2** | `git_archival.rs` | GitHub REST API calls, base64 encoding, registry serialization | `ArchivalService` | 2-3h |
| **P2** | `embed_corpus.rs` | HTTP download, corpus chunking, embedding batch loop, centroid computation | `EmbedService` | 2-3h |
| **P2** | `cns.rs` | CNS runtime access, set-point config, queue depth | `CnsService` | 1-2h |
| **P3** | `skill.rs` | Filesystem discovery, hash computation, visibility mutation | `SkillService` | 2h |
| **P3** | `keystore.rs` | Keychain CRUD, .env file parsing | `KeystoreService` | 1-2h |
| **P3** | `magna_carta.rs` | Manifest loading, structural audits | `VerificationService` | 2h |
| **P3** | `mcp.rs` / `models.rs` / `web_search.rs` | MCP dispatcher invocation patterns | `McpService` | 2-3h |

**Total estimated effort: 28-40 hours** for complete extraction of all inline business logic into service modules, with both surfaces delegating and legacy inline code deleted.

---

## 3. ChatService — Design Decisions

| Decision | Rationale |
|----------|-----------|
| `ChatRequest` carries port overrides | REPL passes pre-resolved `inference_port`, `episodic_storage`, `semantic_storage` as overrides. ServiceContext's ports are the defaults. |
| `ChatResponse` / `TokenUsage` in `hkask-services` | Both CLI and API need the same types. CLI re-exports them as `type` aliases. |
| `agent_webid` derived internally | ChatService always derives WebID from agent name for consistency. CLI's `agent_webid` parameter is now a no-op (`_agent_webid`). |
| `ChatService` is a unit struct | No state — all state comes from `&ServiceContext` + `ChatRequest`. Follows existing service module pattern. |
| Semantic recall returns `Option<String>` | Simple, composable. The service logs failures at debug level and proceeds without context. |
| Episodic storage is fire-and-forget | Same as original behavior — logs at debug level, never blocks the response. |

---

## 4. Remaining Legitimate Legacy Patterns (Do NOT Migrate)

| Pattern | Location | Why legitimate |
|---------|----------|---------------|
| `InferenceContext::from_parts()` fallback | `routes/chat.rs` (removed), `repl/init.rs:53,77` | REPL-specific gate port, before ServiceContext |
| `InferenceContext::from_parts()` for compose | `commands/compose.rs:275` | Standalone command, uses user-provided DB |
| `Database::open` for per-agent memory | `repl/init.rs:194`, `commands/compose.rs:110`, `commands/embed_corpus.rs:106` | Per-agent DBs with user-provided passphrase |
| `Database::open` in onboarding | `onboarding.rs` (5 sites) | Bootstrap — must open DB before ServiceContext |
| `Database::open` in consolidation | `commands/consolidation.rs:41` | Derives secrets from user-provided passphrase |
| `hkask_keystore::*` in onboarding/bootstrap/keystore/consolidation | Multiple | Bootstrap or keystore surface operations |
| `ResolvedSecrets` struct | `onboarding.rs` | Natural home — onboarding creates and consumes it |
| `AuthService::new()` | `middleware/auth.rs:40` | Legacy path kept for tests/standalone |

---

## 5. Key Decisions to Preserve

1–24. **All prior decisions still hold.**

25. **`commands/config.rs` is deleted.** All 9 dead functions removed. `ResolvedSecrets` moved to `onboarding.rs`.

26. **ReplState has zero duplicated ServiceContext fields.** All 5 removed.

27. **`ChatService::chat()` is the canonical chat turn implementation.** Both CLI and API delegate to it. `ChatRequest` carries port overrides for REPL-specific ports.

28. **`ChatResponse` and `TokenUsage` live in `hkask-services`.** CLI re-exports them as `type` aliases.

---

## 6. File Reference Map

| File | Role | Status |
|------|------|--------|
| `crates/hkask-services/src/chat.rs` | ChatService | ✅ DEEP: full chat turn pipeline |
| `crates/hkask-services/src/context.rs` | ServiceContext | ✅ All fields populated |
| `crates/hkask-services/src/config.rs` | ServiceConfig | ✅ |
| `crates/hkask-services/src/lib.rs` | Services public API | ✅ Exports 8 service modules (was 7) |
| `crates/hkask-cli/src/commands/chat.rs` | Chat CLI | ✅ Delegates to ChatService (140 lines, was 470) |
| `crates/hkask-api/src/routes/chat.rs` | Chat API | ✅ Delegates to ChatService |
| `crates/hkask-cli/src/repl/mod.rs` | ReplState | ✅ No duplicated fields |
| All other extracted files | See prior HANDOFF versions | ✅ No regression |

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*