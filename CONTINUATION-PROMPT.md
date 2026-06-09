# CONTINUATION-PROMPT.md — hKask Streaming Completion & MCP Duplication

**Sessions:** 12–27 | **Status:** F3 ✅ RESOLVED | F1 🟡 Override + API SSE done; CLI printing remaining | F4 ✅ Resolved

---

## Skills to Load (Required Before Starting)

Load these skills **before** any continuation work:

1. **`refactor-service-layer`** — **Required.** Governing methodology for all service-layer changes. The depth test (P2), dependency direction (P3), and anti-patterns sections apply to every change that touches `hkask-services` or service operation signatures. Critical for MCP server duplication resolution (anti-pattern #8: MCP servers must NOT depend on `hkask-services`).

2. **`coding-guidelines`** — **Required.** Surgical changes only. Every changed line traces to the task. No "while we're here" refactors.

3. **`constraint-forces`** — **Required.** Classify every design decision by force type. Particularly critical for MCP duplication resolution — the Prohibition on MCP→services dependency, the Guideline on shared logic in domain crates, and any new Guardrails introduced.

4. **`zoom-out`** — **Required before starting any task.** Produce the module map and caller graph for the target area before making changes.

5. **`diagnose`** — Available if anything breaks the build or tests.

6. **`skill-translator`** — Available if extracting shared logic from MCP servers into registry templates or domain crates during Task 4.

---

## Read These Files First (In This Order)

1. **`HANDOFF.md`** — Full session history (Sessions 12–27), key decisions (#1–#80), deep service module inventory (§3), file reference map (§6), completion metrics and open questions (§7).

2. **`CONTINUATION.md`** — Session 27 summary with all task statuses.

3. **`.agents/skills/refactor-service-layer/SKILL.md`** — The governing methodology. Re-read the depth test (P2), dependency direction (P3), and anti-patterns before starting — especially anti-pattern #8 (MCP servers depending on `hkask-services`).

---

## Session 27 Accomplishments

| Fix | What Was Done | Decision # | Status |
|-----|--------------|-----------|--------|
| **F3** — AuthContext completion | Unified `ChatService::chat()` to use `ctx.capability_checker.grant_registry()` for both auth and legacy paths (system WebID as delegator when no `AuthContext`). Documented `mcp_secret`/`acp_secret` split as Guardrail. Added doc comments. | #79 | ✅ Complete |
| **F1** — Streaming override + API SSE | Overrode `generate_stream()` in `OkapiInference` (SSE/NDJSON parsing). Added `generate_stream_with_model()` to `InferencePort` trait + `Arc` impl. Added `POST /api/chat/stream` SSE endpoint with channel bridge. Added `stream: Option<bool>` to `OkapiRequest`. Added `StreamChunk`/`StreamChoice`/`StreamDelta` types. | #80 | 🟡 CLI incremental printing remaining |
| **Service audit** | Grep'd all `DelegationToken::new` and `acp_secret`/`mcp_secret` across `hkask-services`. Only `ChatService::chat()` mints tokens. Audit complete. | — | ✅ Complete |

**Key architectural decisions from Session 27:**

| # | Decision | Rationale |
|---|----------|-----------|
| #79 | `ChatService::chat()` uses `ctx.capability_checker.grant_registry()` for both auth and legacy paths, with `ctx.system_webid` as the delegator when no `AuthContext` | Both paths now derive tokens through the same `mcp_secret`-backed checker, eliminating the secret-divergence risk |
| #80 | `OkapiInference::generate_stream()` sends `stream: true`, parses SSE/NDJSON into `InferenceStreamChunk` items | Okapi supports Ollama-style streaming; SSE format is OpenAI-compatible |
| — | `mcp_secret`/`acp_secret` split is a **Guardrail** (defense in depth), not a bug | `acp_secret` signs in-process tokens (ACP, PodManager); `mcp_secret` signs inter-process tokens (API auth, MCP dispatcher). Different trust boundaries. Collapsing weakens defense in depth. |
| — | Only `ChatService::chat()` in `hkask-services` creates `DelegationToken` | All other `DelegationToken::new` calls are in CLI surfaces, domain crates, template executor, or MCP servers. No other service operations need `AuthContext`. |

**Verification baseline:**
```
cargo check --workspace                                    ✅
cargo clippy --workspace -- -D warnings                    ✅
cargo test -p hkask-services -p hkask-api -p hkask-types -p hkask-templates  ✅ (0 failures)
```

**Known issue:** `hkask-cns` has a pre-existing test compile error (`gate` variable needs `mut` in `allosteric_alert_medium_alpha_warning` test). Unrelated to current work.

---

## Remaining Work (Priority-Ordered)

### Task 1: F1 Completion — CLI Incremental Printing (High Priority)

**Problem:** `POST /api/chat/stream` now streams SSE events to API callers, but `kask chat` still waits for the complete `ChatService::chat()` result before printing anything. Users in the CLI should see text arriving incrementally as the model generates it.

**Goal:** Update `kask chat` to consume `generate_stream_with_model()` and print `text_delta` chunks as they arrive.

**Scope:**

- **`hkask-cli/src/commands/chat.rs`** — The `chat_with_agent()` function calls `ChatService::chat()` which is synchronous (returns complete result). For streaming, the CLI needs to either:
  - (a) Call the inference port's `generate_stream_with_model()` directly (bypassing ChatService's full pipeline), or
  - (b) Add `ChatService::chat_stream()` that uses `generate_stream()` for the inference step while keeping other steps (memory recall, episodic storage) atomic.

- **`hkask-cli/src/repl/turn.rs`** — The REPL's `single_agent_turn()` and `ensemble_turn()` currently call `ChatService::chat()` and print the complete result. These may need streaming equivalents.

**Strategy:**

1. Zoom out on the CLI chat flow: trace the call path from `kask chat` → `chat_with_agent()` → `ChatService::chat()` → `InferencePort::generate_with_model()`.
2. Determine the cleanest insertion point for streaming. The depth test says streaming is a surface concern — the service layer's pipeline is atomic by design. Option (a) is simpler: the CLI calls `generate_stream_with_model()` directly and prints deltas. Memory recall and episodic storage can happen before/after the streaming inference.
3. For the REPL (`turn.rs`): the `single_agent_turn` function already has the inference port available. Replace the synchronous inference call with `generate_stream_with_model()` and print chunks with `print!()` (no newline) followed by `println!()` on the final chunk.
4. Verify: `cargo check --workspace && cargo clippy --workspace -- -D warnings && cargo test --workspace`.
5. Manual smoke test: `echo "hello" | cargo run --bin kask -- chat -f - -m qwen3:8b` should print tokens incrementally.

**Depth test:** Streaming output is surface-specific — it changes how the CLI *presents* inference results, not what the service layer computes. `ChatService::chat_stream()` would be shallow (just delegates to `generate_stream()` for the inference step). Prefer option (a): call the port directly from the CLI surface.

**Constraint classification:**
- Streaming as a surface concern → Guideline (P3 Generative Space — user choice)
- CLI incremental printing → Guideline (surface-specific delivery)

**Estimated effort:** ~1–2 hours.

---

### Task 2: MCP Server Duplication Resolution (Medium Priority)

**Problem:** Three MCP servers duplicate service-layer operations:

| Server | Duplicates | Key Divergence |
|--------|-----------|---------------|
| **goal** | `GoalService` | 3 tools duplicate parse-and-delegate patterns |
| **replicant** | `OnboardingService`, `PodService`, `InferenceService` | Agent loading, pod lifecycle, inference port construction |
| **spec** | `SpecService` | `spec_goal_capture` duplicates capture pipeline |

MCP servers cannot depend on `hkask-services` (Prohibition — anti-pattern #8 from refactor-service-layer skill). They reimplement multi-step service operations with raw domain primitives.

**Goal:** Resolve the duplication using one of three approaches:
- **(a)** MCP servers call service operations through the API (HTTP localhost)
- **(b)** Extract shared operation logic into domain crates that both services and MCP servers can depend on
- **(c)** Add parity integration tests and accept the duplication for thin wrappers

**Strategy:**

1. **Zoom out** on each duplicated server. For each, map the MCP tool's operation against the corresponding service method. Document the divergence: does the MCP tool do something *different* from the service method (OCAP verification, different error handling, additional MCP framing), or is it truly the same logic with different I/O?

2. **Classify** each duplication:
   - If MCP tool ≡ service method (same logic, different framing) → candidate for option (b)
   - If MCP tool adds concerns (OCAP, ACP checks, different auth model) → candidate for option (c)
   - If MCP tool is a thin wrapper → option (c) with parity tests

3. **For option (b) — extract to domain crate:**
   - Apply the depth test: if deleting the extracted module makes complexity vanish across both consumers, don't create it.
   - Target crate: `hkask-storage` (for data access patterns), `hkask-agents` (for agent lifecycle), or `hkask-templates` (for spec/capture).
   - The extracted module must NOT depend on `hkask-services`. Verify dependency direction.
   - Move the *operation logic* (the "how"), not the *service interface* (the "what").

4. **For option (c) — parity tests:**
   - Write integration tests that call both the MCP tool and the service method with the same inputs, then assert equivalent outputs.
   - Document the expected divergence (if any) in the test.

5. **For option (a) — API bridge:**
   - Only if (b) and (c) are both impractical. MCP server calls `http://localhost:PORT/api/...` to delegate to service.
   - Requires the API server to be running. Adds operational complexity. Last resort.

6. Update `OPEN_QUESTIONS.md` with the resolution decision per server.

**Depth test for option (b):** The extracted module must be deep — small interface, much behavior. If it's just a function that calls through to a domain crate with no added logic, it's a shallow pass-through. Don't create it.

**Constraint classification:**
- MCP servers must NOT depend on `hkask-services` → **Prohibition** (anti-pattern from refactor-service-layer skill)
- Shared logic in domain crates → **Guideline** (depth test before extraction)
- Parity tests as safety net → **Guideline** (C8 test depth matching)

**Recommended approach per server (hypothesis, verify before implementing):**
- **goal**: Likely option (b) — `GoalService`'s parse-and-delegate is domain logic that belongs in `hkask-storage` or a new `hkask-goal` crate.
- **replicant**: Likely option (c) — agent loading + pod lifecycle are complex sequences with MCP-specific concerns (OCAP verification, WebID derivation). Parity tests are more appropriate.
- **spec**: Likely option (b) — `spec_goal_capture` is a capture pipeline that could live in `hkask-templates`.

**Estimated effort:** ~4–6 hours.

---

### Task 3: OPEN_QUESTIONS.md Update (Low Priority)

**Problem:** `OPEN_QUESTIONS.md` was created in Session 25 with 5 resolved and 5 deferred items. Since Sessions 26–27, F1, F3, F4, and F8 have progressed significantly. The file needs updating to reflect current state.

**Goal:** Update `OPEN_QUESTIONS.md` to reflect:
- F1: Override + API SSE done (Session 27, #80); remaining: CLI incremental printing
- F3: Resolved (Session 27, #79) — unified `capability_checker`, `mcp_secret`/`acp_secret` documented as Guardrail
- F8: Already marked resolved (Session 26, #76) — confirm entry is current
- F4: Already marked resolved — confirm entry is current
- Add `mcp_secret`/`acp_secret` Guardrail classification to F3 entry

**Estimated effort:** ~15 minutes.

---

### Task 4: Pre-existing hkask-cns Test Compile Error (Low Priority)

**Problem:** `hkask-cns/src/algedonic.rs` test `allosteric_alert_medium_alpha_warning` declares `gate` as immutable but calls `gate.set_alpha(0.7)` which requires `&mut self`. This is a pre-existing compile error that blocks `cargo test --workspace`.

**Goal:** Add `mut` to the `gate` variable declaration in the failing test(s).

**Scope:** `hkask-cns/src/algedonic.rs` — tests around lines 477, 494, and potentially others that call `set_alpha` on an immutable `AllostericGate`.

**Strategy:**

1. Read the test functions that fail.
2. Change `let gate =` to `let mut gate =` where `set_alpha()` is called.
3. Verify: `cargo test -p hkask-cns`.

**Estimated effort:** ~10 minutes.

---

### Task 5: Ensemble standing_start Orchestration (Deferred)

**Problem:** `EnsembleService::standing_start` needs orchestration logic to coordinate multiple agents in a standing session. Currently a stub.

**Goal:** Implement the standing start orchestration flow.

**Strategy:** Zoom out on `hkask-ensemble` and `StandingSessionStore` before implementing. This is a significant feature addition, not a fix. Defer until F1 and MCP duplication are resolved.

**Estimated effort:** ~2 hours.

---

### Task 6: Sovereignty Consent Enforcement Extraction (Deferred)

**Problem:** Sovereignty consent enforcement logic is embedded in API routes. It should be extracted to `SovereigntyService` per the service-layer pattern.

**Strategy:** Apply depth test — is the consent enforcement logic deep enough to warrant extraction, or is it a shallow pass-through? Defer until higher-priority tasks are done.

**Estimated effort:** ~1 hour.

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
[ ] Run tests: cargo test --workspace (or targeted crate)
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
- **Surgical changes:** No style fixes, no renaming, no comment additions beyond what the task requires.
- **MCP servers must NOT depend on `hkask-services`.** (Prohibition from refactor-service-layer skill.)

---

## Files Changed in Session 27

| File | Change |
|------|--------|
| `crates/hkask-services/src/chat.rs` | Unified `capability_checker.grant_registry()` for both auth and legacy paths; removed `DelegationResource` import |
| `crates/hkask-services/src/config.rs` | Added Guardrail doc comments to `acp_secret` and `mcp_secret` fields |
| `crates/hkask-services/src/context.rs` | Added doc comment to `capability_checker` field (mcp_secret-backed) |
| `crates/hkask-types/src/ports/mod.rs` | Added `generate_stream_with_model()` to `InferencePort` trait + `Arc` blanket impl |
| `crates/hkask-templates/src/inference_port.rs` | Added `stream: Option<bool>` to `OkapiRequest`; added `generate_stream` override + `generate_stream_with_model` direct method; added `StreamChunk`/`StreamChoice`/`StreamDelta` types; added `stream: None` to all existing request constructions |
| `crates/hkask-api/src/routes/chat.rs` | Added `POST /api/chat/stream` SSE endpoint with `tokio::sync::mpsc` channel bridge |
| `crates/hkask-api/Cargo.toml` | Added `tokio-stream = "0.1"` dependency |

---

## Build Commands

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*