//! hKask MCP Inference — Okapi-backed LLM inference MCP server
//!
//! Starts an MCP server over stdio exposing 3 tools:
//! - `inference_generate` — Generate text via Okapi LLM
//! - `inference_metrics` — Get current inference metrics
//! - `inference_models` — List available model tiers

use hkask_mcp_inference::tools::InferenceServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-inference",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| InferenceServer::new(ctx.webid),
        vec![],
    )
    .await
}
