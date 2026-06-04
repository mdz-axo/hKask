//! hKask MCP Replicant — Replicant chat MCP server
//!
//! Starts an MCP server over stdio exposing 2 tools:
//! - `replicant:chat` — Send a message to a replicant and receive a response
//! - `replicant:status` — Check replicant registration and identity
//!
//! # Environment Variables
//!
//! - `HKASK_AGENT_PERSONA` — Replicant persona name (default: "Curator")
//! - `HKASK_DEFAULT_MODEL` — Default model for inference (default: "deepseek-v4-pro")
//! - `OKAPI_BASE_URL` — Okapi API base URL (default: "http://127.0.0.1:11435")

use hkask_mcp::server::ServerContext;
use hkask_mcp_replicant::tools::ReplicantServer;

hkask_mcp::mcp_server_main!(
    "hkask-mcp-replicant",
    factory: |ctx: ServerContext| {
        let persona = std::env::var("HKASK_AGENT_PERSONA")
            .unwrap_or_else(|_| "Curator".to_string());
        let default_model = std::env::var("HKASK_DEFAULT_MODEL")
            .unwrap_or_else(|_| "deepseek-v4-pro".to_string());
        ReplicantServer::new(ctx.webid, &persona, &default_model)
    }
);
