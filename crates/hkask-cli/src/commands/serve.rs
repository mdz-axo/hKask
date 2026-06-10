//! `kask serve` — Start the HTTP API server sharing CLI state
//!
//! Creates an `ApiState` via `AgentService::build()` and starts the
//! axum HTTP server. CLI commands issued while the server is running
//! operate on the same shared state.

use hkask_api::ApiState;
use hkask_mcp::runtime::McpRuntime;
use std::sync::Arc;

/// MCP servers to start for the API.
///
/// Each entry maps `(server_id, binary_name)`. The binary must be on PATH
/// or specified via the `HKASK_MCP_{ID}_BIN` environment variable.
const API_SERVERS: &[(&str, &str)] = &[
    ("memory", "hkask-mcp-memory"),
    ("condenser", "hkask-mcp-condenser"),
    ("spec", "hkask-mcp-spec"),
    ("web", "hkask-mcp-web"),
    ("fmp", "hkask-mcp-fmp"),
    ("telnyx", "hkask-mcp-telnyx"),
    ("fal", "hkask-mcp-fal"),
    ("rss-reader", "hkask-mcp-rss-reader"),
    ("doc-knowledge", "hkask-mcp-doc-knowledge"),
    ("markitdown", "hkask-mcp-markitdown"),
];

/// Run the API server, sharing state with the CLI.
///
/// Resolves configuration from the keystore and environment, builds a
/// `AgentService` with all shared infrastructure, starts API MCP servers
/// on the AgentService's runtime, and creates an `ApiState` from it.
pub async fn run_server(port: u16, host: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve configuration from keystore and environment
    let config = hkask_services::ServiceConfig::from_env().unwrap_or_else(|e| {
        tracing::warn!(
            target: "hkask.serve",
            error = %e,
            "Failed to resolve service config from env, using in-memory"
        );
        hkask_services::ServiceConfig::in_memory()
    });

    // Build AgentService with all shared infrastructure
    let ctx = hkask_services::AgentService::build(config)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
        })?;

    // Build improv client from AgentService's inference port
    let improv_client =
        hkask_services::EnsembleService::build_improv_client(&ctx, None).map_err(|e| {
            Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
        })?;
    let base_adapter = Arc::new(improv_client.inner().clone());

    // Start API MCP servers on the AgentService's runtime
    let server_count = start_api_servers(ctx.mcp_runtime()).await;
    if server_count > 0 {
        tracing::info!(target: "hkask.serve", servers = server_count, "MCP servers started");
    } else {
        tracing::warn!(target: "hkask.serve", "No MCP servers started — tool endpoints will return empty results");
    }

    // Build ApiState from AgentService, adding CLI's ensemble adapter
    let state = ApiState::from_service_context(ctx, Some(base_adapter))
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
/// Returns the number of servers that started successfully.
/// Servers that fail to start are logged and skipped.
async fn start_api_servers(runtime: &McpRuntime) -> usize {
    let mut started = 0;

    for (server_id, command) in API_SERVERS {
        match runtime.start_server(server_id, command).await {
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
