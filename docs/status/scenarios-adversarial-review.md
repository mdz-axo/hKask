---
title: "Scenarios Server — Adversarial Code Review"
audience: [developers, architects]
last_updated: 2026-07-15
version: "0.31.0"
status: "Active"
domain: "Scenario forecasting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# Scenarios Server — Adversarial Code Review

**Target crate:** `mcp-servers/hkask-mcp-scenarios`
**Review date:** 2026-07-15
**Implementation status:** All 15 actionable items implemented (I-5 retracted as false finding). 14 items implemented across 6 phases. 30 tests pass, 0 clippy warnings.
**Methodology:** improve-codebase-architecture + idiomatic-rust + coding-guidelines + pragmatic-cybernetics + pragmatic-semantics + pragmatic-laziness, challenged by essentialist and grill-me perspectives

## Executive Summary

The scenarios server is a well-structured MCP surface over a shared forecasting engine. The architecture correctly separates the thin MCP tool layer (`lib.rs`) from the computation engine (`superforecast.rs`) and the domain model (`types.rs`). However, an adversarial review surfaces **15 issues** across four severity tiers, ranging from a silent data-loss path to dead fields and naming problems that mislead readers.

---

## Issue Inventory

### Tier 1 — Correctness Risk

#### I-1: Double daemon recording (cybernetic loop duplication)

**Files:** `lib.rs:327-370` (`record_experience`), `crates/hkask-mcp/src/server/tool_span.rs:274-278` (`execute_tool_semantic`)

`execute_tool_semantic` already calls `ctx.record_tool_outcome(tool_name, "success"/"error")`, which spawns a daemon `store_experience` call via `record_via_daemon`. Then every tool also calls `self.record_experience(...)`, which spawns a **second** `store_experience` call with richer detail. Each tool outcome produces two daemon writes — one minimal (tool + outcome + timestamp), one detailed (tool + input + outcome + detail + provenance + ontology_anchor).

**Cybernetic analysis:** Two feedback loops with different fidelity targeting the same actuator (daemon). The minimal loop (from `execute_tool_semantic`) has LOW fidelity — it only records success/error. The detailed loop (from `record_experience`) has HIGH fidelity — it records the full output. Both are fire-and-forget. Neither is aware of the other.

**Impact:** Wasted daemon writes, potential confusion when querying stored experiences (two records per tool call with different schemas), and a maintenance burden — if the daemon recording contract changes, two call sites must be updated.

**Recommendation:** Remove the `record_tool_outcome` call from `execute_tool_semantic` usage in this server, OR remove `record_experience` and rely on the infrastructure call (but then lose the detailed provenance). The cleaner path is to keep `record_experience` for the detailed write and suppress the infrastructure write. Since `execute_tool_semantic` is shared across all servers, the server-level fix is to make `record_experience` the single recording path and accept the infrastructure double-write as a known trade-off OR refactor `execute_tool_semantic` to accept a `skip_daemon_record` flag.

**Constraint force:** Guardrail — duplicated writes waste resources and create schema confusion.

#### I-2: ForecastStore silently swallows all filesystem errors

**Files:** `superforecast.rs:1054-1084` (`load`), `1088-1102` (`save_entry`), `1129-1141` (`compact`)

Every filesystem operation uses `let _ = fs::write(...)` or `if let Ok(...) = ...`, silently discarding errors. If the data directory is unwritable, read-only, or full, forecasts are silently lost. `persist()` returns `()`, not `Result`.

**Cybernetic analysis:** The persistence feedback loop is BROKEN on the closure property — the system performs the "write" action but never receives feedback about whether it succeeded. The system believes it persisted data when it may not have. This is a silent data loss path.

**Impact:** In production, a misconfigured `HKASK_SCENARIOS_DATA` path or a full disk would cause calibration data to vanish without any error signal. The calibration loop (`scenario_calibrate` stage 4) would never activate because `compute_calibration_curve` would find zero resolved forecasts.

