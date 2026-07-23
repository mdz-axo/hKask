//! hkask-mcp-filesystem — binary entrypoint.
//!
//! Thin wrapper around the filesystem server library. The server struct and
//! tool methods live in lib.rs for fuzz testability (P5 Testing Discipline).

#![allow(unused_crate_dependencies)] // All deps used in this binary — lint produces false positives

use hkask_mcp_filesystem::FileSystemServer;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), hkask_mcp_server::McpError> {
    let boot = hkask_mcp_server::bootstrap_mcp_server(
        "filesystem",
        "hkask.mcp.filesystem",
        "HKASK_MCP_HOST",
    )
    .await?;

    let project_root = std::env::var("HKASK_PROJECT_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Destructive consent (P2 — Affirmative Consent): fs_write/fs_edit/fs_delete/
    // shell_exec are denied unless the human explicitly opts in at launch. Read
    // tools (fs_read/fs_list/fs_search) are always available.
    let destructive_consent = std::env::var("HKASK_FILESYSTEM_DESTRUCTIVE_CONSENT")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    hkask_mcp_server::run_server(
        "hkask-mcp-filesystem",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp_server::server::ServerContext| {
            Ok(FileSystemServer::new(
                ctx.webid,
                boot.userpod.clone(),
                boot.daemon_client.clone(),
                project_root.clone(),
                ctx.capability_tier,
                destructive_consent,
            ))
        },
        vec![], // No required credentials — filesystem access is inherent
    )
    .await
}
