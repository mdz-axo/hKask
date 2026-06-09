# CONTINUATION-PROMPT.md — hKask Session 29 Follow-Up

**Sessions:** 12–29 | **Status:** All tasks evaluated ✅ | F4 ✅ P1 Prohibition documented | Standing_start ✅ Divergent | Consent ✅ Already extracted | PromptStrategy ✅ Depth test fails

---

## Skills to Load (Required Before Starting)

1. **`refactor-service-layer`** — Required for any `hkask-services` change
2. **`coding-guidelines`** — Required (surgical changes)
3. **`constraint-forces`** — Required (decision classification)
4. **`zoom-out`** — Required before each task
5. **`diagnose`** — Available for build/test failures

---

## Read These Files First (In This Order)

1. **`HANDOFF.md`** — Full session history (Sessions 12–28), key decisions (#1–#83), service module inventory (§3), file reference map (§6), completion metrics and open questions (§7).
2. **`CONTINUATION.md`** — Current task status.
3. **`OPEN_QUESTIONS.md`** — F1–F10 status with constraint force classifications.

---

## Session 28 Accomplishments

| Task | What Was Done | Decision # | Status |
|------|--------------|-----------|--------|
| **F1 — CLI incremental printing** | Added `ChatService::prepare_chat()` (deep extraction: agent lookup, prompt composition, semantic recall, inference port resolution). Added `chat_with_agent_streaming()` in CLI surface. Streaming prints `text_delta` chunks via `print!()` + stdout flush, then stores episodic. Refactored `ChatService::chat()` to delegate to `prepare_chat()`. Made `recall_semantic()` and `store_episodic()` public. Added `PreparedChat` struct. Modified `run_chat()` one-shot path and `single_agent_turn()` REPL path to use streaming. | #82 | ✅ Complete |
| **MCP server duplication** | Zoom-out analysis of goal, replicant, spec MCP servers. All three classified as parity-test candidates (option c): goal delegates to same `SqliteGoalRepository`; replicant has P1 Prohibition against `PodService`/`InferenceService`; spec has 8 of 11 MCP-only tools. Added parity tests for goal (4 tests) and spec (3 tests). | #83 | ✅ Analysis complete, parity tests added |
| **OPEN_QUESTIONS.md update** | Updated F1 (streaming complete), F3 (resolved with Guardrail classification), F4 (reclassified as parity-test Guideline), F8 (resolved). | — | ✅ Complete |
| **CNS test fix** | Removed unnecessary `mut` from `VarietyTracker` in `algedonic.rs`. Moved `impl CircuitBreakerPort` before `#[cfg(test)] mod tests` in `circuit_breaker.rs`. | — | ✅ Complete |

**Verification baseline:**
```
cargo check --workspace                                    ✅
cargo clippy --workspace -- -D warnings                    ✅
cargo test -p hkask-services -p hkask-cli -p hkask-types -p hkask-cns  ✅ (0 failures)
```

---

## Remaining Work (Priority-Ordered)

### Task 1: Ensemble standing_start Orchestration (High Priority)

**Problem:** `EnsembleService::standing_start` needs orchestration logic to coordinate multiple agents in a standing session. Currently a stub.

**Goal:** Implement the standing start orchestration flow.

**Strategy:**
1. Zoom out on `hkask-ensemble` and `StandingSessionStore` before implementing.
2. This is a significant feature addition, not a fix. Ensure depth test passes before extraction.
3. Consider whether `standing_start` is a service-layer concern or a surface concern.

**Constraint classification:** Guideline — ensemble sessions are user choice (P3 Generative Space).

**Estimated effort:** ~2 hours.

---

### Task 2: Sovereignty Consent Enforcement Extraction (High Priority)

**Problem:** Sovereignty consent enforcement logic is embedded in API routes. It should be extracted to `SovereigntyService` per the service-layer pattern.

**Goal:** Extract consent enforcement from API routes into `SovereigntyService`.

**Strategy:**
1. Apply depth test — is the consent enforcement logic deep enough to warrant extraction, or is it a shallow pass-through?
2. If deep, extract to `SovereigntyService` following the strangler fig pattern.
3. If shallow, document as a legitimate surface-only concern and move on.

**Constraint classification:** Guideline — service-layer pattern (P3).

**Estimated effort:** ~1 hour.

---

### Task 3: Chat PromptStrategy Framing (Medium Priority)

**Problem:** The chat prompt composition logic is hardcoded in `ChatService::prepare_chat()`. A `PromptStrategy` abstraction could make it easier to customize prompts per agent or use case.

**Goal:** Evaluate whether a PromptStrategy abstraction is warranted, and if so, frame the design.

**Strategy:**
1. Apply the depth test to the proposed PromptStrategy. If deleting it makes complexity vanish, don't create it.
2. The current prompt composition is ~30 lines of straightforward string assembly. A strategy pattern would add indirection without reducing complexity.
3. Likely conclusion: document as a future consideration, don't implement now.

**Constraint classification:** Hypothesis — needs verification.

**Estimated effort:** ~30 minutes (evaluation only).

---

### Task 4: MCP Parity Test Completion (Medium Priority)

**Problem:** Goal parity tests are complete (4 tests). Spec parity tests are complete (3 tests). Replicant parity tests are not yet written, and the zoom-out analysis concluded that the replicant duplication is architecturally intentional (P1 Prohibition).

**Goal:** Document the replicant duplication as intentional in code comments, per the analysis from Session 28.

**Strategy:**
1. Add code comments to `hkask-mcp-replicant` noting the P1 Prohibition against `PodService`/`InferenceService` and explaining why the duplication is intentional.
2. No parity tests needed — the duplication is by design, not by accident.

**Constraint classification:** Prohibition (P1 — MCP servers must NOT depend on `hkask-services`).

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
- **Depth test (P2):** If deleting the proposed module makes complexity vanish, don't create it.
- **Surgical changes:** No style fixes, no renaming, no comment additions beyond what the task requires.
- **MCP servers must NOT depend on `hkask-services`.** (Prohibition from refactor-service-layer skill.)

---

## Files Changed in Session 28

| File | Change |
|------|--------|
| `crates/hkask-services/src/chat.rs` | Added `PreparedChat` struct, `ChatService::prepare_chat()` method; refactored `chat()` to use `prepare_chat()`; made `recall_semantic()` and `store_episodic()` public |
| `crates/hkask-services/src/lib.rs` | Added `PreparedChat` to re-exports |
| `crates/hkask-cli/src/commands/chat.rs` | Added `chat_with_agent_streaming()`; modified `run_chat()` to use streaming for one-shot mode; added `futures-util` and `LLMParameters` imports |
| `crates/hkask-cli/src/commands/mod.rs` | Added `chat_with_agent_streaming` to re-exports |
| `crates/hkask-cli/src/repl/turn.rs` | Modified `single_agent_turn()` to use streaming |
| `crates/hkask-cli/Cargo.toml` | Added `futures-util.workspace = true` |
| `crates/hkask-cns/src/algedonic.rs` | Removed unnecessary `mut` from `VarietyTracker` in test |
| `crates/hkask-cns/src/circuit_breaker.rs` | Moved `impl CircuitBreakerPort` before `#[cfg(test)] mod tests` |
| `crates/hkask-services/src/goal.rs` | Added 4 parity tests (MCP goal server vs GoalService) |
| `crates/hkask-services/src/spec.rs` | Added 3 parity tests (MCP spec server vs SpecService) |
| `OPEN_QUESTIONS.md` | Updated F1 (resolved), F3 (resolved with Guardrail), F4 (reclassified as parity-test Guideline), F8 (resolved) |
| `CONTINUATION.md` | Updated task status, added Session 28 history |
| `HANDOFF.md` | Updated decisions (#82, #83), open questions, file reference map |

---

## Build Commands

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*