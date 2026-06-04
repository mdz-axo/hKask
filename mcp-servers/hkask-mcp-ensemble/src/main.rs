//! hKask MCP Ensemble — Multi-agent coordination MCP server
//!
//! Starts an MCP server over stdio exposing 5 tools:
//! - `coordinate_session` — Create a standing session from a YAML config path
//! - `register_participant` — Register a bot participant in a session
//! - `send_message` — Send a message to a standing session
//! - `get_status` — Get standing session status
//! - `improv_turn` — Execute an improvisation turn in a session

use hkask_ensemble::{
    ChatMessage, ChatParticipant, ParticipantRole, StandingSession, bootstrap_standing_session,
};
use hkask_mcp::server::{McpToolOutput, ToolSpanGuard, validate_identifier};
use hkask_types::McpErrorKind;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CoordinateSessionRequest {
    pub config_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterParticipantRequest {
    pub session_id: String,
    pub agent: String,
    pub role: String,
    pub description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendMessageRequest {
    pub session_id: String,
    pub from_agent: String,
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetStatusRequest {
    pub session_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImprovTurnRequest {
    pub session_id: String,
    pub user_message: String,
}

pub struct EnsembleServer {
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<StandingSession>>>>>,
    webid: WebID,
}

impl EnsembleServer {
    pub fn new(webid: WebID) -> anyhow::Result<Self> {
        Ok(Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            webid,
        })
    }
}

#[tool_router(server_handler)]
impl EnsembleServer {
    #[tool(description = "Create a standing session from a YAML config path")]
    async fn coordinate_session(
        &self,
        Parameters(CoordinateSessionRequest { config_path }): Parameters<CoordinateSessionRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("coordinate_session", &self.webid);

        if let Err(e) = validate_identifier("config_path", &config_path, 512) {
            return span.error(e.kind, e.to_json_string());
        }

        match bootstrap_standing_session(Path::new(&config_path)) {
            Ok(session) => {
                let status = session.get_status();
                let session_id = status.session_id.clone();

                let mut sessions = self.sessions.write().await;
                sessions.insert(session_id.clone(), Arc::new(RwLock::new(session)));

                span.ok(McpToolOutput::new(serde_json::json!({
                    "session_id": session_id,
                    "description": status.description,
                    "participant_count": status.participant_count,
                    "message_count": status.message_count,
                    "bootstrapped": true,
                }))
                .to_json_string())
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolOutput::new(serde_json::json!({
                    "config_path": config_path,
                    "error": e.to_string(),
                }))
                .to_json_string(),
            ),
        }
    }

    #[tool(description = "Register a bot participant in a session")]
    async fn register_participant(
        &self,
        Parameters(RegisterParticipantRequest {
            session_id,
            agent,
            role,
            description,
        }): Parameters<RegisterParticipantRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("register_participant", &self.webid);

        if let Err(e) = validate_identifier("session_id", &session_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }
        if let Err(e) = validate_identifier("agent", &agent, 128) {
            return span.error(e.kind, e.to_json_string());
        }
        if let Err(e) = validate_identifier("role", &role, 64) {
            return span.error(e.kind, e.to_json_string());
        }

        let sessions = self.sessions.read().await;
        match sessions.get(&session_id) {
            Some(session_lock) => {
                let mut session = session_lock.write().await;
                let webid = WebID::from_persona(agent.as_bytes());
                let participant_role = match role.as_str() {
                    "orchestrator" => ParticipantRole::Curator,
                    _ => ParticipantRole::Custom(role.clone()),
                };
                let participant = ChatParticipant {
                    webid,
                    role: participant_role,
                    pod_id: None,
                    capabilities: vec![],
                };
                session.chat.register_participant(participant);
                session.participant_names.insert(webid, agent.clone());
                session
                    .participant_descriptions
                    .insert(webid, description.clone());

                span.ok(McpToolOutput::new(serde_json::json!({
                    "session_id": session_id,
                    "agent": agent,
                    "role": role,
                    "registered": true,
                }))
                .to_json_string())
            }
            None => span.error(
                McpErrorKind::NotFound,
                McpToolOutput::new(serde_json::json!({
                    "session_id": session_id,
                    "error": "session not found",
                }))
                .to_json_string(),
            ),
        }
    }

    #[tool(description = "Send a message to a standing session")]
    async fn send_message(
        &self,
        Parameters(SendMessageRequest {
            session_id,
            from_agent,
            content,
        }): Parameters<SendMessageRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("send_message", &self.webid);

        if let Err(e) = validate_identifier("session_id", &session_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }
        if let Err(e) = validate_identifier("from_agent", &from_agent, 128) {
            return span.error(e.kind, e.to_json_string());
        }

        let sessions = self.sessions.read().await;
        match sessions.get(&session_id) {
            Some(session_lock) => {
                let mut session = session_lock.write().await;
                let webid = WebID::from_persona(from_agent.as_bytes());
                let message = ChatMessage::new(webid, content.clone());
                session.chat.add_message(message);

                span.ok(McpToolOutput::new(serde_json::json!({
                    "session_id": session_id,
                    "from_agent": from_agent,
                    "sent": true,
                }))
                .to_json_string())
            }
            None => span.error(
                McpErrorKind::NotFound,
                McpToolOutput::new(serde_json::json!({
                    "session_id": session_id,
                    "error": "session not found",
                }))
                .to_json_string(),
            ),
        }
    }

    #[tool(description = "Get standing session status")]
    async fn get_status(
        &self,
        Parameters(GetStatusRequest { session_id }): Parameters<GetStatusRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("get_status", &self.webid);

        if let Err(e) = validate_identifier("session_id", &session_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        let sessions = self.sessions.read().await;
        match sessions.get(&session_id) {
            Some(session_lock) => {
                let session = session_lock.read().await;
                let status = session.get_status();

                span.ok(McpToolOutput::new(serde_json::json!({
                    "session_id": status.session_id,
                    "description": status.description,
                    "participant_count": status.participant_count,
                    "message_count": status.message_count,
                    "participants": status.participants.iter().map(|p| serde_json::json!({
                        "name": p.name,
                        "webid": p.webid,
                        "role": p.role,
                        "description": p.description,
                    })).collect::<Vec<_>>(),
                }))
                .to_json_string())
            }
            None => span.error(
                McpErrorKind::NotFound,
                McpToolOutput::new(serde_json::json!({
                    "session_id": session_id,
                    "error": "session not found",
                }))
                .to_json_string(),
            ),
        }
    }

    #[tool(description = "Execute an improvisation turn in a session")]
    async fn improv_turn(
        &self,
        Parameters(ImprovTurnRequest {
            session_id,
            user_message: _user_message,
        }): Parameters<ImprovTurnRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("improv_turn", &self.webid);

        if let Err(e) = validate_identifier("session_id", &session_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        let sessions = self.sessions.read().await;
        match sessions.get(&session_id) {
            Some(_session_lock) => {
                span.ok(McpToolOutput::new(serde_json::json!({
                    "session_id": session_id,
                    "status": "requires_inference_client",
                    "message": "Improv turns require an InferenceClient, which is available via the CLI/API path (kask chat). Use the ensemble API endpoint or kask chat to execute improvisation turns with inference wired.",
                }))
                .to_json_string())
            }
            None => span.error(
                McpErrorKind::NotFound,
                McpToolOutput::new(serde_json::json!({
                    "session_id": session_id,
                    "error": "session not found",
                }))
                .to_json_string(),
            ),
        }
    }
}

hkask_mcp::mcp_server_main!("hkask-mcp-ensemble", EnsembleServer);
