# rJoule Cost Tracking — Implementation Plan

**Version:** v0.30.0  
**Status:** Planning  
**Last updated:** 2026-06-20  
**Depends on:** `docs/architecture/specs/rjoule-cost-system.md`

---

## Gap Audit: Implemented vs. Declared

### Spec Invariants — Implementation Status

| Invariant | CNS Span | Status |
|-----------|----------|--------|
| Gas per step matches expected | `cns.qa.cost.gas_mismatch` | ✅ Implemented |
| Alert threshold warning | `cns.qa.cost.threshold_warning` | ✅ Implemented |
| API cost tracks per classify | `cns.qa.cost.api_untracked` | ✅ Implemented (Phase 1) |
| No missing step tracking | `cns.qa.cost.step_untracked` | ✅ Covered by `gas_mismatch` |
| Cap exceeded alert | `cns.qa.cost.cap_exceeded` | ✅ Implemented (Phase 1) |
| Missing token data on classify | `cns.qa.cost.missing_token_data` | ✅ Implemented (Phase 1) |

### Structural Gaps

| Gap | Severity | Description |
|-----|----------|-------------|
| Failed API call costs lost | High | `_tokens` parsed from error response but discarded; fallback returns `cost_urj: 0`. Provider may charge for input tokens. |
| No training cost tracking | Medium | `training_urj` field exists in CostTracker but nothing writes to it. Training MCP server has no rJoule integration. |
| Per-step cost invisible | Medium | `QaScriptReport` aggregates all costs; users can't see which step was expensive. |
| Shell commands flat rate | Low | 100 gas for any shell command regardless of wall-clock time. `cargo bolero test --timeout 300s` costs same as `echo hello`. |
| Subscription costs static | Low | `monthly_subscriptions_urj` is manifest-declared, not dynamically allocated. |
| Provider pricing manual | Low | Classifier YAMLs need manual `cost_input_nj_per_token`. No auto-lookup from provider name. |
| Kata/bundle dead fields | Low | `cost_per_token` still in kata and bundle GasConfigs — separate domains, deferred. |

---

## Phased Plan

### Phase 1 — Close CNS Span Gaps (Immediate) ✅ COMPLETE

Close the spec/implementation gap on declared invariants. These are the four missing CNS spans.

**Task 1.1:** Emit `cns.qa.cost.api_untracked` when a classify step returns zero `cost_urj` despite having an active API key (i.e., the classify succeeded but cost is zero — implies classifier config is missing pricing).

**Task 1.2:** Emit `cns.qa.cost.step_untracked` when a step completes without incrementing the gas counter. Already partially handled by the `_` catch-all, but need explicit verification that each action type increments gas.

**Task 1.3:** Emit `cns.qa.cost.cap_exceeded` as a CNS span when `exceeded_gas` is true, rather than just `println!`. Include `total_urj`, `cap_urj`, and `manifest_id` in the span.

**Task 1.4:** Emit `cns.qa.cost.missing_token_data` when a classify call returns `prompt_tokens == 0` and `completion_tokens == 0` despite the API key being present (i.e., the API returned no usage data — suspicious).

**Files:** `crates/hkask-test-harness/src/qa_script.rs`, `crates/hkask-cli/src/commands/qa.rs`

### Phase 2 — Failed API Call Cost Recovery (Next) ✅ COMPLETE

Recover token costs from failed API calls. The API may charge for input tokens even when the request fails.

**Task 2.1:** Change `classify_one` to return a result type that carries `cost_urj` on error paths. Currently the error path drops the parsed `_tokens`. Need a way to propagate partial cost information through the `Result<ClassifyResult, ServiceError>` boundary.

*Option A:* Add `cost_urj` to `ServiceError` variants used in classify.
*Option B:* Change classify_batch's error handling to attempt token extraction from error responses and add a separate `failed_api_cost_urj` accumulator.
*Option C:* Return `Result<ClassifyResult, (ServiceError, u64)>` where the u64 is cost_urj from error parsing.

**Task 2.2:** Add `failed_api_cost_urj: u64` to `CostTracker` and `CostSummary`. This is reported separately from successful API costs for transparency.

**Files:** `crates/hkask-services-classify/src/classify_impl.rs`, `crates/hkask-test-harness/src/qa_script.rs`, `crates/hkask-cli/src/commands/qa.rs`

