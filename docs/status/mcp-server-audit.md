---
title: "MCP Server Completeness Audit"
audience: [architects, developers, agents]
last_updated: 2026-06-03
version: "1.2.0"
status: "Active"
domain: "Capability"
ddmvss_categories: [capability, observability]
---

# MCP Server Completeness Audit

**Date:** 2026-05-29
**Version:** hKask v0.21.0
**Total servers:** 15
**Total tools:** 102+

---

## Summary Table

| Server | LOC | Tools | Status | Notes |
|--------|-----|-------|--------|-------|
| `hkask-mcp-inference` | 391 | 3 | **Full** | generate, metrics, models — real Okapi calls, failover, rate limiting |
| `hkask-mcp-rss-reader` | 1,443 | 12 | **Full** | Complete RSS feed management with SQLite persistence |
| `hkask-mcp-spec` | 853 | 8 | **Full** | 8 DDMVSS spec tools (capture, decompose, curate, validate) |
| `hkask-mcp-goal` | ~235 | 3 | **Full** | Goal coordination substrate (OCAP-gated, CNS-observed); CLI/API/MCP parity |
| `hkask-mcp-condenser` | 761 | 5 | **Full** | Context reranking and condensation algorithms |
| `hkask-mcp-web` | 3,389 | 5 | **Full** | Search, scrape, extract with SSRF protection |
| `hkask-mcp-keystore` | 529 | 6 | **Full** | OS keychain with AES-256-GCM vault persistence |
| `hkask-mcp-github` | 459 | 8 | **Full** | GitHub API integration |
| `hkask-mcp-fal` | 434 | 10 | **Full** | FAL image generation service |
| `hkask-mcp-git` | 412 | 6 | **Full** | Git CAS (content-addressed storage) operations |
| `hkask-mcp-fmp` | 369 | 11 | **Full** | Financial Modeling Prep API integration |
| `hkask-mcp-ocap` | 319 | 5 | **Full** | Capability grant, verify, revoke, list operations |
| `hkask-mcp-registry` | 310 | 6 | **Full** | Template registry CRUD and search operations |
| `hkask-mcp-cns` | 280 | 6 | **Full** | CNS health, variety, alerts, metrics operations |
| `hkask-mcp-telnyx` | 244 | 8 | **Full** | Telnyx SMS/voice API integration |

---

## Status Distribution

| Status | Count | Servers |
|--------|-------|---------|
| **Full** | 15 | All servers |
| **Partial** | 0 | — |
| **Shell** | 0 | — |

---

## Per-Server Detail

### `hkask-mcp-inference` (391 LOC, 3 tools)
- **Status:** Full
- **Tools:** `inference:generate`, `inference:metrics`, `inference:models`
- **Notes:** Real Okapi LLM calls with failover, per-caller rate limiting (token bucket), metrics tracking with atomic counters. Primary model + automatic fallback chain.

### `hkask-mcp-rss-reader` (1,443 LOC, 12 tools)
- **Status:** Full
- **Tools:** 12 feed management tools with SQLite-backed persistence
- **Notes:** Most feature-rich server. Includes feed subscription management, article retrieval, import/export, database-backed state.

### `hkask-mcp-spec` (853 LOC, 8 tools)
- **Status:** Full
- **Tools:** spec/goal/capture, spec/goal/decompose, spec/require/bind, spec/curate/evaluate, spec/curate/reconcile, spec/curate/cultivate, spec/graph/query, spec/graph/validate
- **Notes:** DDMVSS specification tools. 2 goal operations + 1 require-bind + 3 curation operations + 2 graph operations.

### `hkask-mcp-goal` (~235 LOC, 3 tools)
- **Status:** Full
- **Tools:** goal_create, goal_list, goal_set_state
- **Notes:** Goal coordination substrate. OCAP-gated with CNS-observed NuEvent emissions. Mirrors the CLI (`kask goal`) and HTTP API (`/api/goals`) surfaces, satisfying MCP ≡ CLI ≡ API (REQ-IFC-001).

### `hkask-mcp-condenser` (761 LOC, 5 tools)
- **Status:** Full
- **Tools:** Context reranking, condensation, deduplication algorithms
- **Notes:** Multiple condensation strategies (rank, compress, deduplicate). Configurable via parameters.

