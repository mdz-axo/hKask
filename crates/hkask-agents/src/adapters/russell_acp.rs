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
use hkask_types::{CapabilityAction, CapabilityResource, CapabilityToken, WebID};
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
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: Value,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    probes: Vec<ProbeInfo>,
}

/// Russell skill info (public metadata)
#[derive(Debug, Deserialize)]
struct SkillInfo {
    id: String,
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    description: String,
    symptoms: Vec<String>,
}

/// Russell probe info
#[derive(Debug, Deserialize)]
struct ProbeInfo {
    id: String,
    #[allow(dead_code)]
    description: String,
}

/// Russell ACP adapter — spawns Russell as child process
///
/// Manages session lifecycle and translates between hKask's AcpPort
/// and Russell's JSON-RPC protocol.
pub struct RussellAcpAdapter {
    child: Mutex<Option<Child>>,
    russell_binary: String,
    macaroon_token: Option<String>,
    cns_emitter: Option<Arc<dyn hkask_cns::CnsEmit + Send + Sync>>,
    /// WebID → Russell session_id mapping
    sessions: Arc<RwLock<HashMap<WebID, String>>>,
    /// Shared secret for signing bridge capability tokens
    bridge_secret: Arc<Zeroizing<Vec<u8>>>,
}

impl RussellAcpAdapter {
    /// Create new Russell ACP adapter
    ///
    /// # Arguments
    /// * `russell_binary` — Path to the Russell ACP server binary
    /// * `bridge_secret` — Shared HMAC secret for signing bridge capability tokens.
    ///   This should be the same secret used by `AcpRuntime` so that tokens
    ///   minted by the bridge are verifiable by the local runtime.
    pub fn new(russell_binary: String, bridge_secret: Arc<Zeroizing<Vec<u8>>>) -> Self {
        Self {
            child: Mutex::new(None),
            russell_binary,
            macaroon_token: None,
            cns_emitter: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            bridge_secret,
        }
    }

    /// Set macaroon token for authentication
    pub fn with_auth(mut self, token: String) -> Self {
        self.macaroon_token = Some(token);
        self
    }

    /// Set CNS emitter for observability
    pub fn with_cns_emitter(mut self, emitter: Arc<dyn hkask_cns::CnsEmit + Send + Sync>) -> Self {
        self.cns_emitter = Some(emitter);
        self
    }

