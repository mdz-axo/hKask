//! ACP protocol types and stdio transport.
//!
//! Implements the Agent Client Protocol v1 wire format (JSON-RPC 2.0 over stdio).
//! Message types follow the specification at agentclientprotocol.com.
//!
//! When the `agent-client-protocol` crate stabilizes a simple `Agent` trait,
//! this module will be replaced by the crate's types and transport.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{info, warn};

use super::HkaskAcpAgent;

// ── JSON-RPC envelope ──────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcNotification {
    jsonrpc: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

// ── ACP initialize ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeRequest {
    protocol_version: u32,
    client_capabilities: Option<Value>,
    client_info: Option<ClientInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientInfo {
    name: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

fn initialize_response(id: Value) -> JsonRpcResponse {
    let result = serde_json::json!({
        "protocolVersion": 1,
        "agentCapabilities": {
            "loadSession": false,
            "promptCapabilities": {
                "image": false,
                "audio": false,
                "embeddedContext": true,
            },
            "mcpCapabilities": {
                "http": false,
                "sse": false,
            },
        },
        "agentInfo": {
            "name": "hkask-acp",
            "title": "hKask Coding Agent",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "authMethods": [],
    });
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    }
}

// ── ACP session/new ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewSessionRequest {
    cwd: String,
    #[serde(default)]
    mcp_servers: Vec<Value>,
}

fn new_session_response(id: Value, session_id: &str) -> JsonRpcResponse {
    let result = serde_json::json!({ "sessionId": session_id });
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    }
}

// ── ACP session/prompt ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PromptRequest {
    session_id: String,
    prompt: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "resource")]
    Resource { resource: ResourceContent },
    #[serde(rename = "resource_link")]
    ResourceLink { uri: String, name: Option<String> },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceContent {
    uri: String,
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

fn prompt_response(id: Value, stop_reason: &str) -> JsonRpcResponse {
    let result = serde_json::json!({ "stopReason": stop_reason });
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    }
}

// ── ACP session/cancel (notification) ──────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CancelNotification {
    session_id: String,
}

// ── ACP session/update (notification, agent → client) ──────────────────

#[allow(dead_code)]
fn agent_message_chunk(session_id: &str, message_id: &str, text: &str) -> JsonRpcNotification {
    JsonRpcNotification {
        jsonrpc: "2.0".into(),
        method: "session/update".into(),
        params: Some(serde_json::json!({
            "sessionId": session_id,
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "messageId": message_id,
                "content": {
                    "type": "text",
                    "text": text,
                },
            },
        })),
    }
}

fn usage_update(session_id: &str, used: u32, size: u32) -> JsonRpcNotification {
    JsonRpcNotification {
        jsonrpc: "2.0".into(),
        method: "session/update".into(),
        params: Some(serde_json::json!({
            "sessionId": session_id,
            "update": {
                "sessionUpdate": "usage_update",
                "used": used,
                "size": size,
            },
        })),
    }
}

fn error_response(id: Value, code: i32, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.into(),
        }),
    }
}

// ── Stdio transport ────────────────────────────────────────────────────

pub struct StdioTransport {}

impl StdioTransport {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn serve(&mut self, agent: Arc<HkaskAcpAgent>) -> anyhow::Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    warn!(target: "hkask.acp", "Failed to parse JSON-RPC: {}", e);
                    continue;
                }
            };

            let response = self.handle_request(&request, &agent).await;

            // Notifications have no id, so no response
            let Some(_id) = request.id else {
                continue;
            };

            // Send response
            let mut json = serde_json::to_string(&response)?;
            json.push('\n');
            stdout.write_all(json.as_bytes()).await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    #[allow(unused_variables)]
    async fn handle_request(
        &mut self,
        req: &JsonRpcRequest,
        agent: &Arc<HkaskAcpAgent>,
    ) -> JsonRpcResponse {
        let id = req.id.clone().unwrap_or(Value::Null);

        match req.method.as_str() {
            "initialize" => {
                info!(target: "hkask.acp", "initialize");
                initialize_response(id)
            }

            "session/new" => {
                let params: NewSessionRequest =
                    match serde_json::from_value(req.params.clone().unwrap_or(Value::Null)) {
                        Ok(p) => p,
                        Err(e) => {
                            return error_response(id, -32602, &format!("Invalid params: {}", e));
                        }
                    };

                let session_id = format!("acp-{}", uuid::Uuid::new_v4());
                let now = chrono::Utc::now().timestamp();

                {
                    let mut sessions = agent.sessions.lock().await;
                    sessions.insert(
                        session_id.clone(),
                        super::SessionState {
                            session_id: session_id.clone(),
                            cwd: params.cwd.clone(),
                            created_at: now,
                        },
                    );
                }

                info!(
                    target: "hkask.acp",
                    session_id = %session_id,
                    cwd = %params.cwd,
                    "Session created"
                );

                let count = agent.sessions.lock().await.len();
                super::cns_emit(
                    hkask_types::cns::CnsSpan::AcpReplicantMemorySize,
                    &agent.replicant,
                    &format!("sessions={}", count),
                );

                new_session_response(id, &session_id)
            }

            "session/prompt" => {
                let params: PromptRequest =
                    match serde_json::from_value(req.params.clone().unwrap_or(Value::Null)) {
                        Ok(p) => p,
                        Err(e) => {
                            return error_response(id, -32602, &format!("Invalid params: {}", e));
                        }
                    };

                // Extract text content
                let prompt_text: String = params
                    .prompt
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        ContentBlock::Resource { resource } => resource.text.as_deref(),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                info!(
                    target: "hkask.acp",
                    session_id = %params.session_id,
                    prompt_len = prompt_text.len(),
                    "Prompt received"
                );

                if prompt_text.is_empty() {
                    return prompt_response(id, "end_turn");
                }

                // Run inference
                match agent.run_inference(&prompt_text, &params.session_id).await {
                    Ok(result) => {
                        let finish = match result.finish_reason.as_str() {
                            "stop" | "end_turn" => "end_turn",
                            "length" => "max_tokens",
                            _ => "end_turn",
                        };
                        prompt_response(id, finish)
                    }
                    Err(e) => {
                        warn!(target: "hkask.acp", "Inference failed: {}", e);
                        prompt_response(id, "end_turn")
                    }
                }
            }

            "session/cancel" => {
                // Notification — handle but no response expected
                if let Some(params) = &req.params {
                    if let Ok(cancel) = serde_json::from_value::<CancelNotification>(params.clone())
                    {
                        info!(target: "hkask.acp", session_id = %cancel.session_id, "Session cancelled");
                    }
                }
                // Notifications shouldn't reach here (id is None), but be safe
                JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: Some(Value::Null),
                    error: None,
                }
            }

            _ => {
                warn!(target: "hkask.acp", "Unknown method: {}", req.method);
                error_response(id, -32601, &format!("Method not found: {}", req.method))
            }
        }
    }
}
