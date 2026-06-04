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
//!
//! Storage uses `hkask-storage::Database` with SQLCipher encryption.
//! - `HKASK_RSS_DB` + `HKASK_DB_PASSPHRASE`: persistent encrypted database
//! - Absent: in-memory (ephemeral, data lost on restart)

mod db;
mod server;
mod types;

use server::RssServer;

hkask_mcp::mcp_server_main!(
    "hkask-mcp-rss-reader",
    factory: |ctx: hkask_mcp::ServerContext| RssServer::new(ctx),
    credentials: vec![
        hkask_mcp::CredentialRequirement::optional(
            "HKASK_RSS_DB",
            "Path to the RSS reader SQLite database (in-memory if absent)",
        ),
        hkask_mcp::CredentialRequirement::optional(
            "HKASK_DB_PASSPHRASE",
            "Passphrase for SQLCipher encryption (required if HKASK_RSS_DB is set)",
        ),
    ]
);
