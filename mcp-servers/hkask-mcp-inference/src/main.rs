//! hKask MCP Inference — Okapi-backed LLM inference MCP server
//!
//! Starts an MCP server over stdio exposing 3 tools:
//! - `inference:generate` — Generate text via Okapi LLM
//! - `inference:metrics` — Get current inference metrics
//! - `inference:models` — List available model tiers

use hkask_mcp_inference::tools::InferenceServer;
use rmcp::ServiceExt;
use rmcp::transport::stdio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = InferenceServer::new();
    let service = server.serve(stdio());
    tracing::info!(
        "hkask-mcp-inference started (v{})",
        hkask_mcp_inference::SERVER_VERSION
    );
    service.await?;
    Ok(())
}
