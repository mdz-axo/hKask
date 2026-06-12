//! Daemon socket — Unix domain socket transport for MCP binary ↔ hKask communication.
//!
//! MCP server binaries connect to the hKask daemon over a Unix domain socket at
//! `~/.config/hkask/daemon.sock` to authenticate, verify role assignments, and
//! check capability tokens. The protocol is newline-delimited JSON.
//!
//! # Protocol
//!
//! Request (MCP binary → daemon):
//! ```json
//! {"type":"auth_query","replicant":"bob"}
//! {"type":"assignment_query","replicant":"bob","role":"research"}
//! {"type":"capability_query","replicant":"bob","tool":"web_search"}
//! ```
//!
//! Response (daemon → MCP binary):
//! ```json
//! {"type":"auth_response","authenticated":true,"webid":"bob-xxxx"}
//! {"type":"auth_response","authenticated":false,"action":"prompt_user"}
//! {"type":"assignment_response","assigned":true}
//! {"type":"capability_response","granted":true}
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

/// Well-known path for the hKask daemon socket.
pub fn daemon_socket_path() -> PathBuf {
    let base = dirs_next().unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("daemon.sock")
}

fn dirs_next() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("hkask"))
}

// ── Protocol Types ──

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonRequest {
    #[serde(rename = "auth_query")]
    AuthQuery { replicant: String },
    #[serde(rename = "assignment_query")]
    AssignmentQuery { replicant: String, role: String },
    #[serde(rename = "capability_query")]
    CapabilityQuery { replicant: String, tool: String },
    /// Store an experience in both episodic (first-person) and semantic (third-person) memory.
    /// Each experience generates two triples from the same event:
    /// - Episodic: specific, time-bound, perspective-scoped, private
    /// - Semantic: generalizable, timeless, no perspective, public
    #[serde(rename = "store_experience")]
    StoreExperience {
        replicant: String,
        entity: String,
        attribute: String,
        value: serde_json::Value,
        confidence: Option<f64>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonResponse {
    #[serde(rename = "auth_response")]
    AuthResponse {
        authenticated: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        webid: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<String>,
    },
    #[serde(rename = "assignment_response")]
    AssignmentResponse { assigned: bool },
    #[serde(rename = "capability_response")]
    CapabilityResponse { granted: bool },
    #[serde(rename = "error")]
    ErrorResponse { message: String },
    #[serde(rename = "store_response")]
    StoreResponse {
        stored: bool,
        episodic_id: Option<String>,
        semantic_id: Option<String>,
    },
}

// ── DaemonClient (used by MCP binaries) ──

/// Client for connecting to the hKask daemon over a Unix domain socket.
///
/// Used by MCP server binaries to authenticate, verify role assignments,
/// and check capability tokens before starting the MCP server.
#[derive(Clone)]
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    /// Create a client that connects to the default daemon socket path.
    pub fn new() -> Self {
        Self {
            socket_path: daemon_socket_path(),
        }
    }

    /// Create a client with a custom socket path (for testing).
    pub fn with_path(path: PathBuf) -> Self {
        Self { socket_path: path }
    }

    /// Send a request and receive a response.
    async fn send_recv(&self, request: &DaemonRequest) -> std::io::Result<DaemonResponse> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        let (reader, mut writer) = stream.into_split();

        let mut json = serde_json::to_string(request)?;
        json.push('\n');
        writer.write_all(json.as_bytes()).await?;
        writer.shutdown().await?;

        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();
        buf_reader.read_line(&mut line).await?;

        serde_json::from_str(&line)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }

    /// Query whether a replicant is authenticated.
    pub async fn auth_query(&self, replicant: &str) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::AuthQuery {
            replicant: replicant.to_string(),
        })
        .await
    }

    /// Query whether a replicant is assigned to a specific MCP role.
    pub async fn assignment_query(
        &self,
        replicant: &str,
        role: &str,
    ) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::AssignmentQuery {
            replicant: replicant.to_string(),
            role: role.to_string(),
        })
        .await
    }

    /// Query whether a replicant holds a capability token for a tool.
    pub async fn capability_query(
        &self,
        replicant: &str,
        tool: &str,
    ) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::CapabilityQuery {
            replicant: replicant.to_string(),
            tool: tool.to_string(),
        })
        .await
    }

    /// Store an experience in both episodic and semantic memory.
    pub async fn store_experience(
        &self,
        replicant: &str,
        entity: &str,
        attribute: &str,
        value: &serde_json::Value,
        confidence: Option<f64>,
    ) -> std::io::Result<DaemonResponse> {
        self.send_recv(&DaemonRequest::StoreExperience {
            replicant: replicant.to_string(),
            entity: entity.to_string(),
            attribute: attribute.to_string(),
            value: value.clone(),
            confidence,
        })
        .await
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new()
    }
}

// ── DaemonListener (runs inside hKask) ──

