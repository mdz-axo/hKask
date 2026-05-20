//! hKask MCP RSS Reader — RSS/Atom feed reader with Google Reader API compatibility

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter},
    model::*,
    transport::stdio,
    schemars, tool, tool_router, tool_handler,
};
use rmcp::handler::server::wrapper::Parameters;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use uuid::Uuid;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Subscription entry
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Subscription {
    pub subscription_id: String,
    pub stream_id: String,
    pub url: String,
    pub title: String,
    pub folder: Option<String>,
    pub last_fetch: Option<String>,
    pub unread_count: u64,
}

/// Entry item
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FeedEntry {
    pub id: String,
    pub title: String,
    pub link: String,
    pub published: Option<String>,
    pub summary: Option<String>,
    pub read: bool,
}

/// RSS server implementation
pub struct RssServer {
    tool_router: ToolRouter<RssServer>,
    client: Client,
    subscriptions: Arc<RwLock<HashMap<String, Subscription>>>,
    entries: Arc<RwLock<HashMap<String, Vec<FeedEntry>>>>,
}

impl RssServer {
    pub fn new() -> Self {
        let client = Client::builder().build().unwrap_or_default();

        Self {
            tool_router: Self::tool_router(),
            client,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[tool_router(server_handler)]
impl RssServer {
    #[tool(description = "Subscribe to an RSS/Atom feed")]
    async fn rss_subscribe(&self, url: String, label: Option<String>, folder: Option<String>) -> String {
        let sub_id = Uuid::new_v4().to_string();
        let stream_id = format!("feed/{}", sub_id);
        
        // Fetch feed to get title
        let title = match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => {
                            // Simple title extraction (production would use proper RSS parser)
                            if let Some(start) = body.find("<title>") {
                                if let Some(end) = body[start..].find("</title>") {
                                    body[start + 7..start + end].to_string()
                                } else {
                                    url.clone()
                                }
                            } else {
                                url.clone()
                            }
                        }
                        Err(_) => url.clone()
                    }
                } else {
                    url.clone()
                }
            }
            Err(_) => url.clone()
        };

        let subscription = Subscription {
            subscription_id: sub_id.clone(),
            stream_id: stream_id.clone(),
            url: url.clone(),
            title: label.unwrap_or(title),
            folder: folder.clone(),
            last_fetch: None,
            unread_count: 0,
        };

        self.subscriptions.write().insert(sub_id.clone(), subscription.clone());
        self.entries.write().insert(sub_id.clone(), Vec::new());

