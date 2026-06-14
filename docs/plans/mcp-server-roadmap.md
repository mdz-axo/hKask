---
title: "MCP Server Roadmap — Consolidation, Deepening, and RAG Pipeline"
version: "1.1.0"
audience: [architects, developers]
last_updated: 2026-06-12
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# MCP Server Roadmap

Surfaced from architecture audit on 2026-06-11. Covers 12 MCP servers.

See also: [`docs/status/mcp-tools-inventory.md`](../status/mcp-tools-inventory.md) for current tool catalog.

---

## Completed (2026-06-11)

### ✅ 1. Consolidation: Collapse `rss-reader` + `web` → `hkask-mcp-research`

**Result:** Created `hkask-mcp-research` with ~17 tools (5 web + 12 RSS) in a unified `ResearchServer`. Web tools always available with at least one search provider key. RSS tools gracefully degrade when `HKASK_RSS_DB` is not configured. Deleted `hkask-mcp-web` and `hkask-mcp-rss-reader`.

**Updated:** 7 code files (workspace, bootstrap, builtin_servers, serve, web_search, energy estimator, embed user agent), 5 docs (AGENTS.md, README, PRINCIPLES, mcp-tools-inventory, test-inventory).

### ✅ 9. Document `hkask-mcp-replica` in AGENTS.md and Inventory

**Result:** Added `hkask-mcp-replica` (6 tools: build, compose, mashup, compare, registry, explain) to AGENTS.md crate map, README.md, `mcp-tools-inventory.md`, PRINCIPLES.md, and test-inventory.md.

---

## 2. Value-Added Layers for External Service Wrappers

`hkask-mcp-fmp` and `hkask-mcp-media` are currently thin API proxies — each tool is a 1:1 passthrough to the external service. They need value-added layers that compose the raw API calls into higher-level capabilities.

### 2.1 `hkask-mcp-fmp` (Financial Modeling Prep) — 11 tools

**Current state:** Each tool calls one FMP endpoint and returns raw JSON.

**Value-add targets:**
- Portfolio tracking: aggregate multiple symbols, track positions over time
- Correlation analysis: compute pairwise correlations across holdings
- Alert conditions: user-defined thresholds on metrics (price, P/E, volume)
- Multi-symbol aggregation: batch `fmp_quote` / `fmp_key_metrics` across a portfolio
- Historical analysis: combine `fmp_historical_price` + `fmp_key_metrics` into time-series summaries

**Architecture note:** Value-add logic should live in the MCP server, not in a separate crate, unless it grows deep enough to justify extraction (C4: "Extract only with 3+ consumers").

| Status | Owner | Priority |
|--------|-------|----------|
| ⬜ Open | — | Medium |

### 2.2 `hkask-mcp-communication` (Local TTS/STT) — 3 tools

**Current state:** Local system TTS via espeak. No external API dependencies.

**Completed (2026-06-12):**
- ✅ `tts_speak` — speak text aloud via system TTS
- ✅ `tts_generate` — generate WAV audio file via system TTS
- ✅ `tts_list_voices` — catalog of 8 espeak voices
- ✅ Telnyx/phone fully deleted — no telecom dependency

**Remaining targets:**
- STT (speech-to-text) — microphone → text via local engine (Whisper, etc.)
- Voice design in onboarding — collect voice description + pick from espeak catalog
- Verbal mode prompt templates — concision/brevity for spoken responses

| Status | Owner | Priority |
|--------|-------|----------|
| ✅ Partial (TTS complete, STT deferred) | — | Medium |

### 2.3 `hkask-mcp-media` (AI Media Generation) — 9 tools

**Current state:** Raw API passthrough for image, video, music, 3D generation.

**Value-add targets:**
- Image pipelines: `fal_generate_image` → `fal_upscale` → `fal_caption` as a composed chain
- Batch operations: generate N variants from a prompt with different seeds
- Style consistency: maintain a style reference across multiple generations
- Media library: track generated assets with metadata (prompt, seed, model, timestamp)
- Prompt enhancement: pre-process prompts for better results before calling generation tools

| Status | Owner | Priority |
|--------|-------|----------|
| ⬜ Open | — | Medium |

---

