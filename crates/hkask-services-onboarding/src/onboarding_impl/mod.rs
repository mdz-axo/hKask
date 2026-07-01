//! Onboarding — secret derivation, keychain, registry init, sign-in.
//! # REQ: P1 (User Sovereignty) — keychain secrets, passphrase-derived keys.
//! # expect: "My service operations flow through sovereignty-verifying boundaries"

use std::sync::Arc;

use hkask_agents::A2ARuntime;
use hkask_keystore::{Keychain, master_key::derive_all_internal_secrets};
use hkask_storage::{
    AgentDefinition, AgentRegistryStore, Charter, Database, RegisteredAgent, UserProfile,
    now_rfc3339,
};
use hkask_types::agent_paths;
use hkask_types::{AgentKind, WebID};

use hkask_services_core::ServiceConfig;
use hkask_services_core::ServiceError;

mod matrix;
pub use matrix::MatrixRegistrationResult;

pub use matrix::conduit_ensure_healthy;
pub use matrix::conduit_health_check;

/// Optional contact and voice configuration for a new replicant.
/// Grouped to keep `register_replicant` argument count manageable.
#[derive(Default)]
pub struct ReplicantContactConfig {
    pub phone_number: Option<String>,
    pub whatsapp_id: Option<String>,
    pub voice_description: Option<String>,
    pub voice_id: Option<String>,
}

/// Pre-resolved secrets from onboarding, passed explicitly instead of
/// mutating environment variables.
#[derive(Debug, Clone)]
pub struct ResolvedSecrets {
    pub master_key_hex: String,
    pub a2a_secret: String,
    pub db_passphrase: String,
    pub mcp_secret: String,
    pub mcp_security_key: String,
}

/// Outcome of a successful sign-in attempt.
#[derive(Debug)]
pub struct SignInOutcome {
    /// The agent name that was verified.
    pub agent_name: String,
    /// Resolved secrets derived from the sign-in passphrase.
    pub resolved_secrets: ResolvedSecrets,
}

/// Result of registry initialization: the A2A runtime and agent store
/// are both ready for use.
pub struct RegistryHandle {
    pub a2a: Arc<A2ARuntime>,
    pub store: AgentRegistryStore,
}

/// Onboarding domain service — secret derivation, keychain management,
/// registry bootstrap, replicant registration, sign-in verification,
/// and failure-path cleanup.
pub struct OnboardingService;

impl OnboardingService {
    /// Derive all internal secrets from a master passphrase.
    ///
    /// If `store` is true, stores secrets in the OS keychain for future sessions.
    /// Returns `ResolvedSecrets` carrying the A2A secret and DB passphrase.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  passphrase must be non-empty; store=true requires writable keychain
    /// post: returns ResolvedSecrets with a2a_secret and db_passphrase; if store=true, secrets are persisted to keychain; Err(Keystore) on keychain failure
    #[must_use = "result must be used"]
    pub fn derive_secrets(passphrase: &str, store: bool) -> Result<ResolvedSecrets, ServiceError> {
        let secrets = derive_all_internal_secrets(passphrase);
        if store {
            let keychain = Keychain::default();
            keychain
                .store_by_key(
                    hkask_types::keychain_keys::KEY_MASTER_KEY,
                    &secrets.master_key_hex,
                )
                .map_err(|e| ServiceError::Keystore {
                    source: Some(Box::new(e)),
                    message: "Failed to store HKASK_MASTER_KEY".into(),
                })?;
            keychain
                .store_by_key(
                    hkask_types::keychain_keys::KEY_A2A_SECRET,
                    &secrets.a2a_secret,
                )
                .map_err(|e| ServiceError::Keystore {
                    source: Some(Box::new(e)),
                    message: "Failed to store a2a-secret".into(),
                })?;
            keychain
                .store_by_key(
                    hkask_types::keychain_keys::KEY_DB_PASSPHRASE,
                    &secrets.capability_key,
                )
                .map_err(|e| ServiceError::Keystore {
                    source: Some(Box::new(e)),
                    message: "Failed to store hkask-db-passphrase".into(),
                })?;
            keychain
                .store_by_key(
                    hkask_types::keychain_keys::KEY_MCP_SECRET,
                    &secrets.mcp_secret,
                )
                .map_err(|e| ServiceError::Keystore {
                    source: Some(Box::new(e)),
                    message: "Failed to store mcp-secret".into(),
                })?;
            keychain
                .store_by_key(
                    hkask_types::keychain_keys::KEY_MCP_SECURITY_KEY,
                    &secrets.mcp_security_key,
                )
                .map_err(|e| ServiceError::Keystore {
                    source: Some(Box::new(e)),
                    message: "Failed to store mcp-security-key".into(),
                })?;
        }
        // P9: CNS span
        tracing::info!(
            target: "cns.onboarding",
            operation = "secrets_derived",
            stored = store,
            "CNS"
        );
        Ok(ResolvedSecrets {
            master_key_hex: secrets.master_key_hex.clone(),
            a2a_secret: secrets.a2a_secret.clone(),
            db_passphrase: secrets.capability_key.clone(),
            mcp_secret: secrets.mcp_secret.clone(),
            mcp_security_key: secrets.mcp_security_key.clone(),
        })
    }

