# CONTINUATION.md — hKask Post-Extraction Follow-Up

**Sessions:** 12–25 | **Status:** Extraction ✅ COMPLETE | Follow-up: active extractions in progress (3 done, 2+ remaining)

---

## Summary

The service layer extraction project completed in Session 23 (2026-06-08). All 27 CLI
commands have been evaluated with the depth test. 17 are extracted to service modules.
10 are documented as surface-only. One API route quality issue was fixed (typed OCAP
error matching in `routes/episodic.rs`).

**Read these files first (in this order):**

1. **`HANDOFF.md`** — Full session history, key decisions (#1–#72), service module
   inventory, file reference map, completion metrics (§7).
2. **`CONTINUATION.md`** (this file) — Current task status and follow-up priority.
3. **`CONTINUATION-PROMPT.md`** — Detailed task plans with per-task strategies,
   skills, constraint classifications, and depth-test questions.

---

## Task Status

| 1 | F10 — Typed DTOs for SemanticStoragePort | 🟡 Medium | ✅ Done (Session 25) | ~1h |
| 2 | F5 — Pod test ACP secret fixture | 🔴 High | ✅ Done (Session 24) | ~1–2h |
| 3 | F9 — Typed DTOs for EpisodicStoragePort | 🔴 High | ✅ Done (Session 24) | ~2–3h |
| 4 | OPEN_QUESTIONS.md (F1–F10) | 🟡 Medium | ✅ Done (Session 25) | ~30m |
| 5 | Test inventory update | 🟡 Medium | ✅ Done (Session 25) | ~1h |
| 6 | Condenser build fix | 🟡 Medium | ✅ Already resolved | 0 |
| 7 | Ensemble get_chat + list_deliberations | 🔴 High | ✅ Done (Session 25) | ~30m |
| 8 | Sovereignty grant_consent_and_fetch | 🔴 High | ✅ Done (Session 25) | ~20m |
| 9 | ConsolidationService rate limit + db_path | 🔴 High | ✅ Done (Session 25) | ~30m |
| 10 | Ensemble standing_start orchestration | 🔴 High | Not started | ~2h |
| 11 | Sovereignty consent enforcement extraction | 🔴 High | Not started | ~1h |
| 12 | Chat PromptStrategy framing | 🟡 Medium | Not started | ~1h |

---

## Session History (Post-Extraction)

### Session 26 (Test Inventory Update)

- **Test inventory update:** Refreshed `docs/status/test-inventory.md` with actual test counts. Fixed per-module header discrepancies (EnsembleService 18→17, SovereigntyService 14→13, GoalService 14→13, CuratorService 8→6, PodService 7→6). Removed non-existent test entries (`parse_data_category` from EnsembleService, `invalid_uuid_returns_pod_not_found` from SovereigntyService). Updated condenser section from 35→53 tests with individual algorithm/type test entries replacing grouped ranges. Added 2 new `classify_tool` tests (priority inversion fix, separator splitting). Updated summary total 192→210. Clarified `hkask-agents` doc test status (1 ignored, 2 passing).
- **Verification:** `cargo check --workspace` ✅. `cargo clippy -p hkask-agents -p hkask-services -p hkask-api -- -D warnings` ✅. `cargo test --workspace` ✅ (0 failures).

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