//! hKask MCP Inference — Okapi-backed LLM inference MCP server
//!
//! Starts an MCP server over stdio exposing 3 tools:
//! - `inference:generate` — Generate text via Okapi LLM
//! - `inference:metrics` — Get current inference metrics
//! - `inference:models` — List available model tiers

use hkask_mcp::server::{ServerContext, run_stdio_server};
use hkask_mcp_inference::tools::InferenceServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_server(
        "hkask-mcp-inference",
        env!("CARGO_PKG_VERSION"),
        |_ctx: ServerContext| Ok(InferenceServer::new()),
        vec![], // no credentials required — Okapi uses default config
    )
    .await
}
