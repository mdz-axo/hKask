//! `kask serve` — Start the HTTP API server sharing CLI state
//!
//! Creates an `ApiState` via `AgentService::build()` and starts the
//! axum HTTP server. CLI commands issued while the server is running
//! operate on the same shared state.

use hkask_api::ApiState;
use hkask_mcp::runtime::McpRuntime;

/// MCP servers to start for the API.
///
/// Each entry maps `(server_id, binary_name)`. The binary must be on PATH
/// or specified via the `HKASK_MCP_{ID}_BIN` environment variable.
const API_SERVERS: &[(&str, &str)] = &[
    ("memory", "hkask-mcp-memory"),
    ("condenser", "hkask-mcp-condenser"),
    ("spec", "hkask-mcp-spec"),
    ("research", "hkask-mcp-research"),
    ("companies", "hkask-mcp-companies"),
    ("communication", "hkask-mcp-communication"),
    ("fal", "hkask-mcp-media"),
    ("docproc", "hkask-mcp-docproc"),
    ("training", "hkask-mcp-training"),
    ("replica", "hkask-mcp-replica"),
];

/// Run the API server, sharing state with the CLI.
///
/// Resolves configuration from the keystore and environment, builds a
/// `AgentService` with all shared infrastructure, starts API MCP servers
/// on the AgentService's runtime, and creates an `ApiState` from it.
/// expect: "I can access all hKask functionality through the kask CLI"
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  port is a valid u16; host is a non-empty bind address string
/// post: starts the HTTP API server on the given host:port; returns Ok(()) on successful bind or Error on failure
pub async fn run_server(
    port: u16,
    host: &str,
    json_logs: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Configure JSON logging if requested
    if json_logs {
        use tracing_subscriber::EnvFilter;
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_env_filter(EnvFilter::from_default_env())
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    }
    // P9: CNS span
    tracing::info!(target: "cns.cli", operation = "serve", host = %host, port = port, "CNS");
    // Resolve configuration from keystore and environment.
    // Refuse to start with in-memory fallback — a server without proper
    // keystore configuration has no security, no persistence, and no auth.
    let config = hkask_services::ServiceConfig::from_env().map_err(|e| {
        format!(
            "Failed to resolve service configuration: {e}\n\
             Run 'kask init' first to set up the server, or 'kask chat' to\n\
             complete onboarding with your master passphrase."
        )
    })?;

    // Build AgentService with all shared infrastructure
    let ctx = hkask_services::AgentService::build(config)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
        })?;

    // Start API MCP servers on the AgentService's runtime
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
/// Each server receives `HKASK_REPLICANT={replicant_name}` in its
/// environment, so per-agent databases (agents/{name}/) are used.
/// Returns the number of servers that started successfully.
/// Servers that fail to start are logged and skipped.
async fn start_api_servers(runtime: &McpRuntime, replicant_name: &str) -> usize {
    let mut started = 0;
    let mut extra_env = std::collections::HashMap::new();
    extra_env.insert("HKASK_REPLICANT".to_string(), replicant_name.to_string());

    for (server_id, command) in API_SERVERS {
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
