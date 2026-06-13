---
title: "Test Inventory"
audience: [architects, developers, agents]
version: "2.3.0"
last_updated: 2026-06-13
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle, curation]
---

# Test Inventory

Re-derived from `cargo test --workspace -- --list` on 2026-06-13.
Per MDS §8 and `docs/specifications/test-program.md`.

---

## Summary

| Crate/MCP Server | Tests | Module |
|------------------|-------|--------|
| `hkask-services` | 35 | chat (9), cns (3), pods (3), goal (3), curator (2), 15 others |
| `hkask-cli` | 25 | settings (12), repl_settings (4), turn/compaction (3), onboarding (3), feedback (3) |
| `hkask-storage` | 18 | spec_store (6), spec_types (5), store_macros (4 doc-tests), lock_helpers (3 doc-tests) |
| `hkask-templates` | 12 | contract_validator (5), lexicon (6), okapi_config (1 doc-test) |
| `hkask-cns` | 11 | governed_tool (4 OCAP + 1 doc-test + 1 integration), algedonic (2), variety (3) |
| `hkask-agents` | 8 | mode (4: activation, exclusion, assignment, switch), curator persona_filter (4) |
| `hkask-mcp-spec` | 7 | goal_capture (2 + 1 fuzz), coherence (1), graph_query (1), writing_quality (1), tool listing (1) |
| `hkask-mcp` | 5 | daemon (5: auth, unauth, assignment, capability, dual-encoding) |
| `hkask-mcp-condenser` | 27 | algorithms (16), types (11) |
| `hkask-mcp-fmp` | 20 | analysis (20: management 6, moat/working capital 14) |
| `hkask-mcp-research` | 23 | strip_html (8), freshness (6), ranking (5), rate_limiter (4) |
| `hkask-inference` | 14 | config (7), chat_protocol (3), fal_backend (4) |
| `hkask-api` | 2 | settings merge, settings validation |
| `hkask-types` | 17 | ocr (6), id (3), event (2), ports (2), capability (2), cns (2) |
| `hkask-mcp-memory` | 0 | Shallow module — pass-through to hkask-memory (C8) |
| `hkask-mcp-replica` | 0 | Shallow module — pass-through to compose/embed services (C8) |
| `hkask-mcp-doc-knowledge` | 0 | Shallow module — pass-through to parsing/chunking (C8) |
| `hkask-mcp-markitdown` | 55 | ocr pipeline (52: calibration 6, complexity 6, routing 6, cross-validation 6, semantic 4, verification 4, pipeline 4, decimation 5, tesseract 4, llm_ocr 3, tools 4) + integration (3) |
| `hkask-mcp-communication` | 0 | Shallow module — local TTS passthrough (C8) |
| `hkask-mcp-fal` | 0 | Shallow module — API proxy (C8) |

**Total: 269 tests across 19 crates** (↑ from 243)
| `hkask-memory` | 0 | Requires external embedding model |
| `hkask-keystore` | 0 | Requires OS keychain |
| `hkask-mcp-condenser` | 0 | External server; tested via integration |
| `hkask-mcp-research` | 0 | External server (consolidated web + rss-reader 2026-06-11) |
| `hkask-mcp-fmp` | 0 | External server |
| `hkask-mcp-communication` | 0 | External server |
| `hkask-mcp-fal` | 0 | External server |
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
| `hkask-cli` | ✅ Compliant | 25 tests: settings validation (12), REPL settings (4), compaction threshold (3), feedback/append (3), passphrase strength (3). |
| `hkask-api` | ✅ Compliant | 2 tests for settings route — merge/validation semantics |
| `hkask-agents` | ⚠️ Thin | 2 doc-tests only. Pod lifecycle, ACP integration untested. Acceptable for current depth. |
| `hkask-mcp` | ⚠️ Thin | 3 doc-tests only. Server dispatch untested. Acceptable for thin port module. |
| `hkask-keystore` | ⚠️ OS-bound | No tests. Requires OS keychain for integration tests. |
| `hkask-memory` | ⚠️ Model-bound | No tests. Requires embedding model for behavioral validation. |
| `hkask-mcp-spec` | ✅ Compliant | 7 tests: capture (3), coherence (1), graph_query (1), writing_quality (1), tool listing (1) |

---

## Recent Additions (2026-06-13 Session)

OCR pipeline deepening + fal.ai integration:

| Crate | Tests Added | P8 Status | Details |
|-------|------------|-----------|--------|
| `hkask-inference` | 4 | ✅ New | `fal_backend` (4: construction fail/succeed, static catalog, vision heuristic) |
| `hkask-types` | 11 | ✅ Expanded | ocr types expanded from 6→17 (id, event, ports, capability, cns types) |
| `hkask-mcp-markitdown` | 11 | ✅ Expanded | calibration (6), decimation (2: contrast stretch + preprocess stub), tools (4: pipeline verification), integration tests now run by default (3) |

---

## Recent Additions (2026-06-11 Session)

Onboarding overhaul — new modules and behavioral seams:

| Crate | Tests Added | P8 Status | Details |
|-------|------------|-----------|--------|
| `hkask-cli` | 6 | ✅ Resolved | `append_feedback` (3 tests: header on first write, no dup header, entry format) in `repl/handlers/feedback.rs`. `passphrase_strength` (3 tests: weak/fair/strong boundaries) in `onboarding.rs`. Total: 19→25. |

---

## Recent Additions (2026-06-10 Session)

From TASK 0–6 architecture audit (see HANDOFF.md):

| Crate | Tests Added | Details |
|-------|------------|--------|
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
| `hkask-cli` | Medium (settings, REPL, onboarding) | Medium (25 tests) | ✅ |
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
