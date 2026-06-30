#![cfg(feature = "api")]
//! `kask serve` — Start the HTTP API server sharing CLI state
//!
//! Creates an `ApiState` via `AgentService::build()` and starts the
//! axum HTTP server. CLI commands issued while the server is running
//! operate on the same shared state.

use hkask_api::ApiState;
use hkask_mcp::BUILTIN_SERVERS;
use hkask_mcp::runtime::McpRuntime;

/// Run the API server, sharing state with the CLI.
///
/// Resolves configuration from the keystore and environment, builds a
/// `AgentService` with all shared infrastructure, starts API MCP servers
/// on the AgentService's runtime, and creates an `ApiState` from it.
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  port is a valid u16; host is a non-empty bind address string
/// post: starts the HTTP API server on the given host:port; returns Ok(()) on successful bind or Error on failure
pub async fn run_server(port: u16, host: &str) -> Result<(), Box<dyn std::error::Error>> {
    // P9: CNS span
    tracing::info!(target: "cns.cli", operation = "serve", host = %host, port = port, "CNS");
    // Resolve configuration from keystore and environment.
    // Refuse to start with in-memory fallback — a server without proper
    // keystore configuration has no security, no persistence, and no auth.
    let config = hkask_services_core::ServiceConfig::from_env().map_err(|e| {
        format!(
            "Failed to resolve service configuration: {e}\n\
             Run 'kask init' first to set up the server, or 'kask chat' to\n\
             complete onboarding with your master passphrase."
        )
    })?;

    // Build AgentService with all shared infrastructure
    let ctx = hkask_services_context::AgentService::build(config)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
        })?;

    // Start API MCP servers on the AgentService's runtime.
    // Derived from hkask_mcp::BUILTIN_SERVERS (canonical registry).
    let replicant_name = ctx.config().agent_name.clone();
    let server_count = start_api_servers(ctx.mcp_runtime(), &replicant_name).await;
    if server_count > 0 {
        tracing::info!(target: "hkask.serve", servers = server_count, "MCP servers started");
    } else {
        tracing::warn!(target: "hkask.serve", "No MCP servers started — tool endpoints will return empty results");
    }

    // Build ApiState from AgentService
    let state = ApiState::from_service_context(ctx)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // Build router (OpenApiRouter -> axum::Router via From impl)
    let app: axum::Router = hkask_api::create_router(state)
        .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)?
        .into();

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(target: "hkask.serve", addr = %addr, "Starting hKask API server");
    println!("hKask API server listening on {}", addr);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

/// Start all API MCP servers and discover their tools.
///
/// Started servers are derived from `hkask_mcp::BUILTIN_SERVERS`.
/// excluded from the API sandbox for security: the API server is a
/// headless HTTP endpoint — it does not expose local filesystem access,
/// governance, registry management, or kanban task coordination.
const API_EXCLUDED: &[&str] = &["filesystem", "curator", "kanban", "skill"];

async fn start_api_servers(runtime: &McpRuntime, replicant_name: &str) -> usize {
    let mut started = 0;
    let mut extra_env = std::collections::HashMap::new();
    extra_env.insert("HKASK_MCP_HOST".to_string(), replicant_name.to_string());

    for (server_id, command) in BUILTIN_SERVERS {
        if API_EXCLUDED.contains(server_id) {
            continue;
        }
        match runtime
            .start_server_with_env(server_id, command, extra_env.clone())
            .await
        {
            Ok(()) => {
                tracing::debug!(target: "hkask.serve", server_id = %server_id, "MCP server started");
                started += 1;
            }
            Err(e) => {
                tracing::warn!(
                    target: "hkask.serve",
                    server_id = %server_id,
                    error = %e,
                    "Failed to start MCP server — tools will be unavailable"
                );
            }
        }
    }

    started
}
