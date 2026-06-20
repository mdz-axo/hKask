//! hkask-mcp-spec — binary entrypoint.
//!
//! Thin wrapper around the spec server library. The server struct and
//! tool methods live in lib.rs for fuzz testability (P5 Testing Discipline).

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "cns.mcp.spec", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(hkask_mcp::DaemonClient::new())
    } else {
        None
    };

    hkask_mcp_spec::run(replicant, daemon_client).await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = hkask_mcp::DaemonClient::new();
    let result = hkask_mcp::verify_startup_gates(&client, replicant, "spec", &[]).await?;
    tracing::info!(target: "cns.mcp.spec", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() { String::new() }
        else { format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools) }
    );
    Ok(())
}