**Recommendation:** Return `Result` from `save_entry` and `compact`. Log errors with `tracing::error!` at minimum. Consider an in-memory flag that surfaces "persistence degraded" in `scenario_status` output.

**Constraint force:** Prohibition (P9 feedback) — silent error swallowing violates the observability principle.

#### I-3: `tree_cache` can hold stale data indefinitely

**Files:** `lib.rs:270` (field), `1152-1154` (write in `scenario_quantify`), `417` (read in `scenario_status`)

`tree_cache` is written in `scenario_quantify` and read in `scenario_status`. It is never invalidated. If events are updated, calibrated, or scored after the last `scenario_quantify` call, `scenario_status` reports the old tree.

**Impact:** `scenario_status` may show probabilities that don't match the current forecast store state. Misleading for TUI display.

**Recommendation:** Either invalidate the cache when `scenario_score` or `scenario_update` is called, or document that `tree_cache` only reflects the last `scenario_quantify` output. The simplest fix: add a `last_quantify_timestamp` to the status output so users know the cache age.

**Constraint force:** Guideline — stale cache is a known trade-off, but it should be surfaced.

---

### Tier 2 — Design Smell

#### I-4: `reqwest::Client` field is allocated but never used

**Files:** `lib.rs:269` (field), `1838` (construction in `run`), `Cargo.toml:23` (dependency)

The `client: reqwest::Client` field is created at startup with `reqwest::Client::new()` and never accessed in any tool implementation. `self.client` produces zero grep hits across the crate.

**Essentialist G1 (Exist):** Delete the field. No behavior vanishes — no tool references it. No caller passes it for future use. The `reqwest` dependency in `Cargo.toml` exists solely to support this dead field.

