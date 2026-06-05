//! Agent definition loading — ACP secret resolution and YAML/database registry discovery
//!
//! This module contains functions for:
//! - Resolving the ACP secret through the full derivation chain (master key → env → keychain → dev)
//! - Loading agent definitions from the SQLite registry database
//! - Falling back to YAML file discovery when the database entry is not found
//!
//! These are standalone functions used during [`ReplicantServer`](super::tools::ReplicantServer)
//! initialization to set up ACP runtime and persona configuration.

use hkask_agents::adapters::FilesystemRegistrySource;
use hkask_agents::ports::RegistrySourcePort;
use hkask_storage::Database;
use hkask_types::{AgentDefinition, AgentKind, Charter, PersonaConstraints, SecretRef};

// ── ACP Secret Resolution ────────────────────────────────────────────────────
// Follow-up #1: Full ACP secret resolution chain matching the CLI's
// `resolve_acp_secret()`. This ensures the MCP server's ACP runtime uses
// the same secret as `kask chat`, so capability tokens are compatible.

/// Resolve the ACP secret through the full derivation chain:
/// 1. Master key derivation (HKDF-SHA256)
/// 2. Direct environment variable (`HKASK_ACP_SECRET`)
/// 3. OS keychain
/// 4. Insecure dev mode (random secret) or deterministic default
pub fn resolve_acp_secret() -> String {
    // 1. Master key derivation (HKDF-SHA256)
    hkask_keystore::resolve(&SecretRef::derived(
        hkask_types::derivation_contexts::MASTER_KEY_ENV,
        hkask_types::derivation_contexts::ACP_SECRET,
    ))
    .map(|s| String::from_utf8_lossy(&s).to_string())
    // 2. Direct environment variable
    .or_else(|_| std::env::var("HKASK_ACP_SECRET"))
    // 3. OS keychain
    .or_else(|_| {
        hkask_keystore::Keychain::default()
            .retrieve_by_key("acp-secret")
            .map_err(|e| e.to_string())
    })
    // 4. Insecure dev mode (random secret, tokens won't survive restarts)
    .or_else(|_| {
        if std::env::var("HKASK_INSECURE_DEV").as_deref() == Ok("1") {
            tracing::warn!(
                target: "hkask.mcp.replicant",
                "⚠ INSECURE DEV MODE: Using random ACP secret. Tokens will not survive restarts."
            );
            use std::fmt::Write;
            let mut bytes = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
            let mut s = String::with_capacity(64);
            for b in &bytes {
                write!(s, "{b:02x}").unwrap();
            }
            Ok(s)
        } else {
            // Fall back to a deterministic default so the server can still start.
            // The CLI resolves this through onboarding; MCP servers may be started
            // independently and need a working default.
            tracing::warn!(
                target: "hkask.mcp.replicant",
                "No ACP secret resolved — using deterministic default. \
                 Set HKASK_ACP_SECRET, HKASK_MASTER_KEY, or HKASK_INSECURE_DEV=1 for proper token verification."
            );
            Ok("hkask-default-acp-secret-for-mcp-server".to_string())
        }
    })
    .unwrap_or_else(|_: String| "hkask-default-acp-secret-for-mcp-server".to_string())
}

// ── Agent Definition Loading ─────────────────────────────────────────────────
// Follow-up #2: Load the full agent definition from the YAML registry.
// This provides charter, responsibilities, rights, and voice/tone for
// rich system prompts. Falls back to None if the registry is unavailable.