### `hkask-mcp-web` (3,389 LOC, 5 tools)
- **Status:** Full
- **Tools:** Web search, scrape, extract operations
- **Notes:** SSRF protection (private IP/loopback rejection), URL validation, strip-html utilities. Multiple search providers.

### `hkask-mcp-keystore` (529 LOC, 6 tools)
- **Status:** Full
- **Tools:** store, retrieve, list, delete, import, export
- **Notes:** AES-256-GCM encryption per entry, atomic file writes (temp + rename), `~/.hkask/keystore/vault.json` persistence with schema versioning.

### `hkask-mcp-github` (459 LOC, 8 tools)
- **Status:** Full
- **Tools:** Repository, issue, PR, and code search operations
- **Notes:** GitHub API integration with OAuth token management.

### `hkask-mcp-fal` (434 LOC, 10 tools)
- **Status:** Full
- **Tools:** Image generation with FAL API, model selection, parameter control
- **Notes:** Ten distinct generation pipelines. Model-specific parameter schemas.

### `hkask-mcp-git` (412 LOC, 6 tools)
- **Status:** Full
- **Tools:** clone, fetch, commit, push, log, status operations
- **Notes:** Git CAS integration via `gix` crate. Content-addressed storage for templates and specs.

### `hkask-mcp-fmp` (369 LOC, 11 tools)
- **Status:** Full
- **Tools:** Financial data queries — company profiles, quotes, ratios, statements
- **Notes:** Financial Modeling Prep API wrapper. Eleven distinct query endpoints.

### `hkask-mcp-ocap` (319 LOC, 5 tools)
- **Status:** Full
- **Tools:** grant, verify, revoke, list, inspect capability tokens
- **Notes:** OCAP capability management. Grant with caveats, verify chains, persistent revocation.

### `hkask-mcp-registry` (310 LOC, 6 tools)
- **Status:** Full
- **Tools:** list, get, register, search, validate, delete template operations
- **Notes:** Template registry CRUD with contract validation and lexicon-term search.

### `hkask-mcp-cns` (280 LOC, 6 tools)
- **Status:** Full
- **Tools:** health, variety, alerts, metrics, reset, clear operations
- **Notes:** CNS monitoring surface. Exposes variety counters, algedonic alerts, runtime health.

### `hkask-mcp-telnyx` (244 LOC, 8 tools)
- **Status:** Full
- **Tools:** SMS send, voice call, message status, number lookup operations
- **Notes:** Telnyx API integration for SMS and voice capabilities. Compact but feature-complete.

### `hkask-mcp-ensemble` (295 LOC, ~6 tools)
- **Status:** Full
- **Tools:** Multi-agent chat coordination, deliberation dispatch
- **Notes:** MCP surface for `hkask-ensemble` crate. Manages standing sessions and confidence routing.

### `hkask-mcp-episodic` (190 LOC, ~5 tools)
- **Status:** Full
- **Tools:** Store, recall, query episodic memories (private, perspective-bound)
- **Notes:** Episodic memory MCP server. Sovereignty boundary: episodic memories are private by design.

### `hkask-mcp-semantic` (290 LOC, ~5 tools)
- **Tools:** Store, recall, query, consolidate semantic memories (public, shared)
- **Notes:** Semantic memory MCP server. Public/shared visibility boundary. Consolidation bridge to episodic.

---

## Recommendations

1. **No shell servers.** All 18 MCP servers register real tools with implementations. Zero stubs remain (P6 compliance).

2. **Per-crate README:** Create individual `README.md` files in each `mcp-servers/hkask-mcp-*/README.md` documenting the tool surface, configuration, and any external service dependencies.

3. **Tool count outliers:** `hkask-mcp-telnyx` (244 LOC, 8 tools — high tool density) vs `hkask-mcp-rss-reader` (1,443 LOC, 12 tools — high LOC per tool). Consider whether `telnyx` tools are thin wrappers around API endpoints.

4. **Dependency hygiene:** Several servers (github, fmp, telnyx, fal) depend on external API services. Document API key requirements and rate limits in per-crate READMEs.

5. **OQ-3 resolved:** This audit satisfies option 2 of OQ-3 — catalog approach with common pattern description and per-crate README for implemented servers.

---

*ℏKask MCP Arsenal — 18 servers, ~110 tools, 0 stubs — v0.22.0*
