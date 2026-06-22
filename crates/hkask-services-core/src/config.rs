//! Service-level configuration resolved once at startup.
//! # REQ: P1 (User Sovereignty) — secrets from OS keychain, never hardcoded.
//! expect: "Service configuration resolves secrets from the OS keychain"
//!
//! `ServiceConfig` holds all configuration that varies per deployment:
//! database paths, secrets, thresholds, and feature flags. Both CLI and API
//! surfaces construct a `ServiceConfig` from environment variables, keychain
//! secrets, or explicit parameters, then pass it to `AgentService::build()`.

use crate::error::ServiceError;
use hkask_inference::InferenceConfig;
use hkask_wallet_types::WalletConfig;

// ── Default values ──────────────────────────────────────────────────────────
// Centralized here so all three constructors share the same defaults.
// Changing a default means changing it once.
// Public so standalone CLI commands (without a ServiceConfig) can use the
// same defaults instead of duplicating string literals.

/// Default path for the primary database file.
pub const DEFAULT_DB_PATH: &str = "data/hkask.db";
const DEFAULT_ENERGY_BUDGET_CAP: u64 = 10_000;
const DEFAULT_GAS_REPLENISH_RATE: u64 = 1_000;
const DEFAULT_CNS_THRESHOLD: u64 = 100;
const DEFAULT_TEMPLATE_CACHE_PATH: &str = "/tmp/hkask-templates";
const DEFAULT_AGENT_NAME: &str = "curator";
const TEST_AGENT_NAME: &str = "test-agent";

/// Configuration resolved once at startup and shared across all services.
///
/// This replaces the four independent assembly paths currently in
/// `ReplState`, `ApiState`, `build_loop_system()`, and `commands/loops.rs`.
///
/// Construction methods:
/// - `from_env()` — resolves secrets from environment variables and keychain
/// - `from_secrets()` — accepts pre-resolved secrets (from onboarding)
/// - `in_memory()` — synthetic config for tests
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Path to the primary database file (hkask.db).
    pub db_path: String,

    /// Passphrase for encrypted database access.
    pub db_passphrase: String,

    /// HMAC secret for A2A token signing (in-process agent capability policy).
    ///
    /// Used by A2ARuntime, PodManager, FullMcpAdapter, ManifestExecutor, and
    /// CLI tool invocation. Tokens signed with this secret are verified within
    /// the same process by the A2A runtime.
    ///
    /// Guardrail: This secret is intentionally separate from `mcp_secret` to
    /// maintain defense in depth — in-process capability tokens and inter-process
    /// MCP auth tokens use independent HMAC keys so compromising one boundary
    /// does not compromise the other.
    pub a2a_secret: Vec<u8>,

    /// HMAC secret for MCP protocol authorization (inter-process).
    ///
    /// Used by API auth middleware (`AuthService`), `AgentService`'s
    /// `capability_checker`, and `McpDispatcher`. Tokens signed with this
    /// secret are verified by external MCP servers and API callers.
    ///
    /// Guardrail: See `a2a_secret` for the rationale for keeping these secrets
    /// independent.
    pub mcp_secret: Vec<u8>,

    /// Inference configuration for the multi-provider router.
    pub inference_config: InferenceConfig,

    /// CNS variety threshold for algedonic alerts.
    pub cns_threshold: u64,

    /// Gas budget cap per session (units).
    pub energy_budget_cap: u64,

    /// Gas replenish rate per turn (units).
    pub gas_replenish_rate: u64,

    /// Whether to use in-memory databases (for tests).
    pub in_memory: bool,

    /// Default inference model name.
    pub default_model: String,

    /// Agent name (from onboarding or config).
    pub agent_name: String,

    /// Path for the template cache (Git CAS storage).
    pub template_cache_path: String,

    /// Path for the memory database (episodic + semantic stores).
    ///
    /// When `in_memory: false`, memory stores persist to this file.
    /// Defaults to `{db_path}-memory.db` (e.g., `hkask.db` → `hkask-memory.db`)
    /// when not explicitly set. Ignored when `in_memory: true`.
    pub memory_db_path: Option<String>,

    /// Passphrase for the memory database encryption.
    ///
    /// Defaults to `db_passphrase` when not set.
    /// Ignored when `in_memory: true`.
    pub memory_passphrase: Option<String>,

    /// Path to YAML agent definition directory.
    ///
    /// Defaults to `registry/bots` when not set.
    pub registry_yaml_path: std::path::PathBuf,

    /// Wallet configuration for rJoule payments and multi-chain deposits.
    pub wallet_config: WalletConfig,
}

