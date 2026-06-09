# CONTINUATION.md — hKask Auth & Streaming Follow-Up

**Sessions:** 12–29 | **Status:** Extraction ✅ COMPLETE | All follow-up tasks evaluated ✅

---

## Summary

The service layer extraction project completed in Session 23. Post-extraction follow-up
has resolved F8 (GovernedTool wiring), F3 (AuthContext), and F1 (streaming).
F4 (MCP server access) was confirmed as architecturally correct.

**Read these files first (in this order):**

1. **`HANDOFF.md`** — Full session history (Sessions 12–27), key decisions (#1–#81),
   service module inventory, file reference map, completion metrics and open questions (§7).
2. **`CONTINUATION.md`** (this file) — Current task status and follow-up priority.
3. **`CONTINUATION-PROMPT.md`** — Detailed task plans with per-task strategies,
   skills, constraint classifications, and depth-test questions.

---

## Task Status

| # | Task | Priority | Status | Effort |
|---|------|----------|--------|--------|
| 1 | F3 — Unified AuthContext completion | 🔴 High | ✅ Done (Session 27, #79) | ~1h |
| 2 | F1 — OkapiInference streaming override + API SSE | 🔴 High | ✅ Done (Sessions 27–28) | ~3h |
| 3 | Service operations audit (AuthContext threading) | 🟡 Medium | ✅ Done (Session 27) | ~30m |
| 4 | MCP server duplication resolution | 🟡 Medium | Not started | ~4–6h |
| 5 | OPEN_QUESTIONS.md update | 🟡 Medium | ✅ Done (Session 28) | ~15m |
| 6 | F1 — CLI incremental printing | 🟡 Medium | ✅ Done (Session 28, #81) | ~1.5h |
| 7 | Ensemble standing_start orchestration | 🔴 High | ✅ Evaluated — depth test fails (Divergent) | ~30m |
| 8 | Sovereignty consent enforcement extraction | 🔴 High | ✅ Evaluated — already extracted | ~15m |
| 9 | Chat PromptStrategy framing | 🟡 Medium | ✅ Evaluated — depth test fails | ~15m |
| 10 | Pre-existing hkask-cns test compile error | 🟡 Medium | ✅ Fixed (Session 28) | ~10m |
| 11 | Replicant MCP P1 Prohibition documentation | 🟡 Medium | ✅ Done (Session 29) | ~15m |

---

## Session History (Post-Extraction)

### Session 29 (Evaluation Sweep + Replicant Documentation)

- **Ensemble standing_start orchestration:** Zoom-out analysis revealed that `EnsembleService` explicitly documents standing sessions as Divergent (CLI: YAML file bootstrap, API: JSON body + MCP discovery + gas governance). No stub exists. The depth test fails — extracting `standing_start` to the service layer would require a complex parameter type that captures divergent surface inputs, adding more interface cost than behavior benefit. Decision #84: Standing sessions remain surface-specific. No code change needed.

- **Sovereignty consent enforcement extraction:** Zoom-out analysis revealed that `SovereigntyService::check_access()` already returns the `AccessCheck` struct with `classification`, `access_required`, and `has_consent` fields. The API route's 6-line enforcement block (if no consent and not PUBLIC → return 403) is surface-specific HTTP error mapping — correct architecture. The service layer already provides all business logic. Decision #85: Consent enforcement is already extracted. No code change needed.

- **Chat PromptStrategy framing:** The existing `PromptStrategy` enum in `hkask-templates` is used in API chat routes for prompt framing. `ChatService::prepare_chat()` composes prompts with ~30 lines of straightforward string assembly (agent definition + tool section + HHH suffix + semantic context + user input). A strategy pattern would add indirection without reducing complexity. Decision #86: PromptStrategy abstraction not warranted. Document as future consideration if prompt composition grows significantly.

- **Replicant MCP P1 Prohibition documentation:** Added P1 Prohibition comments to `hkask-mcp-replicant` `tools.rs` (on `ReplicantServer` struct) and `agent_loader.rs` (module doc). Documents that the apparent duplication of agent loading and ACP secret resolution is intentional — MCP servers must NOT depend on `hkask-services` (P1 User Sovereignty boundary).

- **Verification:** `cargo check --workspace` ✅. `cargo clippy --workspace -- -D warnings` ✅. `cargo test -p hkask-services -p hkask-cli -p hkask-types -p hkask-cns -p hkask-ensemble` ✅ (0 failures).

### Session 28 (CLI Streaming + CNS Fix + OPEN_QUESTIONS)

- **F1 — CLI incremental printing:** Added `ChatService::prepare_chat()` method that
  does agent lookup, prompt composition, semantic recall, and capability token creation
  without executing inference. This extraction is deep (small interface, much behavior)
  and allows the CLI surface to stream inference output directly via
  `generate_stream_with_model()`. Added `chat_with_agent_streaming()` in
  `hkask-cli/src/commands/chat.rs` that calls `prepare_chat()`, streams text deltas
  with `print!()` + stdout flush, then stores episodic. Refactored
  `ChatService::chat()` to delegate to `prepare_chat()` internally, eliminating
  duplication. Made `recall_semantic()` and `store_episodic()` public. Added
  `PreparedChat` struct with prompt, model, inference port, episodic port, agent WebID,
  capability token. Modified `run_chat()` one-shot path to use streaming. Modified
  `single_agent_turn()` REPL path to use streaming. Added `futures-util`
  dependency to `hkask-cli`. (#81)

- **CNS test fix:** Removed unnecessary `mut` from `VarietyTracker` in
  `allosteric_alert_medium_alpha_warning` test and moved `impl CircuitBreakerPort`
  before `#[cfg(test)] mod tests` in `circuit_breaker.rs`. Both clippy warnings
  resolved.

- **OPEN_QUESTIONS.md update:** Updated F1 (streaming progress → CLI incremental printing
  remaining → resolved), F3 (resolved), F4 (reclassified as MCP duplication Prohibition),
  F8 (resolved). Added `mcp_secret`/`acp_secret` Guardrail classification to F3.

- **Verification:** `cargo check --workspace` ✅. `cargo clippy --workspace -- -D warnings` ✅.
  `cargo test -p hkask-services -p hkask-cli -p hkask-types -p hkask-cns` ✅ (0 failures).

---

## Session History (Post-Extraction)

### Session 27 (Auth & Streaming Completion)

- **F3 — AuthContext completion:** Unified `ChatService::chat()` to use `ctx.capability_checker.grant_registry()` for both authenticated and anonymous paths. Previously, the legacy path minted tokens with `config.acp_secret` directly; now both paths derive tokens through the same `mcp_secret`-backed checker. Documented the `mcp_secret`/`acp_secret` split as a valid Guardrail (defense in depth). Added doc comments to `ServiceContext::capability_checker` and `ServiceConfig` secret fields. (#79)

  **Key audit finding:** Only `ChatService::chat()` in `hkask-services` creates `DelegationToken` directly. All other `DelegationToken::new` calls are in CLI surfaces, domain crates, template executor, or MCP servers. Service-layer AuthContext threading is complete — no further service operations need `AuthContext`.

  **Secret split resolution:** `acp_secret` (in-process ACP) vs `mcp_secret` (inter-process MCP auth) serve different trust boundaries. Collapsing them would weaken defense in depth. Classified as Guardrail.

- **F1 — Streaming override + API SSE endpoint:** Overrode `generate_stream()` in `OkapiInference` to send `stream: true` and parse SSE/NDJSON responses into `InferenceStreamChunk` items. Added `generate_stream_with_model()` to `InferencePort` trait with default fallback. Added `POST /api/chat/stream` SSE endpoint with `tokio::sync::mpsc` channel bridge for `'static` stream lifetime. Added `stream: Option<bool>` to `OkapiRequest`. Added SSE response types (`StreamChunk`, `StreamChoice`, `StreamDelta`). Added `tokio-stream` dependency to `hkask-api`. (#80)

- **Service audit (Task 3):** Completed as part of F3 investigation. Only `ChatService::chat()` mints `DelegationToken` in `hkask-services`. No other service operations need `AuthContext` for token derivation. API routes that extract `_auth` use it for access gating, not for business logic.

- **Verification:** `cargo check --workspace` ✅. `cargo clippy --workspace -- -D warnings` ✅. `cargo test -p hkask-services -p hkask-api -p hkask-types -p hkask-templates` ✅ (0 failures).

### Session 26 (F8 Fix + F3 AuthContext + F1 Streaming + Test Inventory)

- **F8 — GovernedTool wiring fix:** Added `.with_governed_tool(governed_tool.clone())` to `PodManager::new(...)` chain in `ServiceContext::build()`. Previously, `PodContext::invoke_tool()` fell through to the raw `mcp_runtime` path, bypassing CNS governance (gas budget, variety tracking, spans) for pod-initiated tool calls. (#76)
- **F3 — Unified AuthContext:** Moved `AuthContext` from `hkask-api` to `hkask-types/src/capability/mod.rs` as the domain type. API's `AuthContext` is now a type alias. Added `auth_context: Option<AuthContext>` to `ChatRequest`. When provided, `ChatService::chat()` uses `CapabilityChecker::grant_registry()` from the caller's identity; when absent (CLI), falls back to legacy system-level token. API chat route now extracts `AuthContext` from middleware extensions and passes it through. Remaining: thread through all service operations, collapse `mcp_secret`/`acp_secret` split. (#77)
- **F1 — Streaming foundation:** Added `generate_stream()` to `InferencePort` with default implementation yielding a single chunk from `generate()`. Defined `InferenceStreamChunk` (text_delta, model, finish_reason, usage, tool_calls). Added blanket `Arc<dyn InferencePort>` impl. Test verifies default yields exactly one chunk. Remaining: `OkapiInference` override for SSE, surface-specific streaming endpoints. (#78)
- **F4 — MCP server access resolved:** MCP servers are correctly separate — called through inference tool-calling, not through the service layer. Architecture is sound.
- **Test inventory update:** Refreshed `docs/status/test-inventory.md` with actual test counts. Fixed per-module headers, condenser 35→53, summary 192→210.
- **Verification:** `cargo check --workspace` ✅. `cargo clippy --workspace -- -D warnings` ✅. `cargo test --workspace` ✅ (0 failures, 1 new hkask-types test).

### Session 25 (F10 Typed DTOs + OPEN_QUESTIONS.md)

- **F10 — Typed DTOs for SemanticStoragePort:** Added `RecalledSemantic` struct (no `perspective` field — semantic triples are perspective-free) in `hkask-agents/src/ports/memory_storage.rs`. Changed `SemanticStoragePort::recall_semantic` return type from `Vec<serde_json::Value>` to `Vec<RecalledSemantic>`. Replaced `triple_to_json` with `triple_to_recalled_semantic` in `MemoryLoopAdapter`. Updated `PodContext::recall_semantic` return type. Simplified `ChatService::recall_semantic` — replaced `t.get("value").and_then(|v| v.as_str())` with `t.value.as_str()`. Deleted `triple_to_json` (no remaining callers). (#75)
- **OPEN_QUESTIONS.md:** Created at project root with structured F1–F10 entries (5 resolved, 5 deferred) including constraint force classifications, affected crates, and recommended resolution approaches.
- **Condenser build fix:** Already resolved — `cargo build/clippy/test -p hkask-mcp-condenser` all pass.
- **Verification:** `cargo check --workspace` ✅. `cargo clippy --workspace -- -D warnings` ✅. `cargo test --workspace` ✅ (0 failures, 138 hkask-services tests, 51 condenser tests).

### Session 24 (F9 Typed DTOs + F5 Pod Test Fixture)

- **F9 — Typed DTOs for EpisodicStoragePort:** Added `RecalledEpisode` struct with domain-typed fields (`Confidence`, `Visibility`, `Option<WebID>`) in `hkask-agents/src/ports/memory_storage.rs`. Changed `EpisodicStoragePort::recall_episodic` return type from `Vec<serde_json::Value>` to `Vec<RecalledEpisode>`. Updated `MemoryLoopAdapter`, `PodContext`, `PodManager`. Simplified `routes/episodic.rs::query_episodes` — replaced fragile `.get().and_then()` destructuring with direct field mapping. Left `recall_semantic` unchanged (separate concern). (#73)
- **F5 — Pod Test ACP Secret Fixture:** Replaced `AcpRuntime::default()` (panics without env var) in `PodManager::new_mock()` with `AcpRuntime::new(MOCK_ACP_SECRET)` using a deterministic 32-byte test secret. Both AcpRuntime and CapabilityChecker now share the same secret so tokens signed by the runtime are verifiable by the checker. 4 previously-failing pod tests now pass. (#74)
- **Verification:** `cargo check --workspace` ✅, `cargo clippy -p hkask-agents -p hkask-services -p hkask-api -- -D warnings` ✅, `cargo test --workspace` ✅ (138 passed in hkask-services, 51 passed in condenser, 0 failed).

### Session 23 (Final Evaluation Sweep + OCAP Error Fix)

- **Phase 1:** Depth-tested 4 remaining CLI files — all surface-only:
  - `git_cmd.rs` CAS ops → shallow pass-through over `GitCASPort` (#67)
  - `loops.rs` → pure CLI orchestration, 43 lines (#68)
  - `serve.rs` → pure server startup, 109 lines (#69)
  - `template.rs` → thin pass-throughs over SqliteRegistry + McpRuntime (#70)
- **Phase 2:** Fixed stringly-typed `MemoryError` matching in `routes/episodic.rs` (#72).
  MemoryService extraction SKIPPED — depth test fails (#71).
- **Phase 3:** Project declared complete. All 6 completion criteria met.
- **Verification:** `cargo check --workspace && cargo clippy --workspace -- -D warnings` ✅.
  `cargo test --workspace` ✅ (134 passed, 4 pre-existing pod failures).

---

## Skills Required

1. **`refactor-service-layer`** — Required for any `hkask-services` change
2. **`coding-guidelines`** — Required (surgical changes)
3. **`constraint-forces`** — Required (decision classification)
4. **`zoom-out`** — Required before each task
5. **`diagnose`** — Available for build/test failures
6. **`magna-carta-verifier`** — If touching OCAP/sovereignty code

---

## Build Commands

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*