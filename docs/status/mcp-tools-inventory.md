---
title: "MCP Tools Inventory"
version: "1.1.0"
last_updated: 2026-06-09
status: Active
domain: "Cross-cutting"
---

# MCP Tools Inventory

Complete catalog of all 10 hKask MCP servers, their tools, gas costs, credentials, and loop assignments.

## Summary

| Server | Crate | Tools | Gas Cost | Loop | Required Credentials | LOC |
|--------|-------|-------|----------|------|----------------------|-----|
| condenser | `hkask-mcp-condenser` | 7 | 10 (thread_summary=25) | L2 (Episodic) | — | 1,790 |
| web | `hkask-mcp-web` | 4 | 50 | L4 (Communication) | — | 3,180 |
| spec | `hkask-mcp-spec` | 11 | 5 | L5 (Curation) | `HKASK_OCAP_SECRET` | 2,576 |
| fmp | `hkask-mcp-fmp` | 10 | 40 | L4 (Communication) | `HKASK_FMP_API_KEY` | 367 |
| telnyx | `hkask-mcp-telnyx` | 7 | 50 | L4 (Communication) | `HKASK_TELNYX_API_KEY` | 240 |
| fal | `hkask-mcp-fal` | 9 | 100 | L4 (Communication) | `HKASK_FAL_API_KEY` | 414 |
| rss-reader | `hkask-mcp-rss-reader` | 10 | 20 | L4 (Communication) | — | 1,408 |
| memory | `hkask-mcp-memory` | 14 | 5 | L2 (Episodic + Semantic) | `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE` | 656 |
| doc-knowledge | `hkask-mcp-doc-knowledge` | 5 | 5 | L2 (Episodic) | — | 747 |
| markitdown | `hkask-mcp-markitdown` | 3 | 5 | L2 (Episodic) | — | 698 |

**Totals:** 10 servers, 80 tools, 12,076 LOC

## Per-Server Detail

### condenser

**Crate:** `hkask-mcp-condenser` · **Loop:** L2 (Episodic) · **Gas:** 10 (per-tool: `condenser_thread_summary`=25) · **LOC:** 1,970

**Credentials:** All optional. `HKASK_DB_PATH` + `HKASK_DB_PASSPHRASE` for persistence; `INFERENCE_URL` + `INFERENCE_MODEL` + `INFERENCE_API_KEY` for thread summarization (legacy `OKAPI_*` aliases also accepted). `INFERENCE_TIMEOUT_SECS` for timeout (default: 30s).

| Tool | Description | Requires |
|------|-------------|----------|
| `condenser_ping` | Liveness and profile info | — |
| `condenser_compress` | Compress tool output using context-aware algorithms | — |
| `condenser_set_profile` | Set compression profile (heavy/normal/soft/light) | — |
| `condenser_stats` | Cumulative compression statistics | — |
| `condenser_classify` | Classify tool name to context category | — |
| `condenser_persist` | Persist a compressed output to episodic memory | DB credentials |
| `condenser_thread_summary` | Summarize conversation history using a local inference engine | INFERENCE_URL |

---

### web

**Crate:** `hkask-mcp-web` · **Loop:** L4 (Communication) · **Gas:** 50 · **LOC:** 3,180

**Credentials:** All optional. `HKASK_BRAVE_API_KEY`, `HKASK_FIRECRAWL_API_KEY`, `HKASK_TAVILY_API_KEY`, `HKASK_SERPAPI_API_KEY`, `HKASK_EXA_API_KEY`, `HKASK_BROWSERBASE_API_KEY`.

| Tool | Description |
|------|-------------|
| `web_ping` | Liveness and provider health check |
| `web_find_similar` | Find pages similar to a given URL using Exa findSimilar |
| `web_scrape` | Extract content from a URL into markdown or structured JSON |
| `web_interact` | Interactive browsing of JS-heavy pages via headless browser |

---

### spec

**Crate:** `hkask-mcp-spec` · **Loop:** L5 (Curation) · **Gas:** 5 · **LOC:** 2,576

**Required:** `HKASK_OCAP_SECRET`. Optional `HKASK_SPEC_DB_PATH`, `HKASK_DB_PASSPHRASE`.

| Tool | Description |
|------|-------------|
| `spec_goal_capture` | Capture a specification goal |
| `spec_goal_decompose` | Decompose a specification goal into ordered sub-goals (max depth 7) |
| `spec_require_bind` | Bind OCAP boundaries to a specification goal as a constraint |
| `spec_curate_evaluate` | Evaluate a specification artifact against curation gradient |
| `spec_curate_reconcile` | Reconcile conflicting specification artifacts |
| `spec_curate_cultivate` | Cultivate a specification artifact for quality improvement |
| `spec_curate_writing_excellence` | Evaluate writing quality against excellence criteria |
| `spec_graph_query` | Query the specification document graph by category or domain anchor |
| `spec_graph_validate` | Validate the specification document graph for consistency |
| `spec_test_invariant` | Register a test invariant for a specification seam |
| `spec_test_verify` | Verify test coverage against specification invariants |

---

### fmp

**Crate:** `hkask-mcp-fmp` · **Loop:** L4 (Communication) · **Gas:** 40 · **LOC:** 367

**Required:** `HKASK_FMP_API_KEY`

