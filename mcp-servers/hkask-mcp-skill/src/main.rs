//! hkask-mcp-skill — binary entrypoint.
//!
//! Thin wrapper around the skill server library. The server struct and
//! tool methods live in lib.rs for testability (P5 Testing Discipline).

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    dotenvy::dotenv().ok();
    let replicant =
        std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_client = match try_daemon_flow(&replicant).await {
        Ok(()) => Some(hkask_mcp::DaemonClient::new()),
        Err(e) => {
            tracing::warn!(
                target: "hkask.mcp.skill",
                replicant = %replicant,
                error = %e,
                "Daemon unavailable"
            );
            None
        }
    };

    hkask_mcp_skill::run(replicant, daemon_client).await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = hkask_mcp::DaemonClient::new();
    let result = hkask_mcp::verify_startup_gates(
        &client,
        replicant,
        "skill",
        &["skill_ping", "skill_list", "skill_execute"],
    )
    .await?;
    tracing::info!(
        target: "hkask.mcp.skill",
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
    Ok(())
}
