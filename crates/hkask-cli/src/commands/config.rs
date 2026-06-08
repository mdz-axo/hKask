//! Registry config and database helper functions

use crate::errors::RegistryError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

pub fn registry_db_path() -> String {
    std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| hkask_services::DEFAULT_DB_PATH.to_string())
}

pub(crate) fn registry_yaml_path() -> PathBuf {
    let p = std::env::var("HKASK_REGISTRY_PATH").unwrap_or_else(|_| "registry/bots".to_string());
    PathBuf::from(p)
}

pub(crate) fn resolve_acp_secret() -> Result<String, RegistryError> {
    hkask_keystore::resolve_acp_secret()
        .map(|s| String::from_utf8_lossy(&s).to_string())
        .map_err(|e| RegistryError::InitFailed(e.to_string()))
}

/// Resolve the MCP secret for tool dispatch signing.
///
/// Delegates to the keystore's domain-specific resolution chain
/// (master key derivation → env var → keychain → ACP secret fallback).
pub fn resolve_mcp_secret() -> Result<String, RegistryError> {
    hkask_keystore::resolve_mcp_secret()
        .map(|s| String::from_utf8_lossy(&s).to_string())
        .map_err(|e| RegistryError::InitFailed(e.to_string()))
}

pub fn resolve_db_passphrase() -> Result<String, RegistryError> {
    hkask_keystore::resolve_db_passphrase()
        .map(|s| String::from_utf8_lossy(&s).to_string())
        .map_err(|e| RegistryError::InitFailed(e.to_string()))
}

/// Open a SovereigntyBoundaryStore (used by `kask sovereignty` subcommands).
/// Opens the shared database and wraps it in a sovereignty store.
pub fn open_sovereignty_store()
-> Result<hkask_storage::SovereigntyBoundaryStore, crate::errors::RegistryError> {
    let db_path = registry_db_path();
    let passphrase = resolve_db_passphrase()?;
    let db = hkask_storage::open_database(&db_path, &passphrase)
        .map_err(|e| crate::errors::RegistryError::DatabaseError(e.to_string()))?;
    Ok(hkask_storage::SovereigntyBoundaryStore::new(db.conn_arc()))
}

/// Open a SqliteSpecStore (used by `kask spec` subcommands).
/// Opens the shared database and initializes the spec schema.
pub fn open_spec_store() -> Result<hkask_storage::SqliteSpecStore, crate::errors::RegistryError> {
    let db_path = registry_db_path();
    let passphrase = resolve_db_passphrase()?;
    let db = hkask_storage::open_database(&db_path, &passphrase)
        .map_err(|e| crate::errors::RegistryError::DatabaseError(e.to_string()))?;
    let store = hkask_storage::SqliteSpecStore::new(db.conn_arc());
    store
        .init_schema()
        .map_err(|e| crate::errors::RegistryError::SchemaError(e.to_string()))?;
    Ok(store)
}

/// Create a governed MCP dispatcher with a *disconnected* CyberneticsLoop.
///
/// This creates a standalone CyberneticsLoop that is not wired into any LoopSystem
/// or REPL session. For REPL-connected tool dispatch, the GovernedTool is created
/// in `repl::run()` using the session's shared CyberneticsLoop.
///
/// Returns `(McpDispatcher, Arc<GovernedTool<RawMcpToolPort>>)` — the dispatcher and
/// the governed tool membrane. The membrane is returned so callers can issue
/// capability tokens if needed.
pub fn create_disconnected_governed_dispatcher(
    runtime: hkask_mcp::runtime::McpRuntime,
    secret: &[u8],
) -> (
    hkask_mcp::McpDispatcher,
    std::sync::Arc<hkask_cns::GovernedTool<hkask_mcp::raw_tool_port::RawMcpToolPort>>,
) {
    use hkask_cns::{CnsRuntime, CompositeGasEstimator, CyberneticsLoop, GovernedTool};
    use hkask_mcp::raw_tool_port::RawMcpToolPort;
    use hkask_types::event::NuEventSink;
    use std::sync::Arc;

    let cns_rwlock: Arc<tokio::sync::RwLock<CnsRuntime>> =
        Arc::new(tokio::sync::RwLock::new(CnsRuntime::default()));
    let (dispatch_tx, _) =
        tokio::sync::mpsc::unbounded_channel::<hkask_types::loops::LoopMessage>();
    let cybernetics = Arc::new(tokio::sync::RwLock::new({
        let event_sink_for_loop: Arc<dyn NuEventSink> = Arc::new(hkask_storage::NuEventStore::new(
            hkask_storage::in_memory_db().conn_arc(),
        ));
        CyberneticsLoop::new(cns_rwlock, dispatch_tx.clone()).with_event_sink(event_sink_for_loop)
    }));

    let raw_port = Arc::new(RawMcpToolPort::new(runtime.clone()));
    let event_sink: Arc<dyn NuEventSink> = Arc::new(hkask_storage::NuEventStore::new(
        hkask_storage::in_memory_db().conn_arc(),
    ));
    let estimator = Arc::new(CompositeGasEstimator::new());
    let agent = hkask_types::WebID::from_persona(b"curator");

    let governed = Arc::new(GovernedTool::new(
        raw_port,
        cybernetics,
        event_sink,
        estimator,
        agent,
        dispatch_tx,
    ));

    let dispatcher =
        hkask_mcp::McpDispatcher::with_governed_tool(runtime, secret, governed.clone());
    (dispatcher, governed)
}