| Tool | Description |
|------|-------------|
| `fmp_ping` | Ping FMP API |
| `fmp_company_profile` | Get company profile |
| `fmp_quote` | Get stock quote |
| `fmp_income_statement` | Get income statement |
| `fmp_balance_sheet` | Get balance sheet |
| `fmp_cash_flow` | Get cash flow statement |
| `fmp_key_metrics` | Get key metrics |
| `fmp_historical_price` | Get historical price data |
| `fmp_symbol_search` | Search for symbols |
| `fmp_analyst_estimates` | Get analyst estimates |
| `fmp_dcf` | Get discounted cash flow analysis |

---

### telnyx

**Crate:** `hkask-mcp-telnyx` · **Loop:** L4 (Communication) · **Gas:** 50 · **LOC:** 240

**Required:** `HKASK_TELNYX_API_KEY`

| Tool | Description |
|------|-------------|
| `telnyx_ping` | Ping Telnyx API |
| `telnyx_list_numbers` | List phone numbers |
| `telnyx_buy_number` | Buy a phone number |
| `telnyx_send_sms` | Send an SMS |
| `telnyx_make_call` | Make a phone call |
| `telnyx_send_whatsapp` | Send a WhatsApp message |
| `telnyx_list_voices` | List available TTS voices |

---

### fal

**Crate:** `hkask-mcp-fal` · **Loop:** L4 (Communication) · **Gas:** 100 · **LOC:** 414

**Required:** `HKASK_FAL_API_KEY`

| Tool | Description |
|------|-------------|
| `fal_ping` | Ping Fal.ai API to verify connectivity and authentication |
| `fal_generate_image` | Generate an image from a prompt |
| `fal_transform_image` | Transform an image with a prompt |
| `fal_upscale_image` | Upscale an image |
| `fal_generate_video` | Generate a video from a prompt |
| `fal_generate_music` | Generate music from a prompt |
| `fal_transcribe_audio` | Transcribe audio to text |
| `fal_caption_image` | Generate a caption for an image |
| `fal_generate_3d` | Generate a 3D model from an image |

---

### rss-reader

**Crate:** `hkask-mcp-rss-reader` · **Loop:** L4 (Communication) · **Gas:** 20 · **LOC:** 1,408

**Credentials:** Optional `HKASK_RSS_DB`, `HKASK_DB_PASSPHRASE`.

| Tool | Description |
|------|-------------|
| `rss_subscribe` | Subscribe to an RSS/Atom feed (Google Reader stream model) |
| `rss_unsubscribe` | Unsubscribe from a feed |
| `rss_list_subscriptions` | List subscriptions, optionally filtered by folder |
| `rss_fetch` | Fetch/sync new entries from a feed (supports ETag/Last-Modified) |
| `rss_mark_read` | Mark all entries in a stream as read |
| `rss_unread_count` | Get unread count for a stream |
| `rss_search` | Full-text search across feed entries |
| `rss_export_opml` | Export subscriptions as OPML 2.0 |
| `rss_import_opml` | Import subscriptions from OPML content |
| `rss_discover` | Discover RSS/Atom feeds from a URL via HTML link autodiscovery |

---

### memory

**Crate:** `hkask-mcp-memory` · **Loop:** L2 (Episodic + Semantic) · **Gas:** 5 · **LOC:** 656

**Required:** `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE`

Consolidation of former `hkask-mcp-episodic` and `hkask-mcp-semantic` servers. 14 tools total (5 episodic + 9 semantic).

| Tool | Subsystem | Description |
|------|-----------|-------------|
| `episodic_ping` | Episodic | Liveness and storage info for episodic memory |
| `episodic_store` | Episodic | Store an episodic triple (private, perspective-bound) |
| `episodic_recall` | Episodic | Recall episodic triples by entity (filtered by caller's WebID) |
| `episodic_budget` | Episodic | Storage usage and budget for episodic memory |
| `semantic_ping` | Semantic | Liveness and storage info for semantic memory |
| `semantic_store` | Semantic | Store a shared semantic triple (no perspective) |
| `semantic_recall` | Semantic | Recall shared semantic triples by entity |
| `semantic_embed` | Semantic | Store an embedding vector for similarity search |
| `semantic_search` | Semantic | KNN similarity search over embeddings |
| `semantic_delete_prefix` | Semantic | Delete all embeddings whose entity_ref starts with a prefix |
| `semantic_budget` | Semantic | Triple and embedding counts for semantic memory |

---

### doc-knowledge

**Crate:** `hkask-mcp-doc-knowledge` · **Loop:** L2 (Episodic) · **Gas:** 5 · **LOC:** 747

**Credentials:** Optional `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE`.

| Tool | Description |
|------|-------------|
| `doc_knowledge_ping` | Liveness check for doc-knowledge server |
| `doc_knowledge_detect_format` | Detect document format from path/extension |
| `doc_knowledge_extract_markdown` | Extract text and image refs from markdown |
| `doc_knowledge_parse` | Parse document into IR with multi-tier chunking (coarse/medium/fine) |
| `doc_knowledge_store_qa` | Store QA items with provenance |

---

### markitdown

**Crate:** `hkask-mcp-markitdown` · **Loop:** L2 (Episodic) · **Gas:** 5 · **LOC:** 698

**Credentials:** Optional `HKASK_OCR_MODEL`, `OKAPI_BASE_URL`.

| Tool | Description |
|------|-------------|
| `markitdown_extract_text` | Extract text from a document with automatic OCR fallback for scanned PDFs |
| `markitdown_detect_format` | Detect the document format from a file path/extension |
| `markitdown_ocr` | OCR a document using a local vision model |
