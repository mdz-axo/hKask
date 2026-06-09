---
title: "Test Inventory — Seam Depth & Behavioral Coverage"
version: "1.1.0"
last_updated: 2026-06-08
status: Active
domain: "Cross-cutting"
---

# Test Inventory

Per MDS §12 and `docs/specifications/test-program.md` — seam depth analysis and behavioral coverage for hKask crates.

## Summary

| Crate | Seams | Tests | Coverage | Deepest Seam |
|-------|-------|-------|----------|-------------|
| hkask-mcp-condenser | 6 | 78 | ✅ Deep | algorithms (35 tests) |
| hkask-services | 6 | 138 | ✅ Deep | sovereignty (37 tests) |
| hkask-types | 3 | 0 | ⚠️ Shallow | ID types (no behavioral tests) |
| hkask-storage | 3 | 0 | ⚠️ Shallow | TripleStore (doc-tests only) |
| hkask-memory | 2 | 0 | ⚠️ Shallow | EpisodicMemory (no tests) |
| hkask-cns | 9 | 110 | ✅ Deep | energy (31 tests) |
| hkask-mcp | 3 | 0 | ⚠️ Shallow | Server/McpToolError (no tests) |
| hkask-cli | 4 | 0 | ⚠️ Shallow | REPL/commands (no tests) |
| hkask-api | 3 | 0 | ⚠️ Shallow | HTTP routes (no tests) |
| hkask-agents | 5 | 0 | ⚠️ Shallow | PodManager/ACP (no tests) |
| hkask-keystore | 2 | 0 | ⚠️ Shallow | Keystore (no tests) |
| hkask-templates | 2 | 1 | ⚠️ Shallow | okapi_config (1 doc-test) |

**Totals:** 12 audited crates, 47 seams, 327 tests

---

## hkask-mcp-condenser

**Status:** ✅ Deep · **Tests:** 78 · **LOC:** 2,072

### Module layout (post-audit)

| Module | LOC | Purpose |
|--------|-----|---------|
| `algorithms.rs` | 838 | `compute_budget`, `CondenserAlgorithm` trait, 3 algorithm impls, `AlgorithmRegistry`, `classify_tool` |
| `engine.rs` | 291 | `CondenserEngine` — profile, stats, compress dispatch, classify delegation |
| `inference.rs` | 276 | Pure functions for thread-summary HTTP request construction and response parsing |
| `types.rs` | 287 | `Profile`, `ContextCategory`, `CompressedOutput`, `CondenserStats`, request types |
| `main.rs` | 380 | MCP server wiring only — no domain logic |

### Seams

| Seam | Module | Tests | Key Invariants |
|------|--------|-------|----------------|
| `Profile` | `types` | 5 | Retention percentages match spec; `max_lines` monotonic; round-trips `FromStr`/`Display`; case-insensitive parse; rejects unknown |
| `ContextCategory` + `CondenserStats` | `types` | 4 | Labels are snake_case; round-trips `FromStr`; unknown fallback; stats defaults |
| `classify_tool()` | `algorithms` | 7 | All category substrings classified correctly; unknown fallback; case-insensitive; `_`/`-` separator splitting; first-token-wins; Phase 2 compound names |
| `compute_budget` + `CondenserAlgorithm` impls + `AlgorithmRegistry` | `algorithms` | 28 | Registry selects correct algorithm per category; `rtk_style` head/tail/ellipsis/passthrough; `saliency_rank` error-priority/order; `flashrank` novelty/brevity/relevance; cross-algorithm non-empty/never-expand; budget arithmetic |
| `CondenserEngine` | `engine` | 17 | Default profile normal; zero stats start; auto-classifies; explicit category override; algorithm name; profile tracking; reduction reporting; passthrough; empty input; `set_profile`; cumulative stats; line/byte counts; `classify()` consistency with `compress()` |
| `inference` pure functions | `inference` | 17 | `format_conversation_text` role/content/empty; `extract_summary` valid/missing/empty/whitespace; `approx_token_count`; `build_summarization_prompt` includes query+conversation; `build_chat_request` model/stream/think/messages/options; `build_summary_output` all fields |

### Test breakdown by module

