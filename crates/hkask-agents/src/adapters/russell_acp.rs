//! Russell ACP Adapter — Bidirectional ACP bridge to Russell
//!
//! Implements AcpPort by communicating with Russell's ACP server
//! over stdio JSON-RPC.
//!
//! # Protocol Compatibility
//!
//! This adapter speaks Russell's JSON-RPC 2.0 protocol:
//! - `acp/session.create` — Creates a session, returns `session_id`
//! - `acp/session.message` — Sends a message to an existing session
//! - `acp/session.close` — Closes a session
//! - `acp/capabilities` — Lists available skills and probes
//!
//! Session IDs are tracked per-WebID so that subsequent messages
//! are routed to the correct Russell session.

use async_trait::async_trait;
use hkask_types::derivation_contexts;
use hkask_types::secret::SecretRef;
use hkask_types::{DelegationAction, DelegationResource, DelegationToken, WebID};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};
use zeroize::Zeroizing;

use crate::acp::{A2AMessage, AcpError};
use crate::ports::AcpPort;

/// Russell JSON-RPC request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<AuthInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    acp_version: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuthInfo {
    auth_type: String,
    token: String,
}

/// Russell JSON-RPC response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

/// Russell session creation response
#[derive(Debug, Deserialize)]
struct CreateSessionResponse {
    session_id: String,
}

/// Russell capabilities response
#[derive(Debug, Deserialize)]
struct CapabilitiesResponse {
    skills: Vec<SkillInfo>,
    probes: Vec<ProbeInfo>,
}

/// Russell skill info (public metadata)
#[derive(Debug, Deserialize)]
struct SkillInfo {
    id: String,
    version: String,
    symptoms: Vec<String>,
}

/// Russell probe info
#[derive(Debug, Deserialize)]
struct ProbeInfo {
    id: String,
}

/// Manages the lifecycle of a Russell subprocess.
///
/// Owns the child process and provides methods to start, communicate with,
/// and shut down the Russell ACP server.
pub struct RussellProcessManager {
    child: Option<Child>,
    binary_path: String,
}

impl RussellProcessManager {
    /// Create a new process manager for the given Russell binary.
    pub fn new(binary_path: String) -> Self {
        Self {
            child: None,
            binary_path,
        }
    }

    /// Ensure the Russell process is running, spawning it if needed.
    async fn ensure_started(&mut self) -> Result<(), AcpError> {
        if self.child.is_none() {
            let child = Command::new(&self.binary_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| AcpError::TransportError(format!("Failed to spawn Russell: {}", e)))?;
            self.child = Some(child);
            info!("Russell ACP adapter started");
        }
        Ok(())
    }

    /// Send a JSON-RPC request and receive the response.
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse, AcpError> {
        self.ensure_started().await?;

        let request_id = request.id.clone();

        let child = self
            .child
            .as_mut()
            .ok_or_else(|| AcpError::TransportError("Russell not started".to_string()))?;

        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| AcpError::TransportError("Russell stdin not available".to_string()))?;

        let json = serde_json::to_string(&request)
            .map_err(|e| AcpError::TransportError(format!("Serialization failed: {}", e)))?;

        stdin
            .write_all(json.as_bytes())
            .await
            .map_err(|e| AcpError::TransportError(format!("Write failed: {}", e)))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| AcpError::TransportError(format!("Write failed: {}", e)))?;
        stdin
            .flush()
            .await
            .map_err(|e| AcpError::TransportError(format!("Flush failed: {}", e)))?;

        let stdout = child
            .stdout
            .as_mut()
            .ok_or_else(|| AcpError::TransportError("Russell stdout not available".to_string()))?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| AcpError::TransportError(format!("Read failed: {}", e)))?;

        let response: JsonRpcResponse = serde_json::from_str(line.trim())
            .map_err(|e| AcpError::TransportError(format!("Parse failed: {}", e)))?;

        if response.jsonrpc != "2.0" {
            warn!(target: "hkask.russell", "Unexpected JSON-RPC version: {}", response.jsonrpc);
        }

        if response.id != request_id {
            warn!(target: "hkask.russell", "Response ID mismatch: expected {:?}, got {:?}", request_id, response.id);
        }

        Ok(response)
    }

    /// Shut down the Russell process gracefully.
    ///
    /// Idempotent: calling on a manager that has no live child is a no-op
    /// (returns `Ok(())`). Safe to call from `Drop`-like cleanup paths.
    pub async fn shutdown(&mut self) -> Result<(), AcpError> {
        if let Some(mut child) = self.child.take() {
            child.kill().await.map_err(|e| {
                AcpError::TransportError(format!("Failed to shut down Russell: {}", e))
            })?;
            info!("Russell process shut down");
        }
        Ok(())
    }
}

