//! Registry config and database helper functions

use crate::errors::{CuratorError, RegistryError};
use std::path::PathBuf;
use std::sync::Arc;

pub fn registry_db_path() -> String {
    std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| "hkask.db".to_string())
}

pub(crate) fn registry_yaml_path() -> PathBuf {
    let p = std::env::var("HKASK_REGISTRY_PATH").unwrap_or_else(|_| "registry/bots".to_string());
    PathBuf::from(p)
}

pub(crate) fn resolve_acp_secret() -> Result<String, RegistryError> {
    // Resolution chain: master key derivation → env var → keychain → insecure dev
    hkask_keystore::resolve(&hkask_types::SecretRef::derived(
        hkask_types::derivation_contexts::MASTER_KEY_ENV,
        hkask_types::derivation_contexts::ACP_SECRET,
    ))
    .map(|s| String::from_utf8_lossy(&s).to_string())
    .or_else(|_| std::env::var("HKASK_ACP_SECRET"))
    .or_else(|_| {
        hkask_keystore::Keychain::default()
            .retrieve(&hkask_types::WebID::from_persona(b"hkask-acp-secret"))
    })
    .or_else(|_| {
        if std::env::var("HKASK_INSECURE_DEV").as_deref() == Ok("1")
            && crate::commands::admin::verify_admin_for_dev_mode()
        {
            tracing::warn!(
                "⚠ INSECURE DEV MODE: Using random ACP secret. Tokens will not survive restarts."
            );
            use rand::RngCore;
            let mut bytes = [0u8; 32];
            rand::rng().fill_bytes(&mut bytes);
            Ok(hex::encode(bytes))
        } else {
            Err(RegistryError::InitFailed(
                "HKASK_ACP_SECRET not set. Run `kask chat` to complete onboarding, \
                 set HKASK_MASTER_KEY, or use HKASK_INSECURE_DEV=1 with `kask admin unlock`."
                    .to_string(),
            ))
        }
    })
}

pub(crate) fn resolve_db_passphrase() -> Result<String, RegistryError> {
    std::env::var("HKASK_DB_PASSPHRASE").or_else(|_| {
        hkask_keystore::keychain::Keychain::default()
            .retrieve(&hkask_types::WebID::from_persona(b"hkask-db-passphrase"))
            .or_else(|_| {
                if std::env::var("HKASK_INSECURE_DEV").as_deref() == Ok("1")
                    && crate::commands::admin::verify_admin_for_dev_mode()
                {
                    tracing::warn!("⚠ INSECURE DEV MODE: Using random DB passphrase.");
                    use rand::RngCore;
                    let mut bytes = [0u8; 32];
                    rand::rng().fill_bytes(&mut bytes);
                    Ok(hex::encode(bytes))
                } else {
                    Err(RegistryError::InitFailed(
                        "HKASK_DB_PASSPHRASE not set. Run `kask chat` to complete onboarding, \
                         or use HKASK_INSECURE_DEV=1 with `kask admin unlock`."
                            .to_string(),
                    ))
                }
            })
    })
}

pub(crate) fn open_registry_db() -> Result<Arc<std::sync::Mutex<rusqlite::Connection>>, CuratorError>
{
    use hkask_storage::Database;

    let db_path = registry_db_path();
    let passphrase =
        resolve_db_passphrase().map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    let db = if db_path == ":memory:" {
        Database::in_memory()
    } else {
        Database::open(&db_path, &passphrase)
    }
    .map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    Ok(db.conn_arc())
}

// ── Shared Database Initialization Helpers ──────────────────────────────────

/// Open a SovereigntyBoundaryStore (used by `kask sovereignty` subcommands).
/// Opens the shared database and wraps it in a sovereignty store.
pub fn open_sovereignty_store()
-> Result<hkask_storage::SovereigntyBoundaryStore, crate::errors::RegistryError> {
    let db_path = registry_db_path();
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| crate::errors::RegistryError::DatabaseError(e.to_string()))?;
    Ok(hkask_storage::SovereigntyBoundaryStore::new(
        std::sync::Arc::new(std::sync::Mutex::new(conn)),
    ))
}

/// Open a SqliteSpecStore (used by `kask spec` subcommands).
/// Opens the shared database and initializes the spec schema.
pub fn open_spec_store() -> Result<hkask_storage::SqliteSpecStore, crate::errors::RegistryError> {
    let db_path = registry_db_path();
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| crate::errors::RegistryError::DatabaseError(e.to_string()))?;
    let store =
        hkask_storage::SqliteSpecStore::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
    store
        .init_schema()
        .map_err(|e| crate::errors::RegistryError::SchemaError(e.to_string()))?;
    Ok(store)
}

/// Create an MCP dispatcher wired to a fresh runtime with a capability token.
/// Returns (McpDispatcher, token) for invoking tools.
pub fn create_mcp_dispatcher() -> (hkask_mcp::McpDispatcher, hkask_types::CapabilityToken) {
    let runtime = hkask_mcp::runtime::McpRuntime::new();
    let secret = b"hkask-devel-mcp-secret-key-32byte!";
    let dispatcher = hkask_mcp::McpDispatcher::new(runtime, secret);
    let from = hkask_types::WebID::new();
    let to = hkask_types::WebID::new();
    let token = dispatcher.issue_capability("tools".to_string(), from, to);
    (dispatcher, token)
}

// ── Registry Initialization ─────────────────────────────────────────────────

pub(crate) async fn init_registry() -> Result<
    (
        Arc<hkask_agents::AcpRuntime>,
        hkask_storage::AgentRegistryStore,
    ),
    RegistryError,
> {
    let secret = resolve_acp_secret()?;
    let acp = Arc::new(hkask_agents::AcpRuntime::new(secret.as_bytes()));

    let db_path = registry_db_path();
    let passphrase = resolve_db_passphrase()?;

    let db = if db_path == ":memory:" {
        hkask_storage::Database::in_memory()
            .map_err(|e| RegistryError::DatabaseError(e.to_string()))?
    } else {
        hkask_storage::Database::open(&db_path, &passphrase)
            .map_err(|e| RegistryError::DatabaseError(e.to_string()))?
    };

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
                webid: hkask_types::WebID::from_string(&ra.definition.name),
                agent_type: ra.definition.agent_kind.as_str().to_string(),
                capabilities: ra.definition.capabilities.clone(),
                registered_at: chrono::DateTime::parse_from_rfc3339(&ra.registered_at)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
                active: true,
            })
            .collect();

        // Restore capability tokens (empty for now - R8 will add token persistence)
        let tokens = std::collections::HashMap::new();

        acp.restore_from_storage(agents, tokens)
            .await
            .map_err(|e| RegistryError::LoadFailed(e.to_string()))?;
    }

    Ok((acp, store))
}