    /// Initialize the A2A runtime and agent registry store from a ServiceConfig.
    ///
    /// Opens the database, initializes the schema, restores A2A state from
    /// persisted agent registrations, and returns both the A2A runtime and
    /// the registry store ready for use.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  config must have valid db_path, db_passphrase, and a2a_secret
    /// post: returns RegistryHandle with A2A runtime and initialized AgentRegistryStore; registered agents restored into ACP; Err on DB open or schema init failure
    #[must_use = "result must be used"]
    pub async fn init_registry(config: &ServiceConfig) -> Result<RegistryHandle, ServiceError> {
        let a2a = Arc::new(A2ARuntime::new(&config.a2a_secret));

        let db = Database::open(&config.db_path, &config.db_passphrase).map_err(|e| {
            ServiceError::Storage {
                source: None,
                message: e.to_string(),
            }
        })?;
        let store = AgentRegistryStore::new(db.conn_arc());
        store
            .initialize_schema()
            .map_err(|e| ServiceError::AgentRegistryStore {
                source: None,
                message: e.to_string(),
            })?;

        // A2A state restoration: reload registered agents from the store
        let registered_agents = store.list().map_err(|e| ServiceError::AgentRegistryStore {
            source: None,
            message: e.to_string(),
        })?;
        if !registered_agents.is_empty() {
            let agents: Vec<hkask_agents::a2a::A2AAgent> = registered_agents
                .iter()
                .map(|ra| hkask_agents::a2a::A2AAgent {
                    webid: WebID::from_persona_with_namespace(
                        ra.definition.name.as_bytes(),
                        "replicant",
                    ),
                    agent_type: ra.definition.agent_kind,
                    capabilities: ra.definition.capabilities.clone(),
                    registered_at: chrono::DateTime::parse_from_rfc3339(&ra.registered_at)
                        .map(|dt| dt.timestamp())
                        .unwrap_or(0),
                    active: true,
                })
                .collect();
            let tokens = std::collections::HashMap::new();
            a2a.restore_from_storage(agents, tokens)
                .await
                .map_err(|e| ServiceError::A2A {
                    source: None,
                    message: e.to_string(),
                })?;
        }

        // P9: CNS span
        tracing::info!(
            target: "cns.onboarding",
            operation = "registry_initialized",
            agent_count = registered_agents.len(),
            "CNS"
        );

        Ok(RegistryHandle { a2a, store })
    }

