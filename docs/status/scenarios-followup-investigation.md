---
title: "Scenarios Server — Follow-Up Investigation"
audience: [developers, architects]
last_updated: 2026-07-15
version: "0.31.0"
status: "Active"
domain: "Scenario forecasting"
mds_categories: [domain, composition, trust, lifecycle]
---

# Scenarios Server — Follow-Up Investigation

Investigation of 8 follow-up questions raised after the adversarial review implementation. The error recording regression (Q1) was fixed; findings for Q2–Q10 are below.

## Q1: Error Recording Regression — FIXED

**Problem:** Suppressing `record_tool_outcome` to a no-op eliminated ALL daemon recording for error outcomes. `record_experience` only runs on success (inside the closure before `Ok(output)`). Additionally, `check_sequence` was only called from `record_experience`, so error calls weren't tracked — causing false sequence violation warnings for successor tools.

**Fix applied:** `record_tool_outcome` now:
1. Always calls `check_sequence(tool)` — errors count as "called"
2. Records errors to the daemon via `record_via_daemon` — error outcomes are persisted
3. Skips the daemon write for successes — `record_experience` handles those with full provenance

**Verified:** 30/30 tests pass, 0 clippy warnings.

## Q2: Are there existing MCP clients that will break?

**Investigation:** Searched all crates for references to the 7 affected tools (`scenario_quantify`, `scenario_full`, `scenario_score`, `scenario_calibrate`, `scenario_synthesize`, `scenario_sensitivity`, `scenario_cross_validate`).

**Finding: No existing callers will break.**

| Crate | References scenarios tools? | Impact |
|-------|---------------------------|--------|
| `hkask-cli` | No | None |
| `hkask-api` | No | None |
| `hkask-repl` | Calls `scenario_status` only (not affected) — reads response JSON via `serde_json::Value` getters | None |
| `hkask-tui` | Display string only (`"via scenario_calibrate…"`) | None |
| `hkask-mcp-companies` | Bridge not wired at runtime | None |
| Skill manifests / YAML configs | No tool call construction | None |
| JSON test fixtures | None exist for these tools | None |

**Risk flagged and verified:** The `Basis` enum change (Q2 agent flagged `tui_bridges.rs:1219` reading `basis` with `as_str()`). Verified safe — `Basis` is a field-less enum with `#[serde(rename_all = "snake_case")]`, so it serializes as a plain JSON string (`"technical_feasibility"`), not an object. `as_str()` works correctly.

**Recommendation:** No action needed. The breaking changes are safe because no in-tree callers construct requests to the affected tools. External MCP clients (if any exist outside this repo) would need to update their request format.

## Q3: What other crates consume the scenarios server's JSON output?

**Investigation:** Searched entire codebase for `joint_probability`, `all_events_probability`, `variance_contribution`, `uncertainty_score`, and scenario JSON field names.

**Finding: Rename is fully complete.** Only 2 crates consume scenario JSON output:

| Crate | What it parses | Updated? |
|-------|---------------|----------|
| `hkask-repl` | `scenario_status` response: pipeline stats, calibration, event tree nodes | ✅ All field references updated |
| `hkask-tui` | Bridge trait + data types (`EventTreeDetail`, `EventNode`, etc.) | ✅ All field references updated |

No other crates (`hkask-cli`, `hkask-api`, `hkask-communication`, etc.) parse scenario JSON. No JSON test fixtures contain the old field names. The only remaining references to old names are in documentation files describing the rename.

**Recommendation:** No action needed.

## Q4: Should `persist()` return `Result`?

**Investigation:** `persist()` is called from `scenario_score` inside an `execute_tool_semantic` closure that returns `Result<Value, McpToolError>`. Propagating a persistence error would be mechanically trivial.

**Finding:** Propagating would degrade user experience. The Brier score computation completes *before* persistence runs. Failing the entire tool call because the secondary persistence side-effect failed would deny the user the scoring result they asked for.

