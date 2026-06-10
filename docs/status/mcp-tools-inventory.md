---
title: "MCP Tools Inventory"
version: "2.0.0"
last_updated: 2026-06-10
status: Active
domain: "Cross-cutting"
generated_from: "grep '#\\[tool' across mcp-servers/*/src/"
---

# MCP Tools Inventory

Catalog of all 10 hKask MCP servers and their tools.
Re-derived from `grep '#\[tool'` on 2026-06-10.

---

## Summary

| Server | Crate | Tools | Loop | Required Credentials |
|--------|-------|-------|------|----------------------|
| condenser | `hkask-mcp-condenser` | 6 | L2 (Episodic) | — |
| web | `hkask-mcp-web` | 4 | L4 (Communication) | — |
| spec | `hkask-mcp-spec` | 5 | L5 (Curation) | `HKASK_OCAP_SECRET` |
| fmp | `hkask-mcp-fmp` | 11 | L4 (Communication) | `HKASK_FMP_API_KEY` |
| telnyx | `hkask-mcp-telnyx` | 7 | L4 (Communication) | `HKASK_TELNYX_API_KEY` |
| fal | `hkask-mcp-fal` | 9 | L4 (Communication) | `HKASK_FAL_API_KEY` |
| rss-reader | `hkask-mcp-rss-reader` | ~10 | L4 (Communication) | — |
| memory | `hkask-mcp-memory` | 13 | L2 (Episodic + Semantic) | `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE` |
| doc-knowledge | `hkask-mcp-doc-knowledge` | 5 | L2 (Episodic) | — |
| markitdown | `hkask-mcp-markitdown` | ~3 | L2 (Episodic) | — |
| **Total** | | **~73** | | |

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

### web

**Crate:** `hkask-mcp-web` · **Loop:** L4 · **Tools:** 4

**Credentials:** `HKASK_BRAVE_API_KEY`, `HKASK_FIRECRAWL_API_KEY`, `HKASK_TAVILY_API_KEY`, `HKASK_SERPAPI_API_KEY`, `HKASK_EXA_API_KEY` (all optional)

| Tool | Description |
|------|-------------|
| `web_ping` | Liveness and provider health check |
| `web_find_similar` | Find pages similar to a given URL |
| `web_extract` | Extract content from a URL into markdown |
| `web_browse` | Interactive browsing of JS-heavy pages |

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

### fmp

**Crate:** `hkask-mcp-fmp` · **Loop:** L4 · **Tools:** 11

**Required:** `HKASK_FMP_API_KEY`

| Tool | Description |
|------|-------------|
| `fmp_ping` | Ping FMP API |
| `fmp_company_profile` | Get company profile |
| `fmp_quote` | Get stock quote |
| `fmp_income_statement` | Get income statement |
| `fmp_balance_sheet` | Get balance sheet |
| `fmp_cash_flow_statement` | Get cash flow statement |
| `fmp_key_metrics` | Get key metrics |
| `fmp_historical_price` | Get historical price data |
| `fmp_search` | Search for symbols |
| `fmp_analyst_estimates` | Get analyst estimates |
| `fmp_dcf` | Get discounted cash flow analysis |

---

### telnyx

**Crate:** `hkask-mcp-telnyx` · **Loop:** L4 · **Tools:** 7

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

**Crate:** `hkask-mcp-fal` · **Loop:** L4 · **Tools:** 9

**Required:** `HKASK_FAL_API_KEY`

| Tool | Description |
|------|-------------|
| `fal_ping` | Ping Fal.ai API |
| `fal_generate_image` | Generate an image from a prompt |
| `fal_image_to_image` | Transform an image with a prompt |
| `fal_upscale` | Upscale an image |
| `fal_generate_video` | Generate a video from a prompt |
| `fal_generate_music` | Generate music from a prompt |
| `fal_whisper` | Transcribe audio to text |
| `fal_caption` | Generate a caption for an image |
| `fal_generate_3d` | Generate a 3D model |

---

### rss-reader

**Crate:** `hkask-mcp-rss-reader` · **Loop:** L4 · **Tools:** ~10

Uses manual tool registration via `run_server` list, not `#[tool]` macros. Credentials optional (`HKASK_RSS_DB`, `HKASK_DB_PASSPHRASE`).

| Tool | Description |
|------|-------------|
| `rss_subscribe` | Subscribe to an RSS/Atom feed |
| `rss_unsubscribe` | Unsubscribe from a feed |
| `rss_list_subscriptions` | List subscriptions |
| `rss_fetch` | Fetch/sync new entries |
| `rss_mark_read` | Mark entries as read |
| `rss_unread_count` | Get unread count |
| `rss_search` | Full-text search across entries |
| `rss_export_opml` | Export subscriptions as OPML 2.0 |
| `rss_import_opml` | Import subscriptions from OPML |
| `rss_discover` | Discover feeds from URL via HTML autodiscovery |

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

### doc-knowledge

**Crate:** `hkask-mcp-doc-knowledge` · **Loop:** L2 · **Tools:** 5

**Credentials:** Optional (`HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE`)

| Tool | Description |
|------|-------------|
| `doc_knowledge_ping` | Liveness check |
| `doc_knowledge_detect_format` | Detect document format from extension |
| `doc_knowledge_extract_markdown` | Extract text and image refs from markdown |
| `doc_knowledge_parse` | Parse document with multi-tier chunking |
| `doc_knowledge_store_qa` | Store QA items with provenance |

---

### markitdown

**Crate:** `hkask-mcp-markitdown` · **Loop:** L2 · **Tools:** ~3

Uses manual tool registration. Credentials optional (`HKASK_OCR_MODEL`, `OKAPI_BASE_URL`).

| Tool | Description |
|------|-------------|
| `markitdown_extract_text` | Extract text with automatic OCR fallback |
| `markitdown_detect_format` | Detect format from file path |
| `markitdown_ocr` | OCR using a local vision model |

---

## Verification Notes

- **Count method:** `grep '#\[tool' mcp-servers/*/src/main.rs` for `#[tool]`-based servers. rss-reader and markitdown use manual `run_server` registration — tool counts estimated from registration lists.
- **Spec server correction:** Previous inventory listed 11 spec tools. Only 5 exist per MDS.md §3 and code verification. The six extra tools (`bind`, `evaluate`, `reconcile`, `cultivate`, `graph_validate`, `test_invariant`, `test_verify`) were either deleted or never existed.
- **Total:** ~73 tools across 10 servers (down from previously claimed 80).
