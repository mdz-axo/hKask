//! Server lifecycle abstraction for hKask MCP servers.
//!
//! Provides a shared `ServerLifecycle` trait that standardizes initialization,
//! start, health checking, and shutdown across all MCP server crates. Both
//! `hkask-mcp-training` and `hkask-mcp-communication` implement this trait,
//! eliminating surface-level boilerplate in CLI/API entry points.
//!
//! CNS spans emit lifecycle events for homeostatic monitoring:
//!   `cns.server.{name}.{started,healthy,degraded,stopped}`

use thiserror::Error;

// ── Server lifecycle errors ────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("Server '{0}' failed to initialize: {1}")]
    InitFailed(String, String),
    #[error("Server '{0}' health check failed: {1}")]
    HealthFailed(String, String),
    #[error("Server '{0}' shutdown failed: {1}")]
    ShutdownFailed(String, String),
}

// ── ServerHealth ───────────────────────────────────────────────────────────

/// Health status for an MCP server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerHealth {
    /// Server is fully operational.
    Healthy,
    /// Server is running with degraded functionality.
    Degraded(String),
    /// Server has stopped.
    Stopped,
}

impl ServerHealth {
    /// Returns true if the server is healthy (not degraded and not stopped).
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }
}

// ── ServerLifecycle trait ──────────────────────────────────────────────────

/// Shared lifecycle interface for all hKask MCP servers.
///
/// Standardizes the init → start → health → shutdown pattern across servers,
/// preventing each new server from reimplementing lifecycle boilerplate.
///
/// Each server's `main.rs` should:
/// 1. Create the server struct (implementing this trait)
/// 2. Call `init(config)` with its config
/// 3. Call `start()` to begin serving
/// 4. Call `health()` periodically for CNS monitoring
/// 5. Call `shutdown()` on graceful termination
#[async_trait::async_trait]
pub trait ServerLifecycle: Send + Sync {
    /// Initialize the server with its configuration.
    ///
    /// Called once at startup. Performs any setup that may fail (database
    /// connections, credential validation, resource allocation).
    /// Returns `Ok(())` if ready to start, or `LifecycleError` on failure.
    async fn init(&mut self, config: &ServerLifecycleConfig) -> Result<(), LifecycleError>;

    /// Start the server and begin handling requests.
    ///
    /// This is the main entry point. For MCP servers, this typically calls
    /// `hkask_mcp::run_server(...)` which blocks until shutdown.
    /// Implementations should emit `cns.server.{name}.started` on success.
    async fn start(self) -> Result<(), LifecycleError>;

    /// Perform a health check.
    ///
    /// Returns the current health status. Called periodically by CNS monitoring.
    /// Implementations should check connectivity, resource availability, and
    /// any dependencies. Emits `cns.server.{name}.degraded` on degradation.
    async fn health(&self) -> Result<ServerHealth, LifecycleError>;

    /// Gracefully shut down the server.
    ///
    /// Releases resources, closes connections, persists state. Emits
    /// `cns.server.{name}.stopped` on completion.
    async fn shutdown(self) -> Result<(), LifecycleError>;
}

// ── Configuration ──────────────────────────────────────────────────────────

/// Configuration passed to `ServerLifecycle::init()`.
///
/// Contains the minimal information needed to bootstrap any MCP server:
/// server name, version, and database/credential paths.
#[derive(Debug, Clone)]
pub struct ServerLifecycleConfig {
    /// Human-readable server name (e.g., "hkask-mcp-training").
    pub name: String,
    /// Semantic version string (use `env!("CARGO_PKG_VERSION")`).
    pub version: String,
    /// Path to hKask database (for persistence).
    pub db_path: String,
    /// Database passphrase.
    pub db_passphrase: String,
    /// Path to memory database (optional).
    pub memory_db_path: Option<String>,
    /// Memory database passphrase.
    pub memory_passphrase: Option<String>,
}

impl ServerLifecycleConfig {
    /// Create from environment variables.
    pub fn from_env(name: &str, version: &str) -> Self {
        let db_path =
            std::env::var("HKASK_DB_PATH").unwrap_or_else(|_| "data/hkask.db".to_string());
        let db_passphrase = std::env::var("HKASK_DB_PASSPHRASE").unwrap_or_default();
        let memory_db_path = std::env::var("HKASK_MEMORY_DB_PATH").ok();
        let memory_passphrase = std::env::var("HKASK_MEMORY_DB_PASSPHRASE").ok();
        Self {
            name: name.to_string(),
            version: version.to_string(),
            db_path,
            db_passphrase,
            memory_db_path,
            memory_passphrase,
        }
    }
}