**Recommendation:** Make `persist()` return `Result<(), PersistenceError>` (fixing the silent-swallowing design smell at the function level), but at the `scenario_score` call site, **don't propagate** — embed the error in the output JSON:

```rust
"persistence_status": match store.persist() {
    Ok(()) => "healthy",
    Err(e) => { tracing::error!(...); format!("degraded: {e}") }
}
```

This gives immediate feedback in the same response (not requiring a separate `scenario_status` call) while preserving the computed score. The `persistence_healthy` flag in `scenario_status` remains as a cross-tool health indicator.

**Priority:** Low — the current `tracing::error!` + health flag is adequate. The embedded status would be a UX improvement.

## Q5: What tests are missing?

**Investigation:** The test module has 19 tests covering engine math. 5 of 6 review-change behaviors are completely untested:

| Behavior | Tested? | Gap |
|----------|---------|-----|
| Invalid `time_horizon`/`scenario_type` returning errors | No | No test deserializes with bad enum values |
| `persistence_healthy` flag on disk failure | No | No test uses unwritable path |
| `Basis` enum serialization round-trip | No | No test verifies snake_case mapping |
| `Option<EventDependency>` rejecting `[]` in JSON | Partial | Validation code exists but empty-`parent_event_ids` path untested |
| Error outcomes recorded to daemon | No | Requires mock `DaemonClient` |
| `uncertainty_score` / `all_events_probability` correctness | Yes ✅ | Existing tests verify computation values |

**Recommendation:** Add 4 tests (priority order):
1. **`persistence_healthy` on disk failure** — highest value, tests the core mitigation for silent data loss
2. **`Basis` enum round-trip** — quick, verifies serialization contract
3. **Empty `parent_event_ids` rejected** — exercises an existing validation path
4. **Invalid `time_horizon` error** — verifies the `parse_time_horizon` Result change

The daemon error-recording test requires a mock `DaemonClient` which doesn't exist yet — defer until test infrastructure is available.

**Priority:** Medium — tests prevent regressions in the behaviors we just changed.

## Q6: Is the `tree_cache` stale indicator useful?

**Investigation:** `tree_cache` is written in `scenario_quantify` and read in `scenario_status`. It is never invalidated. None of the downstream tools (`scenario_update`, `scenario_score`, `scenario_calibrate`) modify the tree in-place — they return new values to the caller.

**Finding:** Invalidating on `scenario_score` or `scenario_update` would be **incorrect** — these tools don't change the tree structure, only probabilities/calibration. Invalidating would empty the cache, making `scenario_status` show `null` for the tree — *less* useful than a stale tree with a caveat.

**The string note (`cache_note`) is static and unhelpful** — it always says the same thing regardless of how stale the cache is.

**Recommendation:** Replace the string note with a timestamp:

```rust
// Add to ScenariosServer:
last_quantify_at: std::sync::Mutex<Option<std::time::Instant>>,

// In scenario_quantify:
*self.last_quantify_at.lock().unwrap_or_else(|e| e.into_inner()) = Some(std::time::Instant::now());

// In scenario_status output:
"cache_age_seconds": last_quantify_at.map(|t| t.elapsed().as_secs())
```

This converts "may be stale" from a vague warning into actionable information. The TUI could display "cached 2m ago" and the user can judge whether to re-run `scenario_quantify`.

**Priority:** Low — the current string note is documentation-only; a timestamp is an improvement but not urgent.

## Q7: What did the initial grep get wrong?

**Already addressed.** I-5 (`EmptyInput` unused) was the only false finding, caused by a grep tool issue. It has been retracted in the review document. I-4 (`reqwest::Client` unused) was verified by compilation — confirmed unused. No other findings relied solely on grep results that might have been incomplete.

**Recommendation:** No action needed.

## Q8: Version bump?

**Per user direction:** Not a version bump. The changes are internal improvements within the v0.31.0 cycle.

**Recommendation:** No action needed.

## Q9: Should `companies_output` and `answers` use typed structs?

