---
title: "Test Inventory"
version: "2.0.0"
last_updated: 2026-06-10
status: Active
domain: "Cross-cutting"
generated_from: "cargo test --workspace -- --list"
---

# Test Inventory

Re-derived from `cargo test --workspace -- --list` on 2026-06-10.
Per MDS §8 and `docs/specifications/test-program.md`.

---

## Summary

| Crate/MCP Server | Tests | Module |
|------------------|-------|--------|
| `hkask-services` | 28 | chat (9), cns (3), pods (3), goal (3), curator (2), 8 others |
| `hkask-cli` | 19 | settings (12), repl_settings (4), turn/compaction (3) |
| `hkask-storage` | 18 | spec_store (6), spec_types (5), store_macros (4 doc-tests), lock_helpers (3 doc-tests) |
| `hkask-templates` | 12 | contract_validator (5), lexicon (6), okapi_config (1 doc-test) |
| `hkask-cns` | 11 | governed_tool (4 OCAP + 1 doc-test + 1 integration), algedonic (2), variety (3) |
| `hkask-mcp-spec` | 7 | goal_capture (2 + 1 fuzz), coherence (1), graph_query (1), writing_quality (1), tool listing (1) |
| `hkask-mcp` | 3 | validate_field (2 doc-tests), server (1 doc-test) |
| `hkask-api` | 2 | settings merge, settings validation |
| `hkask-agents` | 2 | pod (doc-test), lib (doc-test) |
| `hkask-types` | 0 | Shallow module — types only (C8) |
| `hkask-memory` | 0 | Requires external embedding model |
| `hkask-keystore` | 0 | Requires OS keychain |
| `hkask-mcp-condenser` | 0 | External server; tested via integration |
| `hkask-mcp-web` | 0 | External server |
| `hkask-mcp-fmp` | 0 | External server |
| `hkask-mcp-telnyx` | 0 | External server |
| `hkask-mcp-fal` | 0 | External server |
| `hkask-mcp-rss-reader` | 0 | External server |
| **Total** | **102** | |

---

## P8 Compliance

Per PRINCIPLES.md P8: "Every `#[test]` verifies a stated behavioral property of a public seam."
Per C8: "Test depth matches module depth."

| Crate | P8 Status | Notes |
|-------|-----------|-------|
| `hkask-types` | ✅ Compliant | No runtime behavior to test. Types, enums, and derive macros only. |
| `hkask-storage` | ✅ Compliant | 18 tests for spec store and macros — the behavioral seams |
| `hkask-templates` | ✅ Compliant | 12 tests: contract validation (5), lexicon parsing (6), okapi config (1) |
| `hkask-cns` | ✅ Compliant | 11 tests for OCAP governance, algedonic thresholds, variety tracking — all behavioral |
| `hkask-services` | ✅ Compliant | 28 tests covering chat, CNS service, pods, goals, curator — all public seams |
| `hkask-cli` | ✅ Compliant | 19 tests: settings validation (12), REPL settings (4), compaction threshold (3) |
| `hkask-api` | ✅ Compliant | 2 tests for settings route — merge/validation semantics |
| `hkask-agents` | ⚠️ Thin | 2 doc-tests only. Pod lifecycle, ACP integration untested. Acceptable for current depth. |
| `hkask-mcp` | ⚠️ Thin | 3 doc-tests only. Server dispatch untested. Acceptable for thin port module. |
| `hkask-keystore` | ⚠️ OS-bound | No tests. Requires OS keychain for integration tests. |
| `hkask-memory` | ⚠️ Model-bound | No tests. Requires embedding model for behavioral validation. |
| `hkask-mcp-spec` | ✅ Compliant | 7 tests: capture (3), coherence (1), graph_query (1), writing_quality (1), tool listing (1) |

---

## Recent Additions (2026-06-10 Session)

From TASK 0–6 architecture audit (see HANDOFF.md):

| Crate | Tests Added | Details |
|-------|------------|---------|
| `hkask-cns` | 13 | 4 GovernedTool OCAP (domain capability, legacy exact match), 2 algedonic binary-threshold, 3 variety sensor, 1 GovernedTool integration, 3 CnsService |
| `hkask-services` | 3 | CnsService: health defaults, variety empty, alerts empty |

---

## Test Depth vs Module Depth (C8)

| Crate | Module Depth | Test Depth | Match? |
|-------|-------------|------------|--------|
| `hkask-cns` | Deep (OCAP governance, algedonic, variety) | Deep (11 behavioral tests) | ✅ |
| `hkask-services` | Deep (chat, CNS, goals, pods) | Deep (28 tests) | ✅ |
| `hkask-storage` | Deep (bitemporal queries, macros) | Deep (18 tests) | ✅ |
| `hkask-templates` | Deep (validation, lexicon) | Deep (12 tests) | ✅ |
| `hkask-cli` | Medium (settings, REPL) | Medium (19 tests) | ✅ |
| `hkask-mcp-spec` | Medium (5 MDS tools) | Medium (7 tests) | ✅ |
| `hkask-types` | Shallow (types only) | Shallow (0 tests) | ✅ |
| `hkask-api` | Shallow (routes) | Shallow (2 tests) | ✅ |
| `hkask-agents` | Medium | Shallow (2 doc-tests) | ⚠️ |
| `hkask-mcp` | Shallow (ports) | Shallow (3 doc-tests) | ✅ |
| `hkask-memory` | Deep | Shallow (0 tests) | ⚠️ (external deps) |
| `hkask-keystore` | Deep | Shallow (0 tests) | ⚠️ (external deps) |

---

## Methodology

- **Source:** `cargo test --workspace -- --list` run 2026-06-10
- **Count:** `grep ': test$'` against output (excludes benchmarks, doc-test headers)
- **Module depth:** Assessed from code structure — deep = few interface methods hiding substantial behavior; shallow = interface as simple as implementation
- **P8 compliance:** Tests exist where behavior exists. Exceptions documented with rationale.
