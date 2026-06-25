//! hkask-mcp-filesystem — binary entrypoint.
//!
//! Thin wrapper around the filesystem server library. The server struct and
//! tool methods live in lib.rs for fuzz testability (P5 Testing Discipline).

use hkask_mcp_filesystem::FileSystemServer;

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    dotenvy::dotenv().ok();
    let replicant =
        std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    // P4 Gate 1/2/3: Authenticate + verify role + check WebID
    let daemon_client = hkask_mcp::DaemonClient::new();
    let daemon_ok = match hkask_mcp::verify_startup_gates(
        &daemon_client,
        &replicant,
        "filesystem",
        &[],
    )
    .await
    {
        Ok(result) => {
            tracing::info!(
                target: "hkask.mcp.filesystem",
                replicant = %replicant,
                "P4 gates verified{}",
                if result.denied_tools.is_empty() {
                    String::new()
                } else {
                    format!(
                        " — {} tool(s) denied: {:?}",
                        result.denied_tools.len(),
                        result.denied_tools
                    )
                }
            );
            true
        }
        Err(e) => {
            tracing::warn!(
                target: "hkask.mcp.filesystem",
                replicant = %replicant,
                error = %e,
                "Daemon unavailable — falling back to standalone mode"
            );
            false
        }
    };

    hkask_mcp::run_server(
        "hkask-mcp-filesystem",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::server::ServerContext| {
            Ok(FileSystemServer {
                webid: ctx.webid,
                replicant: replicant.clone(),
                daemon: if daemon_ok {
                    Some(hkask_mcp::DaemonClient::new())
                } else {
                    None
                },
            })
        },
        vec![], // No required credentials — filesystem access is inherent
    )
    .await
}