/// Handler trait for daemon queries.
///
/// Implemented by the hKask runtime to provide authentication,
/// assignment verification, capability checking, and dual memory encoding.
#[async_trait::async_trait]
pub trait DaemonHandler: Send + Sync {
    /// Check if a replicant is authenticated. Returns (authenticated, webid).
    async fn check_auth(&self, replicant: &str) -> (bool, Option<String>);

    /// Check if a replicant is assigned to a role.
    async fn check_assignment(&self, replicant: &str, role: &str) -> bool;

    /// Check if a replicant holds a capability token for a tool.
    async fn check_capability(&self, replicant: &str, tool: &str) -> bool;

    /// Store an experience in both episodic and semantic memory.
    /// Returns (stored, episodic_triple_id, semantic_triple_id).
    async fn store_experience(
        &self,
        replicant: &str,
        entity: &str,
        attribute: &str,
        value: &serde_json::Value,
        confidence: Option<f64>,
    ) -> (bool, Option<String>, Option<String>);
}

/// Unix domain socket listener for the hKask daemon.
///
/// Binds to `~/.config/hkask/daemon.sock` and handles JSON-RPC-style
/// queries from MCP server binaries.
pub struct DaemonListener {
    socket_path: PathBuf,
    listener: Option<UnixListener>,
}

impl Default for DaemonListener {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonListener {
    /// Create a listener bound to the default socket path.
    pub fn new() -> Self {
        Self {
            socket_path: daemon_socket_path(),
            listener: None,
        }
    }

    /// Create a listener with a custom socket path (for testing).
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            socket_path: path,
            listener: None,
        }
    }

    /// Bind the socket and start listening.
    pub async fn bind(&mut self) -> std::io::Result<()> {
        // Remove stale socket file if present
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }
        // Ensure parent directory exists
        if let Some(parent) = self.socket_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let listener = UnixListener::bind(&self.socket_path)?;
        self.listener = Some(listener);
        tracing::info!(
            target: "hkask.daemon",
            path = %self.socket_path.display(),
            "Daemon socket bound"
        );
        Ok(())
    }

    /// Accept connections and handle requests in a loop.
    ///
    /// Runs until the listener is closed or an error occurs.
    pub async fn serve(&self, handler: Arc<dyn DaemonHandler>) -> std::io::Result<()> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotConnected, "Not bound"))?;

        loop {
            let (stream, _addr) = listener.accept().await?;
            let handler = Arc::clone(&handler);
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, handler.as_ref()).await {
                    tracing::warn!(
                        target: "hkask.daemon",
                        error = %e,
                        "Daemon connection error"
                    );
                }
            });
        }
    }
}

