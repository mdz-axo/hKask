//! hkask-mcp-research — binary entrypoint.
//!
//! Thin wrapper around the research server library. The server struct and
//! tool methods live in lib.rs for testability (P5 Testing Discipline).

use hkask_mcp::DaemonClient;

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    // Attempt daemon connection for P4 dual-gate verification.
    // Falls back to direct mode if daemon is not available (development).
    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(
                target: "hkask.mcp.research",
                replicant = %replicant,
                error = %e,
                "Daemon unavailable — falling back to direct mode (no OCAP verification)"
            );
            false
        }
    };

    if !daemon_ok {
        tracing::warn!(
            target: "hkask.mcp.research",
            "Running without daemon — P4 dual-gate verification skipped. Start hKask daemon for full OCAP enforcement."
        );
    }

    // Create daemon client if daemon flow succeeded
    let daemon_client = if daemon_ok {
        Some(DaemonClient::new())
    } else {
        None
    };

    hkask_mcp_research::run(replicant, daemon_client).await
}

/// Attempt the daemon-mediated P4 gate flow:
/// 1. Auth query — is the replicant authenticated?
/// 2. Assignment query — is the replicant assigned to "research"?
/// 3. Capability queries — does the replicant hold tool capabilities?
async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = DaemonClient::new();
    let result = hkask_mcp::verify_startup_gates(
        &client,
        replicant,
        "research",
        &["web_search", "web_extract", "web_browse"],
    )
    .await?;
    tracing::info!(target: "hkask.mcp.research", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() { String::new() }
        else { format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools) }
    );
    Ok(())
}
