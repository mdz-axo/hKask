---
title: "MCP Tools Inventory"
audience: [architects, developers, agents]
version: "2.1.0"
last_updated: 2026-06-12
status: "Active"
domain: "Cross-cutting"
mds_categories: [composition, lifecycle]
---

# MCP Tools Inventory

Catalog of all 12 hKask MCP servers and their tools.
Updated 2026-06-13: `hkask-mcp-markitdown` + `hkask-mcp-doc-knowledge` → `hkask-mcp-docproc`. Added `hkask-mcp-training`.

---

## Summary

| Server | Crate | Tools | Loop | Required Credentials |
|--------|-------|-------|------|----------------------|
| condenser | `hkask-mcp-condenser` | 6 | L2 (Episodic) | — |
| research | `hkask-mcp-research` | ~17 | L4 (Communication) | See per-server detail |
| spec | `hkask-mcp-spec` | 5 | L5 (Curation) | `HKASK_OCAP_SECRET` |
| companies | `hkask-mcp-companies` | 21 | L4 (Communication) | `HKASK_FMP_API_KEY`, `HKASK_EODHD_API_KEY` |
| communication | `hkask-mcp-communication` | 3 | L4 (Communication) | — |
| media | `hkask-mcp-media` | 20 | L4 (Communication) | `DI_API_KEY`, `FA_API_KEY`, or `FW_API_KEY` |
| replica | `hkask-mcp-replica` | 6 | L4 (Communication) | `HKASK_EMBEDDING_MODEL` (optional) |
| memory | `hkask-mcp-memory` | 13 | L2 (Episodic + Semantic) | `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE` |
| docproc | `hkask-mcp-docproc` | 9 | L2 (Episodic) | `HKASK_OCR_MODEL` (optional) |
| training | `hkask-mcp-training` | 1 | L2 (Episodic) | `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE` |
| **Total** | | **~105** | | |

---

## Per-Server Detail

### condenser

**Crate:** `hkask-mcp-condenser` · **Loop:** L2 · **Tools:** 6

| Tool | Description |
|------|-------------|
| `condenser_ping` | Liveness and profile info |
| `condenser_compress` | Compress tool output using context-aware algorithms |
| `condenser_set_profile` | Set compression profile (heavy/normal/soft/light) |
| `condenser_stats` | Cumulative compression statistics |
| `condenser_classify` | Classify tool name to context category |
| `condenser_persist` | Persist a compressed output to episodic memory |

**Note:** `condenser_thread_summary` is implemented in `inference.rs` as a pure HTTP function, not as an `#[tool]`-registered MCP tool. If it should be exposed as an MCP tool, it needs registration.

---

### research

**Crate:** `hkask-mcp-research` · **Loop:** L4 · **Tools:** ~17

Consolidation of former `hkask-mcp-web` and `hkask-mcp-rss-reader` (2026-06-11).
Web tools always available with at least one search provider key. RSS tools available when `HKASK_RSS_DB` + `HKASK_DB_PASSPHRASE` are set (graceful degradation otherwise).

**Web credentials (all optional):** `HKASK_BRAVE_API_KEY`, `HKASK_FIRECRAWL_API_KEY`, `HKASK_TAVILY_API_KEY`, `HKASK_SERPAPI_API_KEY`, `HKASK_EXA_API_KEY`, `HKASK_BROWSERBASE_API_KEY`
**RSS credentials (optional):** `HKASK_RSS_DB`, `HKASK_DB_PASSPHRASE`

#### Web tools

| Tool | Description |
|------|-------------|
| `web_ping` | Liveness and provider health check |
| `web_search` | Search the web with RRF fusion across providers |
| `web_find_similar` | Find pages similar to a given URL |
| `web_extract` | Extract content from a URL into markdown |
| `web_browse` | Interactive browsing of JS-heavy pages |

#### RSS tools

