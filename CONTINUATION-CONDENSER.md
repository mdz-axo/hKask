# Continuation Prompt — hKask MCP Condenser + CNS Test Audit

**Session:** 27 (2026-06-08)
**Status:** Two-subject session — CNS behavioral tests (complete), condenser three-skill audit (partially complete)

---

## What Was Done

### 1. hkask-cns: 110 behavioral tests added (was 0), 1 bug fixed

**Bug fixed:** `CircuitBreaker::record_failure()` stored `now.duration_since(Instant::now())` which always yielded 0. The Open→HalfOpen transition was completely broken — once a circuit opened, it stayed open forever. Fixed by adding `created_at: Instant` field and storing nanoseconds-since-creation.

**Test modules added:**

| Module | Tests | Key Invariants |
|--------|-------|----------------|
| `energy::tests` | 30 | GasCost newtype; GasBudget creation/consume/reserve+settle/replenish/usage_ratio/available |
| `circuit_breaker::tests` | 10 | State machine transitions (Closed→Open→HalfOpen→Closed) |
| `table_gas_estimator::tests` | 6 | Known servers; unknown default=10; per-tool overrides; tier ordering |
| `composite_gas_estimator::tests` | 8 | Routing inference→token/others→table; InferenceGasEstimator edge cases |
| `algedonic::tests` | 17 | Binary threshold alerts; allosteric (MWC) alerts; AlgedonicManager; cns_health_check |
| `variety::tests` | 14 | VarietyTracker increment/variety/deficit/reset; VarietyMonitor independent domains |
| `dampener::tests` | 8 | Per-fingerprint dedup; override cooldown; window expiry |
| `set_points::tests` | 5 | Defaults match constants; config from YAML; partial overrides |
| `gas_budget_management::tests` | 12 | Register/reserve/settle; soft limit; override lifecycle; replenish skip; expire_overrides |

**Verification:** `cargo test -p hkask-cns` → 110 passed; `cargo clippy -p hkask-cns -- -D warnings` → clean

### 2. hkask-mcp-condenser: Three-skill audit + 3 fixes applied

**Three skills applied:** coding-guidelines, improve-codebase-architecture, grill-me

#### Fixes Applied

| # | Severity | Fix | Files |
|---|----------|-----|-------|
| G1 | **flag** | Invalid `category` now returns `McpToolError::invalid_argument` instead of silently falling back | `main.rs:118-128` |
| A1 | **strong** | Extracted `compute_budget()` — was copy-pasted 3× across all algorithms | `algorithms.rs:5-16` |
| T1 | **test** | Added Phase 2 `classify_tool` test (was untested code path) | `types.rs:228-236` |
| T2 | **test** | Added 5 `compute_budget` tests | `algorithms.rs:738-775` |

**Verification:** `cargo test -p hkask-mcp-condenser` → 72 passed; `cargo clippy` → clean

#### Remaining Audit Findings (not yet fixed)

##### Flags (action recommended)

| # | File | Finding | Risk |
|---|------|---------|------|
| G2 | `types.rs:285-331` | `classify_tool` Phase 2 false positives: `"logistics"` → `LogOutput` (contains `"log"`), `"margit"` → `ShellCommand` (contains `"git"`). Tradeoff undocumented. | False classification of unrelated tools |
| G3 | `engine.rs:10-12` | `pub` fields on `CondenserEngine` are test-driven visibility leak | Encapsulation break |
| G4 | `engine.rs:11-12,73-76` | Dual source of truth: `self.profile` + `self.stats.current_profile` | Divergence risk |
| G5 | `algorithms.rs:10,29-36` | `handles()` is 100% redundant with `default_for().contains()` — same `matches!` on same categories | Maintenance burden |

##### Architecture (strong/worth-exploring)

