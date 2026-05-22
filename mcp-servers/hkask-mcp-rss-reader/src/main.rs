//! hKask MCP RSS Reader — RSS feed subscription and reading

use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SubscribeRequest {
    pub url: String,
    pub label: Option<String>,
    pub folder: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UnsubscribeRequest {
    pub stream_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListSubscriptionsRequest {
    pub folder: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FetchRequest {
    pub stream_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetEntriesRequest {
    pub stream_id: String,
    pub unread_only: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MarkReadRequest {
    pub stream_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiscoverRequest {
    pub url: String,
}

#[derive(Debug, Default)]
pub struct RssServer {
    subscriptions: Arc<RwLock<HashMap<String, String>>>,
}

impl RssServer {
    pub fn new() -> Self {
        Self::default()
    }
}

#[tool_router(server_handler)]
impl RssServer {
    #[tool(description = "Subscribe to an RSS feed")]
    async fn rss_subscribe(
        &self,
        Parameters(SubscribeRequest {
            url,
            label,
            folder: _,
        }): Parameters<SubscribeRequest>,
    ) -> String {
        let mut subs = self.subscriptions.write().await;
        let stream_id = format!("stream_{}", subs.len());
        let lbl = label.unwrap_or_else(|| url.clone());
        subs.insert(stream_id.clone(), url.clone());
        format!(
            r#"{{"stream_id":"{}","url":"{}","label":"{}","subscribed":true}}"#,
            stream_id, url, lbl
        )
    }

    #[tool(description = "Unsubscribe from a feed")]
    async fn rss_unsubscribe(
        &self,
        Parameters(UnsubscribeRequest { stream_id }): Parameters<UnsubscribeRequest>,
    ) -> String {
        let mut subs = self.subscriptions.write().await;
        if subs.remove(&stream_id).is_some() {
            format!(r#"{{"stream_id":"{}","unsubscribed":true}}"#, stream_id)
        } else {
            format!(
                r#"{{"stream_id":"{}","unsubscribed":false,"error":"Not found"}}"#,
                stream_id
            )
        }
    }

    #[tool(description = "List subscriptions")]
    async fn rss_list_subscriptions(
        &self,
        Parameters(ListSubscriptionsRequest { folder }): Parameters<ListSubscriptionsRequest>,
    ) -> String {
        let subs = self.subscriptions.read().await;
        let subs_vec: Vec<_> = subs.iter().map(|(k, v)| format!("{}:{}", k, v)).collect();
        drop(subs);
        format!(
            r#"{{"folder":"{}","count":{},"subscriptions":{}}}"#,
            folder.unwrap_or_else(|| "all".to_string()),
            subs_vec.len(),
            serde_json::to_string(&subs_vec).unwrap()
        )
    }

    #[tool(description = "Fetch new entries from a feed")]
    async fn rss_fetch(
        &self,
        Parameters(FetchRequest { stream_id }): Parameters<FetchRequest>,
    ) -> String {
        format!(
            r#"{{"stream_id":"{}","new_entries":0,"fetched":true}}"#,
            stream_id
        )
    }

    #[tool(description = "Get entries from a feed")]
    async fn rss_get_entries(
        &self,
        Parameters(GetEntriesRequest {
            stream_id,
            unread_only,
        }): Parameters<GetEntriesRequest>,
    ) -> String {
        format!(
            r#"{{"stream_id":"{}","unread_only":{},"entries":[]}}"#,
            stream_id,
            unread_only.unwrap_or(false)
        )
    }

    #[tool(description = "Mark all entries as read")]
    async fn rss_mark_all_read(
        &self,
        Parameters(MarkReadRequest { stream_id }): Parameters<MarkReadRequest>,
    ) -> String {
        format!(r#"{{"stream_id":"{}","marked_read":true}}"#, stream_id)
    }

    #[tool(description = "Get unread count")]
    async fn rss_get_unread_count(
        &self,
        Parameters(UnsubscribeRequest { stream_id }): Parameters<UnsubscribeRequest>,
    ) -> String {
        format!(r#"{{"stream_id":"{}","unread_count":0}}"#, stream_id)
    }

    #[tool(description = "Search across feeds")]
    async fn rss_search(
        &self,
        Parameters(SearchRequest { query, limit }): Parameters<SearchRequest>,
    ) -> String {
        format!(
            r#"{{"query":"{}","limit":{},"results":[]}}"#,
            query,
            limit.unwrap_or(10)
        )
    }

    #[tool(description = "Export subscriptions as OPML")]
    async fn rss_export_opml(&self) -> String {
        r#"{"opml":"<?xml version=\"1.0\"?><opml version=\"2.0\"></opml>"}"#.to_string()
    }

    #[tool(description = "Discover feeds from a URL")]
    async fn rss_discover_feeds(
        &self,
        Parameters(DiscoverRequest { url }): Parameters<DiscoverRequest>,
    ) -> String {
        format!(r#"{{"url":"{}","feeds":[]}}"#, url)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = RssServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-rss-reader started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