### Phase 3 — Per-Step Cost Breakdown (Quality of Life) ✅ COMPLETE

**Task 3.1:** Add `cost: StepCost` to `StepResult` in `qa_script.rs`:

```rust
pub struct StepCost {
    pub gas_urj: u64,         // gas charged for this step
    pub api_token_urj: u64,   // API cost from this step's classify call
}
```

**Task 3.2:** Update the CLI output to show per-step cost:

```
  [1] run_command → success (12ms) | cost: 400 µrJ (gas)
  [2] classify → high_confidence (567ms) | cost: 430 µrJ (400 gas + 30 API)
  [3] run_command → success (8ms) | cost: 400 µrJ (gas)
```

**Files:** `crates/hkask-test-harness/src/qa_script.rs`, `crates/hkask-cli/src/commands/qa.rs`

### Phase 4 — Training Cost Integration (Dependency)

Integrate training MCP server with rJoule cost tracking.

**Task 4.1:** Add `training_total_cost_urj` field to training job results. The training server knows the provider (Together, Runpod, Baseten) and the job parameters — it can compute cost from published pricing.

**Task 4.2:** When `kask qa run` triggers training (via shell command that calls the training MCP), the cost flows into `CostTracker::training_urj`. This requires the training server to report cost in a parseable format.

**Task 4.3:** Add CNS spans for training cost: `cns.qa.cost.training_job` with `job_id`, `provider`, `cost_urj`.

**Files:** `mcp-servers/hkask-mcp-training/src/lib.rs`, `crates/hkask-cli/src/commands/qa.rs`

### Phase 5 — Shell Command Time-Based Gas (Accuracy)

**Task 5.1:** Add optional `gas_multiplier` field to `QaScriptStep`. When set, the step's gas charge is multiplied. Example:

```yaml
steps:
  - ordinal: 1
    action: run_command
    command: "cargo bolero test --timeout 300s"
    gas_multiplier: 10    # 10× gas for long-running command
```

**Task 5.2:** Alternative: track wall-clock time and apply a time-based gas formula. `gas_charge = max(gas_per_function, elapsed_seconds × 2)`. This would auto-scale gas for long-running commands without manual config.

**Task 5.3:** Track per-step elapsed time and report it alongside cost in the step breakdown.

**Files:** `crates/hkask-test-harness/src/qa_script.rs`

### Phase 6 — Provider Pricing Auto-Detection (Convenience)

**Task 6.1:** Create a provider pricing table in `hkask-services-classify`:

```rust
const PROVIDER_PRICING: &[(&str, u64, u64)] = &[
    ("deepinfra", 30, 60),    // $0.03/M in, $0.06/M out → 30/60 nJ/token
    ("together", 20, 20),     // example
    ("openrouter", 50, 50),   // example
    ("fal", 40, 40),          // example
];
```

**Task 6.2:** When `classify_batch` loads a classifier config with `cost_input_nj_per_token == 0`, auto-derive from the `provider` field if it matches a known entry. Log a warning if the provider is unknown and cost tracking is requested.

**Files:** `crates/hkask-services-classify/src/classify_impl.rs`

---

## Priority Order

| Phase | Priority | Effort | Value |
|-------|----------|--------|-------|
| Phase 1 — CNS gaps | **P0** | Small (4 CNS spans) | Closes spec/code gap, enables observability |
| Phase 2 — Failed API costs | **P1** | Medium (error path refactor) | Recovers lost cost data |
| Phase 3 — Per-step breakdown | **P1** | Small (add field + display) | Debuggability, user visibility |
| Phase 4 — Training costs | **P2** | Large (cross-crate integration) | Closes training cost gap |
| Phase 5 — Time-based gas | **P3** | Medium (time tracking) | Accuracy for long commands |
| Phase 6 — Auto-pricing | **P3** | Small (lookup table) | Convenience |

---

## Files Summary

| File | Phases Touched |
|------|---------------|
| `crates/hkask-test-harness/src/qa_script.rs` | 1, 2, 3, 5 |
| `crates/hkask-cli/src/commands/qa.rs` | 1, 2, 3, 4 |
| `crates/hkask-services-classify/src/classify_impl.rs` | 2, 6 |
| `mcp-servers/hkask-mcp-training/src/lib.rs` | 4 |
| `docs/architecture/specs/rjoule-cost-system.md` | All (keep in sync) |
