---
title: "MCP Tools Inventory"
version: "1.0.0"
last_updated: 2026-06-08
status: Active
domain: "Cross-cutting"
---

# MCP Tools Inventory

Complete catalog of all 21 hKask MCP servers, their tools, gas costs, credentials, and loop assignments.

## Summary

| Server | Crate | Tools | Gas Cost | Loop | Required Credentials | LOC |
|--------|-------|-------|----------|------|----------------------|-----|
| inference | `hkask-mcp-inference` | 3 | token-based | L1 (Inference) | — | 324 |
| condenser | `hkask-mcp-condenser` | 7 | 10 (thread_summary=25) | L2 (Episodic) | — | 1,790 |
| web | `hkask-mcp-web` | 4 | 50 | L4 (Communication) | — | 3,180 |
| ocap | `hkask-mcp-ocap` | 5 | 1 | L6 (Cybernetics) | `HKASK_OCAP_SECRET` | 337 |
| keystore | `hkask-mcp-keystore` | 4 | 2 | L6 (Cybernetics) | — | 491 |
| cns | `hkask-mcp-cns` | 10 | 1 | L6 (Cybernetics) | — | 408 |
| git | `hkask-mcp-git` | 6 | 5 | L4 (Communication) | — | 324 |
| registry | `hkask-mcp-registry` | 6 | 2 | L1↔L5 (bridge) | — | 303 |
| spec | `hkask-mcp-spec` | 11 | 5 | L5 (Curation) | `HKASK_OCAP_SECRET` | 2,576 |
| goal | `hkask-mcp-goal` | 3 | 5 | L2 (Episodic) | — | 209 |
| github | `hkask-mcp-github` | 7 | 30 | L4 (Communication) | `HKASK_GITHUB_TOKEN` | 468 |
| fmp | `hkask-mcp-fmp` | 10 | 40 | L4 (Communication) | `HKASK_FMP_API_KEY` | 367 |
| telnyx | `hkask-mcp-telnyx` | 7 | 50 | L4 (Communication) | `HKASK_TELNYX_API_KEY` | 240 |
| fal | `hkask-mcp-fal` | 9 | 100 | L4 (Communication) | `HKASK_FAL_API_KEY` | 414 |
| rss-reader | `hkask-mcp-rss-reader` | 10 | 20 | L4 (Communication) | — | 1,408 |
| ensemble | `hkask-mcp-ensemble` | 6 | 2 | L5 (Curation) | — | 391 |
| episodic | `hkask-mcp-episodic` | 4 | 5 | L2 (Episodic) | `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE` | 219 |
| semantic | `hkask-mcp-semantic` | 7 | 5 | L2b (Semantic) | `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE` | 437 |
| replicant | `hkask-mcp-replicant` | 3 | 5 | L5 (Curation) | — | 815 |
| doc-knowledge | `hkask-mcp-doc-knowledge` | 5 | 5 | L2 (Episodic) | — | 747 |
| markitdown | `hkask-mcp-markitdown` | 3 | 5 | L2 (Episodic) | — | 698 |

**Totals:** 21 servers, 119 tools, 15,550 LOC

## Per-Server Detail

### inference

**Crate:** `hkask-mcp-inference` · **Loop:** L1 (Inference) · **Gas:** token-based · **LOC:** 324

| Tool | Description |
|------|-------------|
| `inference_generate` | Generate text using Okapi-backed LLM inference. Supports model selection with automatic failover, temperature control, and token limits. |
| `inference_metrics` | Get current inference metrics including total requests, tokens generated, error counts, and failover count. |
| `inference_list_models` | List available model tiers and their configurations. |

---

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

### ocap

**Crate:** `hkask-mcp-ocap` · **Loop:** L6 (Cybernetics) · **Gas:** 1 · **LOC:** 337

**Required:** `HKASK_OCAP_SECRET`

| Tool | Description |
|------|-------------|
| `ocap_create` | Create a delegated capability token with real HMAC signature |
| `ocap_verify` | Verify a capability token with real cryptographic HMAC verification |
| `ocap_revoke` | Revoke a capability token by adding to revocation set |
| `ocap_enumerate` | Enumerate capabilities for a subject |
| `ocap_list_tokens` | List all capability tokens |

---

### keystore

**Crate:** `hkask-mcp-keystore` · **Loop:** L6 (Cybernetics) · **Gas:** 2 · **LOC:** 491

**Credentials:** Optional `HKASK_KEYSTORE_SERVICE`, `HKASK_KEYSTORE_DIR`.

| Tool | Description |
|------|-------------|
| `keystore_set` | Set a key-value pair in the keystore with AES-256-GCM encryption |
| `keystore_rotate` | Rotate a key-value pair with re-encryption |
| `keystore_delete` | Delete a key from the keystore (capability-gated) |
| `keystore_list` | List all keys in the keystore |

---

### cns

**Crate:** `hkask-mcp-cns` · **Loop:** L6 (Cybernetics) · **Gas:** 1 · **LOC:** 408

**Credentials:** Optional `HKASK_CNS_THRESHOLD`.

