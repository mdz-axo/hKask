---
title: "MCP Tools Inventory"
audience: [architects, developers, agents]
last_updated: 2026-06-04
version: "1.0.0"
status: "Active"
domain: "Capability"
ddmvss_categories: [capability, interface]
---

# MCP Tools Inventory

**Complete catalog of all MCP tools across all 21 hKask MCP servers.**

**Version:** hKask v0.23.00
**Total servers:** 21
**Total tools:** 123

---

## Summary by Server

| # | Server | Tools | Gas Cost | Domain |
|---|--------|-------|----------|--------|
| 1 | `hkask-mcp-ocap` | 5 | 1 | Capability management |
| 2 | `hkask-mcp-cns` | 6 | 1 | Observability |
| 3 | `hkask-mcp-keystore` | 5 | 2 | Secret management |
| 4 | `hkask-mcp-registry` | 6 | 2 | Template registry |
| 5 | `hkask-mcp-ensemble` | 5 | 2 | Multi-agent coordination |
| 6 | `hkask-mcp-episodic` | 4 | 5 | Episodic memory |
| 7 | `hkask-mcp-semantic` | 6 | 5 | Semantic memory |
| 8 | `hkask-mcp-goal` | 3 | 5 | Goal coordination |
| 9 | `hkask-mcp-spec` | 8 | 5 | DDMVSS specification |
| 10 | `hkask-mcp-git` | 5 | 5 | Git CAS operations |
| 11 | `hkask-mcp-replicant` | 3 | 5 | Replicant chat bridge |
| 12 | `hkask-mcp-condenser` | 6 | 10 | Context condensation |
| 13 | `hkask-mcp-rss-reader` | 12 | 20 | RSS feed management |
| 14 | `hkask-mcp-github` | 8 | 30 | GitHub API |
| 15 | `hkask-mcp-fmp` | 11 | 40 | Financial data (FMP) |
| 16 | `hkask-mcp-web` | 5 | 50 | Web search (SSRF-protected) |
| 17 | `hkask-mcp-telnyx` | 7 | 50 | SMS/voice (Telnyx) |
| 18 | `hkask-mcp-fal` | 9 | 100 | Media generation (FAL) |
| 19 | `hkask-mcp-inference` | 4 | 0* | Okapi LLM inference |
| 20 | `hkask-mcp-doc-knowledge` | 4 | 5 | Document parsing and chunking |
| 21 | `hkask-mcp-markitdown` | 3 | 10 | Document format conversion and OCR |

\* Inference gas cost is overridden by `InferenceGasEstimator` (token-based).

Gas costs from `crates/hkask-cns/src/table_gas_estimator.rs`. Lower = cheaper internal tools; higher = external API calls with cost/rate implications.

---

## Internal Tools (Gas 1–5)

These tools operate on hKask internal state — no external API calls, no rate limits, no cost.

### `hkask-mcp-ocap` — Capability Management (5 tools)

| Tool | Description |
|------|-------------|
| `ocap_delegate` | Create a delegated capability token with real HMAC signature |
| `ocap_verify` | Verify a capability token with real cryptographic HMAC verification |
| `ocap_revoke` | Revoke a capability token by adding to revocation set |
| `ocap_enumerate` | Enumerate capabilities for a subject |
| `ocap_list_tokens` | List all capability tokens |

### `hkask-mcp-cns` — Observability (6 tools)

| Tool | Description |
|------|-------------|
| `cns_emit` | Emit a CNS observation event |
| `cns_variety` | Get variety count for a span pattern via real VarietyMonitor |
| `cns_alert` | Trigger a real algedonic alert via AlgedonicManager |
| `cns_calibrate` | Calibrate a span threshold |
| `cns_list_alerts` | List active algedonic alerts from real alert manager |
| `cns_health` | Get real CNS health status |

### `hkask-mcp-keystore` — Secret Management (5 tools)

| Tool | Description |
|------|-------------|
| `keystore_set` | Set a key-value pair in the keystore with AES-256-GCM encryption |
| `keystore_get` | Get a value from the keystore (capability-gated: only owner pod can read) |
| `keystore_rotate` | Rotate a key-value pair with re-encryption |
| `keystore_delete` | Delete a key from the keystore (capability-gated) |
| `keystore_list` | List all keys in the keystore |

