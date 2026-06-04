//! `kask serve` — Start the HTTP API server sharing CLI state
//!
//! Creates an `ApiState` wired to the CLI's existing singletons
//! (SessionManager, CyberneticsLoop, InferencePortAdapter) and starts
//! the axum HTTP server. CLI commands issued while the server is running
//! operate on the same shared state.

use hkask_api::ApiState;
use hkask_templates::SqliteRegistry;
use hkask_types::WebID;
use std::sync::Arc;

/// Run the API server, sharing state with the CLI.
///
/// The server uses the same `SessionManager` and `InferencePortAdapter`
/// singletons as the CLI, so sessions and inference are unified.
pub async fn run_server(port: u16, host: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Get CLI singletons (these are the same instances used by CLI commands)
    let session_manager = crate::commands::ensemble::get_session_manager();
    let improv_client = crate::commands::ensemble::get_improv_client();

    // Extract the base InferencePortAdapter from the circuit-breaker-wrapped client
    let base_adapter = Arc::new(improv_client.inner().clone());

    // Open registry (in-memory by default for now)
    let registry = SqliteRegistry::new(None)?;

    // Resolve capability secret
    let capability_secret = resolve_capability_secret();

    // Resolve system WebID
    let system_webid = WebID::from_persona(b"system");

    // Build ApiState sharing CLI's SessionManager
    let state = ApiState::new(
        registry,
        hkask_mcp::runtime::McpRuntime::new(),
        hkask_agents::PodManager::default(),
        &capability_secret,
        system_webid,
        Some(base_adapter),
        None,
        None,
    )
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

/// Resolve the capability secret for OCAP signing.
/// In dev mode, uses a fixed secret. Production should derive from the keystore.
fn resolve_capability_secret() -> Vec<u8> {
    match std::env::var("HKASK_CAPABILITY_SECRET") {
        Ok(s) => s.as_bytes().to_vec(),
        Err(_) => {
            tracing::warn!(
                target: "hkask.serve",
                "HKASK_CAPABILITY_SECRET not set — using dev secret (not for production)"
            );
            b"hkask-dev-capability-secret".to_vec()
        }
    }
}