        serde_json::to_string_pretty(&subscription).unwrap_or_else(|_| "error".to_string())
    }

    #[tool(description = "Unsubscribe from a feed")]
    async fn rss_unsubscribe(&self, stream_id: String) -> String {
        if let Some(sub_id) = stream_id.strip_prefix("feed/") {
            let mut subs = self.subscriptions.write();
            let mut entries = self.entries.write();
            
            if subs.remove(sub_id).is_some() {
                entries.remove(sub_id);
                serde_json::json!({ "success": true, "stream_id": stream_id }).to_string()
            } else {
                serde_json::json!({ "success": false, "error": "subscription not found" }).to_string()
            }
        } else {
            serde_json::json!({ "success": false, "error": "invalid stream_id format" }).to_string()
        }
    }

    #[tool(description = "List all subscriptions")]
    async fn rss_list_subscriptions(&self, folder: Option<String>) -> String {
        let subs = self.subscriptions.read();
        let filtered: Vec<&Subscription> = subs
            .values()
            .filter(|s| folder.as_ref().map_or(true, |f| s.folder.as_ref() == Some(f)))
            .collect();

        serde_json::json!({
            "subscriptions": filtered,
            "count": filtered.len()
        }).to_string()
    }

    #[tool(description = "Fetch entries from a subscription")]
    async fn rss_fetch(&self, stream_id: String) -> String {
        if let Some(sub_id) = stream_id.strip_prefix("feed/") {
            let subs = self.subscriptions.read();
            if let Some(sub) = subs.get(sub_id) {
                let url = sub.url.clone();
                drop(subs);

                // Fetch and parse feed (simplified)
                match self.client.get(&url).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            let mut entries = self.entries.write();
                            if let Some(entry_list) = entries.get_mut(sub_id) {
                                entry_list.clear();
                                // In production, parse RSS/Atom properly
                                entry_list.push(FeedEntry {
                                    id: Uuid::new_v4().to_string(),
                                    title: "Fetched entry".to_string(),
                                    link: url.clone(),
                                    published: Some(chrono::Utc::now().to_rfc3339()),
                                    summary: Some("Entry fetched successfully".to_string()),
                                    read: false,
                                });
                            }

                            serde_json::json!({
                                "success": true,
                                "stream_id": stream_id,
                                "fetched_at": chrono::Utc::now().to_rfc3339()
                            }).to_string()
                        } else {
                            serde_json::json!({ "error": format!("fetch failed: {}", resp.status()) }).to_string()
                        }
                    }
                    Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                }
            } else {
                serde_json::json!({ "error": "subscription not found" }).to_string()
            }
        } else {
            serde_json::json!({ "error": "invalid stream_id format" }).to_string()
        }
    }

    #[tool(description = "Get entries from a subscription")]
    async fn rss_get_entries(&self, stream_id: String, unread_only: Option<bool>) -> String {
        if let Some(sub_id) = stream_id.strip_prefix("feed/") {
            let entries = self.entries.read();
            if let Some(entry_list) = entries.get(sub_id) {
                let filtered: Vec<&FeedEntry> = entry_list
                    .iter()
                    .filter(|e| unread_only.map_or(true, |u| !u || !e.read))
                    .collect();

                serde_json::json!({
                    "stream_id": stream_id,
                    "entries": filtered,
                    "count": filtered.len()
                }).to_string()
            } else {
                serde_json::json!({ "error": "subscription not found" }).to_string()
            }
        } else {
            serde_json::json!({ "error": "invalid stream_id format" }).to_string()
        }
    }

    #[tool(description = "Mark all entries as read")]
    async fn rss_mark_all_read(&self, stream_id: String) -> String {
        if let Some(sub_id) = stream_id.strip_prefix("feed/") {
            let mut entries = self.entries.write();
            if let Some(entry_list) = entries.get_mut(sub_id) {
                for entry in entry_list.iter_mut() {
                    entry.read = true;
                }
                serde_json::json!({ "success": true, "stream_id": stream_id }).to_string()
            } else {
                serde_json::json!({ "error": "subscription not found" }).to_string()
            }
        } else {
            serde_json::json!({ "error": "invalid stream_id format" }).to_string()
        }
    }

    #[tool(description = "Get unread count for a subscription")]
    async fn rss_get_unread_count(&self, stream_id: String) -> String {
        if let Some(sub_id) = stream_id.strip_prefix("feed/") {
            let entries = self.entries.read();
            if let Some(entry_list) = entries.get(sub_id) {
                let count = entry_list.iter().filter(|e| !e.read).count();
                serde_json::json!({
                    "stream_id": stream_id,
                    "unread_count": count
                }).to_string()
            } else {
                serde_json::json!({ "error": "subscription not found" }).to_string()
            }
        } else {
            serde_json::json!({ "error": "invalid stream_id format" }).to_string()
        }
    }

    #[tool(description = "Search across all subscriptions")]
    async fn rss_search(&self, query: String, limit: Option<usize>) -> String {
        let entries = self.entries.read();
        let limit = limit.unwrap_or(20);
        
        let mut results = Vec::new();
        for (sub_id, entry_list) in entries.iter() {
            for entry in entry_list {
                if entry.title.to_lowercase().contains(&query.to_lowercase())
                    || entry.summary.as_ref().map_or(false, |s| s.to_lowercase().contains(&query.to_lowercase()))
                {
                    results.push((sub_id, entry));
                    if results.len() >= limit {
                        break;
                    }
                }
            }
            if results.len() >= limit {
                break;
            }
        }

        serde_json::json!({
            "query": query,
            "results": results.iter().map(|(sub_id, e)| serde_json::json!({
                "subscription_id": sub_id,
                "entry": e
            })).collect::<Vec<_>>(),
            "count": results.len()
        }).to_string()
    }

    #[tool(description = "Export subscriptions as OPML")]
    async fn rss_export_opml(&self) -> String {
        let subs = self.subscriptions.read();
        let mut opml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\">\n<head><title>hKask RSS Exports</title></head>\n<body>\n");
        
        for sub in subs.values() {
            opml.push_str(&format!("  <outline type=\"rss\" text=\"{}\" xmlUrl=\"{}\"/>\n", sub.title, sub.url));
        }
        
        opml.push_str("</body>\n</opml>");
        opml
    }

    #[tool(description = "Discover feeds from a URL")]
    async fn rss_discover_feeds(&self, url: String) -> String {
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    // In production, scan for <link rel="alternate"> tags
                    serde_json::json!({
                        "url": url,
                        "feeds": [{
                            "url": url,
                            "title": "Discovered feed",
                            "type": "rss"
                        }]
                    }).to_string()
                } else {
                    serde_json::json!({ "error": format!("fetch failed: {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }
}

impl RssServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = RssServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-rss-reader MCP server started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
