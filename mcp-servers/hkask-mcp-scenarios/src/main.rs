//! hkask-mcp-scenarios — binary entrypoint.
//!
//! Thin wrapper around the scenarios server library.

#![allow(unused_crate_dependencies)]

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    let boot =
        hkask_mcp::bootstrap_mcp_server("scenarios", "hkask.mcp.scenarios", "HKASK_MCP_HOST")
            .await?;
    hkask_mcp_scenarios::run(boot.replicant, boot.daemon_client).await
}