| Tool | Description |
|------|-------------|
| `cns_observe` | Emit a CNS observation event |
| `cns_variety` | Get variety count for a span pattern via real VarietyMonitor |
| `cns_algedonic` | Trigger a real algedonic alert via AlgedonicManager |
| `cns_calibrate` | Calibrate a span threshold |
| `cns_alerts` | List active algedonic alerts from real alert manager |
| `cns_health` | Get real CNS health status |
| `cns_replenish_gas` | Replenish an agent's gas budget (Curator authority required) |
| `cns_gas_status` | Get an agent's gas budget status (energy level, usage, limits) |
| `cns_backpressure` | Emit a backpressure signal to throttle downstream loops |
| `cns_sovereignty_verify` | Verify Magna Carta compliance (sovereignty audit) |

---

### git

**Crate:** `hkask-mcp-git` · **Loop:** L4 (Communication) · **Gas:** 5 · **LOC:** 324

**Credentials:** Optional `HKASK_CAS_HOME`.

| Tool | Description |
|------|-------------|
| `git_resolve` | Resolve a git reference to a SHA |
| `git_snapshot` | Create a git snapshot (commit) |
| `git_diff` | Show diff between two commits |
| `git_ls_tree` | List files in a git tree |
| `git_verify` | Verify content integrity of a repository |
| `git_log` | List snapshot history for a repository |

---

### registry

**Crate:** `hkask-mcp-registry` · **Loop:** L1↔L5 (bridge) · **Gas:** 2 · **LOC:** 303

**Credentials:** Optional `HKASK_REGISTRY_DB`, `HKASK_DB_PASSPHRASE`.

| Tool | Description |
|------|-------------|
| `registry_index` | Index templates from a root path via real registry |
| `registry_discover` | Discover templates by type and domain via real registry search |
| `registry_validate` | Validate a template via real registry lookup |
| `registry_reload` | Reload templates from a path |
| `registry_compose` | Compose templates with cascade |
| `registry_get` | Get a template by ID via real registry lookup |

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

### goal

**Crate:** `hkask-mcp-goal` · **Loop:** L2 (Episodic) · **Gas:** 5 · **LOC:** 209

**Credentials:** Optional `HKASK_GOAL_DB`, `HKASK_DB_PASSPHRASE`.

| Tool | Description |
|------|-------------|
| `goal_create` | Create a goal owned by the calling agent |
| `goal_list` | List the calling agent's goals, optionally filtered by state |
| `goal_transition` | Transition a goal to a new state (legal transitions only) |

---

### github

**Crate:** `hkask-mcp-github` · **Loop:** L4 (Communication) · **Gas:** 30 · **LOC:** 468

**Required:** `HKASK_GITHUB_TOKEN`

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

### ensemble

**Crate:** `hkask-mcp-ensemble` · **Loop:** L5 (Curation) · **Gas:** 2 · **LOC:** 391

| Tool | Description |
|------|-------------|
| `ensemble_create_session` | Create a standing session from a YAML config path |
| `ensemble_register_bot` | Register a bot participant in a session |
| `ensemble_send_message` | Send a message to a standing session |
| `ensemble_session_status` | Get standing session status |
| `ensemble_prepare_turn` | Prepare an improvisation turn prompt for external inference |
| `ensemble_structure_a2a` | Structure an A2A message for dispatch between agents |

---

### episodic

**Crate:** `hkask-mcp-episodic` · **Loop:** L2 (Episodic) · **Gas:** 5 · **LOC:** 219

**Required:** `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE`

| Tool | Description |
|------|-------------|
| `episodic_ping` | Liveness and storage info for episodic memory |
| `episodic_store` | Store an episodic triple (private, perspective-bound) |
| `episodic_recall` | Recall episodic triples by entity (filtered by caller's WebID) |
| `episodic_budget` | Storage usage and budget for episodic memory |

---

### semantic

**Crate:** `hkask-mcp-semantic` · **Loop:** L2b (Semantic) · **Gas:** 5 · **LOC:** 437

**Required:** `HKASK_MEMORY_DB`, `HKASK_DB_PASSPHRASE`

| Tool | Description |
|------|-------------|
| `semantic_ping` | Liveness and storage info for semantic memory |
| `semantic_store` | Store a shared semantic triple (no perspective) |
| `semantic_recall` | Recall shared semantic triples by entity |
| `semantic_embed` | Store an embedding vector for similarity search |
| `semantic_search` | KNN similarity search over embeddings |
| `semantic_delete_prefix` | Delete all embeddings whose entity_ref starts with a prefix |
| `semantic_budget` | Triple and embedding counts for semantic memory |

---

### replicant

**Crate:** `hkask-mcp-replicant` · **Loop:** L5 (Curation) · **Gas:** 5 · **LOC:** 815

**Credentials:** Optional `HKASK_AGENT_PERSONA`, `HKASK_DEFAULT_MODEL`, `OKAPI_BASE_URL`.

| Tool | Description |
|------|-------------|
| `replicant_chat` | Send a message to a hKask replicant agent and receive a response |
| `replicant_status` | Check the registration status and identity of the hKask replicant |
| `replicant_history` | List recent conversation turns in the current session |

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