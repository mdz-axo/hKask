use std::sync::Arc;

use base64::Engine;
use hkask_mcp::server::{McpToolError, ToolSpanGuard, validate_tool_url};
use hkask_storage::Database;
use hkask_types::{McpErrorKind, WebID};
use reqwest::Client;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use rusqlite::Connection;

use crate::db::*;
use crate::types::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_PAGE_SIZE: usize = 20;
const MAX_PAGE_SIZE: usize = 100;

// ---------------------------------------------------------------------------
// Feed fetching & parsing
// ---------------------------------------------------------------------------

async fn fetch_feed(
    client: &Client,
    url: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<FetchResult, anyhow::Error> {
    let mut request = client.get(url);
    if let Some(e) = etag {
        request = request.header("If-None-Match", e);
    }
    if let Some(lm) = last_modified {
        request = request.header("If-Modified-Since", lm);
    }

    let response = request.send().await?;
    let status = response.status().as_u16();

    if status == 304 {
        let empty_feed = feed_rs::parser::parse(std::io::empty())?;
        return Ok(FetchResult {
            feed: empty_feed,
            etag: None,
            last_modified: None,
            status,
        });
    }

    if !response.status().is_success() {
        anyhow::bail!("HTTP {} fetching {}", response.status(), url);
    }

    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let last_modified = response
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body = response.bytes().await?;
    let feed = feed_rs::parser::parse(body.as_ref())?;

    Ok(FetchResult {
        feed,
        etag,
        last_modified,
        status,
    })
}

async fn discover_feeds(
    client: &Client,
    url: &str,
) -> Result<Vec<serde_json::Value>, anyhow::Error> {
    let response = client.get(url).send().await?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if content_type.contains("rss")
        || content_type.contains("atom")
        || content_type.contains("feed")
    {
        return Ok(vec![serde_json::json!({
            "url": url,
            "type": "feed",
            "content_type": content_type,
        })]);
    }

    let body = response.text().await?;
    let mut feeds = Vec::new();

    let re1 = regex::Regex::new(
        r#"<link[^>]+rel\s*=\s*["']alternate["'][^>]+type\s*=\s*["']application/(rss|atom)\+xml["'][^>]+href\s*=\s*["']([^"']+)["']"#,
    )?;
    let re2 = regex::Regex::new(
        r#"<link[^>]+type\s*=\s*["']application/(rss|atom)\+xml["'][^>]+href\s*=\s*["']([^"']+)["']"#,
    )?;

    for re in [&re1, &re2] {
        for cap in re.captures_iter(&body) {
            let feed_type = cap.get(1).map(|m| m.as_str()).unwrap_or("rss");
            let href = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let feed_url = if href.starts_with("http") {
                href.to_string()
            } else {
                let base = url::Url::parse(url)?;
                base.join(href)
                    .map(|u| u.to_string())
                    .unwrap_or_else(|_| href.to_string())
            };
            if !feeds
                .iter()
                .any(|f: &serde_json::Value| f["url"].as_str() == Some(feed_url.as_str()))
            {
                feeds.push(serde_json::json!({
                    "url": feed_url,
                    "type": feed_type,
                }));
            }
        }
    }

    Ok(feeds)
}

// ---------------------------------------------------------------------------
// RssServer
// ---------------------------------------------------------------------------

pub struct RssServer {
    webid: WebID,
    db: Arc<std::sync::Mutex<Connection>>,
    client: Client,
}

impl RssServer {
    /// Construct from a server context.
    ///
    /// - `HKASK_RSS_DB` + `HKASK_DB_PASSPHRASE` (optional): persistent encrypted database.
    ///   Absent → in-memory (ephemeral, data lost on restart).
    pub fn new(ctx: hkask_mcp::ServerContext) -> Result<Self, anyhow::Error> {
        let db: Arc<std::sync::Mutex<Connection>> = match (
            ctx.credentials.get("HKASK_RSS_DB"),
            ctx.credentials.get("HKASK_DB_PASSPHRASE"),
        ) {
            (Some(path), Some(passphrase)) => {
                Database::open_with_extensions(path, passphrase, RSS_SCHEMA_DDL)
                    .map_err(|e| anyhow::anyhow!("Failed to open RSS database: {e}"))?
                    .conn_arc()
            }
            _ => {
                tracing::warn!(
                    target: "hkask.mcp.rss_reader",
                    "No persistent database configured — RSS data is in-memory and will be lost on restart. \
                     Set HKASK_RSS_DB and HKASK_DB_PASSPHRASE for encrypted persistence."
                );
                Database::in_memory_with_extensions(RSS_SCHEMA_DDL)
                    .map_err(|e| anyhow::anyhow!("Failed to open in-memory RSS database: {e}"))?
                    .conn_arc()
            }
        };

        let client = Client::builder()
            .user_agent(format!("hkask-mcp-rss-reader/{}", SERVER_VERSION))
            .build()?;

        Ok(Self {
            webid: ctx.webid,
            db,
            client,
        })
    }
}

fn spawn_db<F, T>(
    db: Arc<std::sync::Mutex<Connection>>,
    f: F,
) -> tokio::task::JoinHandle<Result<T, anyhow::Error>>
where
    F: FnOnce(&Connection) -> Result<T, anyhow::Error> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let conn = db.lock().unwrap();
        f(&conn)
    })
}

