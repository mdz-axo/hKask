//! hkask-mcp-filesystem — binary entrypoint.
//!
//! Thin wrapper around the filesystem server library. The server struct and
//! tool methods live in lib.rs for fuzz testability (P5 Testing Discipline).

#![allow(unused_crate_dependencies)] // All deps used in this binary — lint produces false positives

use hkask_mcp_filesystem::FileSystemServer;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    let boot =
        hkask_mcp::bootstrap_mcp_server("filesystem", "hkask.mcp.filesystem", "HKASK_MCP_HOST")
            .await?;

    let project_root = std::env::var("HKASK_PROJECT_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    hkask_mcp::run_server(
        "hkask-mcp-filesystem",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::server::ServerContext| {
            Ok(FileSystemServer::new(
                ctx.webid,
                boot.replicant.clone(),
                boot.daemon_client.clone(),
                project_root.clone(),
                ctx.capability_tier,
            ))
        },
        vec![], // No required credentials — filesystem access is inherent
    )
    .await
}