## 3. RAG Pipeline: `doc-knowledge` + `markitdown` → `hkask-mcp-docproc` ✅ COMPLETED

**Completed 2026-06-13.** The two servers have been merged into `hkask-mcp-docproc`, a unified document processing server with 8 tools covering the full pipeline:

```
Document (PDF, MD, HTML, TXT)
  └─ docproc_convert  →  raw text (with OCR fallback)
      └─ docproc_chunk  →  passages (single or multi-tier)
          ├─ docproc_extract_triples  →  RDF (s, p, o) knowledge triples
          ├─ docproc_embed  →  vector embeddings
          ├─ docproc_generate_qa  →  QA pairs (LLM-generated)
          │   └─ docproc_store_qa  →  stored in semantic memory
          │       └─ training_ingest_qa  →  hkask-mcp-training (stub)
          └─ docproc_cache  →  cached markdown reference
```

### 3.1 Resolved Design Questions

1. **Orchestration:** ✅ Merged into single `hkask-mcp-docproc` server — no separate RAG server needed.
2. **Embedding:** ✅ `docproc_embed` uses the inference router's embedding model directly.
3. **Chunking:** ✅ `docproc_chunk` supports single-tier (configurable token size + overlap) and multi-tier (coarse/medium/fine).
4. **Triple extraction:** ✅ `docproc_extract_triples` extracts RDF triples via LLM, analogous to replica's Fish process for chat streams.
5. **QA generation:** ✅ `docproc_generate_qa` actually calls the LLM (not just returns a prompt).

### 3.2 Remaining

| Item | Status |
|------|--------|
| Query + retrieve + generate (end-to-end RAG) | ⬜ Future — needs search/retrieval integration |
| Provenance + citations in answers | ⬜ Future |
| `hkask-mcp-training` full pipeline (dataset assembly, formatting, fine-tuning) | ⬜ Stub only |

| Status | Owner | Priority |
|--------|-------|----------|
| ✅ Completed | — | — |

---

## 4. Fixes and Documentation

### 4.1 Register `condenser_thread_summary` as MCP Tool

**Problem:** `condenser_thread_summary` is implemented in `hkask-mcp-condenser/src/inference.rs` but not registered as an `#[tool]` MCP endpoint. It is called as a raw HTTP function. The `mcp-tools-inventory.md` notes this gap explicitly.

**Fix:** Add `#[tool]` attribute and register in the server's tool router. This brings condenser from 6 to 7 registered tools.

**File:** `mcp-servers/hkask-mcp-condenser/src/main.rs`
**Reference:** `mcp-tools-inventory.md` line 50

| Status | Owner | Priority |
|--------|-------|----------|
| ⬜ Open | — | Low |

**Note (2026-06-11):** This was already complete — `condenser_thread_summary` is registered as `#[tool]` at line 238 of `main.rs`. The `mcp-tools-inventory.md` note was stale.

---

## 5. Test Coverage

### 5.1 Current State

4 of 12 MCP servers have tests. `hkask-mcp-docproc` leads with 54 tests (deep OCR pipeline).

| Server | Tests | Rationale |
|--------|-------|----------|
| `hkask-mcp-docproc` | 54 | Deep OCR pipeline (51 unit + 3 integration) |
| `hkask-mcp-spec` | 7 | capture, coherence, graph_query, writing_quality, tool listing |
| `hkask-mcp-media` | 7 | Gallery state (init, scan, info) |
| `hkask-mcp-condenser` | 29 | algorithms (16), types (11) — tested via library crate |
| `hkask-mcp-research` | 23 | strip_html, freshness, ranking, rate_limiter |
| `hkask-mcp-fmp` | 20 | Financial analysis algorithms |
| `hkask-mcp-memory` | 0 | Thin wrapper; library (`hkask-memory`) requires embedding model |
| `hkask-mcp-replica` | 0 | Thin wrapper; pass-through to compose/embed services |
| `hkask-mcp-training` | 0 | Stub; shallow pass-through to semantic memory |
| `hkask-mcp-communication` | 0 | Thin wrapper; local TTS passthrough |

### 5.2 Test Strategy

