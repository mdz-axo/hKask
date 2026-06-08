//! Service-level configuration resolved once at startup.
//!
//! `ServiceConfig` holds all configuration that varies per deployment:
//! database paths, secrets, thresholds, and feature flags. Both CLI and API
//! surfaces construct a `ServiceConfig` from environment variables, keychain
//! secrets, or explicit parameters, then pass it to `ServiceContext::build()`.

use crate::error::ServiceError;

// ── Default values ──────────────────────────────────────────────────────────
// Centralized here so all three constructors share the same defaults.
// Changing a default means changing it once.

const DEFAULT_DB_PATH: &str = "hkask.db";
const DEFAULT_OKAPI_BASE_URL: &str = "http://127.0.0.1:11435";
const DEFAULT_GAS_BUDGET_CAP: u64 = 10_000;
const DEFAULT_GAS_REPLENISH_RATE: u64 = 1_000;
const DEFAULT_MODEL: &str = "deepseek-v4-pro";
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

    /// HMAC secret for ACP token signing.
    pub acp_secret: Vec<u8>,

    /// MCP capability secret for tool invocation authorization.
    pub mcp_secret: Vec<u8>,

    /// Base URL for Okapi inference server.
    pub okapi_base_url: String,

    /// CNS variety threshold for algedonic alerts.
    pub cns_threshold: u64,

    /// Gas budget cap per session (units).
    pub gas_budget_cap: u64,

    /// Gas replenish rate per turn (units).
    pub gas_replenish_rate: u64,

    /// Whether to use in-memory databases (for tests).
    pub in_memory: bool,

    /// Default inference model name.
    pub default_model: String,

    /// HHH gate model name (for Helpful/Harmless/Honest alignment).
    pub gate_model: String,

    /// Agent name (from onboarding or config).
    pub agent_name: String,

    /// Path for the template cache (Git CAS storage).
    pub template_cache_path: String,
}

impl ServiceConfig {
    /// Resolve configuration from environment variables and keychain.
    ///
    /// Reads `HKASK_DB_PATH` and `OKAPI_BASE_URL` from environment.
    /// ACP and MCP secrets are resolved via `hkask_keystore`.
    /// Falls back to defaults for missing values.
    pub fn from_env() -> Result<Self, ServiceError> {
        let db_path =
            std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
        let okapi_base_url =
            std::env::var("OKAPI_BASE_URL").unwrap_or_else(|_| DEFAULT_OKAPI_BASE_URL.to_string());
        let template_cache_path = std::env::var("HKASK_TEMPLATE_CACHE_PATH")
            .unwrap_or_else(|_| DEFAULT_TEMPLATE_CACHE_PATH.to_string());

        // Resolve secrets from keystore. If keystore resolution fails,
        // fall back to empty secrets (in-memory mode will be used).
        let acp_secret = hkask_keystore::resolve_acp_secret()
            .map_err(|e| ServiceError::Keystore(format!("Failed to resolve ACP secret: {e}")))?
            .to_vec();

        let db_passphrase_bytes = hkask_keystore::resolve_db_passphrase()
            .map_err(|e| ServiceError::Keystore(format!("Failed to resolve DB passphrase: {e}")))?;
        let db_passphrase = String::from_utf8_lossy(db_passphrase_bytes.as_ref()).into_owned();

        let mcp_secret_vec = hkask_keystore::resolve_mcp_secret()
            .map_err(|e| ServiceError::Keystore(format!("Failed to resolve MCP secret: {e}")))?
            .to_vec();

        Ok(Self {
            db_path,
            db_passphrase,
            acp_secret,
            mcp_secret: mcp_secret_vec,
            okapi_base_url,
            cns_threshold: hkask_cns::DEFAULT_THRESHOLD,
            gas_budget_cap: DEFAULT_GAS_BUDGET_CAP,
            gas_replenish_rate: DEFAULT_GAS_REPLENISH_RATE,
            in_memory: false,
            default_model: DEFAULT_MODEL.to_string(),
            gate_model: hkask_agents::hhh_gate::HHH_DEFAULT_GATE_MODEL.to_string(),
            agent_name: DEFAULT_AGENT_NAME.to_string(),
            template_cache_path,
        })
    }

    /// Create a config from pre-resolved secrets (e.g., from onboarding).
    ///
    /// This avoids re-resolving from the keychain, which is important
    /// for the REPL's interactive onboarding flow.
    pub fn from_secrets(
        acp_secret: String,
        db_passphrase: String,
        mcp_secret: String,
        agent_name: String,
    ) -> Self {
        let db_path =
            std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
        let okapi_base_url =
            std::env::var("OKAPI_BASE_URL").unwrap_or_else(|_| DEFAULT_OKAPI_BASE_URL.to_string());
        let template_cache_path = std::env::var("HKASK_TEMPLATE_CACHE_PATH")
            .unwrap_or_else(|_| DEFAULT_TEMPLATE_CACHE_PATH.to_string());

        Self {
            db_path,
            db_passphrase,
            acp_secret: acp_secret.into_bytes(),
            mcp_secret: mcp_secret.into_bytes(),
            okapi_base_url,
            cns_threshold: hkask_cns::DEFAULT_THRESHOLD,
            gas_budget_cap: DEFAULT_GAS_BUDGET_CAP,
            gas_replenish_rate: DEFAULT_GAS_REPLENISH_RATE,
            in_memory: false,
            default_model: DEFAULT_MODEL.to_string(),
            gate_model: hkask_agents::hhh_gate::HHH_DEFAULT_GATE_MODEL.to_string(),
            agent_name,
            template_cache_path,
        }
    }

    /// Create a config suitable for integration tests.
    ///
    /// Uses in-memory databases and synthetic secrets. Never use in production.
    pub fn in_memory() -> Self {
        Self {
            db_path: ":memory:".to_string(),
            db_passphrase: String::new(),
            acp_secret: vec![0u8; 32],
            mcp_secret: vec![0u8; 32],
            okapi_base_url: DEFAULT_OKAPI_BASE_URL.to_string(),
            cns_threshold: hkask_cns::DEFAULT_THRESHOLD,
            gas_budget_cap: DEFAULT_GAS_BUDGET_CAP,
            gas_replenish_rate: DEFAULT_GAS_REPLENISH_RATE,
            in_memory: true,
            default_model: DEFAULT_MODEL.to_string(),
            gate_model: hkask_agents::hhh_gate::HHH_DEFAULT_GATE_MODEL.to_string(),
            agent_name: TEST_AGENT_NAME.to_string(),
            template_cache_path: DEFAULT_TEMPLATE_CACHE_PATH.to_string(),
        }
    }
}
