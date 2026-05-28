# hKask Code Graph Redundancy Analysis

**Generated:** 2026-05-27 | **Source version:** v0.21.0 | **Method:** RDF graph analysis + source cross-reference

---

## Executive Summary

The RDF code graph analysis identified **15 concrete redundancy eliminations** across the hKask codebase. These fall into four categories:

| Category | Count | Impact |
|---|---|---|
| **Dead Code (zero consumers)** | 8 | Removes ~800 lines of dead logic |
| **Duplicate Types** | 3 | Unifies duplicate enum/struct definitions |
| **Orphan Files** | 4 | Removes un-declared, un-compilable source files |
| **Conditional (untested wiring)** | 4 | Flags ready-but-unused implementations for decision |

**Total estimated LOC reduction: ~1,500 lines** without losing any functionality.

---

## 1. Dead Code — Immediate Elimination (8 items)

These types have **zero external consumers** and can be deleted immediately.

### 1.1 `hkask-types/src/cns.rs` — 6 dead types

| Type | Lines | Why Dead |
|---|---|---|
| `KillZoneState` | ~45 | Duplicated by `KillZoneDetector` in sovereignty.rs (3 active consumers) |
| `CnsEvent` (struct) | ~50 | `NuEvent` is the canonical event type (20+ consumers); `CnsEvent` has 0 |
| `TokenBucket` | ~40 | Duplicated by `CnsTokenBucket` in hkask-cns (4 active consumers) |
| `RetryConfig` | ~55 | Dead but **better than** `OkapiRetryConfig` — see §2.3 |
| `ObservabilityPort` (trait) | ~15 | Never implemented anywhere |
| `HealthStatus` (enum) | ~10 | Only referenced by dead `ObservabilityPort` |
| `ObservabilityError` (enum) | ~10 | Only referenced by dead `ObservabilityPort` |

**Action:** Delete `KillZoneState`, `CnsEvent`, `TokenBucket`, `ObservabilityPort`, `HealthStatus`, `ObservabilityError`. For `RetryConfig`: migrate `OkapiRetryConfig` consumers to it (see §2.3).

### 1.2 `hkask-cns/src/review_queue.rs` — `ReviewQueue`, `Violation`

- **68 lines**, zero real consumers
- Only referenced in a log string in `cli/bootstrap.rs`; never instantiated
- Curator review is handled via metacognition, not this queue

**Action:** Delete `review_queue.rs`, remove `pub mod review_queue` and re-exports from `hkask-cns/src/lib.rs`.

### 1.3 `hkask-cns/src/energy.rs` — 6 dead types/functions

The following have **zero external consumers** (only `EnergyBudget` and `EnergyAccount` are used):

| Type | Consumers |
|---|---|
| `EnergyEmitter` | 0 |
| `OpportunityCost` | 0 (only used internally by `EnergyEmitter`) |
| `EnergySpanType` | 0 |
| `EnergyError` | 0 |
| `calculate_energy_cost` | 0 |
| `estimate_tokens` (CNS) | 0 (consumers use `hkask_types::estimate_tokens`) |

**Action:** Remove these 6 exports from `energy.rs`. Keep `EnergyBudget` and `EnergyAccount` (both have active consumers).

---

## 2. Duplicate Types — Unify (3 items)

### 2.1 `AgentType` ⇨ Eliminate in favor of `AgentKind`

| | `AgentKind` (types/agent_def.rs) | `AgentType` (agents/pod/types.rs) |
|---|---|---|
| Variants | `Bot`, `Replicant` | `Bot`, `Replicant` |
| Methods | `as_str()`, `parse()`, `Display` | `Display` only |
| Derives | +`Hash` | No `Hash` |
| External consumers | 5 files | 0 external (internal to hkask-agents only) |

Both are identical two-variant enums. `AgentKind` is strictly richer. `AgentType` adds zero value.

