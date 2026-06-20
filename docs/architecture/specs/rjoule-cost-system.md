# rJoule Dual-Track Cost System — Specification

**Version:** v0.30.0  
**Status:** Approved — pending implementation  
**Last updated:** 2026-06-20  
**Reference:** [Software Carbon Intensity Specification](https://sci.greensoftware.foundation/) — [SCI GitHub](https://github.com/Green-Software-Foundation/sci)

---

## 1. Overview

The hKask QA system tracks costs in a unified energy unit: the **rJoule (rJ)**. rJ is designed as a future carbon-backed stablecoin with a fixed 1 rJ = 1 USD peg. It merges two distinct cost tracks:

- **Track 1 — Gas:** Estimates the carbon shadow price of local software processing for things we *don't* pay cash for (CPU cycles, shell commands, orchestration). Uses SCI methodology as the reference model.

- **Track 2 — API & Service Costs:** Tracks direct economic costs for things we *do* pay cash for (LLM token usage, training jobs, monthly subscriptions). Converted to rJ at the 1:1 USD peg.

The peg is a deliberate v1 simplification. Future versions will make the gas/rJ rate adjustable and introduce an rJ token for crypto network integration.

## 2. Unit System

```
1 rJ (rJoule) = 1 USD                    ← primary unit, pegged to US dollar
1 rJ = 250,000 gas                       ← gas is a micro-subunit of rJoules
1 gas = 0.000004 rJ = $0.000004 USD      ← 4 micro-dollars
1 µrJ (micro-rJ) = 0.000001 rJ           ← internal integer representation
```

### Internal Representation

All rJoule accounting uses integer **micro-rJ (µrJ)** internally to avoid floating-point rounding and remain transferable for v2 tokenization:

```
1 µrJ = 0.000001 rJ = $0.000001 USD
1 gas = 4 µrJ                            (250,000 gas = 1,000,000 µrJ = 1 rJ)
```

### Why 250,000 gas = 1 rJ?

Scales gas to be a micro-unit appropriate for per-function accounting while keeping rJ human-readable. At 100 gas per function call, a 15,000 gas cap = 150 function calls = 0.06 rJ = $0.06. The conversion factor was set at 500,000 in the initial draft but revised to 250,000 after analysis suggested 0.02 kWh per function is more accurate than 0.01 kWh (see §14 for discussion).

## 3. Track 1: Gas — Software Carbon Shadow Price

Gas estimates the energy cost of local software processing using the SCI methodology as a reference model. This is for costs we don't directly pay — the carbon shadow price of compute.

### Derivation

```
SCI reference values (Glass et al. 2021, SCI v1.0, IWG 2023):
  E = 0.02 kWh per software function call (adjusted for infrastructure overhead)
  I = 400 gCO₂e/kWh (global grid average, IEA 2022)
  Carbon price = $50/tonne CO₂e (IWG 2023 central estimate, 3% discount rate)

One software function call:
  Carbon = 0.02 kWh × 0.0004 tonnes/kWh = 0.000008 tonnes CO₂e = 8 g CO₂e
  Cost   = 0.000008 tonnes × $50/tonne  = $0.0004
  In rJ  = 0.0004 rJ
  In µrJ = 400 µrJ

Converting to gas (250,000 gas/rJ):
  0.0004 rJ × 250,000 gas/rJ = 100 gas per function call
  400 µrJ = 100 gas → 1 gas = 4 µrJ
```

### Known Limitation: Embodied Emissions

The SCI specification includes embodied emissions `M` (hardware manufacturing) in the full formula `SCI = (E × I + M) / R`. This specification models operational emissions only (E × I). Embodied emissions for typical server hardware contribute roughly +2% to the total. This is excluded pending hardware lifecycle data; the estimate is slight undercount, not a structural gap.

### Per-Step Gas Accounting

| Step Action | Gas Charged | µrJ Equivalent |
|-------------|------------|---------------|
| `run_command` | +100 gas | +400 µrJ |
| `classify` | +100 gas | +400 µrJ |
| `loop` (shell) | +100 gas × iterations | +400 µrJ × iterations |
| `loop` (classify) | +100 gas × iterations | +400 µrJ × iterations |

Gas is always 100 per function call. Verification: every `run_command`, `classify`, and `loop` iteration must increment the gas counter. The `QaScriptReport` includes a `gas_steps_count` field for audit — the expected gas total must equal `step_count × 100`.

## 4. Track 2: API & Service Costs

Direct economic costs for external services, converted to rJ at the 1:1 USD peg.

### 4.1 Per-Token API Costs (LLM Inference)

```
DeepInfra Gemma 4 26B pricing (June 2026):
  Input tokens:  $0.03 per million → $0.00000003 per token → 0.03 µrJ per token
  Output tokens: $0.06 per million → $0.00000006 per token → 0.06 µrJ per token

One classify call (~400 input, ~300 output):
  API µrJ = 400 × 0.03 + 300 × 0.06 = 12 + 18 = 30 µrJ  (0.000030 rJ)
```

**Failed API calls:** When an API call fails (network error, timeout, rate limit), the provider may still charge for input tokens. The `classify_one` function must extract `usage` from error response bodies as well as success responses. Failed calls charge at least `prompt_tokens × cost_per_input_token` in µrJ.

### 4.2 Training Endpoint Costs

```
Future category. For now: training_rjoules accumulator in CostTracker.
Expected to be per-job pricing (e.g., $2.00/training run = 2.00 rJ = 2,000,000 µrJ).
```

### 4.3 Monthly Subscription Costs

Recurring service subscriptions (FMP, EODHD, web services) are reported as a separate line item, not amortized per-call in real-time. The `CostSummary` includes a `subscriptions_monthly_rj` field for the user to track alongside per-run costs.

## 5. Merged Cost Per Operation

```
Classify call (DeepInfra Gemma 4 26B):
  Gas (software):    100 gas × 4 µrJ/gas   = 400 µrJ
  API (DeepInfra):   ~700 tokens            =  30 µrJ
                                              ───────
                                  Total      = 430 µrJ  (0.000430 rJ = $0.00043)

Gas dominates: the carbon shadow price of local processing is ~13× the API token cost.
```

## 6. Cost Tracker Design

```rust
/// Tracks all costs in micro-rJoules (µrJ) — integer for transferability.
/// 1 µrJ = 0.000001 rJ = $0.000001 USD.
struct CostTracker {
    gas_used: u64,              // software processing gas (×2 = µrJ)
    api_token_urj: u64,         // per-token API costs in µrJ (classify, embed)
    training_urj: u64,          // per-training-job costs in µrJ
    subscription_monthly_urj: u64, // monthly recurring costs in µrJ (not per-run)
}

impl CostTracker {
    /// Total µrJ consumed this run.
    fn total_urj(&self) -> u64 {
        (self.gas_used * 2) + self.api_token_urj + self.training_urj
        // subscription_monthly_urj is NOT included — it's recurring, not per-run
    }

    /// rJoule cap from gas budget.
    fn rjoule_cap_urj(&self, gas_cap: u64) -> u64 {
        gas_cap * 2  // each gas = 2 µrJ
    }
}
```

### Verification Invariant

After every script run, the following must hold:

```
expected_gas = step_count × 100   (each step is one function call)
actual_gas   = cost_tracker.gas_used
assert expected_gas == actual_gas  // CNS alert on mismatch
```

For classify steps, `api_token_urj` must increment by the actual token count × per-token cost. The report includes a line-item breakdown so the user can verify: N classify calls × ~30 µrJ each ≈ actual api_token_urj total.

## 7. GasConfig Changes

```rust
pub struct GasConfig {
    pub cap: u64,                           // gas units (software carbon budget)
    pub gas_per_function: u64,              // gas per software function call (default: 100)
    pub alert_threshold: f64,               // fraction of cap for warning
    pub hard_limit: bool,                   // abort when µrJ total exceeds cap equivalent
    pub monthly_subscriptions_urj: u64,     // monthly recurring costs in µrJ (informational)
}
```

Defaults:

| Field | Default | Notes |
|-------|---------|-------|
| `cap` | 15,000 gas | Existing default (30,000 µrJ = $0.03) |
| `gas_per_function` | 100 gas | Fixed per v1 SCI derivation |
| `alert_threshold` | 0.7 | Warn at 70% of rJoule budget |
| `hard_limit` | true | Abort when exceeded |
| `monthly_subscriptions_urj` | 0 | Informational; not tracked per-run |

### Design: API Costs Flow from the Classify Service

API pricing is NOT stored in the manifest. It lives in the classifier config (`registry/classify/*.yaml`):

```yaml
classifier:
  name: qa-triage
  cost_input_nj_per_token: 30     # $0.03/M input → 30 nJ/token
  cost_output_nj_per_token: 60    # $0.06/M output → 60 nJ/token
```

The `classify_batch` function computes `cost_urj` from actual token usage × provider pricing and returns it in `ClassifyResult.cost_urj`. The runner accumulates it directly — no pricing knowledge needed in the manifest.

### Fields Removed

`cost_per_token: f64` — dead field, removed. `api_cost_input_nj_per_token` and `api_cost_output_nj_per_token` — moved to classifier config YAML, not the script manifest.

## 8. Code Impact

### Files Changed

| File | Change |
|------|--------|
| `crates/hkask-services-classify/src/classify_impl.rs` | Add `Usage` to `ChatResponse`; add `prompt_tokens` + `completion_tokens` to `ClassifyResult`; parse from both success and error responses |
| `crates/hkask-test-harness/src/qa_script.rs` | Add `CostTracker` (integer µrJ); replace `cost_per_token` with `gas_per_function`; track gas per step; implement `alert_threshold`; update `QaScriptReport` with `CostSummary`; add verification invariant |
| `crates/hkask-cli/src/commands/qa.rs` | Propagate token counts through classify closure; display CostSummary |
| `hKask/docs/architecture/specs/rjoule-cost-system.md` | This document |

## 9. CLI Output

```
[QA] Script complete: 5 steps executed, terminal outcome: high_confidence
[QA] Cost summary:
       Gas (software):     500 gas                200 µrJ    (0.000200 rJ, ~0.2 gCO₂e)
       API tokens:         2,100 tokens            30 µrJ    (0.000030 rJ, $0.00003)
       ───────────────────────────────────────────────────
       Run total:                                 230 µrJ    (0.000230 rJ, $0.00023)
       Monthly recurring: $20.00 = 20,000,000 µrJ (not included in run total)
[QA] Budget: 230 / 30,000 µrJ (0.8%)
```

## 10. SCI Reference

The gas track uses SCI as a reference model for estimating the carbon shadow price of local software processing. The SCI formula:

```
SCI = (E × I + M) / R

Where:
  E = Energy consumed (kWh)
  I = Carbon intensity (gCO₂e/kWh)
  M = Embodied emissions (excluded in v1 — see §3 limitation)
  R = Functional unit (1 software function call)
```

Gas per function = (E × I × carbon_price × gas_per_rJ) = 0.02 × 0.0004 × 50 × 250,000 = 100 gas.

This is a cost model, not a compliance audit. The SCI provides the reference values for energy estimation. The conversion to dollars via carbon pricing and to rJ via the fixed peg are policy choices, not SCI requirements.

## 11. Verification Coverage

The CostTracker must be auditable. The following invariants are checked:

| Invariant | Check | CNS Alert |
|-----------|-------|-----------|
| Gas per step | `gas_used == step_count × gas_per_function` | `cns.qa.cost.gas_mismatch` |
| API cost per classify | `api_token_urj` increments by token count × per-token cost for each classify call | `cns.qa.cost.api_untracked` |
| No missing steps | Every action type (run_command, classify, loop) increments gas | Covered by `gas_mismatch` invariant |
| Total within cap | `total_urj < gas_cap × 4` when hard_limit is true | `cns.qa.cost.cap_exceeded` |
| Alert threshold | `total_urj / (cap × 4) >= alert_threshold` emits warning | `cns.qa.cost.threshold_warning` |
| Classify has token data | Every classify call returns non-zero `prompt_tokens` + `completion_tokens` (except fallback/no-key mode) | `cns.qa.cost.missing_token_data` |

## 12. Cost Per Operation Summary

| Operation | Gas (µrJ) | API (µrJ) | Total (µrJ) | rJ |
|-----------|-----------|-----------|-------------|-----|
| `run_command` | 400 | 0 | 400 | 0.000400 |
| `classify` (DeepInfra) | 400 | ~30 | ~430 | 0.000430 |
| `loop` shell (×3) | 1,200 | 0 | 1,200 | 0.001200 |
| `loop` classify (×3) | 1,200 | ~90 | ~1,290 | 0.001290 |

## 13. Design: Gas Tracks hKask Internal, API Costs Track External

Gas tracks only hKask-internal software functions: CNS spans, registry operations, YAML parsing, command execution, report generation. API costs track what happens outside hKask: LLM inference tokens, training jobs, external service calls. These are fundamentally different cost categories, merged into rJ for unified accounting but tracked separately for clarity.

## 14. Discussion: 0.01 vs 0.02 kWh per Software Function

### Why 0.02 kWh?

The initial draft used 0.01 kWh per function call, based on Glass et al. (2021) median serverless function measurements. This was revised to 0.02 kWh for two reasons:

1. **Infrastructure overhead.** The Glass et al. measurement captures only the function execution itself. In hKask, each "function call" triggers a cascading set of helper services: CNS span emission, tracing infrastructure, registry lookups, YAML deserialization, error handling, report formatting. These overhead services are not captured in a bare function execution measurement.

2. **Provisioned vs. utilized energy.** The SCI specification requires tracking energy for *provisioned* hardware, not just *utilized* hardware. A machine running hKask QA scripts is provisioned 24/7 — idle power draw (200-300W for a typical workstation) dwarfs the marginal energy of a single function call. Allocating provisioned energy across function calls roughly doubles the per-call estimate.

### Impact on Gas/rJ Conversion

At 0.02 kWh per function:
- 100 gas per function call × 250,000 gas/rJ = 0.0004 rJ = $0.0004 per call
- 15,000 gas cap = 0.06 rJ = 6 cents for 150 function calls
- Gas is ~13× the API cost (was ~7× at 0.01 kWh)

This doubling better reflects the true energy cost of running hKask software functions. The conversion can be recalibrated if energy measurement or grid intensity data improves.

## 15. Open Questions

1. **Provider pricing table**: Should the system auto-derive `api_cost_*_urj_per_token` from the classifier config's `provider` field and a built-in pricing table?
2. **Non-DeepInfra providers**: Together AI, OpenRouter, fal.ai — need pricing data in µrJ/token.
3. **Shell command energy model**: The 100 gas flat rate may over/under-estimate for long-running shell commands. A wall-clock-time × power model could improve accuracy.
4. **Nano-rJ precision**: The `u64` integer representation stores nJ per token for multiplication precision; should this be a canonical unit?
