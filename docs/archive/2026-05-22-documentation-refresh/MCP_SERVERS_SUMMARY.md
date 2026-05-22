# hKask MCP Servers — Complete Inventory

**Version:** v0.21.0  
**Date:** 2026-05-20  
**Total MCP Servers:** 20 (10 original + 10 new)

---

## Original 10 MCP Servers

| Server | Purpose | Status |
|--------|---------|--------|
| `hkask-mcp-inference` | Okapi-backed LLM inference | Stub |
| `hkask-mcp-storage` | Storage operations (triples, embeddings, blobs) | Stub |
| `hkask-mcp-memory` | Semantic/episodic memory operations | Stub |
| `hkask-mcp-embedding` | Embedding generation, similarity search | Stub |
| `hkask-mcp-condenser` | Template condensation, summarization | Stub |
| `hkask-mcp-ensemble` | Multi-agent coordination, chat orchestration | Stub |
| `hkask-mcp-web` | Web search, scrape, extract | Stub |
| `hkask-mcp-scholar` | Academic research | Stub |
| `hkask-mcp-spandrel` | Graph analysis | Stub |
| `hkask-mcp-doc-knowledge` | Document extraction | Stub |

---

## New 10 MCP Servers (Added 2026-05-20)

### Core Infrastructure (5)

| Server | Purpose | Tools | Env Vars | Status |
|--------|---------|-------|----------|--------|
| `hkask-mcp-ocap` | OCAP delegation & capability management | `ocap_delegate`, `ocap_verify`, `ocap_revoke`, `ocap_enumerate`, `ocap_list_tokens` | None | Stub |
| `hkask-mcp-keystore` | OS keychain integration | `keystore_set`, `keystore_get`, `keystore_rotate`, `keystore_delete`, `keystore_list`, `keystore_prompt` | None | Stub |
| `hkask-mcp-cns` | CNS monitoring & algedonic alerts | `cns_emit`, `cns_variety`, `cns_alert`, `cns_calibrate`, `cns_list_alerts`, `cns_health` | None | Stub |
| `hkask-mcp-git` | Git CAS operations | `git_resolve`, `git_snapshot`, `git_clone`, `git_fork`, `git_diff`, `git_list` | `HKASK_GIT_CAS_ROOT` | Stub |
| `hkask-mcp-registry` | Template registry operations | `registry_index`, `registry_discover`, `registry_validate`, `registry_reload`, `registry_compose`, `registry_get` | `HKASK_REGISTRY_ROOT` | Stub |

### External Integrations (5) — Ported from kask/arsenal

| Server | Purpose | Tools | Env Vars | Status |
|--------|---------|-------|----------|--------|
| `hkask-mcp-github` | GitHub repository operations | `github_get_repo`, `github_list_issues`, `github_get_issue`, `github_create_issue`, `github_list_prs`, `github_get_pr`, `github_add_comment`, `github_search_repos` | `GITHUB_TOKEN` (write ops) | Stub |
| `hkask-mcp-fmp` | Financial Modeling Prep API | `fmp_ping`, `fmp_company_profile`, `fmp_quote`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow_statement`, `fmp_key_metrics`, `fmp_historical_price`, `fmp_search`, `fmp_analyst_estimates`, `fmp_dcf` | `FMP_API_KEY` | Stub |
| `hkask-mcp-telnyx` | Telnyx unified communications | `telnyx_ping`, `telnyx_list_numbers`, `telnyx_buy_number`, `telnyx_send_sms`, `telnyx_make_call`, `telnyx_send_whatsapp`, `telnyx_tts`, `telnyx_list_voices` | `TELNYX_API_KEY` | Stub |
| `hkask-mcp-fal` | Fal.ai media generation | `fal_ping`, `fal_generate_image`, `fal_generate_image_fast`, `fal_image_to_image`, `fal_upscale`, `fal_generate_video`, `fal_generate_music`, `fal_whisper`, `fal_caption`, `fal_tts`, `fal_generate_3d` | `FAL_KEY` | Stub |
| `hkask-mcp-rss-reader` | RSS/Atom feed reader | `rss_subscribe`, `rss_unsubscribe`, `rss_list_subscriptions`, `rss_fetch`, `rss_get_entries`, `rss_mark_all_read`, `rss_get_unread_count`, `rss_search`, `rss_export_opml`, `rss_discover_feeds` | None | Stub |

---

## Build Verification

All 10 new MCP servers compile successfully:

```bash
cargo check -p hkask-mcp-ocap -p hkask-mcp-keystore -p hkask-mcp-cns \
  -p hkask-mcp-git -p hkask-mcp-registry -p hkask-mcp-github \
  -p hkask-mcp-fmp -p hkask-mcp-telnyx -p hkask-mcp-fal -p hkask-mcp-rss-reader
```

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*20 MCP servers ready for rmcp integration.*