| # | Strength | Finding |
|---|----------|---------|
| A2 | **Strong** | `classify_tool` lives in `types.rs` (data module) not with algorithm consumers. Move to `algorithms.rs`. |
| A3 | **Strong** | Engine tests (12) live in `main.rs` instead of `engine.rs`. Move for locality. |
| A4 | **Worth exploring** | `condenser_classify` locks engine for read-only registry access. Make registry immutable/shared. |
| A5 | **Worth exploring** | Thread summary HTTP call has no testable seam. Introduce `InferenceClient` trait. |
| A6 | **Worth exploring** | `CondenserEngine` is borderline pass-through. Deepen by owning classify→select→compress→stats. |
| A7 | **Speculative** | `classify_tool` Phase 2 effectively unreachable for real tool names. Test or remove. |
| A8 | **Speculative** | `ContextCategory::label()` duplicates `serde(rename_all)`. Use serde's rename string. |

##### Warnings (lower priority)

| # | File | Finding |
|---|------|---------|
| W1 | `types.rs:36,241` | `Serialize`/`Deserialize` on `Profile`/`ContextCategory` unused — no caller deserializes them from JSON |
| W2 | `types.rs:334,347` | `Clone`/`Deserialize` on `CompressedOutput`/`CondenserStats` unused |
| W3 | `types.rs:269-282` | `ContextCategory::FromStr` declares `Err = McpToolError` but never returns `Err` (unknown falls through) |
| W4 | `algorithms.rs:64-76` | RtkStyle second truncation path is dead code (head+tail+1 can't exceed budget) |
| W5 | `algorithms.rs:295` | `selected_indices.contains(&i)` is O(n) Vec scan — should be HashSet |
| W6 | `inference.rs:22` | `format_conversation_text` silently loses non-string `content` (multimodal messages) |
| W7 | `inference.rs:48-66` | `extract_summary` returns `(McpErrorKind, McpToolError)` tuple — leaky abstraction |
| W8 | `inference.rs:69` | `approx_token_count` is `pub` but only used within the module |
| W9 | `main.rs:319-330` | Legacy `OKAPI_*` env var aliases double the config surface |
| W10 | `main.rs:257-275` | Hardcoded system prompt and `num_ctx: 8192` in thread_summary handler |

---

## Build State

| Crate | `cargo check` | `cargo test` | `cargo clippy` |
|-------|---------------|-------------|----------------|
| hkask-cns | ✅ | ✅ 110 passed | ✅ clean |
| hkask-mcp-condenser | ✅ | ✅ 72 passed | ✅ clean |
| hkask-templates | ❌ pre-existing (uncommitted `InferenceStreamChunk` stub) | — | — |
| Full workspace | ❌ (blocked by hkask-templates) | — | — |

**Note:** `crates/hkask-templates/src/inference_port.rs` has uncommitted stubs referencing `InferenceStreamChunk` and `generate_stream_with_model` that don't compile. This is from a previous session and is NOT related to current work. Revert with `git checkout crates/hkask-templates/src/inference_port.rs` to restore clean workspace build.

---

## Recommended Next Actions (Priority Order)

### HIGH — Close audit findings that are bugs or divergence risks

1. **G4: Remove dual source of truth in `CondenserEngine`** — Make `profile` `pub(crate)` with accessor; compute `stats.current_profile` from `self.profile.to_string()` in `set_profile()` and `compress()`. This prevents divergence if future code mutates `self.profile` directly.

2. **G5: Remove `handles()` from `CondenserAlgorithm` trait** — It's 100% redundant with `default_for().contains()`. Simplifies `AlgorithmRegistry::select()` to single-pass. Removes `handles_consistent_with_default_for` test (tautology). All three impls lose ~8 lines.

3. **G2: Document `classify_tool` Phase 2 tradeoff** — Add a comment block explaining that Phase 2 trades precision for recall, with examples of known false positives. Consider adding word-boundary matching to reduce false positives.

### MEDIUM — Locality and architecture improvements

4. **A2: Move `classify_tool` to `algorithms.rs`** — Classification is an algorithm concern, not a type definition. Zero API change since it's crate-internal.

5. **A3: Move engine tests from `main.rs` to `engine.rs`** — 12 tests that only use `CondenserEngine`, `Profile`, `ContextCategory`. Pure locality.

6. **G3: Make engine fields `pub(crate)`** — Add `profile()` accessor. Update test assertions to use public API instead of reaching into fields.

### LOW — Polish and documentation

7. **W4: Remove dead re-truncation in `RtkStyleAlgorithm::compress`** (L64-76)
8. **W5: Replace `Vec<usize>` with `HashSet<usize>` in `FlashrankAlgorithm`** for O(1) contains
9. **W7: Simplify `extract_summary` return type** — return just `McpToolError` (it carries its own kind)
10. **W10: Extract hardcoded system prompt and `num_ctx` into constants or config**

### DEFERRED — Requires larger refactor

11. **A5: Introduce `InferenceClient` trait** for testable thread summary seam
12. **A4: Make `AlgorithmRegistry` shared/immutable** to remove lock in `condenser_classify`
13. **A6: Deepen `CondenserEngine`** by owning the full classify→select→compress→stats pipeline

---

## Key Decisions to Preserve

1. **`classify_tool` two-phase:** Token-split exact match (Phase 1) → substring heuristic fallback (Phase 2). First token wins. Phase 2 has known false positives — this is a documented tradeoff, not a bug.

2. **`ThreadSummaryRequest.messages` is `Vec<Value>`, not `String`:** Eliminated JSON-in-string anti-pattern in prior session.

3. **`target_lines` uses `.round()`:** Prevents truncation of retention percentages near 1.0 (e.g., Light profile 0.95 on 2-line input).

4. **`compute_budget()` extracted:** Single source of truth for budget semantics across all three algorithms. Returns `(budget, is_passthrough)` tuple.

5. **Invalid `category` now errors:** Changed from silent fallback to explicit `McpToolError::invalid_argument`. Users get feedback when their category string is unrecognized.

6. **CircuitBreaker `created_at` field:** Stores `Instant` for reconstructing failure timestamps. Nanos-since-creation encoding with u64 (584-year overflow horizon).

7. **Allosteric alert path tested:** `AlgedonicManager::with_default_allosteric()` is the production default (CnsState::new uses it). Tests now cover both binary and MWC sigmoid severity paths.

8. **`condenser_persist` uses `PermissionDenied` for unconfigured persistence:** This is arguably `InvalidArgument` or a custom "not configured" kind — flagged but not fixed.

---

## Files Changed This Session

| File | Change |
|------|--------|
| `crates/hkask-cns/src/energy.rs` | +30 tests |
| `crates/hkask-cns/src/circuit_breaker.rs` | Bug fix (`created_at` field) + 10 tests |
| `crates/hkask-cns/src/table_gas_estimator.rs` | +6 tests |
| `crates/hkask-cns/src/composite_gas_estimator.rs` | +8 tests |
| `crates/hkask-cns/src/algedonic.rs` | +17 tests (binary + allosteric + cns_health) |
| `crates/hkask-cns/src/variety.rs` | +14 tests |
| `crates/hkask-cns/src/dampener.rs` | +8 tests (incl window expiry) |
| `crates/hkask-cns/src/set_points.rs` | +5 tests |
| `crates/hkask-cns/src/gas_budget_management.rs` | +12 tests (incl expire_overrides) |
| `mcp-servers/hkask-mcp-condenser/src/main.rs` | Fix: invalid category now errors |
| `mcp-servers/hkask-mcp-condenser/src/algorithms.rs` | Extract `compute_budget()` + 5 tests |
| `mcp-servers/hkask-mcp-condenser/src/types.rs` | +1 Phase 2 classify test |
| `docs/status/test-inventory.md` | Updated CNS row + detailed section, condenser count |

---

*Session 27 — hKask CNS + Condenser audit — v0.23.0*