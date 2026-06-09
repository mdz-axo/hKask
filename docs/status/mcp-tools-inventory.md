---
title: MCP Tools Inventory
version: "1.0.0"
last_updated: 2026-06-08
status: Active
---

# MCP Tools Inventory

## Summary

| Server | Crate | Tools | Gas Cost | Loop | Credentials | LOC |
|--------|-------|-------|----------|------|-------------|-----|
| inference | `hkask-mcp-inference` | 3 | 0 (token-based) | L1 | — | 324 |
| condenser | `hkask-mcp-condenser` | 7 | 10 (per-tool: condenser_thread_summary=25) | L2 | 5 optional | 1,745 |
| episodic | `hkask-mcp-episodic` | 5 | 5 | L2 | 2 required | 219 |
| semantic | `hkask-mcp-semantic` | 9 | 5 | L2b | 2 required | 437 |
| rss-reader | `hkask-mcp-rss-reader` | 12 | 20 | L2 | 2 optional | 1,408 |
| doc-knowledge | `hkask-mcp-doc-knowledge` | 7 | 10 (default) | L2 | 2 optional | 747 |
| markitdown | `hkask-mcp-markitdown` | 3 | 10 (default) | L2 | 2 optional | 698 |
| web | `hkask-mcp-web` | 5 | 50 | L4 | 8 optional | 3,180 |
| git | `hkask-mcp-git` | 6 | 5 | L4 | 1 optional | 324 |
| github | `hkask-mcp-github` | 8 | 30 | L4 | 1 required | 468 |
| fmp | `hkask-mcp-fmp` | 11 | 40 | L4 | 1 required | 367 |
| telnyx | `hkask-mcp-telnyx` | 7 | 50 | L4 | 1 required | 240 |
| fal | `hkask-mcp-fal` | 9 | 100 | L4 | 1 required | 414 |
| ensemble | `hkask-mcp-ensemble` | 6 | 2 | L4 | — | 391 |
| registry | `hkask-mcp-registry` | 6 | 2 | L1↔L5 (bridge) | 2 optional | 303 |
| spec | `hkask-mcp-spec` | 11 | 5 | L5 | 1 required, 2 optional | 2,576 |
| goal | `hkask-mcp-goal` | 3 | 5 | L5 | 2 optional | 209 |
| replicant | `hkask-mcp-replicant` | 3 | 5 | L5 | 3 optional | 815 |
| ocap | `hkask-mcp-ocap` | 5 | 1 | L6 | 1 required | 337 |
| keystore | `hkask-mcp-keystore` | 5 | 2 | L6 | 2 optional | 491 |
| cns | `hkask-mcp-cns` | 10 | 1 | L6 | 1 optional | 408 |

**Totals:** 21 servers · 133 tools · 14,802 LOC

---

## Per-Server Detail

### inference

**Crate:** `hkask-mcp-inference`
**Loop:** L1 (Inference)
**Gas Cost:** 0 (token-based — `InferenceGasEstimator`: `prompt_chars / 4 + max_tokens`)
**LOC:** 324

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| inference_generate | Generate text using Okapi-backed LLM inference. Supports model selection with automatic failover, temperature control, and token limits. Per-agent rate limiting is handled by the CNS throttle at the MCP dispatch layer. | — |
| inference_metrics | Get current inference metrics including total requests, tokens generated, error counts, and failover count. | — |
| inference_models | List available model tiers and their configurations. | — |

---

### condenser

