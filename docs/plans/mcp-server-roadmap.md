---
title: "MCP Server Roadmap — Consolidation, Deepening, and RAG Pipeline"
version: "1.1.0"
last_updated: 2026-06-11
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, capability, lifecycle, curation]
---

# MCP Server Roadmap

Surfaced from architecture audit on 2026-06-11. Covers 10 MCP servers.

See also: [`docs/status/mcp-tools-inventory.md`](../status/mcp-tools-inventory.md) for current tool catalog (~82 tools across 10 servers).

---

## Completed (2026-06-11)

### ✅ 1. Consolidation: Collapse `rss-reader` + `web` → `hkask-mcp-research`

**Result:** Created `hkask-mcp-research` with ~17 tools (5 web + 12 RSS) in a unified `ResearchServer`. Web tools always available with at least one search provider key. RSS tools gracefully degrade when `HKASK_RSS_DB` is not configured. Deleted `hkask-mcp-web` and `hkask-mcp-rss-reader`.

**Updated:** 7 code files (workspace, bootstrap, builtin_servers, serve, web_search, energy estimator, embed user agent), 5 docs (AGENTS.md, README, PRINCIPLES, mcp-tools-inventory, test-inventory).

### ✅ 9. Document `hkask-mcp-replica` in AGENTS.md and Inventory

**Result:** Added `hkask-mcp-replica` (6 tools: build, compose, mashup, compare, registry, explain) to AGENTS.md crate map, README.md, `mcp-tools-inventory.md`, PRINCIPLES.md, and test-inventory.md.

---

## 2. Value-Added Layers for External Service Wrappers

`hkask-mcp-fmp` and `hkask-mcp-fal` are currently thin API proxies — each tool is a 1:1 passthrough to the external service. They need value-added layers that compose the raw API calls into higher-level capabilities.

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

### 2.3 `hkask-mcp-fal` (AI Media Generation) — 9 tools

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

## 3. RAG Pipeline: `doc-knowledge` + `markitdown`

These two servers are standalone tools today with no composition. They are components of a Retrieval-Augmented Generation pipeline that needs to be defined and built.

### 3.1 Current State

| Server | Tools | Role |
|--------|-------|------|
| `hkask-mcp-markitdown` | `extract_text`, `detect_format`, `ocr` | Document → raw text |
| `hkask-mcp-doc-knowledge` | `parse`, `detect_format`, `extract_markdown`, `store_qa` | Text → chunks + Q&A |

### 3.2 Required Pipeline Definition

```
Document (PDF, DOCX, image)
  └─ markitdown_extract_text  →  raw text
      └─ doc_knowledge_parse  →  chunks with metadata
          └─ embed             →  vector embeddings  (calls hkask-mcp-memory)
              └─ index          →  searchable index
                  └─ query      →  retrieve relevant chunks
                      └─ generate →  LLM-augmented answer
```

### 3.3 Open Design Questions

1. **Orchestration location:** Should the pipeline live in a new `hkask-mcp-rag` server, or should `doc-knowledge` absorb `markitdown` as internal modules?
2. **Embedding integration:** `doc-knowledge` currently has no embedding tool. Should it call `hkask-mcp-memory`'s `semantic_embed` and `semantic_search`, or embed locally?
3. **Chunking strategy:** `doc_knowledge_parse` mentions "multi-tier chunking" — is the strategy defined? (Recursive character, semantic, token-aware?)
4. **Query interface:** What does the end-user tool look like? `rag_query` with natural language → retrieved chunks → generated answer?
5. **Provenance:** Should answers cite source chunks with document + page references?

### 3.4 Implementation Phases

| Phase | Scope | Deliverable |
|-------|-------|-------------|
| **Phase 1** | Pipeline definition | Architecture doc with tool contracts and data flow |
| **Phase 2** | Embedding integration | `doc-knowledge` calls memory server for embed + search |
| **Phase 3** | Query + generate | End-to-end: document → extract → chunk → embed → retrieve → answer |
| **Phase 4** | Provenance + citations | Answers include source references |

| Status | Owner | Priority |
|--------|-------|----------|
| ⬜ Open (Phase 1) | — | High |

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

9 of 10 MCP servers have **zero tests**. Only `hkask-mcp-spec` has 7 tests.

| Server | Tests | Rationale in Inventory |
|--------|-------|----------------------|
| condenser | 0 | "External server; tested via integration" |
| research | 0 | "External server" (consolidated web + rss-reader) |
| fmp | 0 | "External server" |
| communication | 0 | "External server" |
| fal | 0 | "External server" |
| memory | 0 | Thin wrapper; library (`hkask-memory`) requires embedding model |
| doc-knowledge | 0 | Not listed in test inventory |
| markitdown | 0 | Not listed in test inventory |
| replica | 0 | Not listed in test inventory |

### 5.2 Test Strategy

Per the test program (`docs/specifications/test-program.md`), MCP server tests require `rmcp` transport. Open question #5 notes: "Extract `hkask-test-utils` when 3+ servers need shared fixtures (currently 2 — below C4 threshold)."

**Tiered approach:**

| Tier | Servers | Strategy |
|------|---------|----------|
| **Tier 1: Internal logic** | condenser, research, doc-knowledge, markitdown | Unit-test algorithms and request builders directly (no rmcp transport needed). These servers have significant internal logic outside API calls. |
| **Tier 2: Thin wrappers** | fmp, communication, fal | Low-value to unit test passthroughs. Value-add layers (Section 2) should carry tests. |
| **Tier 3: Integration** | condenser, research | `rmcp` transport tests once `hkask-test-utils` is extracted. |

### 5.3 Priority Targets

1. **condenser** — algorithms (`rtk_style`, `saliency_rank`, `flashrank`) are pure functions testable without any transport
2. **research** — `strip_html`, `freshness`, `ranking`, `rate_limiter`, RSS `db.rs` query functions are pure logic
3. **doc-knowledge** — chunking logic is algorithmic and testable

| Status | Owner | Priority |
|--------|-------|----------|
| ⬜ Open (Tier 1) | — | Medium |

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