| Tier | Servers | Strategy |
|------|---------|----------|
| **Tier 1: Internal logic** | docproc, condenser, research, fmp | Unit-test algorithms and request builders directly. These servers have significant internal logic. |
| **Tier 2: Thin wrappers** | memory, replica, training, communication | Low-value to unit test passthroughs. Value-add layers should carry tests. |
| **Tier 3: Integration** | docproc, condenser, research | `rmcp` transport tests once `hkask-test-utils` is extracted. |

### 5.3 Priority Targets

1. **docproc** — ✅ 54 tests already. OCR pipeline is deeply tested.
2. **condenser** — ✅ 29 tests. Algorithms are pure functions.
3. **research** — ✅ 23 tests. Core logic is covered.

| Status | Owner | Priority |
|--------|-------|----------|
| ✅ Largely complete | — | Low |

---

## 6. Integration Test Infrastructure

**Problem:** No shared test utilities for MCP server integration tests using `rmcp` transport.

**Decision:** Extract `hkask-test-utils` when 3+ servers need shared fixtures (C4 threshold). Currently, only `hkask-mcp-spec` has any integration tests.

**Contains:**
- `rmcp` server startup/shutdown helpers
- Mock transport for tool invocation
- Shared test fixtures (sample documents, RSS feeds, search results)
- CNS span assertion helpers

| Status | Owner | Priority |
|--------|-------|----------|
| ⬜ Deferred (C4 threshold not met) | — | Low |

---

## 7. Summary Matrix

| # | Task | Section | Priority | Effort | Dependencies | Status |
|---|------|---------|----------|--------|--------------|--------|
| 1 | Collapse rss-reader + web → research | — | High | Medium | None | ✅ Complete (2026-06-11) |
| 2 | Define RAG pipeline architecture | §3 | High | Design-only | None | ⬜ Open |
| 3 | RAG Phase 1: embed integration | §3.4 | High | Medium | §3 design complete | ⬜ Open |
| 4 | FMP value-add layer (Tier 1: moat, management, working capital, expectations gap) | §2.1 | Medium | High | None | ✅ Complete (2026-06-11) — 4 MAIA-framework tools, 20 tests, 1 new module (analysis.rs) |
| 5 | Communication server (local TTS) | §2.2 | Medium | High | None | ✅ Partial (2026-06-12) — `tts_speak`, `tts_generate`, `tts_list_voices` via espeak. Telnyx fully deleted. Remaining: STT, voice design in onboarding, verbal mode templates. |
| 6 | Fal value-add layer | §2.3 | Medium | High | None | ⬜ Open |
| 7 | Tier 1 unit tests (condenser, research) | §5.3 | Medium | Medium | None | ✅ Complete (2026-06-11) — 50 tests: 27 condenser, 23 research |
| 8 | Verify condenser_thread_summary registration | §4.1 | Low | Small | None | ✅ Complete (2026-06-11) — already `#[tool]` at main.rs:238, 7 tools total |
| 9 | Document replica in AGENTS.md + inventory | — | Low | Small | None | ✅ Complete (2026-06-11) |
| 10 | Extract hkask-test-utils | §6 | Low | Medium | 3+ servers needing integration tests | ⬜ Open |

---

## 8. Related Documents

| Document | Relevance |
|----------|-----------|
| [`docs/status/mcp-tools-inventory.md`](../status/mcp-tools-inventory.md) | Current tool catalog (~82 tools across 10 servers) |
| [`docs/status/test-inventory.md`](../status/test-inventory.md) | Test coverage per crate (102 total tests) |
| [`docs/specifications/test-program.md`](../specifications/test-program.md) | MDS self-applying test methodology |
| [`docs/OPEN_QUESTIONS.md`](../OPEN_QUESTIONS.md) | Open questions including OQ-5 (test isolation), OQ-9 (stub MCP servers) |
| [`AGENTS.md`](../../AGENTS.md) | Crate map with 10 listed MCP servers |
| [`docs/architecture/PRINCIPLES.md`](../architecture/PRINCIPLES.md) | P8 (test behavioral properties), C4 (extraction threshold), C8 (test depth) |
| [`docs/plans/TODO.md`](TODO.md) | General project TODO (P1-07: stub MCP servers ✅ Complete) |
