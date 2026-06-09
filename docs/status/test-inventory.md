---
title: "Test Inventory — Seam Depth & Behavioral Coverage"
version: "1.0.0"
last_updated: 2026-06-08
status: Active
domain: "Cross-cutting"
---

# Test Inventory

Per DDMVSS §12 and `docs/specifications/test-program.md` — seam depth analysis and behavioral coverage for hKask crates.

## Summary

| Crate | Seams | Tests | Coverage | Deepest Seam |
|-------|-------|-------|----------|-------------|
| hkask-mcp-condenser | 5 | 53 | ✅ Deep | algorithms (23 tests) |
| hkask-services | 6 | 138 | ✅ Deep | sovereignty (37 tests) |
| hkask-types | 3 | 0 | ⚠️ Shallow | ID types (no behavioral tests) |
| hkask-storage | 3 | 0 | ⚠️ Shallow | TripleStore (doc-tests only) |
| hkask-memory | 2 | 0 | ⚠️ Shallow | EpisodicMemory (no tests) |
| hkask-cns | 4 | 0 | ⚠️ Shallow | GasEstimator (no tests) |
| hkask-mcp | 3 | 0 | ⚠️ Shallow | Server/McpToolError (no tests) |
| hkask-cli | 4 | 0 | ⚠️ Shallow | REPL/commands (no tests) |
| hkask-api | 3 | 0 | ⚠️ Shallow | HTTP routes (no tests) |
| hkask-agents | 5 | 0 | ⚠️ Shallow | PodManager/ACP (no tests) |
| hkask-keystore | 2 | 0 | ⚠️ Shallow | Keystore (no tests) |
| hkask-templates | 2 | 1 | ⚠️ Shallow | okapi_config (1 doc-test) |

**Totals:** 12 audited crates, 42 seams, 192 tests

---

## hkask-mcp-condenser

**Status:** ✅ Deep · **Tests:** 53 · **LOC:** 1,790

### Seams

| Seam | Type | Depth | Tests | Key Invariants |
|------|------|-------|-------|----------------|
| `Profile` | enum | Deep | 5 | Retention percentages match spec; max_lines monotonic; round-trips through FromStr; case-insensitive parse; rejects unknown |
| `ContextCategory` | enum | Deep | 4 | Labels are snake_case; round-trips through FromStr; unknown fallback |
| `classify_tool()` | fn | Deep | 7 | All category substrings classified correctly; unknown fallback; case-insensitive; `_`/`-` separator splitting; first-token-wins |
| `CondenserAlgorithm` trait | trait | Deep | 23 | Registry selects correct algorithm per category; rtk_style head/tail/ellipsis/passthrough; saliency_rank error-priority/order; flashrank novelty/brevity/relevance; cross-algorithm non-empty/never-expand/handles-consistent |
| `CondenserEngine` | struct | Deep | 12 | Default profile normal; zero stats start; auto-classifies; explicit category; algorithm name; profile tracking; reduction reporting; passthrough; empty input; set_profile; cumulative stats; line/byte counts |
| `CondenserStats` | struct | Shallow | 1 | Defaults to normal/zero (structural) |

### Algorithm Test Matrix

| Algorithm | Category | Tested Behaviors |
|-----------|----------|----------------|
| rtk_style | ShellCommand, TestOutput, BuildOutput | Reduces lines (heavy); passthrough (light); ellipsis marker; always produces output; preserves head+tail |
| saliency_rank | ConversationHistory, LogOutput, Unknown | Reduces lines; prioritizes error lines; passthrough; preserves order |
| flashrank | FileContents, StructuredData | Reduces lines; passthrough; novelty=1 for empty; brevity favors shorter; relevance matches terms; preserves order |

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
| `hkask-cns` | `TableGasEstimator`, `VarietyMonitor`, `AlgedonicManager` | High |
| `hkask-mcp` | `McpToolError`, `ToolSpanGuard`, `CredentialRequirement` | Medium |
| `hkask-cli` | `BootstrapSequence`, REPL commands | Low |
| `hkask-api` | HTTP route handlers | Low |
| `hkask-agents` | `PodManager`, `AcpRuntime`, `PodContext` | Medium |
| `hkask-keystore` | `Keystore` encrypt/decrypt/rotate | Medium |

### MCP server test coverage

All 21 MCP servers currently have **zero** unit tests (the condenser is the exception with 53 tests). The MCP tool surface is tested indirectly through integration, but no server has direct behavioral tests for its tool implementations.

| Server | Tests | Priority for Test Addition |
|--------|-------|--------------------------|
| condenser | 53 | ✅ Done |
| inference | 0 | High (generate, failover) |
| cns | 0 | High (variety, algedonic, gas) |
| ocap | 0 | High (create/verify/revoke cycle) |
| episodic | 0 | Medium (store/recall) |
| semantic | 0 | Medium (embed/search) |
| All others | 0 | Low |

---

## Methodology

- **Seam identification:** All `pub` traits, `pub` structs with `pub` methods, `pub` functions
- **Depth assessment:** Deep = few interface methods, many behaviors tested; Shallow = interface as complex as implementation
- **Coverage:** ✅ = behavioral tests exist for all key seams; ⚠️ = gaps present
- **Priority:** High = security/data-integrity seams; Medium = core domain seams; Low = surface/presentation seams