    async fn ensure_started(&self) -> Result<(), AcpError> {
        let mut child_opt = self.child.lock().await;
        if child_opt.is_none() {
            let child = Command::new(&self.russell_binary)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| AcpError::TransportError(format!("Failed to spawn Russell: {}", e)))?;
            *child_opt = Some(child);
            info!("Russell ACP adapter started");
        }
        Ok(())
    }

    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, AcpError> {
        self.ensure_started().await?;

        let mut child_opt = self.child.lock().await;
        let child = child_opt
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

        Ok(response)
    }

    fn emit_cns_span(&self, span_name: &str, data: Value) {
        if let Some(ref cns) = self.cns_emitter {
            cns.emit_event(span_name, "federation", &data, 1.0);
        }
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
        _agent_type: &str,
        _capabilities: Vec<String>,
    ) -> Result<CapabilityToken, AcpError> {
        // Russell's CreateSessionRequest only accepts { persona: String }
        // We use "jack" as the default persona (Russell's system persona)
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::String(uuid::Uuid::new_v4().to_string()),
            method: "acp/session.create".to_string(),
            params: Some(serde_json::json!({
                "persona": "jack",
            })),
            auth: self.macaroon_token.as_ref().map(|t| AuthInfo {
                auth_type: "macaroon".to_string(),
                token: t.clone(),
            }),
            acp_version: Some("0.1.0".to_string()),
        };

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(AcpError::TransportError(format!(
                "Russell error {}: {}",
                error.code, error.message
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

        self.emit_cns_span(
            "cns.federation.translated",
            serde_json::json!({
                "direction": "hKask→Russell",
                "method": "acp/session.create",
                "webid": webid.to_string(),
                "session_id": session_resp.session_id,
            }),
        );

        // Mint a bridge capability token signed with the shared secret
        // so it passes AcpRuntime::verify_capability
        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "russell:session".to_string(),
            CapabilityAction::Execute,
            WebID::from_persona(b"russell-bridge"),
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
            .ok_or_else(|| AcpError::AgentNotFound(*webid))?;

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
            warn!(
                webid = %webid,
                code = error.code,
                message = %error.message,
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
            return Err(AcpError::TransportError(format!(
                "Russell error {}: {}",
                error.code, error.message
            )));
        }

        self.emit_cns_span(
            "cns.federation.translated",
            serde_json::json!({
                "direction": "hKask→Russell",
                "method": "acp/session.message",
                "session_id": session_id,
            }),
        );

        // Return the correlation ID from the original message, or generate one
        let correlation_id = match &msg {
            A2AMessage::TemplateDispatch { correlation_id, .. } => correlation_id.clone(),
            A2AMessage::TemplateResponse { correlation_id, .. } => correlation_id.clone(),
            A2AMessage::MemoryArtifact { artifact_id, .. } => artifact_id.clone(),
        };

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
            return Err(AcpError::TransportError(format!(
                "Russell error {}: {}",
                error.code, error.message
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
                let mut caps = vec![format!("russell:{}", skill.id)];
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

        self.emit_cns_span(
            "cns.federation.translated",
            serde_json::json!({
                "direction": "Russell→hKask",
                "method": "acp/capabilities",
                "skill_count": caps_resp.skills.len(),
                "probe_count": caps_resp.probes.len(),
                "capability_count": capabilities.len(),
            }),
        );

        Ok(capabilities)
    }

    async fn is_registered(&self, webid: &WebID) -> bool {
        // Check if we have an active session for this WebID
        let sessions = self.sessions.read().await;
        sessions.contains_key(webid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_secret() -> Arc<Zeroizing<Vec<u8>>> {
        Arc::new(Zeroizing::new(b"test-bridge-secret-32-bytes-long".to_vec()))
    }

    #[test]
    fn test_russell_adapter_creation() {
        let adapter = RussellAcpAdapter::new("russell".to_string(), test_secret());
        assert!(adapter.macaroon_token.is_none());
    }

    #[test]
    fn test_russell_adapter_with_auth() {
        let adapter = RussellAcpAdapter::new("russell".to_string(), test_secret())
            .with_auth("test-token".to_string());
        assert_eq!(adapter.macaroon_token, Some("test-token".to_string()));
    }

    #[test]
    fn test_extract_message_text_template_dispatch() {
        let msg = A2AMessage::TemplateDispatch {
            from: WebID::new(),
            to: Some(WebID::new()),
            template_id: "greeting".to_string(),
            input: serde_json::json!({"name": "Alice"}),
            correlation_id: "corr-123".to_string(),
        };
        let text = RussellAcpAdapter::extract_message_text(&msg);
        assert!(text.contains("template_dispatch"));
        assert!(text.contains("greeting"));
        assert!(text.contains("corr-123"));
    }

    #[test]
    fn test_extract_message_text_template_response() {
        let msg = A2AMessage::TemplateResponse {
            correlation_id: "corr-456".to_string(),
            result: serde_json::json!({"output": "Hello!"}),
            error: None,
        };
        let text = RussellAcpAdapter::extract_message_text(&msg);
        assert!(text.contains("template_response"));
        assert!(text.contains("corr-456"));
    }

    #[test]
    fn test_extract_message_text_memory_artifact() {
        let msg = A2AMessage::MemoryArtifact {
            producer: WebID::new(),
            artifact_type: "episodic_triple".to_string(),
            artifact_id: "art-789".to_string(),
            visibility: "private".to_string(),
        };
        let text = RussellAcpAdapter::extract_message_text(&msg);
        assert!(text.contains("memory_artifact"));
        assert!(text.contains("art-789"));
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let adapter = RussellAcpAdapter::new("russell".to_string(), test_secret());
        let webid = WebID::new();

        // Initially no session
        assert!(!adapter.is_registered(&webid).await);
        assert!(adapter.get_session_id(&webid).await.is_none());

        // Store a session
        adapter
            .store_session_id(webid, "session-abc".to_string())
            .await;
        assert!(adapter.is_registered(&webid).await);
        assert_eq!(
            adapter.get_session_id(&webid).await,
            Some("session-abc".to_string())
        );

        // Remove the session
        adapter.remove_session_id(&webid).await;
        assert!(!adapter.is_registered(&webid).await);
        assert!(adapter.get_session_id(&webid).await.is_none());
    }

    #[test]
    fn test_bridge_token_uses_shared_secret() {
        let secret = test_secret();
        let adapter = RussellAcpAdapter::new("russell".to_string(), Arc::clone(&secret));

        // Verify the adapter holds the shared secret
        assert_eq!(
            adapter.bridge_secret.as_ref().as_slice(),
            secret.as_ref().as_slice()
        );

        // Mint a token and verify it's signed with the shared secret
        let webid = WebID::new();
        let token = CapabilityToken::new(
            CapabilityResource::Tool,
            "russell:session".to_string(),
            CapabilityAction::Execute,
            WebID::from_persona(b"russell-bridge"),
            webid,
            adapter.bridge_secret.as_ref(),
        );

        // Token should verify against the shared secret
        assert!(token.verify(secret.as_ref()));

        // Token should NOT verify against a different secret
        let wrong_secret = b"wrong-secret-32-bytes-long-here!";
        assert!(!token.verify(wrong_secret));
    }
}
