//! `kask serve` — Start the HTTP API server sharing CLI state
//!
//! Creates an `ApiState` wired to the CLI's existing singletons
//! (SessionManager, CyberneticsLoop, InferencePortAdapter) and starts
//! the axum HTTP server. CLI commands issued while the server is running
//! operate on the same shared state.

use hkask_api::ApiState;
use hkask_mcp::runtime::McpRuntime;
use hkask_templates::SqliteRegistry;
use hkask_types::WebID;
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
/// The server uses the same `SessionManager` and `InferencePortAdapter`
/// singletons as the CLI, so sessions and inference are unified.
pub async fn run_server(port: u16, host: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Get CLI singletons (these are the same instances used by CLI commands)
    let session_manager = crate::commands::ensemble::get_session_manager();
    let improv_client = crate::commands::ensemble::get_improv_client(None);

    // Extract the base InferencePortAdapter from the circuit-breaker-wrapped client
    let base_adapter = Arc::new(improv_client.inner().clone());

    // Open registry (in-memory by default for now)
    let registry = SqliteRegistry::new(None)?;

    // Resolve capability secret
    let capability_secret = resolve_capability_secret();

    // Resolve system WebID
    let system_webid = WebID::from_persona(b"system");

    // Create MCP runtime and start builtin servers
    let mcp_runtime = McpRuntime::new();
    let server_count = start_api_servers(&mcp_runtime).await;
    if server_count > 0 {
        tracing::info!(target: "hkask.serve", servers = server_count, "MCP servers started");
    } else {
        tracing::warn!(target: "hkask.serve", "No MCP servers started — tool endpoints will return empty results");
    }

    // Build ApiState sharing CLI's SessionManager
    let state = ApiState::new(
        registry,
        mcp_runtime,
        hkask_agents::PodManager::default(),
        &capability_secret,
        system_webid,
        Some(base_adapter),
        None,
        None,
    )
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
    .with_session_manager(session_manager);

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

/// Resolve the capability secret for OCAP signing.
///
/// Resolution chain:
/// 1. Keystore `resolve_capability_key()` (master key derivation → env → keychain)
/// 2. Legacy `HKASK_CAPABILITY_SECRET` env var (backward compat)
/// 3. Dev fallback (with warning — not for production)
fn resolve_capability_secret() -> Vec<u8> {
    // Tier 1: Canonical keystore chain (derived → HKASK_CAPABILITY_KEY env → keychain)
    if let Ok(key) = hkask_keystore::resolve_capability_key() {
        return (*key).clone();
    }

    // Tier 2: Legacy HKASK_CAPABILITY_SECRET env var
    if let Ok(s) = std::env::var("HKASK_CAPABILITY_SECRET") {
        tracing::warn!(
            target: "hkask.serve",
            "HKASK_CAPABILITY_SECRET is deprecated — use HKASK_CAPABILITY_KEY or set HKASK_MASTER_KEY for derivation"
        );
        return s.as_bytes().to_vec();
    }

    // Tier 3: Dev fallback (last resort)
    tracing::warn!(
        target: "hkask.serve",
        "No capability key available via keystore or env — using dev secret (not for production)"
    );
    b"hkask-dev-capability-secret".to_vec()
}