**Action:** Replace all `AgentType` usages with `AgentKind` in `hkask-agents` (4 files: `types.rs`, `mod.rs`, `manager.rs`, `lib.rs`), delete the `AgentType` enum.

### 2.2 `Span` and `CnsSpan` — Synchronize categories

| Status | `Span` (event.rs) | `CnsSpan` (cns.rs) |
|---|---|---|
| **Both have:** | Prompt, Tool, AgentPod, Connector, Sovereignty, Goal, Spec | ← same |
| **Span-only:** | Pipeline, Energy, Review | — |
| **CnsSpan-only:** | — | Template, Curation, Variety, KillZone |

Both use identical `cns.*` prefix convention. Both serve the same namespace at different granularity (Span carries sub-paths; CnsSpan is unit-variant coarse categories). They're **out of sync** — 7 categories overlap, 7 are unique to one or the other.

**Action:** Add missing `Pipeline`, `Energy`, `Review` variants to `CnsSpan`. Add missing `Template`, `Curation`, `Variety`, `KillZone` to `Span`. This eliminates the divergence without removing either type (they serve different granularity roles).

### 2.3 `RetryConfig` ⇨ Migrate `OkapiRetryConfig` consumers to it

`RetryConfig` (cns.rs, 0 consumers) has **serde support, builder methods, configurable multiplier**. `OkapiRetryConfig` (templates/okapi_config.rs, 7 consumers) is simpler but **hardcodes the multiplier to 2.0**. This is the rare case where the dead type is the better one.

**Action:**
1. Add `is_retryable_status()` method to `RetryConfig` (from `OkapiRetryConfig`)
2. Make `RetryConfig::delay_for_attempt()` return `std::time::Duration`
3. Fix the `(multiplier as u64).pow()` bug → use `f64::powf`
4. Replace all `OkapiRetryConfig` usages with `RetryConfig`
5. Delete `OkapiRetryConfig`

---

## 3. Orphan Files — Delete (4 items)

These files exist on disk but are **not declared as modules** in their crate's `lib.rs`. They cannot be compiled and represent dead code.

| File | Notes |
|---|---|
| `crates/hkask-templates/src/rate_limiter.rs` | Duplicate of hkask-cns rate limiter; not declared |
| `crates/hkask-templates/src/russell_mapper.rs` | Duplicate of CLI's russell_mapper; not declared |
| `crates/hkask-templates/src/resolver.rs` | Syntactically broken (unwrapped test code); not declared |
| `crates/hkask-templates/src/security.rs` | Not declared in lib.rs |

**Action:** Delete all four files.

---

## 4. Dead Backend — `GitRegistry` (1 item)

`hkask-templates/src/registry_git.rs` has **zero external callers**. Despite being labeled "production" in comments, nothing instantiates it. It wraps `Registry` with provenance metadata but nobody uses the wrapper.

- `Registry` (in-memory): Used as bootstrap seed by MCP server. **KEEP.**
- `SqliteRegistry`: Used as CLI default. **KEEP.**
- `GitRegistry`: Zero callers. **ELIMINATE.**

**Action:** Delete `registry_git.rs` and its re-export from `templates/src/lib.rs`. If Git provenance is needed later, rebuild it when there's a consumer.

---

## 5. Conditional Decisions (4 items)

These are fully implemented but never wired into production code paths. Each requires a **design decision** — keep and wire, or eliminate.

### 5.1 `SovereigntyObserver` (`hkask-cns/src/observers/sovereignty.rs`)

- **~240 lines**, comprehensive API
- Only exercised by unit tests; never wired into `CnsRuntime`
- **Recommendation:** Wire into CNS runtime bootstrap. Sovereignty observation is architecturally fundamental.

### 5.2 `CompositionObserver` (`hkask-cns/src/observers/composition.rs`)

- **~380 lines**, includes calibration prompts
- Only exercised by unit tests
- **Recommendation:** Same as above — wire into CNS runtime.