async fn handle_connection(stream: UnixStream, handler: &dyn DaemonHandler) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    buf_reader.read_line(&mut line).await?;

    let request: DaemonRequest = serde_json::from_str(&line)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    let response = match request {
        DaemonRequest::AuthQuery { replicant } => {
            let (authenticated, webid) = handler.check_auth(&replicant).await;
            DaemonResponse::AuthResponse {
                authenticated,
                webid,
                action: if authenticated {
                    None
                } else {
                    Some("prompt_user".to_string())
                },
            }
        }
        DaemonRequest::AssignmentQuery { replicant, role } => {
            let assigned = handler.check_assignment(&replicant, &role).await;
            DaemonResponse::AssignmentResponse { assigned }
        }
        DaemonRequest::CapabilityQuery { replicant, tool } => {
            let granted = handler.check_capability(&replicant, &tool).await;
            DaemonResponse::CapabilityResponse { granted }
        }
        DaemonRequest::StoreExperience {
            replicant,
            entity,
            attribute,
            value,
            confidence,
        } => {
            let (stored, episodic_id, semantic_id) = handler
                .store_experience(&replicant, &entity, &attribute, &value, confidence)
                .await;
            DaemonResponse::StoreResponse {
                stored,
                episodic_id,
                semantic_id,
            }
        }
    };

    let mut json = serde_json::to_string(&response)?;
    json.push('\n');
    writer.write_all(json.as_bytes()).await?;
    writer.shutdown().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockHandler {
        authenticated: AtomicBool,
    }

    #[async_trait::async_trait]
    impl DaemonHandler for MockHandler {
        async fn check_auth(&self, replicant: &str) -> (bool, Option<String>) {
            let auth = self.authenticated.load(Ordering::SeqCst);
            (
                auth,
                if auth {
                    Some(format!("{}-webid", replicant))
                } else {
                    None
                },
            )
        }

        async fn check_assignment(&self, _replicant: &str, role: &str) -> bool {
            role == "research"
        }

        async fn check_capability(&self, _replicant: &str, tool: &str) -> bool {
            tool == "web_search"
        }

        async fn store_experience(
            &self,
            replicant: &str,
            _entity: &str,
            _attribute: &str,
            _value: &serde_json::Value,
            _confidence: Option<f64>,
        ) -> (bool, Option<String>, Option<String>) {
            let auth = self.authenticated.load(Ordering::SeqCst);
            if auth {
                (
                    true,
                    Some(format!("episodic-{}-001", replicant)),
                    Some(format!("semantic-{}-001", replicant)),
                )
            } else {
                (false, None, None)
            }
        }
    }

    async fn setup_test_listener() -> (DaemonListener, PathBuf) {
        use std::sync::atomic::AtomicU64;
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "hkask-daemon-test-{}-{}.sock",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_file(&path);
        let mut listener = DaemonListener::with_path(path.clone());
        listener.bind().await.expect("bind test socket");
        (listener, path)
    }

    #[tokio::test]
    async fn daemon_auth_query_authenticated() {
        let (listener, path) = setup_test_listener().await;
        let handler = Arc::new(MockHandler {
            authenticated: AtomicBool::new(true),
        });

        // Spawn listener in background
        let serve_handler = Arc::clone(&handler);
        tokio::spawn(async move {
            let _ = listener.serve(serve_handler).await;
        });

        // Give listener a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = DaemonClient::with_path(path);
        let response = client.auth_query("bob").await.expect("auth query");

        match response {
            DaemonResponse::AuthResponse {
                authenticated,
                webid,
                action,
            } => {
                assert!(authenticated);
                assert_eq!(webid, Some("bob-webid".to_string()));
                assert!(action.is_none());
            }
            _ => panic!("Expected AuthResponse, got {:?}", response),
        }
    }

    #[tokio::test]
    async fn daemon_auth_query_unauthenticated() {
        let (listener, path) = setup_test_listener().await;
        let handler = Arc::new(MockHandler {
            authenticated: AtomicBool::new(false),
        });

        tokio::spawn(async move {
            let _ = listener.serve(handler).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = DaemonClient::with_path(path);
        let response = client.auth_query("bob").await.expect("auth query");

        match response {
            DaemonResponse::AuthResponse {
                authenticated,
                webid,
                action,
            } => {
                assert!(!authenticated);
                assert!(webid.is_none());
                assert_eq!(action, Some("prompt_user".to_string()));
            }
            _ => panic!("Expected AuthResponse"),
        }
    }

    #[tokio::test]
    async fn daemon_assignment_query() {
        let (listener, path) = setup_test_listener().await;
        let handler = Arc::new(MockHandler {
            authenticated: AtomicBool::new(false),
        });

        tokio::spawn(async move {
            let _ = listener.serve(handler).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = DaemonClient::with_path(path);

        // Assigned role
        let response = client
            .assignment_query("bob", "research")
            .await
            .expect("assignment query");
        match response {
            DaemonResponse::AssignmentResponse { assigned } => assert!(assigned),
            _ => panic!("Expected AssignmentResponse"),
        }

        // Unassigned role
        let response = client
            .assignment_query("bob", "condenser")
            .await
            .expect("assignment query");
        match response {
            DaemonResponse::AssignmentResponse { assigned } => assert!(!assigned),
            _ => panic!("Expected AssignmentResponse"),
        }
    }

    #[tokio::test]
    async fn daemon_capability_query() {
        let (listener, path) = setup_test_listener().await;
        let handler = Arc::new(MockHandler {
            authenticated: AtomicBool::new(false),
        });

        tokio::spawn(async move {
            let _ = listener.serve(handler).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = DaemonClient::with_path(path);

        // Granted capability
        let response = client
            .capability_query("bob", "web_search")
            .await
            .expect("capability query");
        match response {
            DaemonResponse::CapabilityResponse { granted } => assert!(granted),
            _ => panic!("Expected CapabilityResponse"),
        }

        // Denied capability
        let response = client
            .capability_query("bob", "admin_panel")
            .await
            .expect("capability query");
        match response {
            DaemonResponse::CapabilityResponse { granted } => assert!(!granted),
            _ => panic!("Expected CapabilityResponse"),
        }
    }

    #[tokio::test]
    async fn daemon_store_experience_dual_encoding() {
        let (listener, path) = setup_test_listener().await;
        let handler = Arc::new(MockHandler {
            authenticated: AtomicBool::new(true),
        });

        tokio::spawn(async move {
            let _ = listener.serve(handler).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = DaemonClient::with_path(path);
        let value = serde_json::json!({
            "tool": "web_search",
            "query": "Rust async patterns",
            "results": 3,
            "timestamp": "2026-06-11T14:32:00Z"
        });

        let response = client
            .store_experience("bob", "session", "observed", &value, Some(0.9))
            .await
            .expect("store experience");

        match response {
            DaemonResponse::StoreResponse {
                stored,
                episodic_id,
                semantic_id,
            } => {
                assert!(stored);
                assert_eq!(episodic_id, Some("episodic-bob-001".to_string()));
                assert_eq!(semantic_id, Some("semantic-bob-001".to_string()));
            }
            _ => panic!("Expected StoreResponse, got {:?}", response),
        }
    }
}
