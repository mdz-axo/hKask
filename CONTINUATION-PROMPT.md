# CONTINUATION-PROMPT.md — hKask Auth & Streaming Completion

**Sessions:** 12–26 | **Status:** Extraction ✅ COMPLETE | F8 ✅ DONE | F3 🟡 Partial | F1 🟡 Foundation laid

---

## Skills to Load (Required Before Starting)

Load these skills **before** any continuation work:

1. **`refactor-service-layer`** — **Required.** Governing methodology for all service-layer changes. The depth test (P2), dependency direction (P3), and anti-patterns sections apply to every change that touches `hkask-services` or service operation signatures.

2. **`coding-guidelines`** — **Required.** Surgical changes only. Every changed line traces to the task. No "while we're here" refactors.

3. **`constraint-forces`** — **Required.** Classify every design decision by force type. Particularly critical for auth-related changes — OCAP gates are Prohibitions, capability derivation paths are Guardrails.

4. **`zoom-out`** — **Required before starting any task.** Produce the module map and caller graph for the target area before making changes.

5. **`diagnose`** — Available if anything breaks the build or tests.

---

## Read These Files First (In This Order)

1. **`HANDOFF.md`** — Full session history (Sessions 12–26), key decisions (#1–#81), deep service module inventory (§3), file reference map (§6), completion metrics and open questions (§7).

2. **`CONTINUATION.md`** — Session 26 summary with all task statuses.

3. **`.agents/skills/refactor-service-layer/SKILL.md`** — The governing methodology. Re-read the depth test (P2), dependency direction (P3), and anti-patterns before starting.

---

## Session 26 Accomplishments

| Fix | What Was Done | Decision # | Status |
|-----|--------------|-----------|--------|
| **F8** — GovernedTool wiring | Added `.with_governed_tool(governed_tool.clone())` to PodManager in `ServiceContext::build()` | #76 | ✅ Complete |
| **F3** — AuthContext type | Moved `AuthContext` from `hkask-api` to `hkask-types`; added `auth_context` to `ChatRequest`; `ChatService::chat()` uses `CapabilityChecker::grant_registry()` when provided; API chat route passes middleware-verified identity | #77 | 🟡 ChatService only; all other services still use legacy ad-hoc tokens |
| **F1** — Streaming foundation | Added `generate_stream()` to `InferencePort` with default impl; defined `InferenceStreamChunk`; blanket `Arc<dyn InferencePort>` impl; test for default behavior | #78 | 🟡 Trait only; `OkapiInference` override + surface endpoints remain |
| **F4** — MCP server access | Resolved — MCP servers are correctly separate, called through inference tool-calling | — | ✅ Complete |
| Test inventory | Refreshed `docs/status/test-inventory.md`; fixed per-module headers; condenser 35→53; summary 192→210 | — | ✅ Complete |

**Verification baseline:**
```
cargo check --workspace                                    ✅
cargo clippy --workspace -- -D warnings                    ✅
cargo test --workspace                                      ✅ (0 failures, 1 new hkask-types test)
```

---

## Remaining Work (Priority-Ordered)

### Task 1: F3 Completion — Thread AuthContext Through All Service Operations (High Priority)

**Problem:** Only `ChatService::chat()` currently uses `auth_context` to derive caller-specific capability tokens. All other service operations that create `DelegationToken` still mint ad-hoc system-level tokens from `config.acp_secret`. The API authenticates the caller via middleware, then discards the identity for every operation except chat.

**Goal:** Thread `AuthContext` through every service operation that creates `DelegationToken`, so that the API's verified caller identity is used to derive operation-specific tokens.

**Scope:**

The audit found **6 distinct paths** that create `DelegationToken`. After Session 26, the chat path is fixed. The remaining paths to update:

| Path | File | What It Does | Needs AuthContext? |
|------|------|-------------|-------------------|
| `PodContext::invoke_tool` via REPL | `cli/src/repl/handlers/invoke.rs:78-84` | Mints token for tool invocation | Yes — caller's token should be attenuated, not system-minted |
| `tool_augmented::invoke_tool_call` | `cli/src/repl/tool_augmented.rs:189-196` | Mints token for GovernedTool call | Yes — same as invoke |
| `ManifestExecutor::execute_tool_invoke` | `templates/src/executor.rs:209-213` | Mints token for template execution | Partially — template executor is system-scoped, not caller-scoped |
| `AgentPod::new` | `agents/src/pod/mod.rs:199-203` | Mints token for pod capability spec | Different — pod creation token, not caller identity |

Additionally, `ServiceContext::build()` creates **3 `CapabilityChecker` instances** with **2 different secrets**:
- `config.mcp_secret` → `ctx.capability_checker` (used by `ChatService` for `AuthContext`-derived tokens)
- `config.acp_secret` → `FullMcpAdapter`'s checker + `PodManager`'s checker

The `mcp_secret` vs `acp_secret` split needs investigation: why are there two secrets, and can they be unified?

**Strategy:**

1. Zoom out on all `DelegationToken::new()` call sites across the codebase.
2. For each, classify: should this token reflect the caller's identity (needs `AuthContext`) or is it system-scoped (stays with config secret)?
3. Add `auth_context: Option<AuthContext>` to service request types that need it.
4. For the `mcp_secret`/`acp_secret` split: investigate whether both are necessary or if one can be eliminated. If both are needed, document why.
5. Verify: every API route that has middleware-verified `AuthContext` passes it through to the service operation.

**Depth test:** `AuthContext` in `hkask-types` passes — deleting it forces N callers (ChatService, API routes, future service ops) to duplicate the type. The `Option<AuthContext>` pattern in service requests is the right level — callers that don't have it (CLI) pass `None` and get legacy behavior.

**Constraint classification:**
- OCAP capability derivation from caller identity → Guardrail (P2 Affirmative Consent)
- `mcp_secret` vs `acp_secret` separation → Evidence (both exist and work) or Hypothesis (needs verification that they serve different purposes)
- API discarding verified identity → Evidence (measured: middleware inserts `AuthContext` that routes never extract)

**Estimated effort:** ~3–4 hours.

---

### Task 2: F1 Completion — OkapiInference Streaming Override + Surface Endpoints (High Priority)

**Problem:** `InferencePort::generate_stream()` has a default implementation that yields a single chunk. For real streaming (incremental text display in CLI chat, SSE in API), `OkapiInference` needs to override `generate_stream()` with an SSE-based implementation, and surfaces need streaming endpoints.

**Goal:** Implement `OkapiInference::generate_stream()` that streams tokens from the Okapi inference server, and add streaming endpoints to the API and CLI.

**Scope:**

- **`hkask-templates/src/inference_port.rs`** — Override `generate_stream()` in `OkapiInference` to use Okapi's streaming API (if supported). Check Okapi's `/v1/chat/completions` endpoint for `stream: true` support.
- **`hkask-api/src/routes/chat.rs`** — Add `POST /api/chat/stream` SSE endpoint that calls `generate_stream()` and yields `InferenceStreamChunk` items as SSE events.
- **`hkask-cli/src/commands/chat.rs`** — For `kask chat`, print `text_delta` chunks incrementally as they arrive instead of waiting for the complete result.
- **`hkask-services/src/chat.rs`** — Consider adding `ChatService::chat_stream()` that uses `generate_stream()` for the inference step while keeping other steps (memory recall, episodic storage) atomic.

**Strategy:**

1. Zoom out on Okapi's inference API to determine if streaming is supported. Check Okapi server's API spec or `OkapiInference::execute_request` for `stream` parameter.
2. If streaming is supported: override `generate_stream()` in `OkapiInference` to use `reqwest`'s streaming response handling, parsing SSE events into `InferenceStreamChunk` items.
3. If streaming is not supported yet: document this as a dependency on Okapi, and surface endpoints can still use the default (single-chunk) implementation with the streaming interface in place for future use.
4. Add SSE endpoint to the API for chat streaming.
5. Update CLI `kask chat` to print incrementally.
6. Verify: `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`.

**Depth test:** Streaming is a surface concern — the service layer orchestrates atomic steps (prompt → memory → inference → store). Only the inference step streams. `ChatService::chat_stream()` would be a separate method that uses `generate_stream()` for the inference step while keeping the rest of the pipeline atomic. This is shallow enough that it might belong in the surface layer rather than the service. Consider carefully before adding.

**Constraint classification:**
- Streaming as a surface concern → Guideline (P3 Generative Space — user choice)
- SSE endpoint in API → Guideline (surface-specific delivery)
- CLI incremental printing → Guideline (surface-specific delivery)

**Estimated effort:** ~4–6 hours (depends on Okapi SSE support).

---

### Task 3: Service Operations Audit — Complete AuthContext Threading (Medium Priority)

**Problem:** Even after Task 1 threads `AuthContext` through the primary operations, secondary service operations that touch ACP or capability-gated resources may need the caller's identity. A full audit is needed.

**Goal:** Audit every service operation for places where the caller's identity affects the operation's behavior (capability token derivation, ACP checks, sovereignty checks) and thread `AuthContext` where appropriate.

**Scope:**

Walk all `*Service` methods in `hkask-services/src/*.rs` and check:
- Does this method create a `DelegationToken`? → Needs `AuthContext` if the token should reflect caller identity
- Does this method access `config.acp_secret` or `config.mcp_secret` directly? → Likely needs `AuthContext` instead
- Does this method call `PodContext` or `CapabilityChecker`? → May need caller identity

**Strategy:**

1. Grep for `DelegationToken::new` and `acp_secret`/`mcp_secret` across `hkask-services/src/`.
2. For each occurrence, classify: system-scoped or caller-scoped?
3. Add `auth_context: Option<AuthContext>` to request types that need it.
4. Update API routes to extract and pass `AuthContext`.
5. Verify both surfaces.

**Estimated effort:** ~2–3 hours.

---

### Task 4: MCP Server Duplication Resolution (Medium Priority)

**Problem:** The MCP server audit (Session 26) found 3 servers with concrete duplication:

| Server | Duplicates | Impact |
|--------|-----------|--------|
| **goal** | `GoalService` | All 3 tools duplicate parse-and-delegate patterns |
| **replicant** | `OnboardingService`, `PodService`, `InferenceService` | Agent loading, pod lifecycle, inference port construction |
| **spec** | `SpecService` | `spec_goal_capture` duplicates capture pipeline |

MCP servers cannot depend on `hkask-services` (out-of-process). But they reimplement multi-step service operations with raw domain primitives.

**Goal:** Resolve the duplication. Options:
- **(a)** MCP servers call service operations through the API (HTTP localhost)
- **(b)** Extract shared operation logic into domain crates that both services and MCP servers can depend on
- **(c)** Add parity integration tests and accept the duplication for thin wrappers

**Strategy:**

1. For each duplicated server, assess the divergence between MCP server logic and service logic. Is the MCP server doing something different, or is it truly the same operation with different framing?
2. If truly the same: option (b) — move the shared logic into the domain crate (e.g., `GoalService`'s parse-and-delegate into `hkask-storage` or `hkask-agents`).
3. If the MCP server has additional concerns (OCAP verification, different error handling): option (c) — add parity tests.
4. Update `OPEN_QUESTIONS.md` with the resolution decision per server.

**Constraint classification:**
- MCP servers must NOT depend on `hkask-services` → Prohibition (anti-pattern from refactor-service-layer skill)
- Shared logic in domain crates → Guideline (depth test before extraction)

**Estimated effort:** ~4–6 hours.

---

### Task 5: OPEN_QUESTIONS.md Update (Low Priority)

**Problem:** `OPEN_QUESTIONS.md` was created in Session 25 with 5 resolved and 5 deferred items. Since Session 26, F1, F3, F4, and F8 have progressed. The file needs updating.

**Goal:** Update `OPEN_QUESTIONS.md` to reflect Session 26 progress: F1 foundation laid, F3 partially resolved, F4 resolved, F8 resolved.

**Estimated effort:** ~15 minutes.

---

## Per-Task Discipline

For every task that touches code:

```
[ ] Zoom out on the target area (module map, caller graph, data flow)
[ ] Apply depth test — is the proposed change deep or shallow?
[ ] Classify constraints with constraint-forces skill
[ ] State assumptions explicitly before implementing
[ ] Implement with surgical changes — every line traces to the task
[ ] Verify: cargo check --workspace
[ ] Run clippy: cargo clippy --workspace -- -D warnings
[ ] Run tests: cargo test --workspace
[ ] Update HANDOFF.md (add key decision, update file map if needed)
[ ] Update CONTINUATION.md (mark task done or document new findings)
```

---

## Key Constraints (Still Apply)

- **P3 (Dependency direction):** CLI → services → domain. No circular deps.
- **P5 (One domain per change):** Each change touches exactly one concern.
- **Headless:** No visual UI, no dashboards, no monitoring stacks.
- **P8 (Test quality):** Every `#[test]` verifies a stated behavioral property.
- **Depth test (P2):** If deleting the proposed module/type makes complexity vanish, don't create it.
- **Surgical changes:** No style fixes, no renaming, no comment additions.
- **MCP servers must NOT depend on `hkask-services`.** (Prohibition from refactor-service-layer skill.)

---

## Build Commands

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*