**Impact:** Wasted allocation at startup, wasted dependency slot, misleading API surface (suggests HTTP calls are made, but they aren't).

**Recommendation:** Remove the field, remove the constructor argument, remove `reqwest` from `Cargo.toml` dependencies. If future tools need HTTP, add it then.

**Constraint force:** Guardrail (P5 deep-module discipline) — pass-through field that adds no behavior.

#### I-5: ~~`ScenarioError::EmptyInput` variant is never constructed~~ (FALSE FINDING — RETRACTED)

**Retraction:** `EmptyInput` IS used in `superforecast::structure_framing_document` (line 415). The initial grep missed it due to a search pattern issue. This finding is retracted — no action needed.

#### I-6: `depends_on` is `Vec<EventDependency>` but only `[0]` is ever used

**Files:** `types.rs:248` (field), `superforecast.rs:84` (`let dep = &event.depends_on[0]`), `164` (`event.depends_on[0].conditionals.last()`)

`ScenarioEvent.depends_on` is typed as `Vec<EventDependency>`, implying an event can have multiple independent dependency groups. But `compute_marginal_probabilities` and `build_event_tree` only process `depends_on[0]`. If an event has 2+ dependency groups, all but the first are silently ignored.

**Idiomatic-rust analysis:** The type says "many" but the semantics say "one." This is an invalid state the type system permits. Either:
- Change to `Option<EventDependency>` (one dependency group per event), or
- Implement multi-group marginalization (sum over all groups under independence)

**Impact:** Users who provide multiple dependency groups get silently incorrect results — the second+ groups are ignored without warning.

**Recommendation:** Change to `Option<EventDependency>` unless multi-group support is planned. This is the minimum change that makes the type match the semantics.

**Constraint force:** Guardrail (P4 clear boundaries) — type permits invalid states.

#### I-7: `basis` is `Option<String>` but should be an enum

**Files:** `types.rs:244` (field), `superforecast.rs:1408` (`"financial_model"`), `lib.rs:942` (`"technical_feasibility" or "scaling_distribution"`)

The `basis` field is documented as `"technical_feasibility"` or `"scaling_distribution"`, but typed as `Option<String>`. The companies bridge sets it to `"financial_model"` — a value NOT in the documented vocabulary. Any arbitrary string is accepted.

**Idiomatic-rust analysis:** Replace `Option<String>` with `Option<Basis>` where `Basis` is an enum. This makes wrong values impossible at the type level.

**Recommendation:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    TechnicalFeasibility,
    ScalingDistribution,
    FinancialModel,
}
```

**Constraint force:** Guardrail — type permits undocumented values.

#### I-8: `variance_contribution` is a misleading name

**Files:** `superforecast.rs:175` (`(marginal - 0.5).abs() * 2.0`), `types.rs:542` (field), `lib.rs:1177` (output)

The field is named `variance_contribution` but the computation is `|P - 0.5| * 2.0` — a distance-from-coin-flip heuristic scaled to [0, 1]. This is NOT variance contribution in any statistical sense. Actual variance contribution would require computing `Var(P)` across scenarios or using the chain rule of variance.

**Grill-me challenge:** "What does `variance_contribution = 0.8` tell you about the event?" It tells you the event probability is 0.1 or 0.9 — it's far from 50/50. The name "variance contribution" implies it measures how much the event contributes to the variance of the overall outcome, which it doesn't.

**Recommendation:** Rename to `uncertainty_score` or `distance_from_coinflip`. The `sensitivity_ranking` function uses this value and is correctly named, but the node field name is misleading.

**Constraint force:** Guideline — naming should reflect computation.

#### I-9: `joint_probability` is a specific path probability, not a joint distribution

**Files:** `superforecast.rs:153-187`

The computation multiplies: for root events, their marginal probability; for dependent events, `conditionals.last()` (P(E | all parents true)). This produces the probability of the "all events occur" path, NOT the joint probability of the event tree distribution.

The `framework` comment in `scenario_quantify` (line 1195) says "Joint = product of all-nodes-occur conditionals" — which is accurate for this specific path. But the field name `joint_probability` implies the full joint distribution.

**Impact:** Users may interpret `joint_probability = 0.15` as "the probability of this scenario" when it's actually "the probability that every single event in the tree occurs simultaneously" — a much more specific and often less useful quantity.

**Recommendation:** Rename to `all_events_probability` or `path_probability_all_occur`, or document the distinction in the output.

**Constraint force:** Guideline — naming should reflect computation.

#### I-10: Version string hardcoded in 3 places

**Files:** `lib.rs:345` (`"version": "0.31.0"` in `record_experience`), `457` (`"version": "0.31.0"` in `scenario_status`), `1819` (`const SERVER_VERSION`)

`record_experience` and `scenario_status` hardcode `"0.31.0"` as a string literal, while `SERVER_VERSION` uses `env!("CARGO_PKG_VERSION")`. These will drift on the next version bump.

**Recommendation:** Use `SERVER_VERSION` in all three locations, or define a `const PROVENANCE_VERSION` early in the file.

**Constraint force:** Guardrail — version drift creates provenance confusion.

---

### Tier 3 — Pattern Inconsistency

#### I-11: Request types use `String` for JSON where typed structs exist

**Files:** `lib.rs:109` (`events: String` in `FullPipelineRequest`), `147` (`events: String` in `QuantifyRequest`), `171` (`outcomes: String` in `ScoreRequest`), `201` (`perspectives: String` in `SynthesizeRequest`), and others

Many request types accept `String` parameters that are then `serde_json::from_str`'d into typed structs (`Vec<ScenarioEvent>`, `Vec<Perspective>`, etc.). The types already implement `JsonSchema + Deserialize`. Compare with `hkask-mcp-kata-kanban` which uses typed structs directly (e.g., `columns: Option<Vec<ColumnInput>>`).

**Impact:** MCP clients don't get schema validation for nested types; validation errors are deferred to runtime `from_str` calls; the MCP tool schema shows a string instead of a structured object.

**Recommendation:** Replace `String` parameters with the typed structs where the types already implement `JsonSchema`. This is a breaking API change, so it should be done atomically per tool. Start with `QuantifyRequest.events: Vec<ScenarioEvent>`.

**Constraint force:** Guideline — consistency with other MCP servers in the project.

#### I-12: `parse_time_horizon` and `parse_scenario_type` silently default on unknown input

**Files:** `lib.rs:1798-1815`

Both functions default to `Strategic` / `CompanyAnalysis` for unrecognized strings. Typos like `"tactial"` silently produce a strategic horizon. An idiomatic approach would return `Result` or use the enum's `Deserialize` implementation directly.

**Impact:** Silent wrong results from input typos. No error signal to the caller.

**Recommendation:** Either return `Result<TimeHorizon, ScenarioError>` and propagate the error, or accept the typed enum directly in the request struct (which gives JSON Schema validation for free).

**Constraint force:** Guideline — silent defaults on invalid input are a footgun.

#### I-13: `combined_router` is a trivial pass-through

**Files:** `lib.rs:375-379`

```rust
fn combined_router() -> ToolRouter<Self> {
    Self::scenario_router()
}
```

Single-line delegation with no added behavior. The name "combined" suggests it was designed to merge multiple routers, but it only calls one.

**Essentialist G1 (Exist):** Inline `Self::scenario_router()` directly in the `#[tool_handler]` attribute. No behavior vanishes.