| Tool | Description |
|------|-------------|
| `rss_subscribe` | Subscribe to an RSS/Atom feed |
| `rss_unsubscribe` | Unsubscribe from a feed |
| `rss_list_subscriptions` | List subscriptions |
| `rss_fetch` | Fetch/sync new entries (supports ETag/Last-Modified) |
| `rss_get_entries` | Get entries from a stream with continuation pagination |
| `rss_mark_all_read` | Mark all entries in a stream as read |
| `rss_get_unread_count` | Get unread count for a stream |
| `rss_search` | Full-text search across feed entries |
| `rss_export_opml` | Export subscriptions as OPML 2.0 |
| `rss_import_opml` | Import subscriptions from OPML |
| `rss_discover_feeds` | Discover feeds from URL via HTML autodiscovery |
| `rss_edit_tag` | Edit tags on entries (read/unread, star, labels) |

---

### spec

**Crate:** `hkask-mcp-spec` · **Loop:** L5 · **Tools:** 5

**Required:** `HKASK_OCAP_SECRET`. Optional: `HKASK_SPEC_DB_PATH`, `HKASK_DB_PASSPHRASE`.

Per MDS.md §3 — five tools only. Curation tools (`evaluate`, `reconcile`, `cultivate`) deleted. Bind tool deleted. All five are OCAP-gated via `GovernedTool`.

| Tool | Status | Description |
|------|--------|-------------|
| `spec/goal/capture` | ✅ Implemented | Capture a specification goal with OCAP boundaries |
| `spec/goal/decompose` | ✅ Implemented | Decompose a goal into ordered sub-goals (max depth 7) |
| `spec/require/writing-quality` | ✅ Implemented | Assess writing quality against excellence criteria |
| `spec/graph/query` | ✅ Implemented | Query spec document graph by category or domain anchor |
| `spec/graph/coherence` | ✅ Implemented | Validate graph coherence and return score |

**Not in MDS tool surface** (deleted from spec server):
- `spec/require/bind` — Deleted: OCAP boundaries declared inline during capture
- `spec/curate/evaluate` — Deleted: curation is external to spec server
- `spec/curate/reconcile` — Deleted: curation is external to spec server
- `spec/curate/cultivate` — Deleted: curation is external to spec server

---

### companies

**Crate:** `hkask-mcp-companies` · **Loop:** L4 · **Tools:** 21

**Required:** `HKASK_FMP_API_KEY`, `HKASK_EODHD_API_KEY`

**Architecture:** Dual-provider abstraction layer (`providers.rs`) with:
- **Auto-routing:** Exchange-qualified symbols (`VOD.L`, `BMW.DE`) → EODHD primary; plain symbols (`AAPL`) → FMP primary
- **Automatic fallback:** Primary failure → secondary provider; plain symbols get `.US` suffix for EODHD fallback
- **Response normalization:** EODHD's nested `/fundamentals/{symbol}` JSON normalized to FMP's flat array format so `analysis.rs` functions work transparently with either provider
- **Derived metrics:** `key_metrics` computes `grossProfitMargin`, `roic`, `daysOfPayablesOutstanding`, `daysOfSalesOutstanding` from EODHD financial statements when native metrics unavailable

**Coverage:** FMP (US-focused, deep fundamentals) + EODHD (global, 70+ exchanges, broad coverage). MAIA deep fundamental analysis works best with FMP; EODHD expands global reach for profiles, quotes, search, and historical prices.

**Financial data tools:**

| Tool | Description |
|------|-------------|
| `company_profile` | Get company profile |
| `stock_quote` | Get stock quote |
| `income_statement` | Get income statement |
| `balance_sheet` | Get balance sheet |
| `cash_flow_statement` | Get cash flow statement |
| `key_metrics` | Get key metrics (with derived metrics from EODHD financials) |
| `historical_price` | Get historical price data |
| `symbol_search` | Search for symbols (FMP primary, EODHD fallback) |
| `moat_check` | MAIA competitive moat analysis (gross margin stability + working capital signal) |
| `management_scorecard` | MAIA CEO capital allocation scorecard (ROIC vs invested capital trend) |
| `working_capital_cycle` | MAIA CFO working capital analysis (DPO/DSO/DIO/CCC over time) |
| `expectations_gap` | Gordon Growth Model: market-implied vs historical growth across 3 valuation sets |

