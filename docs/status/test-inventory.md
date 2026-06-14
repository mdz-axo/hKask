---
title: "Test Inventory"
audience: [architects, developers, agents]
version: "2.4.0"
last_updated: 2026-06-14
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle, curation]
---

# Test Inventory

Re-derived from `cargo test --workspace -- --list` on 2026-06-14.
Per MDS §8 and `docs/specifications/test-program.md`.

---

## Summary

| Crate/MCP Server | Tests | Module |
|------------------|-------|--------|
| `hkask-services` | 60 | chat, cns, pods, goals, curator, kata, wallet |
| `hkask-cli` | 41 | settings (12), repl_settings (4), turn/compaction (3), onboarding (3), feedback (3), mcp parse (13), passphrase (3) |
| `hkask-storage` | 50 | spec_store (6), spec_types (5), wallet (12), gallery (8), escalation (6), triples (5), agent_registry (2), store_macros (4 doc), lock_helpers (3 doc) |
| `hkask-templates` | 13 | contract_validator (5), lexicon (6), okapi_config (1 doc-test), manifest (1) |
| `hkask-cns` | 20 | governed_tool (4 OCAP + 1 doc-test + 1 integration), algedonic (2), variety (3), alert (2), cns_service (6), runtime (1) |
| `hkask-agents` | 10 | mode (4), curator persona_filter (4), pod (2) |
| `hkask-mcp-spec` | 10 | goal_capture (3 + 1 fuzz), coherence (1), graph_query (1), writing_quality (1), tool listing (1), replica (2) |
| `hkask-mcp` | 8 | daemon (5: auth, unauth, assignment, capability, dual-encoding), server (3) |
| `hkask-mcp-condenser` | 29 | algorithms (16), types (11), engine (2) |
| `hkask-mcp-companies` | 82 | analysis (20) + providers (9) + portfolio (23) + screening (19) + tools (11) |
| `hkask-mcp-research` | 46 | strip_html (8), freshness (6), ranking (5), rate_limiter (4), extraction (8), search (9), browsing (6) |
| `hkask-inference` | 20 | config (7), chat_protocol (3), fal_backend (4), embedding_router (4), ollama_backend (2) |
| `hkask-api` | 2 | settings merge, settings validation |
| `hkask-types` | 21 | ocr (6), id (3), event (2), ports (2), capability (2), cns (2), voice (2), transcript (2) |
| `hkask-mcp-memory` | 0 | Shallow module — pass-through to hkask-memory (C8) |
| `hkask-mcp-replica` | 0 | Shallow module — pass-through to compose/embed services (C8) |
| `hkask-mcp-docproc` | 73 | ocr pipeline (51) + tools (22: strip_json_fences 5, chunk 5, cache 2, cosine 4, validation 5, triples 1) |
| `hkask-mcp-training` | 0 | Stub — shallow pass-through to semantic memory (C8) |
| `hkask-mcp-communication` | 0 | Shallow module — local TTS passthrough (C8) |
| `hkask-mcp-media` | 16 | Gallery state (7) + Levenshtein (5) + Integration (4) |
| `hkask-memory` | 14 | consolidation, episodic, semantic pipelines |
| `hkask-keystore` | 6 | key derivation, encryption round-trip |

**Total: 534 tests across 22 crates** (↑ from 305; expanded test coverage across multiple crates)

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

## Recent Additions (2026-06-14 Session)

Media MCP server completion — collage, tests, voice, listen, talk:

| Crate | Tests Added | P8 Status | Details |
|-------|------------|-----------|--------|
| `hkask-mcp-media` | 4 | ✅ New | 4 integration tests: `gallery_lifecycle_init_to_search`, `collage_compose_grid_layout`, `gallery_store_image_not_found`, `gallery_three_state_policy`. Tagged `// REQ: media-gallery-lifecycle-01`, `media-collage-compose-01`, `media-gallery-error-01`, `media-gallery-policy-01`. |
| `hkask-cli` | 0 | ✅ Expanded | New `/listen start|stop|view` and `/talk on|off|voice` REPL commands. Talk mode includes speech summarizer (LLM-condensed spoken output) and ffplay-based TTS playback. Listen mode includes capture→transcribe_bundle→save pipeline with `/listen view` opening TUI transcript viewer with word-level highlighting. |
| `hkask-mcp-media` (tools) | — | ✅ Expanded | `image_create_collage` now supports three modes: `search_terms` (semantic tag search), `similar_to_index` (similar images), `image_indices` (explicit). Mutual exclusivity enforced. `record_and_transcribe` now returns `TranscriptBundle` with word-level timestamps (same format as `transcribe_bundle`). Gallery error messages consolidated from `gallery_init` → `gallery_set_root`. |

## Recent Additions (2026-06-13 Session)

DocProc server consolidation + training stub:

| Crate | Tests Added | P8 Status | Details |
|-------|------------|-----------|--------|
| `hkask-mcp-docproc` | 72 | ✅ New | Merged markitdown (OCR pipeline) + doc-knowledge (chunk/parse/QA). 51 OCR pipeline + 21 tools (strip_json_fences, chunk, cache, cosine similarity, validation) + 3 integration. Added triple extraction, embedding, RAG query/retrieval, auto-indexing, cache. |
| `hkask-mcp-training` | 0 | ✅ New | Stub server for model training data ingestion. Single tool: `training_ingest_qa`. |
| `hkask-inference` | 4 | ✅ New | `fal_backend` (4: construction fail/succeed, static catalog, vision heuristic) |
| `hkask-types` | 11 | ✅ Expanded | ocr types expanded from 6→17 (id, event, ports, capability, cns types) |

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
