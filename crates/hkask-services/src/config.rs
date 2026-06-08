//! Service-level configuration resolved once at startup.
//!
//! `ServiceConfig` holds all configuration that varies per deployment:
//! database paths, secrets, thresholds, and feature flags. Both CLI and API
//! surfaces construct a `ServiceConfig` from environment variables, keychain
//! secrets, or explicit parameters, then pass it to `ServiceContext::build()`.

/// Configuration resolved once at startup and shared across all services.
///
/// This replaces the four independent assembly paths currently in
/// `ReplState`, `ApiState`, `build_loop_system()`, and `commands/loops.rs`.
///
/// Construction methods (`from_env()`, `in_memory()`) will be added as
/// the extraction proceeds. The initial skeleton contains only the fields
/// needed for the first extracted domain.
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
}

impl ServiceConfig {
    /// Create a config suitable for integration tests.
    ///
    /// Uses in-memory databases and synthetic secrets. Never use in production.
    pub fn in_memory() -> Self {
        Self {
            db_path: ":memory:".to_string(),
            db_passphrase: String::new(),
            acp_secret: vec![0u8; 32],
            mcp_secret: vec![0u8; 32],
            okapi_base_url: "http://127.0.0.1:11435".to_string(),
            cns_threshold: hkask_cns::DEFAULT_THRESHOLD,
            gas_budget_cap: 10_000,
            gas_replenish_rate: 1_000,
            in_memory: true,
        }
    }
}
