//! MCP Server Supervision Tree
//!
//! Provides process supervision for MCP servers with automatic restart
//! and health monitoring capabilities.

use hkask_cns::CnsEmit;
use serde_json::json;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Supervision errors
#[derive(Error, Debug)]
pub enum SupervisionError {
    #[error("Failed to spawn server {name}: {source}")]
    SpawnFailed {
        name: String,
        source: std::io::Error,
    },

    #[error("Server {name} not found")]
    ServerNotFound { name: String },

    #[error("Server {name} already running")]
    AlreadyRunning { name: String },

    #[error("Failed to stop server {name}: {source}")]
    StopFailed {
        name: String,
        source: std::io::Error,
    },
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server name (unique identifier)
    pub name: String,
    /// Command to execute
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Restart policy
    pub restart_policy: RestartPolicy,
    /// Health check interval
    pub health_check_interval: Duration,
}

/// Restart policy for crashed servers
#[derive(Debug, Clone, Copy)]
pub enum RestartPolicy {
    /// Never restart
    Never,
    /// Always restart
    Always,
    /// Restart with backoff (max restarts, backoff duration)
    OnFailure {
        max_restarts: u32,
        backoff: Duration,
    },
}

/// Server status
#[derive(Debug, Clone)]
pub struct ServerStatus {
    /// Server name
    pub name: String,
    /// Whether the server is running
    pub running: bool,
    /// Process ID (if running)
    pub pid: Option<u32>,
    /// Number of restarts
    pub restart_count: u32,
    /// Last restart time
    pub last_restart: Option<Instant>,
    /// Uptime (if running)
    pub uptime: Option<Duration>,
}

/// Internal server state
struct ServerState {
    config: ServerConfig,
    child: Option<Child>,
    restart_count: u32,
    last_restart: Option<Instant>,
    started_at: Option<Instant>,
}

/// MCP Server Supervisor
///
/// Manages the lifecycle of MCP server processes with automatic
/// restart and health monitoring.
pub struct McpSupervisor {
    servers: Arc<RwLock<HashMap<String, ServerState>>>,
    /// Optional CNS emitter for connector lifecycle events
    cns_emitter: Option<Arc<dyn CnsEmit + Send + Sync>>,
}

