//! hkask-mcp-curator — binary entrypoint.
//!
//! Thin wrapper around the curator server library.

#![allow(unused_crate_dependencies)] // All deps used in this binary — lint produces false positives

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    let replicant =
        std::env::var("HKASK_CURATOR_REPLICANT").unwrap_or_else(|_| "curator".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(
                target: "hkask.mcp.curator",
                replicant = %replicant,
                error = %e,
                "Daemon unavailable — falling back to direct mode"
            );
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(hkask_mcp::DaemonClient::new())
    } else {
        None
    };

    hkask_mcp_curator::run(replicant, daemon_client).await
}

async fn try_daemon_flow(replicant: &str) -> Result<(), hkask_mcp::McpError> {
    let client = hkask_mcp::DaemonClient::new();
    let result = hkask_mcp::verify_startup_gates(&client, replicant, "curator", &[]).await?;
    tracing::info!(
        target: "hkask.mcp.curator",
        replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() {
            String::new()
        } else {
            format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools)
        }
    );
    Ok(())
}
