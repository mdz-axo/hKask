//! hKask MCP RSS Reader — Google Reader–compatible feed reader
//!
//! Data model follows the Google Reader spec:
//! - Stream-based content access (`feed/{url}`, `user/-/state/com.google/*`, `user/-/label/*`)
//! - Subscription management with labels/folders
//! - Read/unread/star state tracking per entry
//! - FTS5 full-text search
//! - OPML import/export
//! - Feed autodiscovery via HTML <link> parsing
//! - Conditional HTTP requests (ETag / Last-Modified)
//! - Continuation-token pagination

mod db;
mod server;
mod types;

use hkask_mcp::server::{ServerContext, run_stdio_server};
use server::RssServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_server(
        "hkask-mcp-rss-reader",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| RssServer::new(ctx.webid),
        vec![],
    )
    .await
}

