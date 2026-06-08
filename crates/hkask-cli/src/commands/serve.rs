//! `kask serve` — Start the HTTP API server sharing CLI state
//!
//! Creates an `ApiState` via `ServiceContext::build()` and starts the
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
    ("inference", "hkask-mcp-inference"),
    ("cns", "hkask-mcp-cns"),
    ("condenser", "hkask-mcp-condenser"),
    ("episodic", "hkask-mcp-episodic"),
    ("semantic", "hkask-mcp-semantic"),
    ("ocap", "hkask-mcp-ocap"),
    ("keystore", "hkask-mcp-keystore"),
    ("git", "hkask-mcp-git"),
    ("registry", "hkask-mcp-registry"),
    ("goal", "hkask-mcp-goal"),
    ("doc-knowledge", "hkask-mcp-doc-knowledge"),
];

/// Run the API server, sharing state with the CLI.
///
/// Resolves configuration from the keystore and environment, builds a
/// `ServiceContext` with all shared infrastructure, starts API MCP servers
/// on the ServiceContext's runtime, and creates an `ApiState` from it.
pub async fn run_server(port: u16, host: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Get CLI singleton for improv client
    let improv_client = crate::commands::ensemble::get_improv_client(None);

    // Extract the base InferencePortAdapter from the circuit-breaker-wrapped client
    let base_adapter = Arc::new(improv_client.inner().clone());

    // Resolve configuration from keystore and environment
    let config = hkask_services::ServiceConfig::from_env().unwrap_or_else(|e| {
        tracing::warn!(
            target: "hkask.serve",
            error = %e,
            "Failed to resolve service config from env, using in-memory"
        );
        hkask_services::ServiceConfig::in_memory()
    });

    // Build ServiceContext with all shared infrastructure
    let ctx = hkask_services::ServiceContext::build(config)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
        })?;

    // Start API MCP servers on the ServiceContext's runtime
    let server_count = start_api_servers(&ctx.mcp_runtime).await;
    if server_count > 0 {
        tracing::info!(target: "hkask.serve", servers = server_count, "MCP servers started");
    } else {
        tracing::warn!(target: "hkask.serve", "No MCP servers started — tool endpoints will return empty results");
    }

    // Build ApiState from ServiceContext, adding CLI's ensemble adapter
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