#[tool_router(server_handler)]
impl RssServer {
    #[tool(description = "Subscribe to an RSS/Atom feed (Google Reader stream model)")]
    async fn rss_subscribe(
        &self,
        Parameters(SubscribeRequest { url, label, folder }): Parameters<SubscribeRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("rss_subscribe", &self.webid);
        if let Err(e) = validate_tool_url(&url) {
            return span.error(e.kind, e.to_json_string());
        }
        let fetch_result = match fetch_feed(&self.client, &url, None, None).await {
            Ok(r) => r,
            Err(e) => {
                return span.error(
                    McpErrorKind::Unavailable,
                    McpToolError::unavailable(format!("Fetch failed: {}", e)).to_json_string(),
                );
            }
        };

        let stream_id = format!("feed/{url}");
        let db = self.db.clone();
        let url_c = url;
        let label_c = label;
        let folder_c = folder;
        let etag = fetch_result.etag.clone();
        let lm = fetch_result.last_modified.clone();
        let feed_title = fetch_result
            .feed
            .title
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or_default();
        let entry_count = fetch_result.feed.entries.len();

        let result = spawn_db(db, move |conn| {
            let feed_id = upsert_feed(conn, &url_c, &fetch_result.feed)?;
            insert_entries(conn, feed_id, &fetch_result.feed.entries)?;
            update_feed_cache_headers(conn, feed_id, etag.as_deref(), lm.as_deref())?;

            let exists: bool = conn.query_row(
                "SELECT COUNT(*) FROM subscriptions WHERE stream_id = ?1",
                [&stream_id],
                |row| row.get::<_, i64>(0),
            ).map(|c| c > 0)?;

            if exists {
                return Ok(serde_json::json!({
                    "stream_id": stream_id,
                    "url": url_c,
                    "subscribed": true,
                    "note": "Already subscribed, feed refreshed",
                }));
            }

            conn.execute(
                "INSERT INTO subscriptions (feed_id, stream_id, title, label, folder) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![feed_id, stream_id, feed_title, label_c, folder_c],
            )?;

            Ok::<serde_json::Value, anyhow::Error>(serde_json::json!({
                "stream_id": stream_id,
                "url": url_c,
                "label": label_c,
                "folder": folder_c,
                "subscribed": true,
                "entry_count": entry_count,
            }))
        }).await;

        match result {
            Ok(Ok(v)) => span.ok_json(v),
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }

    #[tool(description = "Unsubscribe from a feed (stream_id e.g. 'feed/http://...')")]
    async fn rss_unsubscribe(
        &self,
        Parameters(UnsubscribeRequest { stream_id }): Parameters<UnsubscribeRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("rss_unsubscribe", &self.webid);
        let db = self.db.clone();
        let sid = stream_id.clone();
        let result = spawn_db(db, move |conn| {
            let removed = conn.execute("DELETE FROM subscriptions WHERE stream_id = ?1", [&sid])?;
            Ok::<usize, anyhow::Error>(removed)
        })
        .await;

        match result {
            Ok(Ok(removed)) => span.ok_json(serde_json::json!({
                "stream_id": stream_id,
                "unsubscribed": removed > 0,
                "removed": removed,
            })),
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }

    #[tool(description = "List subscriptions, optionally filtered by folder")]
    async fn rss_list_subscriptions(
        &self,
        Parameters(ListSubscriptionsRequest { folder }): Parameters<ListSubscriptionsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("rss_list_subscriptions", &self.webid);
        let db = self.db.clone();
        let result = spawn_db(db, move |conn| list_subscriptions(conn, folder.as_deref())).await;

        match result {
            Ok(Ok(subs)) => span.ok_json(serde_json::json!({
                "count": subs.len(),
                "subscriptions": subs,
            })),
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }

    #[tool(description = "Fetch/sync new entries from a feed (supports ETag/Last-Modified)")]
    async fn rss_fetch(
        &self,
        Parameters(FetchRequest { stream_id }): Parameters<FetchRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("rss_fetch", &self.webid);
        let db1 = self.db.clone();
        let sid1 = stream_id.clone();
        let lookup = tokio::task::spawn_blocking(move || {
            let conn = db1.lock().unwrap();
            let url = resolve_feed_url(&conn, &sid1)
                .ok_or_else(|| anyhow::anyhow!("Feed URL not found for stream_id"))?;
            let etag: Option<String> = conn
                .query_row("SELECT etag FROM feeds WHERE url = ?1", [&url], |row| {
                    row.get(0)
                })
                .ok();
            let lm: Option<String> = conn
                .query_row(
                    "SELECT last_modified FROM feeds WHERE url = ?1",
                    [&url],
                    |row| row.get(0),
                )
                .ok();
            Ok::<(String, Option<String>, Option<String>), anyhow::Error>((url, etag, lm))
        })
        .await;

        let (feed_url, cached_etag, cached_lm) = match lookup {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                return span.error(
                    McpErrorKind::NotFound,
                    McpToolError::not_found(e.to_string()).to_json_string(),
                );
            }
            Err(e) => {
                return span.internal_error(serde_json::json!({
                    "error": format!("Task error: {}", e),
                }));
            }
        };

        let fetch_result = match fetch_feed(
            &self.client,
            &feed_url,
            cached_etag.as_deref(),
            cached_lm.as_deref(),
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return span.error(
                    McpErrorKind::Unavailable,
                    McpToolError::unavailable(format!("Fetch failed: {}", e)).to_json_string(),
                );
            }
        };

        if fetch_result.status == 304 {
            return span.ok_json(serde_json::json!({
                "stream_id": stream_id,
                "new_entries": 0,
                "fetched": true,
                "not_modified": true,
            }));
        }

        let db2 = self.db.clone();
        let sid2 = stream_id.clone();
        let etag = fetch_result.etag.clone();
        let lm = fetch_result.last_modified.clone();

        let result = spawn_db(db2, move |conn| {
            let feed_id = upsert_feed(conn, &feed_url, &fetch_result.feed)?;
            let new_count = insert_entries(conn, feed_id, &fetch_result.feed.entries)?;
            update_feed_cache_headers(conn, feed_id, etag.as_deref(), lm.as_deref())?;
            Ok::<usize, anyhow::Error>(new_count)
        })
        .await;

        match result {
            Ok(Ok(new_count)) => span.ok_json(serde_json::json!({
                "stream_id": sid2,
                "new_entries": new_count,
                "fetched": true,
            })),
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }

    #[tool(
        description = "Get entries from a stream (Google Reader stream IDs: feed/*, user/-/state/*, user/-/label/*)"
    )]
    async fn rss_get_entries(
        &self,
        Parameters(GetEntriesRequest {
            stream_id,
            unread_only,
            starred_only,
            count,
            continuation_token,
        }): Parameters<GetEntriesRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("rss_get_entries", &self.webid);
        let limit = (count.unwrap_or(DEFAULT_PAGE_SIZE as u32) as usize).min(MAX_PAGE_SIZE);
        let offset = continuation_token
            .as_ref()
            .and_then(|t| {
                let bytes = base64::engine::general_purpose::STANDARD.decode(t).ok()?;
                serde_json::from_slice::<Continuation>(&bytes).ok()
            })
            .map(|c| c.offset)
            .unwrap_or(0);

        let db = self.db.clone();
        let sid = stream_id.clone();
        let result = spawn_db(db, move |conn| {
            query_entries(
                conn,
                &sid,
                unread_only.unwrap_or(false),
                starred_only.unwrap_or(false),
                offset,
                limit + 1,
            )
        })
        .await;

        match result {
            Ok(Ok(mut entries)) => {
                let has_more = entries.len() > limit;
                if has_more {
                    entries.truncate(limit);
                }
                let next_token = if has_more {
                    let cont = Continuation {
                        offset: offset + limit,
                        stream_id: stream_id.clone(),
                    };
                    Some(
                        base64::engine::general_purpose::STANDARD
                            .encode(serde_json::to_vec(&cont).unwrap_or_default()),
                    )
                } else {
                    None
                };
                span.ok_json(serde_json::json!({
                    "stream_id": stream_id,
                    "entries": entries,
                    "count": entries.len(),
                    "continuation_token": next_token,
                }))
            }
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }

    #[tool(description = "Mark all entries in a stream as read")]
    async fn rss_mark_all_read(
        &self,
        Parameters(MarkReadRequest { stream_id }): Parameters<MarkReadRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("rss_mark_all_read", &self.webid);
        let db = self.db.clone();
        let sid = stream_id.clone();
        let result = spawn_db(db, move |conn| mark_stream_read(conn, &sid)).await;

        match result {
            Ok(Ok(marked)) => span.ok_json(serde_json::json!({
                "stream_id": stream_id,
                "marked_read": marked,
            })),
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }

    #[tool(description = "Get unread count for a stream")]
    async fn rss_get_unread_count(
        &self,
        Parameters(UnreadCountRequest { stream_id }): Parameters<UnreadCountRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("rss_get_unread_count", &self.webid);
        let db = self.db.clone();
        let sid = stream_id.clone();
        let result = spawn_db(db, move |conn| count_entries(conn, &sid, true)).await;

        match result {
            Ok(Ok(count)) => span.ok_json(serde_json::json!({
                "stream_id": stream_id,
                "unread_count": count,
            })),
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }

    #[tool(description = "Full-text search across feed entries")]
    async fn rss_search(
        &self,
        Parameters(SearchRequest { query, limit }): Parameters<SearchRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("rss_search", &self.webid);
        let limit = (limit.unwrap_or(10) as usize).min(MAX_PAGE_SIZE);
        let db = self.db.clone();
        let q = query.clone();
        let result = spawn_db(db, move |conn| search_entries(conn, &q, limit)).await;

        match result {
            Ok(Ok(results)) => span.ok_json(serde_json::json!({
                "query": query,
                "results": results,
                "count": results.len(),
            })),
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }

    #[tool(description = "Export subscriptions as OPML 2.0")]
    async fn rss_export_opml(&self) -> String {
        let span = ToolSpanGuard::new("rss_export_opml", &self.webid);
        let db = self.db.clone();
        let result = spawn_db(db, export_opml).await;

        match result {
            Ok(Ok(opml)) => span.ok_json(serde_json::json!({"opml": opml})),
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }

    #[tool(description = "Import subscriptions from OPML content")]
    async fn rss_import_opml(
        &self,
        Parameters(ImportOpmlRequest { opml_content }): Parameters<ImportOpmlRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("rss_import_opml", &self.webid);
        let db = self.db.clone();
        let result = spawn_db(db, move |conn| import_opml(conn, &opml_content)).await;

        match result {
            Ok(Ok(v)) => span.ok_json(v),
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }

    #[tool(description = "Discover RSS/Atom feeds from a URL via HTML link autodiscovery")]
    async fn rss_discover_feeds(
        &self,
        Parameters(DiscoverRequest { url }): Parameters<DiscoverRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("rss_discover_feeds", &self.webid);
        if let Err(e) = validate_tool_url(&url) {
            return span.error(e.kind, e.to_json_string());
        }
        match discover_feeds(&self.client, &url).await {
            Ok(feeds) => span.ok_json(serde_json::json!({
                "url": url,
                "feeds": feeds,
                "count": feeds.len(),
            })),
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(e.to_string()).to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Edit tags on entries: mark read/unread, star/unstar, add/remove labels (Google Reader edit-tag)"
    )]
    async fn rss_edit_tag(&self, Parameters(req): Parameters<EditTagRequest>) -> String {
        let span = ToolSpanGuard::new("rss_edit_tag", &self.webid);
        let db = self.db.clone();
        let result = spawn_db(db, move |conn| edit_tags(conn, &req)).await;

        match result {
            Ok(Ok(v)) => span.ok_json(v),
            Ok(Err(e)) => span.internal_error(serde_json::json!({
                "error": e.to_string(),
            })),
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Task error: {}", e),
            })),
        }
    }
}
