//! Russell ACP Adapter — Bidirectional ACP bridge to Russell
//!
//! Implements AcpPort by communicating with Russell's ACP server
//! over stdio JSON-RPC.

use async_trait::async_trait;
use hkask_types::{CapabilityToken, WebID};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::info;

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
    #[allow(dead_code)]
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

/// Russell ACP adapter — spawns Russell as child process
pub struct RussellAcpAdapter {
    child: Mutex<Option<Child>>,
    russell_binary: String,
    macaroon_token: Option<String>,
    cns_emitter: Option<Arc<dyn hkask_cns::CnsEmit + Send + Sync>>,
}

impl RussellAcpAdapter {
    /// Create new Russell ACP adapter
    pub fn new(russell_binary: String) -> Self {
        Self {
            child: Mutex::new(None),
            russell_binary,
            macaroon_token: None,
            cns_emitter: None,
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

        let stdin = child.stdin.as_mut().ok_or_else(|| {
            AcpError::TransportError("Russell stdin not available".to_string())
        })?;

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

        let stdout = child.stdout.as_mut().ok_or_else(|| {
            AcpError::TransportError("Russell stdout not available".to_string())
        })?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| AcpError::TransportError(format!("Read failed: {}", e)))?;

        let response: JsonRpcResponse = serde_json::from_str(&line)
            .map_err(|e| AcpError::TransportError(format!("Parse failed: {}", e)))?;

        Ok(response)
    }

    fn emit_cns_span(&self, span_name: &str, data: Value) {
        if let Some(ref cns) = self.cns_emitter {
            cns.emit_event(span_name, "federation", &data, 1.0);
        }
    }
}

#[async_trait]
impl AcpPort for RussellAcpAdapter {
    async fn register_agent(
        &self,
        webid: WebID,
        agent_type: &str,
        capabilities: Vec<String>,
    ) -> Result<CapabilityToken, AcpError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::String(uuid::Uuid::new_v4().to_string()),
            method: "acp/session.create".to_string(),
            params: Some(serde_json::json!({
                "persona": agent_type,
                "webid": webid.to_string(),
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
            return Err(AcpError::TransportError(format!(
                "Russell error {}: {}",
                error.code, error.message
            )));
        }

        self.emit_cns_span(
            "cns.federation.translated",
            serde_json::json!({
                "direction": "hKask→Russell",
                "method": "acp/session.create",
                "webid": webid.to_string(),
            }),
        );

        // For now, create a local capability token
        // In a full implementation, Russell would return a signed token
        let token = CapabilityToken::new(
            hkask_types::CapabilityResource::Tool,
            "russell:session".to_string(),
            hkask_types::CapabilityAction::Execute,
            WebID::from_persona(b"russell-system"),
            webid,
            b"russell-bridge-secret",
        );

        Ok(token)
    }

    async fn unregister_agent(&self, webid: &WebID) -> Result<(), AcpError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::String(uuid::Uuid::new_v4().to_string()),
            method: "acp/session.close".to_string(),
            params: Some(serde_json::json!({
                "webid": webid.to_string(),
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

        Ok(())
    }

    async fn send_message(&self, msg: A2AMessage) -> Result<String, AcpError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::String(uuid::Uuid::new_v4().to_string()),
            method: "acp/session.message".to_string(),
            params: Some(serde_json::to_value(&msg).unwrap_or_default()),
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
            }),
        );

        Ok(uuid::Uuid::new_v4().to_string())
    }

    async fn list_capabilities(&self, _webid: &WebID) -> Result<Vec<String>, AcpError> {
        // Russell doesn't expose capability listing via ACP
        // Return empty list for now
        Ok(vec![])
    }

    async fn is_registered(&self, _webid: &WebID) -> bool {
        // Would need to query Russell's session state
        // For now, assume registered if we have a child process
        let child_opt = self.child.lock().await;
        child_opt.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_russell_adapter_creation() {
        let adapter = RussellAcpAdapter::new("russell".to_string());
        assert!(adapter.macaroon_token.is_none());
    }

    #[test]
    fn test_russell_adapter_with_auth() {
        let adapter = RussellAcpAdapter::new("russell".to_string())
            .with_auth("test-token".to_string());
        assert_eq!(adapter.macaroon_token, Some("test-token".to_string()));
    }
}