### `hkask-mcp-registry` — Template Registry (6 tools)

| Tool | Description |
|------|-------------|
| `registry_index` | Index templates from a root path via real registry |
| `registry_discover` | Discover templates by type and domain via real registry search |
| `registry_validate` | Validate a template via real registry lookup |
| `registry_reload` | Reload templates from a path |
| `registry_compose` | Compose templates with cascade |
| `registry_get` | Get a template by ID via real registry lookup |

### `hkask-mcp-ensemble` — Multi-Agent Coordination (5 tools)

| Tool | Description |
|------|-------------|
| `coordinate_session` | Create a standing session from a YAML config path |
| `register_participant` | Register a bot participant in a session |
| `send_message` | Send a message to a standing session |
| `get_status` | Get standing session status |
| `improv_turn` | Execute an improvisation turn in a session |

### `hkask-mcp-episodic` — Episodic Memory (4 tools)

| Tool | Description |
|------|-------------|
| `episodic_ping` | Liveness and storage info for episodic memory |
| `episodic_store` | Store an episodic triple (private, perspective-bound) |
| `episodic_recall` | Recall episodic triples by entity (filtered by caller's WebID) |
| `episodic_budget` | Storage usage and budget for episodic memory |

### `hkask-mcp-semantic` — Semantic Memory (6 tools)

| Tool | Description |
|------|-------------|
| `semantic_ping` | Liveness and storage info for semantic memory |
| `semantic_store` | Store a shared semantic triple (no perspective) |
| `semantic_recall` | Recall shared semantic triples by entity |
| `semantic_embed` | Store an embedding vector for similarity search |
| `semantic_search` | KNN similarity search over embeddings |
| `semantic_count` | Triple and embedding counts for semantic memory |

### `hkask-mcp-goal` — Goal Coordination (3 tools)

| Tool | Description |
|------|-------------|
| `goal_create` | Create a goal owned by the calling agent (OCAP-gated) |
| `goal_list` | List the calling agent's goals, optionally filtered by state |
| `goal_set_state` | Transition a goal to a new state (legal transitions only) |

### `hkask-mcp-spec` — DDMVSS Specification (8 tools)

| Tool | Description |
|------|-------------|
| `spec_goal_capture` | Capture a goal as a binding specification requirement |
| `spec_goal_decompose` | Decompose a goal into ordered sub-goals (max depth 7) |
| `spec_require_bind` | Bind OCAP boundaries to a goal as a constraint |
| `spec_curate_evaluate` | Evaluate a specification for collection coherence |
| `spec_curate_reconcile` | Reconcile tensions between specifications without collapsing them |
| `spec_curate_cultivate` | Cultivate the specification collection toward coherence |
| `spec_graph_query` | Query the specification graph by category or domain anchor |
| `spec_graph_validate` | Validate the full specification collection for coherence and completeness |

### `hkask-mcp-git` — Git CAS Operations (5 tools)

| Tool | Description |
|------|-------------|
| `git_resolve` | Resolve a git reference to a SHA |
| `git_snapshot` | Create a git snapshot (commit) |
| `git_clone` | Clone a git repository |
| `git_diff` | Show diff between two commits |
| `git_list` | List files in a git path |

### `hkask-mcp-replicant` — Replicant Chat Bridge (2 tools)

| Tool | Description |
|------|-------------|
| `replicant_chat` | Send a message to a hKask replicant and receive a response. Persona configured via `HKASK_AGENT_PERSONA` (default: Curator). Optional model override per request. |
| `replicant_status` | Check registration status and identity of the configured replicant |
| `replicant_history` | List recent conversation turns in the current session (session persistence across calls) |

**Architecture:** Bridges external MCP clients (Zed, VS Code) with hKask's pod-mediated inference. Resolves persona → WebID, creates pod via `PodManagerBuilder`, routes through `InferencePort`. See `docs/status/mcp-server-audit.md` §Architecture Spotlight for full diagram.

---

## Moderate-Cost Tools (Gas 10)

### `hkask-mcp-condenser` — Context Condensation (6 tools)

> **Context** is **condensed**; **memory** is **consolidated**. The condenser operates on the ephemeral conversation/tool-output window, not on persistent episodic/semantic triples.

| Tool | Description |
|------|-------------|
| `condenser_ping` | Liveness and profile info |
| `condenser_compress` | Compress tool output using context-aware algorithms |
| `condenser_set_profile` | Set compression profile (heavy/normal/soft/light) |
| `condenser_stats` | Cumulative compression statistics |
| `condenser_classify` | Classify tool name to context category |
| `condenser_persist` | Persist a compressed output to episodic memory |

---

## External API Tools (Gas 20–100)

These tools call external services — rate limits, API keys, and costs apply.

### `hkask-mcp-rss-reader` — RSS Feed Management (12 tools)

| Tool | Description |
|------|-------------|
| `rss_subscribe` | Subscribe to an RSS/Atom feed (Google Reader stream model) |
| `rss_unsubscribe` | Unsubscribe from a feed |
| `rss_list_subscriptions` | List subscriptions, optionally filtered by folder |
| `rss_fetch` | Fetch/sync new entries from a feed (supports ETag/Last-Modified) |
| `rss_get_entries` | Get entries from a stream |
| `rss_mark_all_read` | Mark all entries in a stream as read |
| `rss_get_unread_count` | Get unread count for a stream |
| `rss_search` | Full-text search across feed entries |
| `rss_export_opml` | Export subscriptions as OPML 2.0 |
| `rss_import_opml` | Import subscriptions from OPML content |
| `rss_discover_feeds` | Discover RSS/Atom feeds from a URL via HTML link autodiscovery |
| `rss_edit_tag` | Edit tags on entries: mark read/unread, star/unstar, add/remove labels |

### `hkask-mcp-github` — GitHub API (8 tools)

| Tool | Description |
|------|-------------|
| `github_get_repo` | Get repository information |
| `github_list_issues` | List issues in a repository |
| `github_get_issue` | Get a specific issue |
| `github_create_issue` | Create a new issue |
| `github_add_comment` | Add a comment to an issue or PR |
| `github_list_prs` | List pull requests |
| `github_get_pr` | Get a specific pull request |
| `github_search_repos` | Search repositories |

**Credential:** `HKASK_GITHUB_TOKEN` (required)

### `hkask-mcp-fmp` — Financial Data (11 tools)

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

**Credential:** `HKASK_FMP_API_KEY` (required)

### `hkask-mcp-web` — Web Search (5 tools)

| Tool | Description |
|------|-------------|
| `web_ping` | Liveness and provider health check |
| `web_search` | Search the web with RRF fusion across providers |
| `web_find_similar` | Find pages similar to a given URL using Exa findSimilar |
| `web_extract` | Extract content from a URL into markdown or structured JSON |
| `web_browse` | Interactive browsing of JS-heavy pages via headless browser |

**Security:** SSRF protection (private IP/loopback rejection). `HKASK_WEB_*` provider keys.

### `hkask-mcp-telnyx` — SMS/Voice (7 tools)

| Tool | Description |
|------|-------------|
| `telnyx_ping` | Ping Telnyx API |
| `telnyx_list_numbers` | List phone numbers |
| `telnyx_buy_number` | Buy a phone number |
| `telnyx_send_sms` | Send an SMS |
| `telnyx_make_call` | Make a phone call |
| `telnyx_send_whatsapp` | Send a WhatsApp message |
| `telnyx_list_voices` | List available TTS voices |

**Credential:** `HKASK_TELNYX_API_KEY` (required)

### `hkask-mcp-fal` — Media Generation (9 tools)

| Tool | Description |
|------|-------------|
| `fal_ping` | Ping Fal.ai API to verify connectivity and authentication |
| `fal_generate_image` | Generate an image from a prompt |
| `fal_image_to_image` | Transform an image with a prompt |
| `fal_upscale` | Upscale an image |
| `fal_generate_video` | Generate a video from a prompt |
| `fal_generate_music` | Generate music from a prompt |
| `fal_whisper` | Transcribe audio to text |
| `fal_caption` | Generate a caption for an image |
| `fal_generate_3d` | Generate a 3D model from an image |

**Credential:** `HKASK_FAL_API_KEY` (required)

---

## Inference (Special Gas Model)

### `hkask-mcp-inference` — Okapi LLM (4 tools)

| Tool | Description |
|------|-------------|
| `inference_generate` | Generate text using Okapi-backed LLM inference. Model selection with automatic failover. |
| `inference_generate_vision` | Generate text from images (vision/multimodal) via Okapi. Requires a vision-capable model. |
| `inference_metrics` | Get current inference metrics (requests, tokens, errors, failovers) |
| `inference_models` | List available model tiers and their configurations |

**Gas model:** Overridden by `InferenceGasEstimator` — cost is token-based, not flat-rate. Table entry is `0` as sentinel.

---

## Document Processing

### `hkask-mcp-doc-knowledge` — Document Parsing & Chunking (4 tools)

| Tool | Description |
|------|-------------|
| `doc_knowledge_detect_format` | Detect document format from path/extension |
| `doc_knowledge_parse` | Parse document into IR with multi-tier chunking |
| `doc_knowledge_chunk` | Chunk text into segments (coarse/medium/fine) |
| `doc_knowledge_extract_html` | Extract text from HTML, removing script/style tags |

**Credential:** `HKASK_SPEC_DB_PATH` + `HKASK_DB_PASSPHRASE` (SQLCipher)

---

### `hkask-mcp-markitdown` — Document Conversion & OCR (3 tools)

| Tool | Description |
|------|-------------|
| `markitdown_convert` | Extract text from document with automatic OCR fallback for scanned PDFs |
| `markitdown_detect_format` | Detect document format from path/extension |
| `markitdown_ocr` | Explicitly OCR a document using local vision model (requires `HKASK_OCR_MODEL`) |

**Credential:** `HKASK_OCR_MODEL` (optional, required for OCR), `OKAPI_BASE_URL` (optional, default `http://127.0.0.1:11435`)

---

## Tool Count Distribution

```
  12 │                    █  rss-reader
  11 │                    █  
  9  │              █     █  
  8  │    █         █     █  
  7  │    █    █    █     █  
  6  │ ▅  █ ▅  █ ▅ █  ▅  █ ▅
  5  │ █  █ █  █ █ █  █  █ █
  4  │ █  █ █  █ █ █  █  █ █  █
  3  │ █  █ █  █ █ █  █  █ █  █  █
  2  │ █  █ █  █ █ █  █  █ █  █  █  █  █  █
  1  │ █  █ █  █ █ █  █  █ █  █  █  █  █  █  █  █  █  █  █
     └──────────────────────────────────────────────────────
      oc cp ns ke re en ep se go sp gi re co rs gi fm te we fa in
```

Servers ordered by gas cost (cheapest to most expensive). Tool count on Y axis.

---

## Credential Requirements

| Server | Required Credential | Description |
|--------|-------------------|-------------|
| `hkask-mcp-github` | `HKASK_GITHUB_TOKEN` | GitHub personal access token |
| `hkask-mcp-fmp` | `HKASK_FMP_API_KEY` | Financial Modeling Prep API key |
| `hkask-mcp-telnyx` | `HKASK_TELNYX_API_KEY` | Telnyx API key |
| `hkask-mcp-fal` | `HKASK_FAL_API_KEY` | Fal.ai API key |
| `hkask-mcp-web` | `HKASK_WEB_*` | Web search provider keys (optional per provider) |
| `hkask-mcp-spec` | `HKASK_SPEC_DB_PATH` + `HKASK_DB_PASSPHRASE` | SQLCipher database path and passphrase |
| `hkask-mcp-doc-knowledge` | `HKASK_SPEC_DB_PATH` + `HKASK_DB_PASSPHRASE` | SQLCipher database path and passphrase |
| `hkask-mcp-markitdown` | `HKASK_OCR_MODEL` (optional) | Vision model for OCR; `OKAPI_BASE_URL` (optional) |

Servers without credential requirements: `ocap`, `cns`, `keystore`, `registry`, `ensemble`, `episodic`, `semantic`, `goal`, `git`, `replicant`, `condenser`, `rss-reader`, `inference`, `doc-knowledge` (uses spec DB passphrase if SQLCipher), `markitdown` (optional OCR model).

---

*ℏKask MCP Tools Inventory — 21 servers, 123 tools — v0.23.00*