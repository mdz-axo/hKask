//! Registry config and database helper functions

use crate::errors::{CuratorError, RegistryError};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn registry_db_path() -> String {
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
        if std::env::var("HKASK_INSECURE_DEV").as_deref() == Ok("1") {
            tracing::warn!("⚠ INSECURE DEV MODE: Using random ACP secret. Tokens will not survive restarts.");
            use rand::RngCore;
            let mut bytes = [0u8; 32];
            rand::rng().fill_bytes(&mut bytes);
            Ok(hex::encode(bytes))
        } else {
            Err(RegistryError::InitFailed(
                "HKASK_ACP_SECRET not set. Set HKASK_MASTER_KEY, HKASK_ACP_SECRET, or use HKASK_INSECURE_DEV=1 for local development.".to_string(),
            ))
        }
    })
}

fn resolve_db_passphrase() -> Result<String, RegistryError> {
    std::env::var("HKASK_DB_PASSPHRASE").or_else(|_| {
        hkask_keystore::keychain::Keychain::default()
            .retrieve(&hkask_types::WebID::from_persona(b"hkask-db-passphrase"))
            .or_else(|_| {
                if std::env::var("HKASK_INSECURE_DEV").as_deref() == Ok("1") {
                    tracing::warn!("⚠ INSECURE DEV MODE: Using random DB passphrase.");
                    use rand::RngCore;
                    let mut bytes = [0u8; 32];
                    rand::rng().fill_bytes(&mut bytes);
                    Ok(hex::encode(bytes))
                } else {
                    Err(RegistryError::InitFailed(
                        "HKASK_DB_PASSPHRASE not set. Set it explicitly or use HKASK_INSECURE_DEV=1 for local development.".to_string(),
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

pub(crate) async fn init_registry() -> Result<
    (
        Arc<hkask_agents::AcpRuntime>,
        hkask_storage::AgentRegistryStore,
    ),
    RegistryError,
> {
    let secret = resolve_acp_secret()?;
    let acp = Arc::new(hkask_agents::AcpRuntime::new(secret.as_bytes(), None));
    let cns_emitter = Arc::new(hkask_agents::adapters::CnsEmitterAdapter::new(
        hkask_types::WebID::new(),
    ));
    use hkask_agents::ports::AcpPort;
    acp.set_cns_emitter(cns_emitter);

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
