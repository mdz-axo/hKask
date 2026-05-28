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

use server::RssServer;

hkask_mcp::mcp_server_main!("hkask-mcp-rss-reader", RssServer);
