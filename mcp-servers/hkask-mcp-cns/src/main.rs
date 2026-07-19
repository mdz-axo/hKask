//! hkask-mcp-cns — binary entrypoint.
//!
//! Thin wrapper around the CNS server library.

#![allow(unused_crate_dependencies)]

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    let boot = hkask_mcp::bootstrap_mcp_server("cns", "hkask.mcp.cns", "HKASK_MCP_HOST").await?;
    hkask_mcp_cns::run(boot.replicant, boot.daemon_client).await
}
