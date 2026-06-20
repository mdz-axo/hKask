# hkask-mcp-research

Web search, extraction, and feed-based research MCP server.

## Tools (29)

| Tool | Description |
|------|-------------|
| `web_ping` | Web search health check |
| `web_search` | Web search |
| `web_browse` | Browse web page |
| `web_extract` | Extract structured data |
| `web_find_similar` | Find similar content |
| `search_with_fallback` | Search with provider fallback |
| `search_compound` | Compound search across providers |
| `search_by_capability` | Search by capability |
| `browse_with_fallback` | Browse with fallback |
| `extract_with_fallback` | Extract with fallback |
| `find_similar` | Find similar content |
| `fetch_feed` | Fetch RSS/Atom feed |
| `discover_feeds` | Discover feeds from page |
| `rss_subscribe` | Subscribe to feed |
| `rss_unsubscribe` | Unsubscribe from feed |
| `rss_list_subscriptions` | List subscriptions |
| `rss_fetch` | Fetch feed entries |
| `rss_get_entries` | Get feed entries |
| `rss_get_unread_count` | Get unread count |
| `rss_mark_all_read` | Mark all as read |
| `rss_search` | Search feeds |
| `rss_discover_feeds` | Discover feeds |
| `rss_edit_tag` | Edit feed tags |
| `rss_export_opml` | Export OPML |
| `rss_import_opml` | Import OPML |
| `health_check_all` | Health check all providers |
| `get` | Get item |
| `insert` | Insert item |
| `run` | Main run loop |

## Quick Start

```bash
# No API keys required for web search (uses free providers with fallback)
# The server starts automatically with kask
kask chat
# Or standalone:
hkask-mcp-research
```

## Usage

```
"Search for latest AI research papers"   → web_search
"Browse this URL and summarize it"        → web_browse
"Subscribe to the Rust blog RSS feed"     → rss_subscribe
"What's new in my feeds?"                → rss_get_entries
```