**Crate:** `hkask-mcp-condenser`
**Loop:** L2 (Episodic)
**Gas Cost:** 10 (per-tool: condenser_thread_summary=25)
**LOC:** 1,745

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| condenser_ping | Liveness and profile info | — |
| condenser_compress | Compress tool output using context-aware algorithms | — |
| condenser_set_profile | Set compression profile (heavy/normal/soft/light) | — |
| condenser_stats | Cumulative compression statistics | — |
| condenser_classify | Classify tool name to context category | — |
| condenser_persist | Persist a compressed output to episodic memory | HKASK_DB_PATH, HKASK_DB_PASSPHRASE (optional) |
| condenser_thread_summary | Summarize conversation history using Okapi local inference for context compaction. Call when approaching context window limits to condense older messages. | OKAPI_URL (optional) |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_DB_PATH | optional | Path to the SQLite database for episodic persistence (in-memory if absent) |
| HKASK_DB_PASSPHRASE | optional | Passphrase for the database (required if HKASK_DB_PATH is set) |
| OKAPI_URL | optional | Okapi inference engine URL for thread summarization (e.g. http://127.0.0.1:11435) |
| OKAPI_MODEL | optional | Okapi model for summarization (default: qwen3:8b) |
| OKAPI_API_KEY | optional | Okapi API key if authentication is enabled |

---

### episodic

**Crate:** `hkask-mcp-episodic`
**Loop:** L2 (Episodic)
**Gas Cost:** 5
**LOC:** 219

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| episodic_ping | Liveness and storage info for episodic memory | — |
| episodic_store | Store an episodic triple (private, perspective-bound) | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |
| episodic_recall | Recall episodic triples by entity (filtered by caller's WebID) | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |
| episodic_budget | Storage usage and budget for episodic memory | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |
| episodic_consolidate_status | Check consolidation candidates and budget status for episodic→semantic promotion | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_MEMORY_DB | required | Path to per-agent memory database file (episodic + semantic) |
| HKASK_DB_PASSPHRASE | required | SQLCipher encryption passphrase |

---

### semantic

**Crate:** `hkask-mcp-semantic`
**Loop:** L2b (Semantic)
**Gas Cost:** 5
**LOC:** 437

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| semantic_ping | Liveness and storage info for semantic memory | — |
| semantic_store | Store a shared semantic triple (no perspective) | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |
| semantic_recall | Recall shared semantic triples by entity | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |
| semantic_embed | Store an embedding vector for similarity search | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |
| semantic_search | KNN similarity search over embeddings | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |
| semantic_centroid | Compute mean embedding vector (centroid) for embeddings matching a prefix | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |
| semantic_purge | Delete all embeddings whose entity_ref starts with a prefix | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |
| semantic_chunk | Chunk text into passages for embedding, with optional Gutenberg header stripping | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |
| semantic_count | Triple and embedding counts for semantic memory | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_MEMORY_DB | required | Path to per-agent memory database file (episodic + semantic) |
| HKASK_DB_PASSPHRASE | required | SQLCipher encryption passphrase |

---

### rss-reader

**Crate:** `hkask-mcp-rss-reader`
**Loop:** L2 (Episodic)
**Gas Cost:** 20
**LOC:** 1,408

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| rss_subscribe | Subscribe to an RSS/Atom feed (Google Reader stream model) | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |
| rss_unsubscribe | Unsubscribe from a feed (stream_id e.g. 'feed/http://...') | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |
| rss_list_subscriptions | List subscriptions, optionally filtered by folder | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |
| rss_fetch | Fetch/sync new entries from a feed (supports ETag/Last-Modified) | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |
| rss_get_entries | Get entries from a stream (Google Reader stream IDs: feed/*, user/-/state/*, user/-/label/*) | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |
| rss_mark_all_read | Mark all entries in a stream as read | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |
| rss_get_unread_count | Get unread count for a stream | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |
| rss_search | Full-text search across feed entries | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |
| rss_export_opml | Export subscriptions as OPML 2.0 | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |
| rss_import_opml | Import subscriptions from OPML content | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |
| rss_discover_feeds | Discover RSS/Atom feeds from a URL via HTML link autodiscovery | — |
| rss_edit_tag | Edit tags on entries: mark read/unread, star/unstar, add/remove labels (Google Reader edit-tag) | HKASK_RSS_DB, HKASK_DB_PASSPHRASE (optional) |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_RSS_DB | optional | Path to the RSS reader SQLite database (in-memory if absent) |
| HKASK_DB_PASSPHRASE | optional | Passphrase for SQLCipher encryption (required if HKASK_RSS_DB is set) |

---

### doc-knowledge

**Crate:** `hkask-mcp-doc-knowledge`
**Loop:** L2 (Episodic)
**Gas Cost:** 10 (default — not in gas table)
**LOC:** 747

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| doc_knowledge_ping | Liveness check for doc-knowledge server | — |
| doc_knowledge_chunk | Chunk text at configurable token granularity (delegates to SemanticMemory::chunk_text) | — |
| doc_knowledge_detect_format | Detect document format from path/extension | — |
| doc_knowledge_extract_markdown | Extract text and image refs from markdown | — |
| doc_knowledge_extract_html | Extract text from HTML. Removes script/style tags and preserves word boundaries for block-level elements. | — |
| doc_knowledge_parse | Parse document into IR with multi-tier chunking (coarse/medium/fine) | — |
| doc_knowledge_generate_qa | Generate QA prompt from text chunk (returns structured prompt for LLM; actual LLM call routed through hkask-mcp-inference) | — |
| doc_knowledge_store_qa | Store QA items with provenance | HKASK_MEMORY_DB, HKASK_DB_PASSPHRASE (optional) |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_MEMORY_DB | optional | Path to per-agent memory database for QA storage (in-memory if absent) |
| HKASK_DB_PASSPHRASE | optional | Passphrase for the database (required if HKASK_MEMORY_DB is set) |

---

### markitdown

**Crate:** `hkask-mcp-markitdown`
**Loop:** L2 (Episodic)
**Gas Cost:** 10 (default — not in gas table)
**LOC:** 698

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| markitdown_convert | Extract text from a document. Detects format, extracts text with automatic OCR fallback for scanned/image-based PDFs. For PDF: tries text extraction first, falls back to vision OCR if result is near-empty. For other supported formats (TXT, MD, HTML): extracts plain text. Requires HKASK_OCR_MODEL for OCR fallback. | HKASK_OCR_MODEL (optional) |
| markitdown_detect_format | Detect the document format from a file path/extension. Returns format name, whether text extraction is supported, and note for unsupported formats. | — |
| markitdown_ocr | OCR a document using a local vision model. Requires HKASK_OCR_MODEL env var or explicit model parameter. The model must be a vision-capable model available in the Okapi catalog. | HKASK_OCR_MODEL (optional) |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_OCR_MODEL | optional | Vision model for OCR (must exist in Okapi catalog). Required for OCR functionality. |
| OKAPI_BASE_URL | optional | Okapi API base URL (default: http://127.0.0.1:11435) |

---

### web

**Crate:** `hkask-mcp-web`
**Loop:** L4 (Communication)
**Gas Cost:** 50
**LOC:** 3,180

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| web_ping | Liveness and provider health check | — |
| web_search | Search the web with RRF fusion across providers. Strategy selects providers: quick (single keyword), web (all), news (news-capable), deep (all + rerank) | HKASK_BRAVE_API_KEY (optional) |
| web_find_similar | Find pages similar to a given URL using Exa findSimilar | HKASK_EXA_API_KEY (optional) |
| web_extract | Extract content from a URL into markdown or structured JSON | HKASK_FIRECRAWL_API_KEY (optional) |
| web_browse | Interactive browsing of JS-heavy pages via headless browser | HKASK_BROWSERBASE_API_KEY (optional) |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_BRAVE_API_KEY | optional | Brave Search API key for web search |
| HKASK_FIRECRAWL_API_KEY | optional | Firecrawl API key for web extract/browse |
| HKASK_TAVILY_API_KEY | optional | Tavily API key for AI-optimized web search |
| HKASK_SERPAPI_API_KEY | optional | SerpAPI API key for Google search results |
| HKASK_EXA_API_KEY | optional | Exa API key for neural/semantic web search |
| HKASK_BROWSERBASE_API_KEY | optional | Browserbase API key for headless browser browsing |
| HKASK_WEB_CACHE_TTL_SECS | optional | Cache TTL in seconds (default: 3600) |
| HKASK_WEB_CACHE_MAX_ENTRIES | optional | Maximum cache entries (default: 1000) |

---

### git

**Crate:** `hkask-mcp-git`
**Loop:** L4 (Communication)
**Gas Cost:** 5
**LOC:** 324

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| git_resolve | Resolve a git reference to a SHA | — |
| git_snapshot | Create a git snapshot (commit) | — |
| git_diff | Show diff between two commits | — |
| git_list | List files in a git tree | — |
| git_verify | Verify content integrity of a repository | — |
| git_log | List snapshot history for a repository | — |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_CAS_HOME | optional | Base path for Git CAS operations |

---

### github

**Crate:** `hkask-mcp-github`
**Loop:** L4 (Communication)
**Gas Cost:** 30
**LOC:** 468

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| github_get_repo | Get repository information | HKASK_GITHUB_TOKEN |
| github_list_issues | List issues in a repository | HKASK_GITHUB_TOKEN |
| github_get_issue | Get a specific issue | HKASK_GITHUB_TOKEN |
| github_create_issue | Create a new issue | HKASK_GITHUB_TOKEN |
| github_add_comment | Add a comment to an issue or PR | HKASK_GITHUB_TOKEN |
| github_list_prs | List pull requests | HKASK_GITHUB_TOKEN |
| github_get_pr | Get a specific pull request | HKASK_GITHUB_TOKEN |
| github_search_repos | Search repositories | HKASK_GITHUB_TOKEN |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_GITHUB_TOKEN | required | GitHub personal access token for API authentication |

---

### fmp

**Crate:** `hkask-mcp-fmp`
**Loop:** L4 (Communication)
**Gas Cost:** 40
**LOC:** 367

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| fmp_ping | Ping FMP API | HKASK_FMP_API_KEY |
| fmp_company_profile | Get company profile | HKASK_FMP_API_KEY |
| fmp_quote | Get stock quote | HKASK_FMP_API_KEY |
| fmp_income_statement | Get income statement | HKASK_FMP_API_KEY |
| fmp_balance_sheet | Get balance sheet | HKASK_FMP_API_KEY |
| fmp_cash_flow_statement | Get cash flow statement | HKASK_FMP_API_KEY |
| fmp_key_metrics | Get key metrics | HKASK_FMP_API_KEY |
| fmp_historical_price | Get historical price data | HKASK_FMP_API_KEY |
| fmp_search | Search for symbols | HKASK_FMP_API_KEY |
| fmp_analyst_estimates | Get analyst estimates | HKASK_FMP_API_KEY |
| fmp_dcf | Get discounted cash flow analysis | HKASK_FMP_API_KEY |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_FMP_API_KEY | required | Financial Modeling Prep API key |

---

### telnyx

**Crate:** `hkask-mcp-telnyx`
**Loop:** L4 (Communication)
**Gas Cost:** 50
**LOC:** 240

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| telnyx_ping | Ping Telnyx API | HKASK_TELNYX_API_KEY |
| telnyx_list_numbers | List phone numbers | HKASK_TELNYX_API_KEY |
| telnyx_buy_number | Buy a phone number | HKASK_TELNYX_API_KEY |
| telnyx_send_sms | Send an SMS | HKASK_TELNYX_API_KEY |
| telnyx_make_call | Make a phone call | HKASK_TELNYX_API_KEY |
| telnyx_send_whatsapp | Send a WhatsApp message | HKASK_TELNYX_API_KEY |
| telnyx_list_voices | List available TTS voices (static catalog from Telnyx docs) | HKASK_TELNYX_API_KEY |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_TELNYX_API_KEY | required | Telnyx API key for messaging and number management |

---

### fal

**Crate:** `hkask-mcp-fal`
**Loop:** L4 (Communication)
**Gas Cost:** 100
**LOC:** 414

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| fal_ping | Ping Fal.ai API to verify connectivity and authentication | HKASK_FAL_API_KEY |
| fal_generate_image | Generate an image from a prompt | HKASK_FAL_API_KEY |
| fal_image_to_image | Transform an image with a prompt | HKASK_FAL_API_KEY |
| fal_upscale | Upscale an image | HKASK_FAL_API_KEY |
| fal_generate_video | Generate a video from a prompt | HKASK_FAL_API_KEY |
| fal_generate_music | Generate music from a prompt | HKASK_FAL_API_KEY |
| fal_whisper | Transcribe audio to text | HKASK_FAL_API_KEY |
| fal_caption | Generate a caption for an image | HKASK_FAL_API_KEY |
| fal_generate_3d | Generate a 3D model from an image | HKASK_FAL_API_KEY |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_FAL_API_KEY | required | Fal.ai API key for AI image generation |

---

### ensemble

**Crate:** `hkask-mcp-ensemble`
**Loop:** L4 (Communication)
**Gas Cost:** 2
**LOC:** 391

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| ensemble_coordinate | Create a standing session from a YAML config path | — |
| ensemble_register | Register a bot participant in a session | — |
| ensemble_send | Send a message to a standing session | — |
| ensemble_status | Get standing session status | — |
| ensemble_improv | Prepare an improvisation turn prompt for external inference | — |
| ensemble_a2a | Structure an A2A message for dispatch between agents | — |

**Credential Requirements:** None

---

### registry

**Crate:** `hkask-mcp-registry`
**Loop:** L1↔L5 (bridge)
**Gas Cost:** 2
**LOC:** 303

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| registry_index | Index templates from a root path via real registry | HKASK_REGISTRY_DB, HKASK_DB_PASSPHRASE (optional) |
| registry_discover | Discover templates by type and domain via real registry search | HKASK_REGISTRY_DB, HKASK_DB_PASSPHRASE (optional) |
| registry_validate | Validate a template via real registry lookup | HKASK_REGISTRY_DB, HKASK_DB_PASSPHRASE (optional) |
| registry_reload | Reload templates from a path | HKASK_REGISTRY_DB, HKASK_DB_PASSPHRASE (optional) |
| registry_compose | Compose templates with cascade | HKASK_REGISTRY_DB, HKASK_DB_PASSPHRASE (optional) |
| registry_get | Get a template by ID via real registry lookup | HKASK_REGISTRY_DB, HKASK_DB_PASSPHRASE (optional) |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_REGISTRY_DB | optional | Path to registry SQLite database (encrypted with HKASK_DB_PASSPHRASE) |
| HKASK_DB_PASSPHRASE | optional | Passphrase for registry database encryption (required if HKASK_REGISTRY_DB is set) |

---

### spec

**Crate:** `hkask-mcp-spec`
**Loop:** L5 (Curation)
**Gas Cost:** 5
**LOC:** 2,576

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| spec_goal_capture | Capture a goal as a binding specification requirement in a spec document | HKASK_OCAP_SECRET (required) |
| spec_goal_decompose | Decompose a specification goal into ordered sub-goals (max depth 7) | HKASK_OCAP_SECRET (required) |
| spec_require_bind | Bind OCAP boundaries to a specification goal as a constraint | HKASK_OCAP_SECRET (required) |
| spec_curate_evaluate | Assess specification artifact against collection coherence. Evaluates spec-document completeness (internal consistency, cross-reference integrity, section coverage), not code-implementation status. When writing_excellence scores are provided, includes 4-perspective test results (Hopper/Lovelace/Schriver/Gentle) and accounts for publication standard (3 of 4 passing). | HKASK_OCAP_SECRET (required) |
| spec_curate_reconcile | Reconcile spec-domain tensions between specification documents without collapsing them | HKASK_OCAP_SECRET (required) |
| spec_curate_cultivate | Grow specification collection toward coherence. Suggestions target spec-document gaps (missing sections, unstated constraints), not code gaps. | HKASK_OCAP_SECRET (required) |
| spec_curate_writing_excellence | Assess a specification document against the Writing Excellence 4-perspective test (Hopper: accessibility, Lovelace: precision, Schriver: findability, Gentle: agent-correctness). Per WRITING_EXCELLENCE.md §3: 3 of 4 passing is the publication standard; 1 of 4 blocks publication. | HKASK_OCAP_SECRET (required) |
| spec_graph_query | Query the specification document graph by category or domain anchor | HKASK_OCAP_SECRET (required) |
| spec_graph_validate | Validate specification collection for internal consistency and spec-document coherence, not code-implementation completeness | HKASK_OCAP_SECRET (required) |
| spec_test_invariant | Create a test traceability record linking a test to a specification requirement | HKASK_OCAP_SECRET (required) |
| spec_test_verify | Verify test coverage for a specification seam or spec category, returning gaps and debt | HKASK_OCAP_SECRET (required) |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_OCAP_SECRET | required | Hex-encoded OCAP secret for minting/verifying spec capability tokens |
| HKASK_SPEC_DB_PATH | optional | Path to the spec SQLite database (in-memory if absent) |
| HKASK_DB_PASSPHRASE | optional | Passphrase for the spec database (required if HKASK_SPEC_DB_PATH is set) |

---

### goal

**Crate:** `hkask-mcp-goal`
**Loop:** L5 (Curation)
**Gas Cost:** 5
**LOC:** 209

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| goal_create | Create a goal owned by the calling agent | HKASK_GOAL_DB, HKASK_DB_PASSPHRASE (optional) |
| goal_list | List the calling agent's goals, optionally filtered by state | HKASK_GOAL_DB, HKASK_DB_PASSPHRASE (optional) |
| goal_set_state | Transition a goal to a new state (legal transitions only) | HKASK_GOAL_DB, HKASK_DB_PASSPHRASE (optional) |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_GOAL_DB | optional | Path to the goal SQLite database (in-memory if absent) |
| HKASK_DB_PASSPHRASE | optional | Passphrase for the goal database (required if HKASK_GOAL_DB is set) |

---

### replicant

**Crate:** `hkask-mcp-replicant`
**Loop:** L5 (Curation)
**Gas Cost:** 5
**LOC:** 815

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| replicant_chat | Send a message to a hKask replicant agent and receive a response. The replicant persona is configured via HKASK_AGENT_PERSONA (default: 'Curator'). Optionally override the model per request. Conversation history is maintained across calls within the same session. | OKAPI_BASE_URL (optional) |
| replicant_status | Check the registration status and identity of the hKask replicant configured for this MCP server. | — |
| replicant_history | List recent conversation turns in the current session. Shows the last N turns of conversation history maintained across replicant:chat calls. | — |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_AGENT_PERSONA | optional | Replicant persona name (default: Curator) |
| HKASK_DEFAULT_MODEL | optional | Default LLM model for inference (default: deepseek-v4-pro) |
| OKAPI_BASE_URL | optional | Okapi API base URL (default: http://127.0.0.1:11435) |

---

### ocap

**Crate:** `hkask-mcp-ocap`
**Loop:** L6 (Cybernetics)
**Gas Cost:** 1
**LOC:** 337

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| ocap_delegate | Create a delegated capability token with real HMAC signature | HKASK_OCAP_SECRET |
| ocap_verify | Verify a capability token with real cryptographic HMAC verification | HKASK_OCAP_SECRET |
| ocap_revoke | Revoke a capability token by adding to revocation set | HKASK_OCAP_SECRET |
| ocap_enumerate | Enumerate capabilities for a subject | HKASK_OCAP_SECRET |
| ocap_list_tokens | List all capability tokens | HKASK_OCAP_SECRET |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_OCAP_SECRET | required | OCAP signing secret for capability token HMAC |

---

### keystore

**Crate:** `hkask-mcp-keystore`
**Loop:** L6 (Cybernetics)
**Gas Cost:** 2
**LOC:** 491

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| keystore_set | Set a key-value pair in the keystore with AES-256-GCM encryption | — |
| keystore_get | Get a value from the keystore (capability-gated: only owner pod can read) | — |
| keystore_rotate | Rotate a key-value pair with re-encryption | — |
| keystore_delete | Delete a key from the keystore (capability-gated) | — |
| keystore_list | List all keys in the keystore | — |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_KEYSTORE_SERVICE | optional | Service name for OS keychain (default: hkask-mcp-keystore) |
| HKASK_KEYSTORE_DIR | optional | Path to keystore vault directory (default: ~/.hkask/keystore) |

---

### cns

**Crate:** `hkask-mcp-cns`
**Loop:** L6 (Cybernetics)
**Gas Cost:** 1
**LOC:** 408

| Tool | Description | Credentials Required |
|------|-------------|---------------------|
| cns_emit | Emit a CNS observation event | — |
| cns_variety | Get variety count for a span pattern via real VarietyMonitor | — |
| cns_alert | Trigger a real algedonic alert via AlgedonicManager | — |
| cns_calibrate | Calibrate a span threshold | — |
| cns_list_alerts | List active algedonic alerts from real alert manager | — |
| cns_health | Get real CNS health status | — |
| cns_replenish_budget | Replenish an agent's gas budget (Curator authority required) | — |
| cns_energy | Get an agent's gas budget status (energy level, usage, limits) | — |
| cns_backpressure | Emit a backpressure signal to throttle downstream loops | — |
| cns_verify_magna_carta | Verify Magna Carta compliance (sovereignty audit) | — |

**Credential Requirements:**

| Credential | Required | Description |
|-----------|----------|-------------|
| HKASK_CNS_THRESHOLD | optional | CNS variety deficit threshold (default: 100) |

---

## Gas Cost Tier Reference

| Tier | Servers | Cost Range | Rationale |
|------|---------|------------|-----------|
| Internal | ocap, keystore, cns, registry, ensemble | 1–2 | In-process, negligible compute |
| Local I/O | spec, git, goal, episodic, semantic, replicant | 5 | Local I/O, no network |
| Moderate | condenser, doc-knowledge, markitdown | 10 | Some computation + local I/O |
| Moderate+Network | condenser (thread_summary) | 25 | HTTP call to inference engine |
| External API | rss-reader, github, fmp, telnyx, web | 20–50 | Network I/O, rate-limited |
| Heavy | fal | 100 | GPU compute, expensive |
| Inference | hkask-mcp-inference | 0 (table) | Handled by `InferenceGasEstimator` |

**Inference gas model:** `prompt_chars / 4 + max_tokens` (token-based, not flat per-call).

**Default for unlisted servers:** 10 (moderate — conservative middle ground).

Source: `crates/hkask-cns/src/table_gas_estimator.rs`

---

## Loop Assignment Reference

| Loop | Servers | Domain Authority |
|------|---------|-----------------|
| L1 (Inference) | inference | LLM inference — core transform |
| L2 (Episodic) | condenser, episodic, rss-reader, doc-knowledge, markitdown | Episodic boundary — conversation window, private memory |
| L2b (Semantic) | semantic | Semantic (shared) memory |
| L4 (Communication) | web, git, github, fmp, telnyx, fal, ensemble | External I/O dispatch, agent messaging |
| L5 (Curation) | spec, goal, replicant | DDMVSS spec capture, goal prioritization, agent persona |
| L6 (Cybernetics) | ocap, keystore, cns | Authority governance, sovereignty, self-regulation |
| L1↔L5 (bridge) | registry | Cross-loop: template discovery (L1) + skill/bundle composition (L5) |

Source: `docs/architecture/loop-architecture.md` §3.4