impl McpSupervisor {
    /// Create a new supervisor
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            cns_emitter: None,
        }
    }

    /// Set the CNS emitter for connector lifecycle span emission
    pub fn with_cns_emitter(mut self, emitter: Arc<dyn CnsEmit + Send + Sync>) -> Self {
        self.cns_emitter = Some(emitter);
        self
    }

    /// Register a server configuration
    pub async fn register(&self, config: ServerConfig) {
        let mut servers = self.servers.write().await;
        servers.insert(
            config.name.clone(),
            ServerState {
                config,
                child: None,
                restart_count: 0,
                last_restart: None,
                started_at: None,
            },
        );
    }

    /// Start a registered server
    pub async fn start(&self, name: &str) -> Result<(), SupervisionError> {
        let mut servers = self.servers.write().await;
        let state = servers
            .get_mut(name)
            .ok_or_else(|| SupervisionError::ServerNotFound {
                name: name.to_string(),
            })?;

        if state.child.is_some() {
            return Err(SupervisionError::AlreadyRunning {
                name: name.to_string(),
            });
        }

        let child =
            Self::spawn_server(&state.config).map_err(|e| SupervisionError::SpawnFailed {
                name: name.to_string(),
                source: e,
            })?;

        state.child = Some(child);
        state.started_at = Some(Instant::now());

        if let Some(ref emitter) = self.cns_emitter {
            emitter.emit_event(
                &format!("cns.connector.{}.started", name),
                "observe",
                &json!({"server": name}),
                1.0,
            );
        }

        info!(server = %name, "MCP server started");
        Ok(())
    }

    /// Stop a running server
    pub async fn stop(&self, name: &str) -> Result<(), SupervisionError> {
        let mut servers = self.servers.write().await;
        let state = servers
            .get_mut(name)
            .ok_or_else(|| SupervisionError::ServerNotFound {
                name: name.to_string(),
            })?;

        if let Some(mut child) = state.child.take() {
            child
                .kill()
                .await
                .map_err(|e| SupervisionError::StopFailed {
                    name: name.to_string(),
                    source: e,
                })?;
            state.started_at = None;
            if let Some(ref emitter) = self.cns_emitter {
                emitter.emit_event(
                    &format!("cns.connector.{}.stopped", name),
                    "observe",
                    &json!({"server": name}),
                    1.0,
                );
            }
            info!(server = %name, "MCP server stopped");
        }

        Ok(())
    }

    /// Restart a server
    pub async fn restart(&self, name: &str) -> Result<(), SupervisionError> {
        // Stop if running (ignore errors if not running)
        let _ = self.stop(name).await;

        // Start the server
        self.start(name).await?;

        // Update restart count
        let mut servers = self.servers.write().await;
        if let Some(state) = servers.get_mut(name) {
            state.restart_count += 1;
            state.last_restart = Some(Instant::now());
        }

        if let Some(ref emitter) = self.cns_emitter {
            emitter.emit_event(
                &format!("cns.connector.{}.restarted", name),
                "observe",
                &json!({"server": name}),
                1.0,
            );
        }

        info!(server = %name, "MCP server restarted");
        Ok(())
    }

    /// Get status of a server
    pub async fn status(&self, name: &str) -> Result<ServerStatus, SupervisionError> {
        let servers = self.servers.read().await;
        let state = servers
            .get(name)
            .ok_or_else(|| SupervisionError::ServerNotFound {
                name: name.to_string(),
            })?;

        let (running, pid) = if let Some(ref child) = state.child {
            (true, child.id())
        } else {
            (false, None)
        };

        let uptime = state.started_at.map(|started| started.elapsed());

        Ok(ServerStatus {
            name: name.to_string(),
            running,
            pid,
            restart_count: state.restart_count,
            last_restart: state.last_restart,
            uptime,
        })
    }

    /// Get status of all servers
    pub async fn status_all(&self) -> Vec<ServerStatus> {
        let servers = self.servers.read().await;
        let mut statuses = Vec::new();

        for (name, state) in servers.iter() {
            let (running, pid) = if let Some(ref child) = state.child {
                (true, child.id())
            } else {
                (false, None)
            };

            let uptime = state.started_at.map(|started| started.elapsed());

            statuses.push(ServerStatus {
                name: name.clone(),
                running,
                pid,
                restart_count: state.restart_count,
                last_restart: state.last_restart,
                uptime,
            });
        }

        statuses
    }

    /// Check health of all servers and restart if needed
    pub async fn check_and_restart(&self) {
        let mut servers = self.servers.write().await;
        let mut to_restart = Vec::new();

        for (name, state) in servers.iter_mut() {
            if let Some(ref mut child) = state.child {
                // Try to get exit status without blocking
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // Process has exited
                        if let Some(ref emitter) = self.cns_emitter {
                            emitter.emit_event(
                                &format!("cns.connector.{}.error", name),
                                "observe",
                                &json!({"server": name, "exit_status": status.to_string()}),
                                0.0,
                            );
                        }
                        warn!(
                            server = %name,
                            status = %status,
                            "MCP server exited"
                        );

                        // Check restart policy
                        let should_restart = match state.config.restart_policy {
                            RestartPolicy::Never => false,
                            RestartPolicy::Always => true,
                            RestartPolicy::OnFailure {
                                max_restarts,
                                backoff: _,
                            } => !status.success() && state.restart_count < max_restarts,
                        };

                        if should_restart {
                            to_restart.push(name.clone());
                        }
                    }
                    Ok(None) => {
                        // Process is still running
                    }
                    Err(e) => {
                        error!(
                            server = %name,
                            error = %e,
                            "Failed to check server status"
                        );
                    }
                }
            }
        }

        // Drop the lock before restarting
        drop(servers);

        // Restart servers that need it
        for name in to_restart {
            if let Err(e) = self.restart(&name).await {
                error!(
                    server = %name,
                    error = %e,
                    "Failed to restart server"
                );
            }
        }
    }

    /// Start all registered servers
    pub async fn start_all(&self) {
        let servers = self.servers.read().await;
        let names: Vec<String> = servers.keys().cloned().collect();
        drop(servers);

        for name in names {
            if let Err(e) = self.start(&name).await {
                error!(
                    server = %name,
                    error = %e,
                    "Failed to start server"
                );
            }
        }
    }

    /// Stop all running servers
    pub async fn stop_all(&self) {
        let servers = self.servers.read().await;
        let names: Vec<String> = servers.keys().cloned().collect();
        drop(servers);

        for name in names {
            if let Err(e) = self.stop(&name).await {
                error!(
                    server = %name,
                    error = %e,
                    "Failed to stop server"
                );
            }
        }
    }

    /// Spawn a server process
    fn spawn_server(config: &ServerConfig) -> Result<Child, std::io::Error> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        if let Some(ref dir) = config.working_dir {
            cmd.current_dir(dir);
        }

        cmd.spawn()
    }
}

impl Default for McpSupervisor {
    fn default() -> Self {
        Self::new()
    }
}