| Module | Tests | Notes |
|--------|-------|-------|
| `algorithms::tests` | 35 | classify_tool (7) + compute_budget (5) + registry (6) + rtk_style (5) + saliency_rank (4) + flashrank (6) + cross-algorithm (2) |
| `inference::tests` | 17 | format_conversation (4) + extract_summary (6) + approx_token (1) + build_summarization_prompt (1) + build_chat_request (4) + build_summary_output (1) |
| `engine::tests` | 17 | compress lifecycle (12) + classify (3) + profile/stats (2) |
| `types::tests` | 9 | Profile (5) + ContextCategory (3) + CondenserStats (1) |

### Algorithm test matrix

| Algorithm | Categories | Tested Behaviors |
|-----------|-----------|-----------------|
| `rtk_style` | ShellCommand, TestOutput, BuildOutput | Reduces lines (heavy); passthrough (light/short); ellipsis marker; always produces output; preserves head+tail |
| `saliency_rank` | ConversationHistory, LogOutput, Unknown | Reduces lines; prioritizes error lines; passthrough; preserves order |
| `flashrank` | FileContents, StructuredData | Reduces lines; passthrough; novelty=1 for empty selected; brevity favors shorter; relevance matches terms; preserves order |

### Architecture decisions recorded

| Decision | Force | Rationale |
|----------|-------|-----------|
| `handles()` removed from `CondenserAlgorithm` trait | Guideline | 100% redundant with `default_for().contains()` — failed depth test |
| `classify_tool` moved from `types.rs` to `algorithms.rs` | Guideline | Classification is an algorithm concern, not a type definition |
| Engine tests moved from `main.rs` to `engine.rs` | Guideline | Locality — tests live with the code they verify |
| `compute_budget` extracted as shared function | Guideline | Was copy-pasted 3× across algorithm impls; single source of truth |
| `CondenserEngine.classify()` added | Guideline | Unifies classify→select path; `main.rs` no longer calls `classify_tool` directly |
| `inference::build_chat_request()` extracted | Guideline | Chat request construction is now a pure, testable function |
| `Arc<AlgorithmRegistry>` split **deferred** | Guideline (relaxed) | Deletion test failed — adds complexity for contention that doesn't exist at single-agent scale |

---

## hkask-cns

**Status:** ✅ Deep · **Tests:** 110 · **LOC:** ~3,060 (test-bearing modules)

### Seams

| Seam | Module | Tests | Key Invariants |
|------|--------|-------|----------------|
| `GasCost` + `EnergyBudget` | `energy` | 31 | `ZERO`=0; from_raw/as_raw round-trips; `From<u64>`/`Into<u64>`; Display; Ord; cap=remaining on new; replenish_rate=cap/10; hard_limit default; `can_proceed` hard/soft/reserved; consume deducts/fails; reserve+settle hold-settle; settle refund/extra; replenish cap/weighted/min-1; usage_ratio; available saturating_sub |
| `CircuitBreaker` | `circuit_breaker` | 9 | Starts Closed; failures→Open at threshold; Open→HalfOpen after timeout; HalfOpen→Closed after successes; HalfOpen→Open on failure; success resets count; `default_for_inference` |
| `TableGasEstimator` | `table_gas_estimator` | 6 | Known servers; unknown default=10; per-tool overrides server; inference=0; tier ordering; `Default`=`New` |
| `CompositeGasEstimator` | `composite_gas_estimator` | 7 | Routes inference→token; routes others→table; `InferenceGasEstimator` prompt+max_tokens; default max=100; minimum cost=1; empty args; `Default`=`New` |
| `RuntimeAlert` + `AlgedonicManager` | `algedonic` | 18 | Critical at deficit>threshold; Warning at deficit>threshold/2; Info below; escalated only Critical; message contains domain; severity ordering; manager check produces alert; domain expected variety; critical_alerts filter; total_deficit sums; threshold accessor; allosteric (MWC sigmoid) low/medium/high alpha; `cns_health` healthy/unhealthy |
| `VarietyTracker` + `VarietyMonitor` | `variety` | 13 | Starts empty; increment creates key; same key no increase; different keys increase; deficit saturating_sub; surplus→0 deficit; reset; Monitor independent domains; `domains()` list; default matches new; variety_for untracked=0 |
| `Dampener` | `dampener` | 8 | First not dampened; same within window dampened; different target not dampened; same variant+target dampened; override cooldown suppresses ALL overrides within window; non-metacognitive not subject to cooldown; custom window; window expiry allows refire |
| `SetPoints` | `set_points` | 5 | Defaults match constants; empty config→defaults; partial config overrides; YAML parse; invalid YAML fails |
| `EnergyBudgetManager` | `energy_budget_management` | 13 | register+can_proceed; no budget=soft limit; hold-settle; no budget reserve/settle=ZERO; replenish skips overrides; clear override resumes; status None when unregistered; acquire; replenish by directive; energy_ratios; Default=New; `expire_overrides` removes expired; `agent_gas_status` none when unregistered |