**Portfolio management tools:**

| Tool | Description |
|------|-------------|
| `portfolio_delete` | Delete a portfolio and all associated data |
| `portfolio_list` | List all portfolios |
| `ledger_import` | Import transactions from CSV or JSON (auto-creates portfolio) |
| `ledger_export` | Export full ledger to CSV or JSON |
| `transaction_note_append` | Append a note to an existing transaction |
| `note_add` | Add a research note to a company/security |
| `note_list` | List notes for a symbol with optional date/tag filtering |
| `note_delete` | Delete a note by ID |
| `file_attach` | Attach a file (base64-encoded) to a company/security |
| `file_list` | List attached files for a symbol |
| `file_delete` | Delete an attached file (record + disk) |
| `portfolio_attribution` | What moved the portfolio — position-level contribution ranking |
| `portfolio_characteristics` | Weighted-average fundamentals across holdings |
| `portfolio_comparison` | Side-by-side portfolio comparison |
| `portfolio_returns` | Time-weighted (Modified Dietz) and money-weighted (IRR) returns |

---

### communication

**Crate:** `hkask-mcp-communication` · **Loop:** L4 · **Tools:** 3

**Required:** — (local system TTS, no API key needed)

| Tool | Description |
|------|-------------|
| `tts_speak` | Speak text aloud via system TTS (espeak) |
| `tts_generate` | Generate TTS audio file (espeak), returns WAV path |
| `tts_list_voices` | List available system TTS voices (espeak) |

---

### media

**Crate:** `hkask-mcp-media` · **Loop:** L4 · **Tools:** 28 (3 gallery + 4 analysis + 4 image + 7 video + 2 voice + 4 audio + 4 generation)

**Required:** `DI_API_KEY`, `FA_API_KEY`, or `FW_API_KEY` (at least one)

| Tool | Description |
|------|-------------|
| `gallery_organize` | Point at a photo folder — auto-creates index and scans for images |
| `gallery_status` | Get gallery summary: path, mode, image count, size |
| `gallery_search` | Fuzzy search by describing what you're looking for (Levenshtein) |
| `describe_image` | Describe an image in detail (descriptive/artistic/technical/alt_text) |
| `remove_background` | Remove background from an image (Bria RMBG 2.0) |
| `apply_style` | Apply style transfer to an image (Flux img2img) |
| `create_collage` | Create a collage from gallery images (search, similar, or explicit) |
| `video_clip` | Trim a video to a segment (ffmpeg) |
| `video_to_gif` | Convert a video segment to GIF (ffmpeg) |
| `image_to_video` | Animate a still image into a short video |
| `video_add_caption` | Add text overlay to a video (ffmpeg) |
| `video_remix` | Clip + caption + GIF composite |
| `video_concat` | Concatenate multiple clips (ffmpeg) |
| `video_from_images` | Create video/GIF from image sequence (ffmpeg) |
| `voice_design` | Design a synthetic voice from a character description |
| `generate_speech` | Generate speech audio from text + voice design |
| `transcribe` | Transcribe speech audio to text |
| `transcribe_bundle` | Transcribe with word-level timings for interactive UIs |
| `audio_capture` | Record audio from microphone (ffmpeg) |
| `record_and_transcribe` | Record + transcribe in one call |
| `generate_image` | Generate an image from a text prompt |
| `transform_image` | Transform an existing image with a text prompt |
| `upscale_image` | Upscale an image to higher resolution |
| `generate_video` | Generate a short video from a text prompt |

---

### replica

**Crate:** `hkask-mcp-replica` · **Loop:** L4 · **Tools:** 6

**Credentials:** `HKASK_EMBEDDING_MODEL` (optional, defaults to `Qwen/Qwen3-Embedding-0.6B` via DeepInfra)