    /// Register a new replicant in A2A and the agent registry store.
    ///
    /// Creates a WebID, registers with A2A (granting default replicant
    /// capabilities), builds an `AgentDefinition` and `RegisteredAgent`,
    /// and persists them.
    ///
    /// If `user_profile` is provided, the replicant's display name follows
    /// the naming protocol: "{chosen_name} r{human_last_name}".
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  a2a must be initialized; store must be initialized; name and description must be non-empty
    /// post: replicant is registered in A2A with default capabilities and persisted to store; Err(A2A) on registration failure; Err(AgentRegistryStore) on persistence failure
    #[must_use = "result must be used"]
    pub async fn register_replicant(
        a2a: &Arc<A2ARuntime>,
        store: &AgentRegistryStore,
        name: &str,
        description: &str,
        user_profile: Option<&UserProfile>,
        _voice_description: Option<&str>,
        _voice_id: Option<&str>,
    ) -> Result<(), ServiceError> {
        let display_name = if let Some(profile) = user_profile {
            profile.replicant_display_name(name)
        } else {
            name.to_string()
        };

        // ── Idempotent retry guard ──────────────────────────────────────
        // If init_registry() restored this replicant from a prior DB
        // (e.g., leftover from a previous install where the keychain was
        // cleared but data/hkask.db and its salt survived), the agent is
        // already in both the store and A2A.  Re-registering the same
        // deterministic WebID would hit AgentAlreadyRegistered.
        if store.get(&display_name).is_ok() {
            tracing::info!(
                target: "cns.onboarding",
                operation = "replicant_already_registered",
                name = %display_name,
                "Replicant already in store — skipping A2A registration (idempotent retry)",
            );
            return Ok(());
        }

        let webid = WebID::from_persona_with_namespace(display_name.as_bytes(), "replicant");

        let default_capabilities = vec![
            "tool:inference:call".to_string(),
            "tool:mcp:invoke".to_string(),
            "registry:episodic_memory:read".to_string(),
            "registry:episodic_memory:write".to_string(),
        ];

        let token = a2a
            .register_agent(webid, AgentKind::Replicant, default_capabilities.clone())
            .await
            .map_err(|e| ServiceError::A2A {
                source: None,
                message: e.to_string(),
            })?;

        // Build the self-contained agent definition YAML.
        // Written to agents/{name}/agent.yaml for Curator discovery and
        // stored as source_yaml so the REPL can load persona + process_manifest.
        let source_yaml = format!(
            "# Agent definition for {name} — created by hKask onboarding.\n\
             agent:\n  name: \"{name}\"\n  type: replicant\n\n\
             charter:\n  description: \"{desc}\"\n\n\
             capabilities:\n  - tool:inference:call\n  - tool:mcp:invoke\n  - registry:episodic_memory:read\n  - registry:episodic_memory:write\n\n\
             # Directories containing public artifacts (synced to Curator).\n\n\
             public_dirs:\n  - artifacts\n  - library\n  - gallery\n  - documents\n  - adapters\n\n\
             # Directories containing private data (never leaves agent folder).\n\n\
             private_dirs:\n  - sessions\n  - portfolios\n",
            name = display_name,
            desc = description,
        );

        // Persist the agent YAML to the agent's directory on disk.
        let yaml_path = agent_paths::agent_definition_yaml(&display_name);
        if let Some(parent) = yaml_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&yaml_path, &source_yaml).map_err(|e| ServiceError::Storage {
            source: None,
            message: format!("Failed to write agent YAML to {}: {e}", yaml_path.display()),
        })?;

        let registered = RegisteredAgent {
            definition: AgentDefinition {
                name: display_name,
                agent_kind: AgentKind::Replicant,
                charter: Some(Charter {
                    description: description.to_string(),
                    archetype: String::new(),
                    visibility: String::new(),
                }),
                capabilities: default_capabilities,
                rights: vec![],
                responsibilities: vec![],
                depends_on: vec![],
                persona: None,
                process_manifest: None,
                voice_description: None,
                voice_id: None,
            },
            token_hash: hex::encode(token.signature_bytes()),
            registered_at: now_rfc3339(),
            source_yaml,
        };

        store
            .insert(&registered)
            .map_err(|e| ServiceError::AgentRegistryStore {
                source: None,
                message: e.to_string(),
            })?;

        // P9: CNS span
        tracing::info!(
            target: "cns.onboarding",
            operation = "replicant_registered",
            name = %registered.definition.name,
            webid = %webid,
            "CNS"
        );