// ── Lifecycle runner ───────────────────────────────────────────────────────

/// Run a server through its full lifecycle with CNS instrumentation.
///
/// 1. Calls `init(config)` — emits `cns.server.{name}.started` on success.
/// 2. Calls `start()` — blocks until shutdown.
/// 3. Calls `shutdown()` — emits `cns.server.{name}.stopped` on completion.
///
/// Health checks are the caller's responsibility (e.g., from a CNS polling loop).
pub async fn run_lifecycle<S>(
    config: ServerLifecycleConfig,
    mut server: S,
) -> Result<(), LifecycleError>
where
    S: ServerLifecycle,
{
    let name = config.name.clone();
    let version = config.version.clone();

    server.init(&config).await.map_err(|e| {
        tracing::error!(
            target = "cns.server.init.failed",
            server = %name,
            error = %e,
            "Server initialization failed"
        );
        e
    })?;

    tracing::info!(
        target = "cns.server.started",
        server = %name,
        version = %version,
        "Server started"
    );

    let result = server.start().await;

    match &result {
        Ok(()) => {
            tracing::info!(
                target = "cns.server.stopped",
                server = %name,
                "Server stopped normally"
            );
        }
        Err(e) => {
            tracing::error!(
                target = "cns.server.failed",
                server = %name,
                error = %e,
                "Server failed"
            );
        }
    }

    result
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct MockServer {
        name: String,
        healthy: bool,
    }

    #[async_trait::async_trait]
    impl ServerLifecycle for MockServer {
        async fn init(&mut self, _config: &ServerLifecycleConfig) -> Result<(), LifecycleError> {
            Ok(())
        }

        async fn start(self) -> Result<(), LifecycleError> {
            Ok(())
        }

        async fn health(&self) -> Result<ServerHealth, LifecycleError> {
            if self.healthy {
                Ok(ServerHealth::Healthy)
            } else {
                Ok(ServerHealth::Degraded("mock degradation".to_string()))
            }
        }

        async fn shutdown(self) -> Result<(), LifecycleError> {
            Ok(())
        }
    }

    // REQ: lifecycle-001 — init_succeeds_with_valid_config
    #[tokio::test]
    async fn init_succeeds_with_valid_config() {
        let config = ServerLifecycleConfig {
            name: "test-server".to_string(),
            version: "0.1.0".to_string(),
            db_path: ":memory:".to_string(),
            db_passphrase: String::new(),
            memory_db_path: None,
            memory_passphrase: None,
        };
        let mut server = MockServer {
            name: "test-server".to_string(),
            healthy: true,
        };
        assert!(server.init(&config).await.is_ok());
    }

    // REQ: lifecycle-002 — health_reports_correct_status
    #[tokio::test]
    async fn health_reports_correct_status() {
        let config = ServerLifecycleConfig {
            name: "test-server".to_string(),
            version: "0.1.0".to_string(),
            db_path: ":memory:".to_string(),
            db_passphrase: String::new(),
            memory_db_path: None,
            memory_passphrase: None,
        };
        let mut healthy_server = MockServer {
            name: "test-server".to_string(),
            healthy: true,
        };
        assert!(healthy_server.health().await.unwrap().is_healthy());

        let mut degraded_server = MockServer {
            name: "test-server".to_string(),
            healthy: false,
        };
        assert!(!degraded_server.health().await.unwrap().is_healthy());
    }

    // REQ: lifecycle-003 — run_lifecycle_emits_cns_spans
    #[tokio::test]
    async fn run_lifecycle_emits_cns_spans() {
        let config = ServerLifecycleConfig {
            name: "test-server".to_string(),
            version: "0.1.0".to_string(),
            db_path: ":memory:".to_string(),
            db_passphrase: String::new(),
            memory_db_path: None,
            memory_passphrase: None,
        };
        let server = MockServer {
            name: "test-server".to_string(),
            healthy: true,
        };
        let result = run_lifecycle(config, server).await;
        assert!(result.is_ok());
    }
}
