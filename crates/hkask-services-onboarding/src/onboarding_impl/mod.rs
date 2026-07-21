//! Onboarding — secret derivation, keychain, registry init, sign-in.
//! # REQ: P1 (User Sovereignty) — keychain secrets, passphrase-derived keys.
//! # expect: "My service operations flow through sovereignty-verifying boundaries"

use std::sync::Arc;

use hkask_agents::A2ARuntime;
use hkask_database::sqlite::SqliteDriver;
use hkask_keystore::{Keychain, master_key::derive_all_internal_secrets};
use hkask_storage::{AgentRegistryStore, Database, now_rfc3339};
use hkask_types::WebID;
use hkask_types::agent_paths;
// agent registry removed

use hkask_services_core::{DomainKind, ErrorKind, ServiceConfig, ServiceError};

pub mod matrix;
pub use matrix::MatrixRegistrationResult;

pub use matrix::conduit_ensure_healthy;
pub use matrix::conduit_health_check;

/// Optional contact and voice configuration for a new replicant.
/// Grouped to keep `register_userpod` argument count manageable.
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
                .map_err(|e| ServiceError::Domain {
                    kind: ErrorKind::BadRequest,
                    domain: DomainKind::Infrastructure,
                    source: Some(Box::new(e)),
                    message: "Failed to store HKASK_MASTER_KEY".into(),
                })?;
            keychain
                .store_by_key(
                    hkask_types::keychain_keys::KEY_A2A_SECRET,
                    &secrets.a2a_secret,
                )
                .map_err(|e| ServiceError::Domain {
                    kind: ErrorKind::BadRequest,
                    domain: DomainKind::Infrastructure,
                    source: Some(Box::new(e)),
                    message: "Failed to store a2a-secret".into(),
                })?;
            keychain
                .store_by_key(hkask_types::keychain_keys::KEY_DB_PASSPHRASE, passphrase)
                .map_err(|e| ServiceError::Domain {
                    kind: ErrorKind::BadRequest,
                    domain: DomainKind::Infrastructure,
                    source: Some(Box::new(e)),
                    message: "Failed to store hkask-db-passphrase".into(),
                })?;
        }
        // P9: CNS span
        tracing::info!(
            target: "hkask.onboarding",
            operation = "secrets_derived",
            stored = store,
            "CNS"
        );
        Ok(ResolvedSecrets {
            master_key_hex: secrets.master_key_hex.clone(),
            a2a_secret: secrets.a2a_secret.clone(),
            db_passphrase: passphrase.to_string(),
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
            ServiceError::Domain {
                kind: ErrorKind::BadRequest,
                domain: DomainKind::Storage,
                source: None,
                message: e.to_string(),
            }
        })?;
        let pool = db.sqlite_pool().map_err(|e| ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::Storage,
            source: None,
            message: e.to_string(),
        })?;
        let driver = Arc::new(SqliteDriver::new(pool));
        let store = AgentRegistryStore::from_driver(driver);

        // A2A state restoration: reload registered agents from the store
        let registered_agents = store.list().map_err(|e| ServiceError::Domain {
            domain: DomainKind::Agent,
            kind: ErrorKind::ServiceUnavailable,
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
                .map_err(|e| ServiceError::Domain {
                    domain: DomainKind::Agent,
                    kind: ErrorKind::Forbidden,
                    source: None,
                    message: e.to_string(),
                })?;
        }

        // P9: CNS span
        tracing::info!(
            target: "hkask.onboarding",
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
    pub async fn register_userpod(
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
                target: "hkask.onboarding",
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
            .register_agent(webid, default_capabilities.clone())
            .await
            .map_err(|e| ServiceError::Domain {
                domain: DomainKind::Agent,
                kind: ErrorKind::Forbidden,
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
        if let Some(parent) = yaml_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::warn!(
                target: "hkask.onboarding",
                path = %parent.display(),
                error = %e,
                "Failed to create agent directory — will attempt write anyway"
            );
        }
        std::fs::write(&yaml_path, &source_yaml).map_err(|e| ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::Storage,
            source: None,
            message: format!("Failed to write agent YAML to {}: {e}", yaml_path.display()),
        })?;

        let registered = RegisteredAgent {
            definition: AgentDefinition {
                name: display_name,
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
            .map_err(|e| ServiceError::Domain {
                domain: DomainKind::Agent,
                kind: ErrorKind::ServiceUnavailable,
                source: None,
                message: e.to_string(),
            })?;

        // P9: CNS span
        tracing::info!(
            target: "hkask.onboarding",
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
        tracing::info!(target: "hkask.onboarding", operation = "store_user_profile", "CNS");
        store
            .store_user_profile(profile)
            .map_err(|e| ServiceError::Domain {
                domain: DomainKind::Agent,
                kind: ErrorKind::ServiceUnavailable,
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
        tracing::info!(target: "hkask.onboarding", operation = "get_user_profile", "CNS");
        store.get_user_profile().map_err(|e| ServiceError::Domain {
            domain: DomainKind::Agent,
            kind: ErrorKind::ServiceUnavailable,
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
        tracing::info!(target: "hkask.onboarding", operation = "try_sign_in", agent = %agent_name, "CNS");
        let handle = Self::init_registry(config).await?;

        // Verify the replicant exists
        handle
            .store
            .get(agent_name)
            .map_err(|_| ServiceError::Domain {
                domain: DomainKind::Agent,
                kind: ErrorKind::NotFound,
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
            .map_err(|e| ServiceError::Domain {
                kind: ErrorKind::BadRequest,
                domain: DomainKind::Infrastructure,
                source: Some(Box::new(e)),
                message: "Failed to store a2a-secret".into(),
            })?;
        keychain
            .store_by_key(
                hkask_types::keychain_keys::KEY_DB_PASSPHRASE,
                &resolved_secrets.db_passphrase,
            )
            .map_err(|e| ServiceError::Domain {
                kind: ErrorKind::BadRequest,
                domain: DomainKind::Infrastructure,
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
        tracing::info!(target: "hkask.onboarding", operation = "try_list_existing_replicants", "CNS");
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

        let pool = match db.sqlite_pool() {
            Ok(pool) => pool,
            Err(_) => return Vec::new(),
        };
        let driver = Arc::new(SqliteDriver::new(pool));
        let store = AgentRegistryStore::from_driver(driver);
        store.list().unwrap_or_default()
    }

    /// Check for an orphaned DB from a previous failed onboarding attempt.
    ///
    /// If the DB exists but has no replicants (or can't be opened with the
    /// current passphrase), it's orphaned and should be removed before
    /// starting a fresh onboarding. Returns `true` if cleanup was performed.
    ///
    /// **Known limitation**: When the passphrase doesn't match the DB,
    /// SQLCipher's C-level codec logs `hmac check failed for pgno=1` errors
    /// to stderr. These are SQLCipher's internal diagnostics and cannot be
    /// suppressed from Rust. They are cosmetic — the function correctly
    /// returns `true` (orphaned) despite the noise.
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
                let pool = match db.sqlite_pool() {
                    Ok(pool) => pool,
                    Err(_) => return true,
                };
                let driver = Arc::new(SqliteDriver::new(pool));
                let store = AgentRegistryStore::from_driver(driver);
                matches!(store.list(), Ok(r) if r.is_empty())
            }
            Err(_) => true, // Can't open — likely orphaned/corrupted
        }
    }

    /// Remove an orphaned DB without re-opening it.
    ///
    /// Use this when `has_orphaned_db` has already returned `true` — the caller
    /// has already paid the cost of probing the DB (and absorbing SQLCipher's
    /// unsuppressable `hmac check failed` diagnostics). Re-opening here would
    /// emit a second round of those diagnostics for no information gain.
    ///
    /// Unlike a re-checking variant, this does NOT re-check for replicants: the
    /// caller is responsible for having confirmed orphaned status first.
    ///
    /// pre:  `has_orphaned_db(config)` returned `true`; config.db_path is not `:memory:`
    /// post: returns true if the DB file was removed; false if cleanup failed to delete it
    #[must_use]
    pub fn remove_orphaned_db_unchecked(config: &ServiceConfig) -> bool {
        tracing::info!(
            target: "hkask.onboarding",
            operation = "remove_orphaned_db_unchecked",
            "CNS"
        );
        let db_path = &config.db_path;
        if db_path == ":memory:" || !std::path::Path::new(db_path).exists() {
            return false;
        }
        Self::cleanup_failed_onboarding(config);
        !std::path::Path::new(db_path).exists()
    }

    /// Roll back a failed onboarding by removing keychain entries, the
    /// database file, and the salt file.
    ///
    /// Called when onboarding fails after partial setup (e.g., keychain
    /// stored but registration failed). Prevents orphaned state from
    /// poisoning subsequent attempts.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  config must be valid; best-effort cleanup (errors are logged via tracing)
    /// post: keychain entries (a2a-secret, hkask-db-passphrase) are removed; DB and salt files deleted if not :memory:
    pub fn cleanup_failed_onboarding(config: &ServiceConfig) {
        // P9: CNS span
        tracing::info!(target: "hkask.onboarding", operation = "cleanup_failed_onboarding", "CNS");
        let keychain = Keychain::default();
        if let Err(e) = keychain.delete_by_key(hkask_types::keychain_keys::KEY_A2A_SECRET) {
            tracing::warn!(target: "hkask.onboarding", error = %e, "Failed to delete A2A secret from keychain during cleanup");
        }
        if let Err(e) = keychain.delete_by_key(hkask_types::keychain_keys::KEY_DB_PASSPHRASE) {
            tracing::warn!(target: "hkask.onboarding", error = %e, "Failed to delete DB passphrase from keychain during cleanup");
        }

        let db_path = &config.db_path;
        if db_path != ":memory:" {
            let db_file = std::path::Path::new(db_path);
            if db_file.exists()
                && let Err(e) = std::fs::remove_file(db_file)
            {
                tracing::warn!(target: "hkask.onboarding", path = %db_file.display(), error = %e, "Failed to remove orphaned DB file");
            }
            let salt_file = std::path::PathBuf::from(format!("{}.salt", db_path));
            if salt_file.exists()
                && let Err(e) = std::fs::remove_file(&salt_file)
            {
                tracing::warn!(target: "hkask.onboarding", path = %salt_file.display(), error = %e, "Failed to remove salt file");
            }

            // Also remove the agent's sub-directory under agents/{name}/
            // to prevent stale per-agent databases (memory.db, wallet.db,
            // kanban.db) from being reused on re-onboarding.
            let agent_dir = std::path::Path::new(db_path)
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(hkask_types::agent_paths::AGENTS_DIR)
                .join(hkask_types::agent_paths::sanitize_name(&config.agent_name));
            if agent_dir.exists() {
                match std::fs::remove_dir_all(&agent_dir) {
                    Err(e) => {
                        tracing::warn!(target: "hkask.onboarding", path = %agent_dir.display(), error = %e, "Failed to remove agent database directory");
                    }
                    Ok(()) => {
                        tracing::info!(
                            target: "hkask.onboarding",
                            path = %agent_dir.display(),
                            "Removed agent database directory"
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod secret_tests {
    use super::*;

    #[test]
    fn onboarding_preserves_explicit_database_passphrase() {
        let resolved = OnboardingService::derive_secrets("explicit-db-passphrase", false)
            .expect("derive onboarding secrets");

        assert_eq!(resolved.db_passphrase, "explicit-db-passphrase");
    }
}

#[cfg(test)]
mod orphaned_db_tests {
    use super::*;

    /// Regression: `has_orphaned_db` must return `true` when the DB exists
    /// but the passphrase doesn't match. This is the precondition for the
    /// `remove_orphaned_db` fix — when `sqlite_pool()` fails (wrong passphrase),
    /// `remove_orphaned_db` must proceed to cleanup instead of returning `false`.
    ///
    /// Before the fix, `remove_orphaned_db` returned `false` when
    /// `sqlite_pool()` failed (wrong passphrase), leaving stale DB + salt files
    /// on disk. `has_orphaned_db` correctly returned `true` for the same
    /// condition, but the caller ignored the return value.
    #[test]
    fn has_orphaned_db_returns_true_for_wrong_passphrase() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let db_path = temp_dir.path().join("test_registry.db");
        let db_path_str = db_path.to_str().expect("valid path");

        // Create an encrypted DB with passphrase "correct-passphrase"
        let db = Database::open(db_path_str, "correct-passphrase")
            .expect("open DB with correct passphrase");
        let _pool = db
            .sqlite_pool()
            .expect("create pool with correct passphrase");

        // DB and salt files should exist
        assert!(db_path.exists(), "DB file should exist after pool creation");
        assert!(
            std::path::Path::new(&format!("{}.salt", db_path_str)).exists(),
            "salt file should exist"
        );

        // Construct a config with the WRONG passphrase
        let mut config = ServiceConfig::from_secrets(
            "test-a2a-secret".to_string(),
            "wrong-passphrase".to_string(),
            "test-agent".to_string(),
        );
        config.db_path = db_path_str.to_string();

        // has_orphaned_db should return true — the DB exists but can't be opened
        // with this passphrase, so it's orphaned.
        assert!(
            OnboardingService::has_orphaned_db(&config),
            "DB with wrong passphrase should be detected as orphaned"
        );

        // has_orphaned_db must NOT remove the files — it's a check, not a cleanup
        assert!(
            db_path.exists(),
            "has_orphaned_db must not delete the DB file"
        );
    }

    /// `remove_orphaned_db_unchecked` removes a DB whose orphaned status has
    /// already been confirmed by `has_orphaned_db`. It must not re-open the
    /// DB (which would emit a second round of unsuppressable SQLCipher hmac
    /// diagnostics). Verifies the DB and salt files are removed.
    #[test]
    fn remove_orphaned_db_unchecked_removes_orphaned_db() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let db_path = temp_dir.path().join("test_registry.db");
        let db_path_str = db_path.to_str().expect("valid path");

        // Create an encrypted DB with a passphrase that won't match the config.
        let db = Database::open(db_path_str, "correct-passphrase")
            .expect("open DB with correct passphrase");
        let _pool = db
            .sqlite_pool()
            .expect("create pool with correct passphrase");
        drop(_pool);
        drop(db);

        assert!(db_path.exists(), "DB file should exist");
        let salt_path = format!("{}.salt", db_path_str);
        assert!(
            std::path::Path::new(&salt_path).exists(),
            "salt file should exist"
        );

        let mut config = ServiceConfig::from_secrets(
            "test-a2a-secret".to_string(),
            "wrong-passphrase".to_string(),
            "test-agent".to_string(),
        );
        config.db_path = db_path_str.to_string();

        // Precondition: has_orphaned_db confirms orphaned status.
        assert!(
            OnboardingService::has_orphaned_db(&config),
            "precondition: DB should be detected as orphaned"
        );

        // Act: remove without re-opening.
        assert!(
            OnboardingService::remove_orphaned_db_unchecked(&config),
            "unchecked removal should succeed and report true"
        );

        assert!(!db_path.exists(), "DB file should be removed");
        assert!(
            !std::path::Path::new(&salt_path).exists(),
            "salt file should be removed"
        );
    }

    /// `remove_orphaned_db_unchecked` returns `false` for `:memory:` and
    /// non-existent paths without invoking cleanup (no keychain side effects).
    #[test]
    fn remove_orphaned_db_unchecked_returns_false_for_no_file() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let db_path = temp_dir.path().join("nonexistent.db");

        let mut config = ServiceConfig::from_secrets(
            "test-a2a-secret".to_string(),
            "any-passphrase".to_string(),
            "test-agent".to_string(),
        );
        config.db_path = db_path.to_str().unwrap().to_string();

        assert!(
            !OnboardingService::remove_orphaned_db_unchecked(&config),
            "unchecked removal should return false for non-existent DB"
        );

        config.db_path = ":memory:".to_string();
        assert!(
            !OnboardingService::remove_orphaned_db_unchecked(&config),
            "unchecked removal should return false for :memory: DB"
        );
    }
}