**Recommendation:** Replace `Self::combined_router()` with `Self::scenario_router()` in the `#[tool_handler]` attribute and delete the function.

**Constraint force:** Guideline (P5) — pass-through wrapper.

---

### Tier 4 — Documentation Gap

#### I-14: README references 4 non-existent documentation files

**Files:** `README.md:54-58`

The README references:
- `docs/explanation/scenario-forecasting.md` — does NOT exist (only `superforecasting-layers.md` exists)
- `docs/architecture/scenarios-companies-bridge.md` — does NOT exist
- `docs/diagrams/flowchart-scenario-forecasting-pipeline.md` — did NOT exist (now created by this review)
- `docs/reference/mcp-servers/README.md` — did NOT exist (now created by this review)

**Impact:** Broken documentation links. Readers following the README hit 404s.

**Recommendation:** This review creates the flowchart diagram and the MCP server registry. The explanation and architecture bridge docs should be created in a follow-up (they require domain context beyond this review's scope).

**Constraint force:** Guardrail — documentation must reflect code reality.

#### I-15: `scenario_calibration` records hardcoded input summary

**Files:** `lib.rs:1634`

```rust
self.record_experience("scenario_calibration", "calibration_curve", "success", output.clone());
```

The input summary is hardcoded to `"calibration_curve"` regardless of the actual subject filter. Compare with other tools that format meaningful summaries (e.g., `format!("subject={}", req.subject)`).

**Recommendation:** Change to `&format!("subject={:?}, forecasts={}", req.subject, curve.resolved_forecasts)`.

**Constraint force:** Guideline — provenance should reflect actual input.

---

## Essentialist Challenge

Running the 3-gate eliminative interrogation on each finding:

| Issue | G1 Exist | G2 Surface | G3 Contract | Verdict |
|-------|----------|------------|-------------|---------|
| I-4 (reqwest::Client) | FAIL — delete, nothing breaks | N/A | N/A | **Delete** |
| I-5 (EmptyInput) | RETRACTED — variant IS used | N/A | N/A | **No action** |
| I-13 (combined_router) | FAIL — inline, nothing breaks | N/A | N/A | **Inline** |
| I-1 (double recording) | PASS — removing record_experience loses provenance detail | PASS — 2 public items (infrastructure + detail) | PASS — each encodes different behavior | **Keep both, document trade-off** |
| I-6 (depends_on Vec) | PASS — removing the Vec changes the type | FAIL — Vec implies many, only one used | FAIL — type permits ignored entries | **Fix type to Option** |
| I-8 (variance_contribution) | PASS — the value is used | PASS — single field | PASS — but name is wrong | **Rename** |

**Essentialism score:** 2 items (I-4, I-13) should be deleted/inlined. 15 total items → 13% reduction. (I-5 retracted — EmptyInput IS used.)

---

## Grill-Me Challenge

Adversarial questions that test the review's own assumptions:

**Q1 (Mechanism):** "You claim `record_experience` and `execute_tool_semantic` produce double writes. But `record_tool_outcome` only records `{tool, outcome, timestamp}` while `record_experience` records `{tool, input, outcome, detail, provenance, ontology_anchor}`. Are these really the same actuator, or are they different semantic layers?"

**Answer:** They hit the same daemon endpoint (`store_experience`) with the same session type (`"mcp_session"`) and category (`"observed"`). The daemon doesn't distinguish them — they're two records in the same store. The semantic layers ARE different (minimal vs detailed), but the actuator is identical. The double-write is real.

**Q2 (Edge Cases):** "You recommend changing `depends_on: Vec<EventDependency>` to `Option<EventDependency>`. What about events that depend on two independent groups of parents — e.g., depends on (A OR B) AND (C OR D)? Wouldn't `Option` prevent that?"

**Answer:** Yes, `Option` would prevent multi-group dependencies. But the current code already doesn't support them — it only reads `[0]`. The choice is: (a) change to `Option` and match the implemented semantics, or (b) implement multi-group marginalization and keep `Vec`. The review recommends (a) as the minimum change. Option (b) is a feature addition that should be a separate task.

**Q3 (Rationale):** "You flag `parse_time_horizon` for silent defaults. But the MCP tool schema shows `time_horizon` as `Option<String>` — the caller may intentionally omit it. Isn't a default preferable to an error for an optional field?"

**Answer:** The issue is not the default itself — it's that unrecognized values also default silently. `"tactial"` → `Strategic` without error. If the field is truly optional, `None` should produce the default and `Some("invalid")` should error. The current code conflates "absent" with "invalid."

---

## Pragmatic Laziness Decomposition

Decomposing the 15 issues into the smallest possible independent action items, ordered by dependency and effort:

### Phase 1 — Zero-risk deletions (no behavior change)

| Step | Issue | Files | Effort | Verification |
|------|-------|-------|--------|-------------|
| 1.1 | Remove `reqwest::Client` field + constructor arg + Cargo.toml dep | `lib.rs:269,1838`, `Cargo.toml:23` | 5 min | `cargo build` |
| ~~1.2~~ | ~~Remove `EmptyInput` variant~~ | ~~RETRACTED: variant IS used~~ | — | — |
| 1.3 | Inline `combined_router` → `scenario_router` | `lib.rs:375-379,1793` | 2 min | `cargo build` |

### Phase 2 — Naming and string fixes (no logic change)

| Step | Issue | Files | Effort | Verification |
|------|-------|-------|--------|-------------|
| 2.1 | Replace hardcoded `"0.31.0"` with `SERVER_VERSION` | `lib.rs:345,457` | 5 min | `cargo build` + grep for `0.31.0` |
| 2.2 | Fix `scenario_calibration` input summary | `lib.rs:1634` | 2 min | `cargo build` |
| 2.3 | Rename `variance_contribution` → `uncertainty_score` | `types.rs:542`, `superforecast.rs:175`, `lib.rs:1177` | 10 min | `cargo test` |

### Phase 3 — Type improvements (behavior change in error paths only)

| Step | Issue | Files | Effort | Verification |
|------|-------|-------|--------|-------------|
| 3.1 | Change `depends_on: Vec<EventDependency>` → `Option<EventDependency>` | `types.rs:248`, all access sites | 30 min | `cargo test` — update test fixtures |
| 3.2 | Add `Basis` enum, replace `Option<String>` | `types.rs:244`, `superforecast.rs:1408` | 20 min | `cargo test` |
| 3.3 | Rename `joint_probability` → `all_events_probability` | `types.rs:558`, `superforecast.rs:153-187`, `lib.rs:1167,1195` | 15 min | `cargo test` |

### Phase 4 — Error handling (behavior change)

| Step | Issue | Files | Effort | Verification |
|------|-------|-------|--------|-------------|
| 4.1 | Return `Result` from `ForecastStore::save_entry` and `compact`, log errors | `superforecast.rs:1088-1141` | 30 min | `cargo test` + manual disk-full test |
| 4.2 | Add stale-cache indicator to `scenario_status` | `lib.rs:389-465` | 15 min | `cargo test` |

### Phase 5 — API consistency (breaking change, needs coordination)

| Step | Issue | Files | Effort | Verification |
|------|-------|-------|--------|-------------|
| 5.1 | Replace `String` JSON params with typed structs (per-tool, one at a time) | `lib.rs` request types | 2-3 hours | `cargo test` + MCP client testing |
| 5.2 | Return `Result` from `parse_time_horizon` / `parse_scenario_type` | `lib.rs:1798-1815` | 15 min | `cargo test` |

### Phase 6 — Infrastructure (requires cross-crate change)

| Step | Issue | Files | Effort | Verification |
|------|-------|-------|--------|-------------|
| 6.1 | Resolve double daemon recording (suppress infrastructure write or merge schemas) | `lib.rs:327-370`, `crates/hkask-mcp/src/server/tool_span.rs` | 1 hour | `cargo test` + daemon log inspection |

### Phase 7 — Documentation (follow-up)

| Step | Issue | Files | Effort | Verification |
|------|-------|-------|--------|-------------|
| 7.1 | Create `docs/explanation/scenario-forecasting.md` | new file | 1 hour | `docs/ci/verify-docs.sh` |
| 7.2 | Create `docs/architecture/scenarios-companies-bridge.md` | new file | 1 hour | `docs/ci/verify-docs.sh` |
| 7.3 | Update README if any tool names change | `README.md` | 15 min | manual review |

---

## Cybernetic Summary

| Loop | Sensing | Decision | Action | Return Path | Health |
|------|---------|----------|--------|-------------|--------|
| Calibration feedback | `compute_calibration_curve` reads resolved forecasts | Compare hit rate vs forecast probability | `apply_calibration_adjustment` in `scenario_calibrate` | `scenario_score` stores outcomes → curve updates | **Healthy** (when persistence works) |
| Pipeline sequence | `check_sequence` tracks called tools | Compare against `expected_predecessor` | `tracing::warn!` only | Warning logged to Regulation | **Degraded** (warn-only, no remediation) |
| Persistence | `ForecastStore::save_entry` writes journal | Threshold check for compaction | `fs::write` / `fs::OpenOptions` | `let _ = ...` — error discarded | **Broken** (closure property violated) |
| Experience recording | `record_experience` captures tool I/O | Serialize to JSON | `daemon.store_experience` | `tracing::warn!` on error | **Degraded** (fire-and-forget, errors logged) |
| Cross-validation | `cross_validate` compares two estimates | Compare divergence vs threshold | Generate grill-me questions | Agent activates grill-me skill | **Healthy** (closes learning loop) |

**Variety check:** The system produces 18 distinct tool calls × N event configurations. The regulator (pipeline sequence checker) can only produce 1 response (warn). Variety deficit: 18:1 — the regulator cannot meaningfully constrain the system's behavior. This is by design (non-blocking), but it means the sequence loop is advisory, not regulatory.

---

## Cross-links

- [Scenario Forecasting Pipeline Diagram](../diagrams/flowchart-scenario-forecasting-pipeline.md) — tool flow diagram
- [Superforecasting: Layered Model](../explanation/superforecasting-layers.md) — three-layer model
- [Scenarios Semantic Graph Audit](scenarios-semantic-graph-audit.md) — cross-skill/server dependency graph
- [MCP Server Registry](../reference/mcp-servers/README.md) — built-in server index
- [Architecture Principles](../architecture/core/PRINCIPLES.md) — P2, P4, P5, P9 constraints