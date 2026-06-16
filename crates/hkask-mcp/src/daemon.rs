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
///
/// REQ: MCP-016
/// post: returns PathBuf to the daemon socket (config dir or /tmp fallback)
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
    ///
    /// REQ: MCP-017
    /// post: returns DaemonClient with default socket path
    pub fn new() -> Self {
        Self {
            socket_path: daemon_socket_path(),
        }
    }

    /// Create a client with a custom socket path (for testing).
    ///
    /// REQ: MCP-018
    /// pre:  path is a valid filesystem path
    /// post: returns DaemonClient with custom socket path
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
    ///
    /// REQ: MCP-019
    /// post: returns DaemonListener with default socket path, listener=None
    pub fn new() -> Self {
        Self {
            socket_path: daemon_socket_path(),
            listener: None,
        }
    }

    /// Create a listener with a custom socket path (for testing).
    ///
    /// REQ: MCP-020
    /// pre:  path is a valid filesystem path
    /// post: returns DaemonListener with custom socket path
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

    // ── Protocol contract tests ────────────────────────────────────────────

    // REQ: daemon-contract-001 — all 4 request variants serialize to correct JSON shape
    #[test]
    fn request_variants_serialize_to_correct_shape() {
        // AuthQuery
        let req = DaemonRequest::AuthQuery {
            replicant: "alice".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "auth_query");
        assert_eq!(v["replicant"], "alice");
        assert!(
            v.as_object().unwrap().len() == 2,
            "AuthQuery should have 2 fields"
        );

        // AssignmentQuery
        let req = DaemonRequest::AssignmentQuery {
            replicant: "alice".into(),
            role: "research".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "assignment_query");
        assert_eq!(v["replicant"], "alice");
        assert_eq!(v["role"], "research");
        assert!(v.as_object().unwrap().len() == 3);

        // CapabilityQuery
        let req = DaemonRequest::CapabilityQuery {
            replicant: "alice".into(),
            tool: "web_search".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "capability_query");
        assert_eq!(v["replicant"], "alice");
        assert_eq!(v["tool"], "web_search");
        assert!(v.as_object().unwrap().len() == 3);

        // StoreExperience (with confidence)
        let req = DaemonRequest::StoreExperience {
            replicant: "alice".into(),
            entity: "session".into(),
            attribute: "observed".into(),
            value: serde_json::json!({"key": "val"}),
            confidence: Some(0.85),
        };
        let json = serde_json::to_string(&req).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "store_experience");
        assert_eq!(v["replicant"], "alice");
        assert_eq!(v["entity"], "session");
        assert_eq!(v["attribute"], "observed");
        assert_eq!(v["confidence"], 0.85);
    }

    // REQ: daemon-contract-002 — all 5 response variants serialize to correct JSON shape
    #[test]
    fn response_variants_serialize_to_correct_shape() {
        // AuthResponse (authenticated)
        let resp = DaemonResponse::AuthResponse {
            authenticated: true,
            webid: Some("alice-webid".into()),
            action: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "auth_response");
        assert_eq!(v["authenticated"], true);
        assert_eq!(v["webid"], "alice-webid");
        // action should be absent when None (skip_serializing_if)
        assert!(v.get("action").is_none());

        // AuthResponse (unauthenticated)
        let resp = DaemonResponse::AuthResponse {
            authenticated: false,
            webid: None,
            action: Some("prompt_user".into()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "auth_response");
        assert_eq!(v["authenticated"], false);
        assert!(v.get("webid").is_none());
        assert_eq!(v["action"], "prompt_user");

        // AssignmentResponse
        let resp = DaemonResponse::AssignmentResponse { assigned: true };
        let json = serde_json::to_string(&resp).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "assignment_response");
        assert_eq!(v["assigned"], true);

        // CapabilityResponse
        let resp = DaemonResponse::CapabilityResponse { granted: false };
        let json = serde_json::to_string(&resp).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "capability_response");
        assert_eq!(v["granted"], false);

        // ErrorResponse
        let resp = DaemonResponse::ErrorResponse {
            message: "something broke".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "error");
        assert_eq!(v["message"], "something broke");

        // StoreResponse
        let resp = DaemonResponse::StoreResponse {
            stored: true,
            episodic_id: Some("ep-001".into()),
            semantic_id: Some("sem-001".into()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "store_response");
        assert_eq!(v["stored"], true);
        assert_eq!(v["episodic_id"], "ep-001");
        assert_eq!(v["semantic_id"], "sem-001");
    }

    // REQ: daemon-contract-003 — forward compatibility: unknown fields tolerated
    #[test]
    fn forward_compat_unknown_fields_tolerated() {
        // A future client might send extra fields. The current daemon must not reject them.
        let json =
            r#"{"type":"auth_query","replicant":"alice","future_field":"v2_data","another":42}"#;
        let req: DaemonRequest =
            serde_json::from_str(json).expect("unknown fields should be tolerated");
        match req {
            DaemonRequest::AuthQuery { replicant } => assert_eq!(replicant, "alice"),
            _ => panic!("wrong variant"),
        }

        // Same for response — client should tolerate extra fields from future daemon
        let json = r#"{"type":"assignment_response","assigned":true,"future_meta":"ok"}"#;
        let resp: DaemonResponse =
            serde_json::from_str(json).expect("unknown fields in response should be tolerated");
        match resp {
            DaemonResponse::AssignmentResponse { assigned } => assert!(assigned),
            _ => panic!("wrong variant"),
        }
    }

    // REQ: daemon-contract-004 — backward compatibility: missing optional fields
    #[test]
    fn backward_compat_missing_optional_fields() {
        // StoreExperience without confidence (optional field)
        let json = r#"{"type":"store_experience","replicant":"alice","entity":"s","attribute":"a","value":{}}"#;
        let req: DaemonRequest =
            serde_json::from_str(json).expect("missing optional confidence should work");
        match req {
            DaemonRequest::StoreExperience { confidence, .. } => {
                assert!(confidence.is_none(), "missing confidence should be None");
            }
            _ => panic!("wrong variant"),
        }

        // AuthResponse without webid and action (both optional)
        let json = r#"{"type":"auth_response","authenticated":true}"#;
        let resp: DaemonResponse =
            serde_json::from_str(json).expect("missing optional fields in response should work");
        match resp {
            DaemonResponse::AuthResponse {
                authenticated,
                webid,
                action,
            } => {
                assert!(authenticated);
                assert!(webid.is_none());
                assert!(action.is_none());
            }
            _ => panic!("wrong variant"),
        }

        // StoreResponse without IDs
        let json = r#"{"type":"store_response","stored":false}"#;
        let resp: DaemonResponse =
            serde_json::from_str(json).expect("missing optional IDs should work");
        match resp {
            DaemonResponse::StoreResponse {
                stored,
                episodic_id,
                semantic_id,
            } => {
                assert!(!stored);
                assert!(episodic_id.is_none());
                assert!(semantic_id.is_none());
            }
            _ => panic!("wrong variant"),
        }
    }

    // REQ: daemon-contract-005 — failure mode: unknown type tag
    #[test]
    fn failure_unknown_type_tag() {
        let json = r#"{"type":"future_variant_v3","replicant":"alice"}"#;
        let result: Result<DaemonRequest, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "unknown type tag should fail deserialization"
        );
    }

    // REQ: daemon-contract-006 — failure mode: missing required field
    #[test]
    fn failure_missing_required_field() {
        // AuthQuery without replicant
        let json = r#"{"type":"auth_query"}"#;
        let result: Result<DaemonRequest, _> = serde_json::from_str(json);
        assert!(result.is_err(), "missing required field should fail");

        // AssignmentQuery without role
        let json = r#"{"type":"assignment_query","replicant":"alice"}"#;
        let result: Result<DaemonRequest, _> = serde_json::from_str(json);
        assert!(result.is_err(), "missing required role should fail");
    }

    // REQ: daemon-contract-007 — failure mode: malformed JSON
    #[test]
    fn failure_malformed_json() {
        let result: Result<DaemonRequest, _> = serde_json::from_str("not json at all");
        assert!(result.is_err(), "malformed JSON should fail");

        let result: Result<DaemonRequest, _> = serde_json::from_str("{\"type\":\"auth_query\",");
        assert!(result.is_err(), "truncated JSON should fail");
    }

    // REQ: daemon-contract-008 — failure mode: wrong type for field
    #[test]
    fn failure_wrong_field_type() {
        // replicant should be string, not number
        let json = r#"{"type":"auth_query","replicant":42}"#;
        let result: Result<DaemonRequest, _> = serde_json::from_str(json);
        assert!(result.is_err(), "wrong field type should fail");

        // authenticated should be bool, not string
        let json = r#"{"type":"auth_response","authenticated":"yes"}"#;
        let result: Result<DaemonResponse, _> = serde_json::from_str(json);
        assert!(result.is_err(), "wrong field type in response should fail");
    }

    // ── Idempotency tests ───────────────────────────────────────────────
    //
    // Daemon query idempotency contract (PR 2.5.1):
    //
    // | Operation           | Idempotent? | Reason                    |
    // |---------------------|:-----------:|---------------------------|
    // | auth_query          | ✅          | Read-only (UserStore)     |
    // | assignment_query    | ✅          | Read-only (PodManager)    |
    // | capability_query    | ✅          | Read-only (PodManager)    |
    // | store_experience    | ❌          | Creates new UUID triples  |
    //
    // The three query operations are naturally idempotent — they perform
    // no mutations. store_experience is documented as non-idempotent
    // (each call generates new TripleIDs).

    // REQ: daemon-idem-001 — repeated auth_query returns consistent result
    #[tokio::test]
    async fn daemon_auth_query_is_idempotent() {
        let (listener, path) = setup_test_listener().await;
        let handler = Arc::new(MockHandler {
            authenticated: AtomicBool::new(true),
        });

        tokio::spawn(async move {
            let _ = listener.serve(handler).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = DaemonClient::with_path(path);

        // First call
        let r1 = client.auth_query("bob").await.expect("first auth query");
        // Second call — must return same result
        let r2 = client.auth_query("bob").await.expect("second auth query");

        // Both should be identical AuthResponse with same fields
        match (&r1, &r2) {
            (
                DaemonResponse::AuthResponse {
                    authenticated: a1,
                    webid: w1,
                    action: ac1,
                },
                DaemonResponse::AuthResponse {
                    authenticated: a2,
                    webid: w2,
                    action: ac2,
                },
            ) => {
                assert_eq!(a1, a2, "authenticated must be consistent");
                assert_eq!(w1, w2, "webid must be consistent");
                assert_eq!(ac1, ac2, "action must be consistent");
            }
            _ => panic!("expected AuthResponse from both calls"),
        }
    }

    // REQ: daemon-idem-002 — repeated assignment_query returns consistent result
    #[tokio::test]
    async fn daemon_assignment_query_is_idempotent() {
        let (listener, path) = setup_test_listener().await;
        let handler = Arc::new(MockHandler {
            authenticated: AtomicBool::new(false),
        });

        tokio::spawn(async move {
            let _ = listener.serve(handler).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = DaemonClient::with_path(path);

        let r1 = client
            .assignment_query("bob", "research")
            .await
            .expect("first assignment query");
        let r2 = client
            .assignment_query("bob", "research")
            .await
            .expect("second assignment query");

        match (&r1, &r2) {
            (
                DaemonResponse::AssignmentResponse { assigned: a1 },
                DaemonResponse::AssignmentResponse { assigned: a2 },
            ) => {
                assert_eq!(a1, a2, "assignment result must be idempotent");
            }
            _ => panic!("expected AssignmentResponse from both calls"),
        }
    }

    // REQ: daemon-idem-003 — repeated capability_query returns consistent result
    #[tokio::test]
    async fn daemon_capability_query_is_idempotent() {
        let (listener, path) = setup_test_listener().await;
        let handler = Arc::new(MockHandler {
            authenticated: AtomicBool::new(false),
        });

        tokio::spawn(async move {
            let _ = listener.serve(handler).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = DaemonClient::with_path(path);

        let r1 = client
            .capability_query("bob", "web_search")
            .await
            .expect("first capability query");
        let r2 = client
            .capability_query("bob", "web_search")
            .await
            .expect("second capability query");

        match (&r1, &r2) {
            (
                DaemonResponse::CapabilityResponse { granted: g1 },
                DaemonResponse::CapabilityResponse { granted: g2 },
            ) => {
                assert_eq!(g1, g2, "capability result must be idempotent");
            }
            _ => panic!("expected CapabilityResponse from both calls"),
        }
    }
}