### Bug Found and Fixed

**CircuitBreaker `record_failure` timestamp bug:** `now.duration_since(Instant::now())` always yielded 0, meaning `last_failure_time` was always 0. The Open→HalfOpen transition never happened — once the circuit opened, it stayed open forever. Fixed by adding `created_at: Instant` field and storing `now.duration_since(created_at).as_nanos()`, then reconstructing the failure instant in `allow_request()`.

---

## hkask-services

**Status:** ✅ Deep · **Tests:** 138 · **LOC:** ~2,500

### Seams

| Seam | Tests | Key Invariants |
|------|-------|----------------|
| `AgentService` | ~15 | CRUD lifecycle; unregister unknown returns error |
| `ArchivalService` | 4 | 4 public operations; result carries path+sha; service error mapping; default registry path |
| `ChatService` | 4 | TokenUsage gas_cost; model switch; persona; conversation |
| `ComposeService` | 12 | CognitionConfig YAML; default retrieval; cosine distance (identical/opposite/orthogonal/mismatched); centroid validation; system prompt |
| `ServiceConfig` | 7 | effective_memory_db_path; defaults; credential resolution |
| `SovereigntyService` | 37 | P1–P4 compliance; JSON output; principle filtering |

---

## Gaps & Debt

### Shallow crates (0 behavioral tests)

These crates have public seams but no `#[test]` blocks verifying behavioral properties. Per P8, every public seam should have at least one test verifying a stated invariant.

| Crate | Key Untested Seams | Priority |
|-------|-------------------|----------|
| `hkask-types` | `WebID`, `McpErrorKind`, `R7` bot identities | Medium |
| `hkask-storage` | `TripleStore`, `Database::open()`, SQLCipher | High |
| `hkask-memory` | `EpisodicMemory`, `SemanticMemory`, consolidation bridge | High |
| `hkask-mcp` | `McpToolError`, `ToolSpanGuard`, `CredentialRequirement` | Medium |
| `hkask-cli` | `BootstrapSequence`, REPL commands | Low |
| `hkask-api` | HTTP route handlers | Low |
| `hkask-agents` | `PodManager`, `AcpRuntime`, `PodContext` | Medium |
| `hkask-keystore` | `Keystore` encrypt/decrypt/rotate | Medium |

### MCP server test coverage

All MCP servers except the condenser have zero unit tests. The MCP tool surface is tested indirectly through integration, but no server has direct behavioral tests for its tool implementations.

| Server | Tests | Priority for Test Addition |
|--------|-------|--------------------------|
| condenser | 78 | ✅ Done |
| inference | 0 | High (generate, failover) |
| cns | 0 | High (variety, algedonic, gas) |
| ocap | 0 | High (create/verify/revoke cycle) |
| episodic | 0 | Medium (store/recall) |
| semantic | 0 | Medium (embed/search) |
| All others | 0 | Low |

### Open refactor debt (condenser)

| Item | Force | Status |
|------|-------|--------|
| `Arc<AlgorithmRegistry>` split for lock-free `condenser_classify` | Guideline (relaxed) | Deferred — no contention at single-agent scale. Revisit if concurrent multi-agent load or user-registered algorithms are added. |
| Full `InferenceClient` trait + stub for `condenser_thread_summary` end-to-end tests | Guideline | Deferred — `build_chat_request` already makes request construction testable; HTTP wiring remains dark. Activate when modifying the thread-summary flow or switching inference backends. |

---

## Methodology

- **Seam identification:** All `pub` traits, `pub` structs with `pub` methods, `pub` functions
- **Depth assessment:** Deep = few interface methods, many behaviors tested; Shallow = interface as complex as implementation
- **Coverage:** ✅ = behavioral tests exist for all key seams; ⚠️ = gaps present
- **Priority:** High = security/data-integrity seams; Medium = core domain seams; Low = surface/presentation seams