        Ok(())
    }

    /// Store the human user's profile in the registry.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  store must be initialized; profile must be a valid UserProfile
    /// post: profile is persisted to the registry store; Err(AgentRegistryStore) on store failure
    #[must_use = "result must be used"]
    pub fn store_user_profile(
        store: &AgentRegistryStore,
        profile: &UserProfile,
    ) -> Result<(), ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.onboarding", operation = "store_user_profile", "CNS");
        store
            .store_user_profile(profile)
            .map_err(|e| ServiceError::AgentRegistryStore {
                source: None,
                message: e.to_string(),
            })
    }

    /// Retrieve the human user's profile from the registry.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  store must be initialized
    /// post: returns Some(UserProfile) if stored; None if no profile; Err(AgentRegistryStore) on store failure
    #[must_use = "result must be used"]
    pub fn get_user_profile(
        store: &AgentRegistryStore,
    ) -> Result<Option<UserProfile>, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.onboarding", operation = "get_user_profile", "CNS");
        store
            .get_user_profile()
            .map_err(|e| ServiceError::AgentRegistryStore {
                source: None,
                message: e.to_string(),
            })
    }

    /// Verify sign-in: initialize the registry with the given config and
    /// confirm the named replicant exists in the store.
    ///
    /// On success, stores the secrets in the keychain for future sessions
    /// and returns a `SignInOutcome`.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  config must be valid; agent_name must match a registered replicant; resolved_secrets must be valid
    /// post: returns SignInOutcome on success; secrets stored in keychain; Err(AgentNotFound) if replicant missing; Err on registry init failure
    #[must_use = "result must be used"]
    pub async fn try_sign_in(
        config: &ServiceConfig,
        agent_name: &str,
        resolved_secrets: &ResolvedSecrets,
    ) -> Result<SignInOutcome, ServiceError> {
        // P9: CNS span
        tracing::info!(target: "cns.onboarding", operation = "try_sign_in", agent = %agent_name, "CNS");
        let handle = Self::init_registry(config).await?;

        // Verify the replicant exists
        handle
            .store
            .get(agent_name)
            .map_err(|_| ServiceError::AgentNotFound {
                source: None,
                message: agent_name.to_string(),
            })?;

        // Success — store secrets in keychain for future sessions
        let keychain = Keychain::default();
        keychain
            .store_by_key(
                hkask_types::keychain_keys::KEY_A2A_SECRET,
                &resolved_secrets.a2a_secret,
            )
            .map_err(|e| ServiceError::Keystore {
                source: Some(Box::new(e)),
                message: "Failed to store a2a-secret".into(),
            })?;
        keychain
            .store_by_key(
                hkask_types::keychain_keys::KEY_DB_PASSPHRASE,
                &resolved_secrets.db_passphrase,
            )
            .map_err(|e| ServiceError::Keystore {
                source: Some(Box::new(e)),
                message: "Failed to store hkask-db-passphrase".into(),
            })?;

        Ok(SignInOutcome {
            agent_name: agent_name.to_string(),
            resolved_secrets: resolved_secrets.clone(),
        })
    }

    /// Try to list existing replicants from the database without requiring
    /// an A2A runtime. Used to determine which onboarding path to take.
    ///
    /// Returns an empty Vec if the DB can't be opened or has no replicants.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  config.db_path must be set; returns empty Vec on any failure
    /// post: returns `Vec<RegisteredAgent>` of replicants; empty Vec if DB inaccessible or no replicants
    #[must_use]
    pub fn try_list_existing_replicants(config: &ServiceConfig) -> Vec<RegisteredAgent> {
        // P9: CNS span
        tracing::info!(target: "cns.onboarding", operation = "try_list_existing_replicants", "CNS");
        let db_path = &config.db_path;

        if db_path == ":memory:" || !std::path::Path::new(db_path).exists() {
            return Vec::new();
        }

        let db = match Database::open(db_path, &config.db_passphrase) {
            Ok(db) => db,
            Err(_) => {
                // Can't open with passphrase — try without encryption
                match Database::open(db_path, "") {
                    Ok(db) => db,
                    Err(_) => return Vec::new(),
                }
            }
        };

        let store = AgentRegistryStore::new(db.conn_arc());
        if store.initialize_schema().is_err() {
            return Vec::new();
        }

        store.list_by_kind(AgentKind::Replicant).unwrap_or_default()
    }

    /// Check for an orphaned DB from a previous failed onboarding attempt.
    ///
    /// If the DB exists but has no replicants (or can't be opened with the
    /// current passphrase), it's orphaned and should be removed before
    /// starting a fresh onboarding. Returns `true` if cleanup was performed.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// Check whether an orphaned database exists from a previous failed onboarding.
    /// Returns true if the DB file exists and contains no replicants (i.e., is orphaned).
    /// Does NOT remove the database — caller should confirm before calling remove_orphaned_db.
    #[must_use]
    pub fn has_orphaned_db(config: &ServiceConfig) -> bool {
        let db_path = &config.db_path;
        if db_path == ":memory:" || !std::path::Path::new(db_path).exists() {
            return false;
        }
        if config.db_passphrase.is_empty() {
            return false;
        }
        match Database::open(db_path, &config.db_passphrase) {
            Ok(db) => {
                let store = AgentRegistryStore::new(db.conn_arc());
                if store.initialize_schema().is_ok() {
                    matches!(store.list_by_kind(AgentKind::Replicant), Ok(r) if r.is_empty())
                } else {
                    true // Can't read schema — likely orphaned
                }
            }
            Err(_) => true, // Can't open — likely orphaned/corrupted
        }
    }

    /// pre:  config.db_path must be set; :memory: paths are never orphaned
    /// post: returns true if orphaned DB was cleaned up; false if DB has replicants or doesn't exist
    #[must_use]
    pub fn remove_orphaned_db(config: &ServiceConfig) -> bool {
        // P9: CNS span
        tracing::info!(target: "cns.onboarding", operation = "remove_orphaned_db", "CNS");
        let db_path = &config.db_path;
        if db_path == ":memory:" {
            return false;
        }

        if !std::path::Path::new(db_path).exists() {
            return false;
        }

        // Try to open the DB and check for replicants
        let has_replicants = if !config.db_passphrase.is_empty() {
            match Database::open(db_path, &config.db_passphrase) {
                Ok(db) => {
                    let store = AgentRegistryStore::new(db.conn_arc());
                    if store.initialize_schema().is_ok() {
                        matches!(store.list_by_kind(AgentKind::Replicant), Ok(r) if !r.is_empty())
                    } else {
                        false
                    }
                }
                Err(_) => false,
            }
        } else {
            false
        };

        if has_replicants {
            return false;
        }

        // Orphaned — remove DB, salt file, and stale keychain entries
        Self::cleanup_failed_onboarding(config);
        true
    }

    /// Roll back a failed onboarding by removing keychain entries, the
    /// database file, and the salt file.
    ///
    /// Called when onboarding fails after partial setup (e.g., keychain
    /// stored but registration failed). Prevents orphaned state from
    /// poisoning subsequent attempts.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  config must be valid; best-effort cleanup (errors are silently ignored)
    /// post: keychain entries (a2a-secret, hkask-db-passphrase) are removed; DB and salt files deleted if not :memory:
    pub fn cleanup_failed_onboarding(config: &ServiceConfig) {
        // P9: CNS span
        tracing::info!(target: "cns.onboarding", operation = "cleanup_failed_onboarding", "CNS");
        let keychain = Keychain::default();
        let _ = keychain.delete_by_key(hkask_types::keychain_keys::KEY_A2A_SECRET);
        let _ = keychain.delete_by_key(hkask_types::keychain_keys::KEY_DB_PASSPHRASE);

        let db_path = &config.db_path;
        if db_path != ":memory:" {
            let db_file = std::path::Path::new(db_path);
            if db_file.exists() {
                let _ = std::fs::remove_file(db_file);
            }
            let salt_file = std::path::PathBuf::from(format!("{}.salt", db_path));
            if salt_file.exists() {
                let _ = std::fs::remove_file(salt_file);
            }
        }
    }
}