/// Create an MCP dispatcher wired with GovernedTool and a capability token.
/// Returns (McpDispatcher, token) for invoking tools.
///
/// **Note:** This creates a dispatcher with NO live MCP servers — the
/// underlying `McpRuntime` is empty. All tool invocations will fail because
/// no servers have been started via `McpRuntime::start_server()`. Use
/// `create_mcp_dispatcher_with_servers()` for dispatchers that can actually
/// invoke tools, or start servers manually on the runtime before creating
/// the dispatcher.
///
/// This function also creates a *disconnected* CyberneticsLoop — it is not
/// wired into any LoopSystem or REPL session. For REPL-connected tool
/// dispatch, the GovernedTool is created in `repl::run()` using the
/// session's shared CyberneticsLoop and dispatch channel.
pub fn create_mcp_dispatcher()
-> Result<(hkask_mcp::McpDispatcher, hkask_types::DelegationToken), RegistryError> {
    let runtime = hkask_mcp::runtime::McpRuntime::new();
    let mcp_secret = resolve_mcp_secret()?;
    let (dispatcher, _) = create_disconnected_governed_dispatcher(runtime, mcp_secret.as_bytes());
    let from = hkask_types::WebID::new();
    let to = hkask_types::WebID::new();
    let token = dispatcher.issue_capability("tools".to_string(), from, to);
    Ok((dispatcher, token))
}

/// Create an MCP dispatcher wired with GovernedTool, a capability token,
/// and the specified MCP servers started as child processes.
///
/// Each `(server_id, command)` pair is passed to `McpRuntime::start_server()`
/// which spawns the server binary, discovers tools, and registers them.
/// If any server fails to start, logs a warning and continues.
///
/// Returns `(McpDispatcher, token)` for invoking tools.
pub fn create_mcp_dispatcher_with_servers(
    rt: &tokio::runtime::Runtime,
    servers: &[(&str, &str)],
) -> Result<(hkask_mcp::McpDispatcher, hkask_types::DelegationToken), RegistryError> {
    let runtime = hkask_mcp::runtime::McpRuntime::new();

    // Start each MCP server as a child process
    for (server_id, command) in servers {
        match rt.block_on(runtime.start_server(server_id, command)) {
            Ok(()) => {
                tracing::info!(
                    target: "hkask.cli",
                    server_id = %server_id,
                    "MCP server started"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target: "hkask.cli",
                    server_id = %server_id,
                    error = %e,
                    "Failed to start MCP server"
                );
            }
        }
    }

    let mcp_secret = resolve_mcp_secret()?;
    let (dispatcher, _) = create_disconnected_governed_dispatcher(runtime, mcp_secret.as_bytes());
    let from = hkask_types::WebID::new();
    let to = hkask_types::WebID::new();
    let token = dispatcher.issue_capability("tools".to_string(), from, to);
    Ok((dispatcher, token))
}

/// Pre-resolved secrets for onboarding, passed explicitly instead of
/// mutating environment variables.
pub struct ResolvedSecrets {
    pub acp_secret: String,
    pub db_passphrase: String,
}

/// Initialize the registry by resolving secrets from env/keychain/derivation.
pub(crate) async fn init_registry() -> Result<
    (
        Arc<hkask_agents::AcpRuntime>,
        hkask_storage::AgentRegistryStore,
    ),
    RegistryError,
> {
    let secrets = ResolvedSecrets {
        acp_secret: resolve_acp_secret()?,
        db_passphrase: resolve_db_passphrase()?,
    };
    init_registry_with_secrets(&secrets).await
}

/// Initialize the registry with pre-resolved secrets (from onboarding).
///
/// Uses the provided secrets directly instead of resolving from
/// environment variables or keychain, avoiding runtime env mutation.
pub async fn init_registry_with_secrets(
    secrets: &ResolvedSecrets,
) -> Result<
    (
        Arc<hkask_agents::AcpRuntime>,
        hkask_storage::AgentRegistryStore,
    ),
    RegistryError,
> {
    let acp = Arc::new(hkask_agents::AcpRuntime::new(secrets.acp_secret.as_bytes()));

    let db_path = registry_db_path();

    let db = hkask_storage::open_database(&db_path, &secrets.db_passphrase)
        .map_err(|e| RegistryError::DatabaseError(e.to_string()))?;

    let store = hkask_storage::AgentRegistryStore::new(db.conn_arc());
    store
        .initialize_schema()
        .map_err(|e| RegistryError::SchemaError(e.to_string()))?;

    // R2: Restore agent state from persistent storage
    let registered_agents = store
        .list()
        .map_err(|e| RegistryError::LoadFailed(e.to_string()))?;

    if !registered_agents.is_empty() {
        // Restore AcpRuntime state from storage
        let agents: Vec<hkask_agents::acp::AcpAgent> = registered_agents
            .iter()
            .map(|ra| hkask_agents::acp::AcpAgent {
                webid: hkask_types::WebID::from_str(&ra.definition.name).unwrap_or_else(|_| {
                    hkask_types::WebID::from_persona(ra.definition.name.as_bytes())
                }),
                agent_type: ra.definition.agent_kind,
                capabilities: ra.definition.capabilities.clone(),
                registered_at: chrono::DateTime::parse_from_rfc3339(&ra.registered_at)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
                active: true,
            })
            .collect();

        // Restore capability tokens (empty for now - R8 will add token persistence)
        let tokens = HashMap::new();

        acp.restore_from_storage(agents, tokens)
            .await
            .map_err(|e| RegistryError::LoadFailed(e.to_string()))?;
    }

    Ok((acp, store))
}