/// Russell ACP adapter — bridges hKask's AcpPort to Russell over stdio JSON-RPC
///
/// Delegates process lifecycle to `RussellProcessManager` and translates
/// between hKask's AcpPort trait and Russell's JSON-RPC protocol.
pub struct RussellAcpAdapter {
    process: Mutex<RussellProcessManager>,
    macaroon_token: Option<String>,
    /// WebID → Russell session_id mapping
    sessions: Arc<RwLock<HashMap<WebID, String>>>,
    /// Bridge secret derived from master key via HKDF-SHA256 (ADR-027)
    bridge_secret: Arc<Zeroizing<Vec<u8>>>,
}

impl RussellAcpAdapter {
    /// Create new Russell ACP adapter.
    ///
    /// The bridge secret is derived from the master key via
    /// HKDF-SHA256 with context `"hkask:russell-bridge-secret"`.
    /// Both hKask and Russell must share the same master passphrase
    /// and derivation context to produce matching HMAC signing keys.
    ///
    /// # Arguments
    /// * `russell_binary` — Path to the Russell ACP server binary
    pub fn new(russell_binary: String) -> Result<Self, AcpError> {
        let bridge_secret = hkask_keystore::resolve(&SecretRef::derived(
            derivation_contexts::MASTER_KEY_ENV,
            derivation_contexts::RUSSELL_BRIDGE_SECRET,
        ))
        .map_err(|e| AcpError::KeyDerivation(e.into()))?;

        let process = RussellProcessManager::new(russell_binary);

        Ok(Self {
            process: Mutex::new(process),
            macaroon_token: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            bridge_secret: Arc::new(bridge_secret),
        })
    }

    /// Set macaroon token for authentication
    pub fn with_auth(mut self, token: String) -> Self {
        self.macaroon_token = Some(token);
        self
    }

    /// Shut down the underlying Russell process.
    ///
    /// P3.3: delegates to `RussellProcessManager::shutdown` so the adapter
    /// exposes a single, documented shutdown entry point. Idempotent.
    pub async fn shutdown(&self) -> Result<(), AcpError> {
        let mut process = self.process.lock().await;
        process.shutdown().await
    }

    /// Delegate request sending to `RussellProcessManager`.
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, AcpError> {
        let mut process = self.process.lock().await;
        process.send_request(request).await
    }

    /// Look up the Russell session_id for a given WebID
    async fn get_session_id(&self, webid: &WebID) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions.get(webid).cloned()
    }

    /// Store a Russell session_id for a given WebID
    async fn store_session_id(&self, webid: WebID, session_id: String) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(webid, session_id);
    }

    /// Remove the Russell session_id for a given WebID
    async fn remove_session_id(&self, webid: &WebID) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(webid);
    }

    /// Extract a human-readable message string from an A2AMessage
    fn extract_message_text(msg: &A2AMessage) -> String {
        match msg {
            A2AMessage::TemplateDispatch {
                template_id,
                input,
                correlation_id,
                ..
            } => serde_json::json!({
                "type": "template_dispatch",
                "template_id": template_id,
                "input": input,
                "correlation_id": correlation_id,
            })
            .to_string(),
            A2AMessage::TemplateResponse {
                correlation_id,
                result,
                error,
            } => serde_json::json!({
                "type": "template_response",
                "correlation_id": correlation_id,
                "result": result,
                "error": error,
            })
            .to_string(),
            A2AMessage::MemoryArtifact {
                producer,
                artifact_type,
                artifact_id,
                visibility,
            } => serde_json::json!({
                "type": "memory_artifact",
                "producer": producer.to_string(),
                "artifact_type": artifact_type,
                "artifact_id": artifact_id,
                "visibility": visibility,
            })
            .to_string(),
        }
    }
}