| Tool | Description |
|------|-------------|
| `replica_build` | Embed a corpus and create a style replica |
| `replica_compose` | Generate prose in an author's style |
| `replica_mashup` | Blend two authors' styles via centroid interpolation |
| `replica_compare` | Measure stylistic distance between two authors |
| `replica_registry` | List, inspect, and manage built replicas |
| `replica_explain` | Explain centroids and style-space topology |

---

### memory

**Crate:** `hkask-mcp-memory` · **Loop:** L2 · **Tools:** 13

**Required:** `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE`

Consolidation of former `hkask-mcp-episodic` and `hkask-mcp-semantic` servers.

| Tool | Subsystem | Description |
|------|-----------|-------------|
| `episodic_ping` | Episodic | Liveness and storage info |
| `episodic_store` | Episodic | Store an episodic triple |
| `episodic_recall` | Episodic | Recall episodic triples by entity |
| `episodic_budget` | Episodic | Storage usage and budget |
| `semantic_ping` | Semantic | Liveness and storage info |
| `semantic_store` | Semantic | Store a shared semantic triple |
| `semantic_recall` | Semantic | Recall shared semantic triples |
| `semantic_embed` | Semantic | Store an embedding vector |
| `semantic_search` | Semantic | KNN similarity search |
| `semantic_purge` | Semantic | Purge embeddings by prefix |
| `semantic_count` | Semantic | Triple and embedding counts |
| `memory_backup` | Memory | Backup memory database |
| `memory_restore` | Memory | Restore memory database |

---

### docproc

**Crate:** `hkask-mcp-docproc` · **Loop:** L2 · **Tools:** 9

Unified document processing server — supersedes former `hkask-mcp-markitdown` and `hkask-mcp-doc-knowledge` (2026-06-13).

**Credentials:** `HKASK_OCR_MODEL` (optional, for OCR)

| Tool | Description |
|------|-------------|
| `docproc_convert` | Extract text from documents (PDF/MD/HTML/TXT) with OCR fallback |
| `docproc_ocr` | Explicit OCR using vision model |
| `docproc_chunk` | Chunk text or file into passages (single or multi-tier), auto-indexes for query |
| `docproc_extract_triples` | Extract RDF triples from text via LLM |
| `docproc_embed` | Generate embedding vectors for passages or triples |
| `docproc_generate_qa` | Generate QA pairs from text via LLM |
| `docproc_cache` | Cache processed text to ~/.config/hkask/docproc-cache/ |
| `docproc_query` | Search indexed passages by natural language query, optionally generate LLM answer |
| `docproc_clear_index` | Reset the vector index for a new document set |

**Pipeline flow:**
```
convert → chunk (auto-index) → query → (generate answer)
              ↘ extract_triples → embed
              ↘ generate_qa → training_ingest_qa
              ↘ cache
```

---

### training

**Crate:** `hkask-mcp-training` · **Loop:** L2 · **Tools:** 1

Stub server for model training data ingestion (2026-06-13). Full training pipeline (dataset assembly, formatting, fine-tuning) planned for future release.

**Credentials:** `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE`

| Tool | Description |
|------|-------------|
| `training_ingest_qa` | Ingest QA pairs for model training |

---

## Verification Notes

- **Count method:** `grep '#\[tool' mcp-servers/*/src/main.rs` for `#[tool]`-based servers.
- **Consolidation (2026-06-11):** `hkask-mcp-web` (4 tools) + `hkask-mcp-rss-reader` (~10 tools) → `hkask-mcp-research` (~17 tools).
- **Consolidation (2026-06-13):** `hkask-mcp-markitdown` (3 tools) + `hkask-mcp-doc-knowledge` (5 tools) → `hkask-mcp-docproc` (8 tools). Added `hkask-mcp-training` (1 tool, stub).
- **New (2026-06-11):** `hkask-mcp-replica` added (6 tools, style embedding and composition).
- **Spec server correction:** Previous inventory listed 11 spec tools. Only 5 exist per MDS.md §3 and code verification.
- **Total:** ~86 tools across 12 servers.