impl ServiceConfig {
    /// Resolve configuration from environment variables and keychain.
    ///
    /// Reads `HKASK_DB_PATH`, `HKASK_TEMPLATE_CACHE_PATH`,
    /// and `HKASK_MEMORY_DB_PATH` from environment. A2A and MCP secrets are
    /// resolved via `hkask_keystore`. Falls back to defaults for missing values.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  keystore must have a2a_secret, db_passphrase, and mcp_secret configured
    /// post: returns ServiceConfig with env-derived values and keystore secrets; Err(Keystore) on secret resolution failure
    pub fn from_env() -> Result<Self, ServiceError> {
        let db_path =
            std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
        let inference_config = InferenceConfig::from_env();
        let default_model = inference_config.default_model.clone();
        let template_cache_path = std::env::var("HKASK_TEMPLATE_CACHE_PATH")
            .unwrap_or_else(|_| DEFAULT_TEMPLATE_CACHE_PATH.to_string());
        let memory_db_path = std::env::var("HKASK_MEMORY_DB_PATH").ok();
        let registry_yaml_path = std::path::PathBuf::from(
            std::env::var("HKASK_REGISTRY_PATH").unwrap_or_else(|_| "registry/bots".to_string()),
        );

        // Resolve secrets from keystore. If keystore resolution fails,
        // fall back to empty secrets (in-memory mode will be used).
        let a2a_secret = hkask_keystore::keychain::resolve_a2a_secret()
            .map_err(|e| ServiceError::Keystore {
                source: Some(Box::new(e)),
                message: "Failed to resolve A2A secret".into(),
            })?
            .to_vec();

        let db_passphrase_bytes =
            hkask_keystore::keychain::resolve_db_passphrase().map_err(|e| {
                ServiceError::Keystore {
                    source: Some(Box::new(e)),
                    message: "Failed to resolve DB passphrase".into(),
                }
            })?;
        let db_passphrase = String::from_utf8_lossy(db_passphrase_bytes.as_ref()).into_owned();

        let mcp_secret_vec = hkask_keystore::keychain::resolve_mcp_secret()
            .map_err(|e| ServiceError::Keystore {
                source: Some(Box::new(e)),
                message: "Failed to resolve MCP secret".into(),
            })?
            .to_vec();

        Ok(Self {
            db_path,
            db_passphrase,
            a2a_secret,
            mcp_secret: mcp_secret_vec,
            default_model,
            inference_config,
            cns_threshold: DEFAULT_CNS_THRESHOLD,
            energy_budget_cap: DEFAULT_ENERGY_BUDGET_CAP,
            gas_replenish_rate: DEFAULT_GAS_REPLENISH_RATE,
            in_memory: false,
            agent_name: DEFAULT_AGENT_NAME.to_string(),
            template_cache_path,
            memory_db_path,
            memory_passphrase: None,
            registry_yaml_path,
            wallet_config: WalletConfig::default(),
        })
    }

    /// Create a config from pre-resolved secrets (e.g., from onboarding).
    ///
    /// This avoids re-resolving from the keychain, which is important
    /// for the REPL's interactive onboarding flow.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  a2a_secret, db_passphrase, mcp_secret, agent_name must be non-empty
    /// post: returns ServiceConfig with provided secrets and env-derived or default values
    pub fn from_secrets(
        a2a_secret: String,
        db_passphrase: String,
        mcp_secret: String,
        agent_name: String,
    ) -> Self {
        let db_path =
            std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
        let inference_config = InferenceConfig::from_env();
        let template_cache_path = std::env::var("HKASK_TEMPLATE_CACHE_PATH")
            .unwrap_or_else(|_| DEFAULT_TEMPLATE_CACHE_PATH.to_string());
        let memory_db_path = std::env::var("HKASK_MEMORY_DB_PATH").ok();
        let registry_yaml_path = std::path::PathBuf::from(
            std::env::var("HKASK_REGISTRY_PATH").unwrap_or_else(|_| "registry/bots".to_string()),
        );

        Self {
            db_path,
            db_passphrase,
            a2a_secret: a2a_secret.into_bytes(),
            mcp_secret: mcp_secret.into_bytes(),
            inference_config: inference_config.clone(),
            cns_threshold: DEFAULT_CNS_THRESHOLD,
            energy_budget_cap: DEFAULT_ENERGY_BUDGET_CAP,
            gas_replenish_rate: DEFAULT_GAS_REPLENISH_RATE,
            in_memory: false,
            default_model: inference_config.default_model.clone(),
            agent_name,
            template_cache_path,
            memory_db_path,
            memory_passphrase: None,
            registry_yaml_path,
            wallet_config: WalletConfig::default(),
        }
    }

    /// Create a config suitable for integration tests.
    ///
    /// Uses in-memory databases and synthetic secrets. Never use in production.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  none (always succeeds)
    /// post: returns ServiceConfig with :memory: DB, zeroed secrets, and test agent name
    pub fn in_memory() -> Self {
        let inference_config = InferenceConfig::default();
        Self {
            db_path: ":memory:".to_string(),
            db_passphrase: String::new(),
            a2a_secret: vec![0u8; 32],
            mcp_secret: vec![0u8; 32],
            inference_config: inference_config.clone(),
            cns_threshold: DEFAULT_CNS_THRESHOLD,
            energy_budget_cap: DEFAULT_ENERGY_BUDGET_CAP,
            gas_replenish_rate: DEFAULT_GAS_REPLENISH_RATE,
            in_memory: true,
            default_model: inference_config.default_model.clone(),
            agent_name: TEST_AGENT_NAME.to_string(),
            template_cache_path: DEFAULT_TEMPLATE_CACHE_PATH.to_string(),
            memory_db_path: None,
            memory_passphrase: None,
            registry_yaml_path: std::path::PathBuf::from("registry/bots"),
            wallet_config: WalletConfig::default(),
        }
    }

    /// Returns the effective memory DB path when `in_memory: false`.
    ///
    /// Uses the standard agent directory layout: `agents/{agent_name}/memory.db`.
    /// This puts the agent's memory database alongside its pod database in the
    /// same directory, so all of an agent's data is self-contained in one folder.
    ///
    /// Returns `None` when `in_memory: true` (memory stores are ephemeral).
    ///
    /// pre:  none (always succeeds)
    /// post: returns Some(path) using agent dir layout if not in_memory; None if in_memory
    pub fn effective_memory_db_path(&self) -> Option<String> {
        if self.in_memory {
            return None;
        }
        Some(
            hkask_types::agent_paths::agent_memory_db(&self.agent_name)
                .to_string_lossy()
                .to_string(),
        )
    }
}