#[async_trait]
impl AcpPort for RussellAcpAdapter {
    async fn register_agent(
        &self,
        webid: WebID,
        agent_type: hkask_types::AgentKind,
        capabilities: Vec<String>,
    ) -> Result<DelegationToken, AcpError> {
        let persona = agent_type.as_russell_persona();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::String(uuid::Uuid::new_v4().to_string()),
            method: "acp/session.create".to_string(),
            params: Some(serde_json::json!({
                "persona": persona,
                "capabilities": capabilities,
            })),
            auth: self.macaroon_token.as_ref().map(|t| AuthInfo {
                auth_type: "macaroon".to_string(),
                token: t.clone(),
            }),
            acp_version: Some("0.1.0".to_string()),
        };

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            let detail = error
                .data
                .as_ref()
                .map(|d| format!(": {}", d))
                .unwrap_or_default();
            return Err(AcpError::TransportError(format!(
                "Russell error {}: {}{}",
                error.code, error.message, detail
            )));
        }

        // Parse the session_id from Russell's response
        let result = response
            .result
            .ok_or_else(|| AcpError::TransportError("Russell returned no result".to_string()))?;

        let session_resp: CreateSessionResponse = serde_json::from_value(result).map_err(|e| {
            AcpError::TransportError(format!("Failed to parse Russell session response: {}", e))
        })?;

        // Store the session_id for this WebID
        self.store_session_id(webid, session_resp.session_id.clone())
            .await;

        // Mint a bridge delegation token signed with the shared secret
        // so it passes AcpRuntime::verify_capability
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "russell:session".to_string(),
            DelegationAction::Execute,
            WebID::from_persona_with_namespace(b"russell-bridge", "russell"),
            webid,
            self.bridge_secret.as_ref(),
        );

        Ok(token)
    }

    async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError> {
        // Look up the session_id for this WebID
        let session_id = self
            .get_session_id(webid)
            .await
            .ok_or(AcpError::AgentNotFound(*webid))?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::String(uuid::Uuid::new_v4().to_string()),
            method: "acp/session.close".to_string(),
            params: Some(serde_json::json!({
                "session_id": session_id,
            })),
            auth: self.macaroon_token.as_ref().map(|t| AuthInfo {
                auth_type: "macaroon".to_string(),
                token: t.clone(),
            }),
            acp_version: Some("0.1.0".to_string()),
        };

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            let detail = error
                .data
                .as_ref()
                .map(|d| format!(": {}", d))
                .unwrap_or_default();
            warn!(
                webid = %webid,
                code = error.code,
                message = %error.message,
                detail = %detail,
                "Russell session close returned error"
            );
            // Still remove the session mapping even if close failed
        }

        // Remove the session mapping regardless of close result
        self.remove_session_id(webid).await;

        Ok(())
    }

    async fn send_message(&self, msg: A2AMessage) -> Result<String, AcpError> {
        // Determine the target WebID from the message
        let target_webid = match &msg {
            A2AMessage::TemplateDispatch { to, from, .. } => to.unwrap_or(*from),
            A2AMessage::TemplateResponse { .. } => {
                // Responses don't have a clear target; use first available session
                let sessions = self.sessions.read().await;
                sessions.keys().next().copied().ok_or_else(|| {
                    AcpError::TransportError("No active Russell sessions".to_string())
                })?
            }
            A2AMessage::MemoryArtifact { producer, .. } => *producer,
        };

        // Look up the session_id for the target WebID
        let session_id = self.get_session_id(&target_webid).await.ok_or_else(|| {
            AcpError::TransportError(format!("No Russell session for WebID {}", target_webid))
        })?;

        // Extract message text and wrap in Russell's expected format
        let message_text = Self::extract_message_text(&msg);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::String(uuid::Uuid::new_v4().to_string()),
            method: "acp/session.message".to_string(),
            params: Some(serde_json::json!({
                "session_id": session_id,
                "message": message_text,
            })),
            auth: self.macaroon_token.as_ref().map(|t| AuthInfo {
                auth_type: "macaroon".to_string(),
                token: t.clone(),
            }),
            acp_version: Some("0.1.0".to_string()),
        };

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            let detail = error
                .data
                .as_ref()
                .map(|d| format!(": {}", d))
                .unwrap_or_default();
            return Err(AcpError::TransportError(format!(
                "Russell error {}: {}{}",
                error.code, error.message, detail
            )));
        }

        // Return the correlation ID from the original message
        let correlation_id = msg.correlation_id().to_string();

        Ok(correlation_id)
    }

    async fn list_capabilities(&self, _webid: &WebID) -> Result<Vec<String>, AcpError> {
        // Call Russell's acp/capabilities to get available skills
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::String(uuid::Uuid::new_v4().to_string()),
            method: "acp/capabilities".to_string(),
            params: None,
            auth: self.macaroon_token.as_ref().map(|t| AuthInfo {
                auth_type: "macaroon".to_string(),
                token: t.clone(),
            }),
            acp_version: Some("0.1.0".to_string()),
        };

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            let detail = error
                .data
                .as_ref()
                .map(|d| format!(": {}", d))
                .unwrap_or_default();
            return Err(AcpError::TransportError(format!(
                "Russell error {}: {}{}",
                error.code, error.message, detail
            )));
        }

        let result = response
            .result
            .ok_or_else(|| AcpError::TransportError("Russell returned no result".to_string()))?;

        let caps_resp: CapabilitiesResponse = serde_json::from_value(result).map_err(|e| {
            AcpError::TransportError(format!("Failed to parse Russell capabilities: {}", e))
        })?;

        // Map Russell skills to hKask capability strings
        // Each skill's symptoms become "russell:<skill_id>" capabilities
        let mut capabilities: Vec<String> = caps_resp
            .skills
            .iter()
            .flat_map(|skill| {
                let mut caps = vec![format!("russell:{}@{}", skill.id, skill.version)];
                // Also add symptom-based capabilities for hLexicon mapping
                for symptom in &skill.symptoms {
                    caps.push(format!("russell:symptom:{}", symptom));
                }
                caps
            })
            .collect();

        // Add probe capabilities
        for probe in &caps_resp.probes {
            capabilities.push(format!("russell:probe:{}", probe.id));
        }

        // Deduplicate
        capabilities.sort();
        capabilities.dedup();

        Ok(capabilities)
    }

    async fn is_registered(&self, webid: &WebID) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(webid)
    }

    async fn revoke_capability(&self, token_id: &str, _holder: &WebID) -> Result<(), AcpError> {
        tracing::info!(
            target: "hkask.russell",
            token_id = %token_id,
            "Token revocation requested (Russell does not support granular revocation)"
        );
        Ok(())
    }

    async fn get_capabilities(&self, webid: &WebID) -> Vec<DelegationToken> {
        if self.is_registered(webid).await {
            vec![DelegationToken::new(
                DelegationResource::Tool,
                "russell:session".to_string(),
                DelegationAction::Execute,
                WebID::from_persona_with_namespace(b"russell-bridge", "russell"),
                *webid,
                self.bridge_secret.as_ref(),
            )]
        } else {
            vec![]
        }
    }

    async fn list_agents(&self) -> Vec<crate::acp::AcpAgent> {
        let sessions = self.sessions.read().await;
        sessions
            .keys()
            .map(|webid| crate::acp::AcpAgent {
                webid: *webid,
                agent_type: hkask_types::AgentKind::Bot,
                capabilities: vec![],
                registered_at: 0,
                active: true,
            })
            .collect()
    }
}

// ── P3.3 property tests ─────────────────────────────────────────────────────
//
// Every test below verifies a stated behavioral property of a public seam
// introduced by the RussellProcessManager extraction. No test depends on
// spawning a real `russell-acp-server` binary; all assertions run against
// the lazy-spawn, idempotent-shutdown contract.