**Investigation:** Read `convert_companies_output` and `structure_framing_document` to identify expected JSON schemas.

### `answers` — yes, cleanly typable

`structure_framing_document` expects a 12-key JSON object that maps 1:1 onto the existing `FramingDocument` struct. The target types (`FramingDocument`, `StakeholderConfig`, `UseCase`, `TimeHorizon`) already exist with `Deserialize + JsonSchema`.

**Recommendation:** Define a `FramingAnswers` request struct replacing `FrameDocumentRequest.answers: String`. Low-risk, high-value — gives MCP clients schema validation. The only friction is lenient enum string aliases (e.g., `"12-18 months"` for `TimeHorizon`), which can be handled with `#[serde(alias = ...)]` attributes on the enum variants.

**Priority:** Medium — consistency with the typed-struct changes already made to other request types.

### `companies_output` — schema mismatch found (pre-existing bug)

**Finding:** `convert_companies_output` expects `scenarios[i].intrinsic_per_share`, but the companies server's `calibrate_forecast` tool emits `intrinsic` (not `intrinsic_per_share`). The `scenario_analysis` tool is closer but nests `current_price` under `summary` instead of top-level. This means the bridge would silently degrade on real output — `intrinsic_per_share` defaults to `0.0`, and no Fermi sub-questions are generated from `applied_growth`/`applied_margin`.

This is a **pre-existing bug** hidden by the untyped `String` parameter. The bridge was never tested end-to-end because the runtime MCP call path between the two servers isn't wired yet.

**Recommendation:** Before typing `companies_output`:
1. **Fix the schema mismatch** — align `calibrate_forecast` output to include `intrinsic_per_share`, `applied_growth`, `applied_margin`, and top-level `current_price`
2. **Introduce a shared `CompaniesCalibrationOutput` struct** in `hkask-types` or a shared crate
3. **Replace `companies_output: String`** with the typed struct

**Priority:** The schema mismatch is a real bug (Priority: Medium). Typing the parameter is Priority: Low until the mismatch is fixed.

## Q10: What's the state of `check_sequence` after the regression fix?

**Fixed as part of Q1.** `check_sequence` is now called from `record_tool_outcome` which runs on both success and error paths. Error calls are tracked in `called_tools`, preventing false sequence violation warnings for successor tools.

On success, `check_sequence` is called twice (once from `record_experience` inside the closure, once from `record_tool_outcome` after). This is harmless — `HashSet::insert` is idempotent.

**Recommendation:** No further action needed. The double-call on success is a minor inefficiency but not a correctness issue. If it becomes a concern, remove the `check_sequence` call from `record_experience` and rely solely on `record_tool_outcome`.

---

## Priority Summary

| Question | Finding | Priority | Action |
|----------|---------|----------|--------|
| Q1 | Error recording regression | **Fixed** | ✅ Done |
| Q2 | No callers will break | None | No action |
| Q3 | Rename fully complete | None | No action |
| Q4 | `persist()` should return `Result` but not propagate | Low | Embed status in output JSON |
| Q5 | 5 of 6 behavior changes untested | Medium | Add 4 targeted tests |
| Q6 | String note is static; timestamp would help | Low | Add `last_quantify_at: Instant` |
| Q7 | I-5 retracted, no other grep errors | None | No action |
| Q8 | Not a version bump (per user) | None | No action |
| Q9a | `answers` is cleanly typable | Medium | Define `FramingAnswers` struct |
| Q9b | `companies_output` has schema mismatch | Medium | Fix `calibrate_forecast` output keys first |
| Q10 | `check_sequence` fixed via Q1 | None | No action |

## Cross-links

- [Scenarios Adversarial Review](scenarios-adversarial-review.md) — original review with 15 issues
- [Scenario Forecasting Pipeline Diagram](../diagrams/flowchart-scenario-forecasting-pipeline.md) — tool flow
- [Scenarios ↔ Companies Bridge](../architecture/scenarios-companies-bridge.md) — bridge architecture (note: schema mismatch documented here needs updating)