/// Load an agent definition by persona name.
///
/// Tries the SQLite registry database first, then falls back to YAML file
/// discovery under `registry_path`.
pub fn load_agent_definition(persona: &str) -> Option<AgentDefinition> {
    let registry_path =
        std::env::var("HKASK_REGISTRY_PATH").unwrap_or_else(|_| "registry/bots".to_string());

    let db_path = std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| "hkask.db".to_string());

    // Try to open the registry database. If it doesn't exist or we can't
    // read it, we fall back to the minimal persona definition.
    let passphrase = std::env::var("HKASK_DB_PASSPHRASE")
        .or_else(|_| {
            hkask_keystore::Keychain::default()
                .retrieve_by_key("hkask-db-passphrase")
                .map_err(|e| e.to_string())
        })
        .or_else(|_: String| {
            // In insecure dev mode, use a placeholder passphrase
            if std::env::var("HKASK_INSECURE_DEV").as_deref() == Ok("1") {
                Ok::<String, String>("insecure-dev-passphrase".to_string())
            } else {
                // Try empty passphrase for unencrypted databases
                Ok::<String, String>(String::new())
            }
        })
        .unwrap_or_default();

    let db = match Database::open(&db_path, &passphrase) {
        Ok(db) => db,
        Err(e) => {
            tracing::debug!(
                target: "hkask.mcp.replicant",
                error = %e,
                "Registry database not available, using minimal persona for '{}'",
                persona
            );
            return None;
        }
    };

    let store = hkask_storage::AgentRegistryStore::new(db.conn_arc());
    if let Err(e) = store.initialize_schema() {
        tracing::debug!(
            target: "hkask.mcp.replicant",
            error = %e,
            "Schema init failed, using minimal persona for '{}'",
            persona
        );
        return None;
    }

    match store.get(persona) {
        Ok(agent) => {
            tracing::info!(
                target: "hkask.mcp.replicant",
                persona = %persona,
                "Loaded full agent definition from registry"
            );
            Some(agent.definition)
        }
        Err(_) => {
            // Not found in the database — try loading from YAML files
            // via the registry loader as a secondary path.
            tracing::debug!(
                target: "hkask.mcp.replicant",
                persona = %persona,
                "Agent '{}' not found in database, attempting YAML discovery",
                persona
            );
            load_definition_from_yaml(persona, &registry_path)
        }
    }
}

/// Load an agent definition from a YAML file on disk.
///
/// Looks for `{registry_path}/{persona}.yaml` and `{registry_path}/{persona}.yml`.
pub fn load_definition_from_yaml(persona: &str, registry_path: &str) -> Option<AgentDefinition> {
    // The agent name is used as filename: registry/bots/{name}.yaml
    let yaml_path = format!("{}/{}.yaml", registry_path, persona.to_lowercase());
    let yaml_path_alt = format!("{}/{}.yml", registry_path, persona.to_lowercase());

    let source = FilesystemRegistrySource::new();
    let content = source
        .load_yaml(&yaml_path)
        .or_else(|_| source.load_yaml(&yaml_path_alt))
        .ok()?;

    // Parse the raw YAML to extract the agent definition
    let raw: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    let agent_section = raw.get("agent")?;

    let name = agent_section.get("name")?.as_str()?.to_string();
    let agent_type = agent_section
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("Replicant");
    let agent_kind = match agent_type {
        "Replicant" | "replicant" => AgentKind::Replicant,
        _ => AgentKind::Bot,
    };

    let mut def = AgentDefinition {
        name,
        agent_kind,
        charter: None,
        capabilities: vec![],
        rights: vec![],
        responsibilities: vec![],
        persona: None,
        depends_on: vec![],
        process_manifest: None,
    };

    // Charter
    if let Some(charter) = raw
        .get("charter")
        .and_then(|c| c.get("description"))
        .and_then(|d| d.as_str())
    {
        def.charter = Some(Charter {
            description: charter.to_string(),
            archetype: raw
                .get("charter")
                .and_then(|c| c.get("archetype"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            visibility: raw
                .get("charter")
                .and_then(|c| c.get("visibility"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }

    // Capabilities
    if let Some(caps) = raw.get("capabilities").and_then(|c| c.as_sequence()) {
        def.capabilities = caps
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }

    // Persona (tone, verbosity, forbidden, required)
    if let Some(persona_section) = raw.get("persona") {
        def.persona = Some(PersonaConstraints {
            tone: persona_section
                .get("tone")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            verbosity: persona_section
                .get("verbosity")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            formatting: persona_section
                .get("formatting")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            forbidden: persona_section
                .get("forbidden")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            required: persona_section
                .get("required")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        });
    }

    tracing::info!(
        target: "hkask.mcp.replicant",
        persona = %persona,
        "Loaded agent definition from YAML file"
    );
    Some(def)
}