### 5.3 CSP Executor (`hkask-templates/src/csp.rs`)

- **~300 lines**, full stage isolation engine
- Only exercised by integration tests
- **Recommendation:** If CSP stage isolation is planned, keep. If not, eliminate.

### 5.4 Prompt Cache (`hkask-templates/src/prompt_cache.rs`)

- **~200 lines**, SQLite-backed LRU with TTL
- Not wired into `OkapiInference` pipeline
- **Recommendation:** Wire into inference pipeline (cost saving). If prompt caching is deferred, eliminate.

---

## 6. Not Redundant — Confirmed Keep

These were investigated but found to serve **distinct, non-overlapping purposes**:

| Pair | Why Not Redundant |
|---|---|
| `Goal` vs `GoalSpec` | Goal = runtime state machine with ownership/visibility; GoalSpec = static planning template with recursive decomposition. 3 shared fields out of 9+ each. |
| `CurationRecord` vs `SpecCurationRecord` | CurationRecord curates template invocations; SpecCurationRecord curates specifications. Different domains despite shared OCAP vocabulary. |
| `GoalVerdict` vs `GoalVerification` vs `GoalState` | Form a pipeline: Verifier → Verification(Verdict) → State transition. No duplication. |
| `Criterion` vs `GoalCriterion` | Criterion used in planning-phase GoalSpec; GoalCriterion used in runtime SQL persistence. Different tables, shared concept — could be unified but low priority. |

---

## 7. Summary: What to Delete

### Priority 1 — Dead Code (no consumers, zero risk)

```
Delete types from hkask-types/src/cns.rs:
  - KillZoneState (~45 lines)
  - CnsEvent struct (~50 lines)
  - TokenBucket (~40 lines)
  - ObservabilityPort trait (~15 lines)
  - HealthStatus enum (~10 lines)
  - ObservabilityError enum (~10 lines)

Delete from hkask-cns/src/lib.rs re-exports:
  - ReviewQueue, Violation → delete review_queue.rs (~68 lines)

Delete from hkask-cns/src/energy.rs:
  - EnergyEmitter, OpportunityCost, EnergySpanType, EnergyError
  - calculate_energy_cost, estimate_tokens (~100 lines)
  → Keep: EnergyAccount, EnergyBudget

Delete orphan files:
  - crates/hkask-templates/src/rate_limiter.rs
  - crates/hkask-templates/src/russell_mapper.rs
  - crates/hkask-templates/src/resolver.rs
  - crates/hkask-templates/src/security.rs

Delete dead registry backend:
  - crates/hkask-templates/src/registry_git.rs (~180 lines)
  → Remove re-export from templates/src/lib.rs
```

### Priority 2 — Unify Duplicates (requires migration)

```
Replace AgentType → AgentKind (4 files, ~30 lines of change)
Migrate OkapiRetryConfig consumers → RetryConfig (7 call sites)
Synchronize Span/CnsSpan categories (add 7 missing variants across both enums)
```

### Priority 3 — Conditional (design decision needed)

```
Wire SovereigntyObserver → CnsRuntime OR delete
Wire CompositionObserver → CnsRuntime OR delete
Wire PromptCache → InferencePort OR delete
Wire CspExecutor → TemplateEngine OR delete
```

---

## 8. Code Quality Impact

| Dimension | Improvement |
|---|---|
| **Security** | Fewer code paths = smaller attack surface. Dead OCAP/observability types removed. |
| **Transparency** | Single `AgentKind` → one source of truth. Unified span categories → no divergence. |
| **Performance** | Fewer unused codepaths = smaller binary. ~1,500 lines removed. |
| **Logical Precision** | No dead `CnsEvent` confusing what "the" event type is. No duplicate kill-zone logic. |

---

*Analysis performed via RDF code graph structural analysis + source-level grep/usages cross-reference of the full hkask v0.21.0 codebase.*
