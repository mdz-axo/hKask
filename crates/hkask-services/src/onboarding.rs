//! Onboarding — secret derivation, keychain, registry init, sign-in.
//! # REQ: P1 (User Sovereignty) — keychain secrets, passphrase-derived keys.

use std::sync::Arc;

use hkask_agents::AcpRuntime;
use hkask_keystore::{Keychain, derive_all_internal_secrets};
use hkask_storage::{AgentRegistryStore, Database};
use hkask_types::{
    AgentDefinition, AgentKind, Charter, RegisteredAgent, UserProfile, WebID, now_rfc3339,
};

use crate::config::ServiceConfig;
use crate::error::ServiceError;

/// Pre-resolved secrets from onboarding, passed explicitly instead of
/// mutating environment variables.
#[derive(Debug, Clone)]
pub struct ResolvedSecrets {
    pub acp_secret: String,
    pub db_passphrase: String,
}

/// Outcome of a successful sign-in attempt.
#[derive(Debug)]
pub struct SignInOutcome {
    /// The agent name that was verified.
    pub agent_name: String,
    /// Resolved secrets derived from the sign-in passphrase.
    pub resolved_secrets: ResolvedSecrets,
}

/// Result of registry initialization: the ACP runtime and agent store
/// are both ready for use.
pub struct RegistryHandle {
    pub acp: Arc<AcpRuntime>,
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
    /// Returns `ResolvedSecrets` carrying the ACP secret and DB passphrase.
    pub fn derive_secrets(passphrase: &str, store: bool) -> Result<ResolvedSecrets, ServiceError> {
        let secrets = derive_all_internal_secrets(passphrase);
        if store {
            let keychain = Keychain::default();
            keychain
                .store_by_key("acp-secret", &secrets.acp_secret)
                .map_err(|e| ServiceError::Keystore(e.to_string()))?;
            keychain
                .store_by_key("hkask-db-passphrase", &secrets.capability_key)
                .map_err(|e| ServiceError::Keystore(e.to_string()))?;
        }
        Ok(ResolvedSecrets {
            acp_secret: secrets.acp_secret.clone(),
            db_passphrase: secrets.capability_key.clone(),
        })
    }

    /// Initialize the ACP runtime and agent registry store from a ServiceConfig.
    ///
    /// Opens the database, initializes the schema, restores ACP state from
    /// persisted agent registrations, and returns both the ACP runtime and
    /// the registry store ready for use.
    pub async fn init_registry(config: &ServiceConfig) -> Result<RegistryHandle, ServiceError> {
        let acp = Arc::new(AcpRuntime::new(&config.acp_secret));

        let db = Database::open(&config.db_path, &config.db_passphrase)?;
        let store = AgentRegistryStore::new(db.conn_arc());
        store.initialize_schema()?;

        // ACP state restoration: reload registered agents from the store
        let registered_agents = store.list().map_err(ServiceError::AgentRegistryStore)?;
        if !registered_agents.is_empty() {
            let agents: Vec<hkask_agents::acp::AcpAgent> = registered_agents
                .iter()
                .map(|ra| hkask_agents::acp::AcpAgent {
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
            acp.restore_from_storage(agents, tokens)
                .await
                .map_err(ServiceError::Acp)?;
        }

        Ok(RegistryHandle { acp, store })
    }

    /// Register a new replicant in ACP and the agent registry store.
    ///
    /// Creates a WebID, registers with ACP (granting default replicant
    /// capabilities), builds an `AgentDefinition` and `RegisteredAgent`,
    /// and persists them.
    ///
    /// If `user_profile` is provided, the replicant's display name follows
    /// the naming protocol: "{chosen_name} r{human_last_name}".
    pub async fn register_replicant(
        acp: &Arc<AcpRuntime>,
        store: &AgentRegistryStore,
        name: &str,
        description: &str,
        user_profile: Option<&UserProfile>,
        phone_number: Option<&str>,
        whatsapp_id: Option<&str>,
    ) -> Result<(), ServiceError> {
        let display_name = if let Some(profile) = user_profile {
            profile.replicant_display_name(name)
        } else {
            name.to_string()
        };
        let webid = WebID::from_persona_with_namespace(display_name.as_bytes(), "replicant");

        let default_capabilities = vec![
            "tool:inference:call".to_string(),
            "tool:mcp:invoke".to_string(),
            "registry:episodic_memory:read".to_string(),
            "registry:episodic_memory:write".to_string(),
        ];

        let token = acp
            .register_agent(webid, AgentKind::Replicant, default_capabilities.clone())
            .await
            .map_err(ServiceError::Acp)?;

        let definition = AgentDefinition {
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
            persona: None,
            depends_on: vec![],
            process_manifest: None,
            phone_number: phone_number.map(|s| s.to_string()),
            whatsapp_id: whatsapp_id.map(|s| s.to_string()),
        };

        let registered = RegisteredAgent {
            definition,
            token_hash: token.signature.clone(),
            registered_at: now_rfc3339(),
            source_yaml: "onboarding".to_string(),
        };

        store
            .insert(&registered)
            .map_err(ServiceError::AgentRegistryStore)?;

        Ok(())
    }

    /// Store the human user's profile in the registry.
    pub fn store_user_profile(
        store: &AgentRegistryStore,
        profile: &UserProfile,
    ) -> Result<(), ServiceError> {
        store
            .store_user_profile(profile)
            .map_err(|e| ServiceError::AgentRegistryStore(e.to_string()))
    }

    /// Retrieve the human user's profile from the registry.
    pub fn get_user_profile(
        store: &AgentRegistryStore,
    ) -> Result<Option<UserProfile>, ServiceError> {
        store
            .get_user_profile()
            .map_err(|e| ServiceError::AgentRegistryStore(e.to_string()))
    }

    /// Verify sign-in: initialize the registry with the given config and
    /// confirm the named replicant exists in the store.
    ///
    /// On success, stores the secrets in the keychain for future sessions
    /// and returns a `SignInOutcome`.
    pub async fn try_sign_in(
        config: &ServiceConfig,
        agent_name: &str,
        resolved_secrets: &ResolvedSecrets,
    ) -> Result<SignInOutcome, ServiceError> {
        let handle = Self::init_registry(config).await?;

        // Verify the replicant exists
        handle
            .store
            .get(agent_name)
            .map_err(|_| ServiceError::AgentNotFound(agent_name.to_string()))?;

        // Success — store secrets in keychain for future sessions
        let keychain = Keychain::default();
        keychain
            .store_by_key("acp-secret", &resolved_secrets.acp_secret)
            .map_err(|e| ServiceError::Keystore(e.to_string()))?;
        keychain
            .store_by_key("hkask-db-passphrase", &resolved_secrets.db_passphrase)
            .map_err(|e| ServiceError::Keystore(e.to_string()))?;

        Ok(SignInOutcome {
            agent_name: agent_name.to_string(),
            resolved_secrets: resolved_secrets.clone(),
        })
    }

    /// Try to list existing replicants from the database without requiring
    /// an ACP runtime. Used to determine which onboarding path to take.
    ///
    /// Returns an empty Vec if the DB can't be opened or has no replicants.
    pub fn try_list_existing_replicants(config: &ServiceConfig) -> Vec<RegisteredAgent> {
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
    pub fn remove_orphaned_db(config: &ServiceConfig) -> bool {
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
    pub fn cleanup_failed_onboarding(config: &ServiceConfig) {
        let keychain = Keychain::default();
        let _ = keychain.delete_by_key("acp-secret");
        let _ = keychain.delete_by_key("hkask-db-passphrase");

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
