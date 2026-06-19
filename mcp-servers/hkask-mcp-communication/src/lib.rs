//! hkask-mcp-communication — MCP server for Matrix communication and TTS.
//!
//! Provides 9 MCP tools for communication:
//!   tts_speak       — speak text aloud via system TTS (espeak)
//!   tts_generate    — generate TTS audio file (espeak)
//!   tts_list_voices — list available system TTS voices
//!   send_message    — send a message to a Matrix room
//!   create_thread   — create a threaded conversation
//!   invite_agent    — invite another replicant to a room
//!   list_threads    — list active communication threads
//!   monitor_thread  — assign a thread to an agent's watchlist
//!   tag_agent       — pull an agent into a discussion
//!
//! All Matrix operations delegate to `hkask-communication`. The daemon owns
//! the Matrix connection and 7R7 listener. This binary is a thin MCP wrapper.
//!
//! The CommunicationServer struct and tool methods are exported from the library
//! target to enable testability (P5 Testing Discipline).

pub mod types;

// Re-export core communication types for backward compatibility
pub use hkask_communication::agent_registration;
pub use hkask_communication::listener;
pub use hkask_communication::matrix;

use hkask_communication::agent_registration::AgentRegistry;
use hkask_communication::matrix::{MatrixTransport, RoomId};
use hkask_mcp::server::{McpToolError, ServerContext, ToolSpanGuard};
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::str::FromStr;
use std::sync::Arc;

// ── Server ───────────────────────────────────────────────────────────────

pub struct CommunicationServer {
    pub webid: WebID,
    pub matrix: Arc<MatrixTransport>,
    pub registry: Arc<AgentRegistry>,
}

impl CommunicationServer {
    pub fn new(webid: WebID, matrix: Arc<MatrixTransport>, registry: Arc<AgentRegistry>) -> Self {
        Self {
            webid,
            matrix,
            registry,
        }
    }
}

// ── Tools ────────────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl CommunicationServer {
    #[tool(description = "Speak text aloud using the system TTS engine (espeak)")]
    async fn tts_speak(
        &self,
        Parameters(TtsSpeakRequest { text, voice }): Parameters<TtsSpeakRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("tts_speak", &self.webid);
        let voice_arg = if voice == "default" {
            String::new()
        } else {
            format!("-v{}", voice)
        };
        let mut args = vec!["-s", "150"];
        if !voice_arg.is_empty() {
            args.push(&voice_arg);
        }
        args.push("--");
        args.push(&text);
        match std::process::Command::new("espeak")
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .and_then(|mut c| c.wait())
        {
            Ok(status) if status.success() => span.ok_json(
                serde_json::json!({"spoken": true, "text_length": text.len(), "engine": "espeak"}),
            ),
            Ok(_) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal("espeak exited with error").to_json_string(),
            ),
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("espeak not available: {e}")).to_json_string(),
            ),
        }
    }

    #[tool(description = "Generate TTS audio file using system TTS. Returns path to WAV file.")]
    async fn tts_generate(
        &self,
        Parameters(TtsGenerateRequest { text, voice }): Parameters<TtsGenerateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("tts_generate", &self.webid);
        let file_name = format!("hkask-tts-{}.wav", uuid::Uuid::new_v4());
        let path = std::env::temp_dir().join(&file_name);
        let path_str = path.to_string_lossy().to_string();
        let voice_arg = if voice == "default" {
            String::new()
        } else {
            format!("-v{}", voice)
        };
        let mut args = vec!["-w", &path_str, "-s", "150"];
        if !voice_arg.is_empty() {
            args.push(&voice_arg);
        }
        args.push("--");
        args.push(&text);
        match std::process::Command::new("espeak").args(&args).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().and_then(|mut c| c.wait()) {
            Ok(status) if status.success() => span.ok_json(serde_json::json!({"audio_path": path_str, "voice": voice, "text_length": text.len(), "engine": "espeak"})),
            Ok(_) => span.error(McpErrorKind::Internal, McpToolError::internal("espeak exited with error").to_json_string()),
            Err(e) => span.error(McpErrorKind::Unavailable, McpToolError::unavailable(format!("espeak not available: {e}")).to_json_string()),
        }
    }

    #[tool(description = "List available system TTS voices (espeak)")]
    async fn tts_list_voices(
        &self,
        Parameters(ListVoicesRequest { language }): Parameters<ListVoicesRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("tts_list_voices", &self.webid);
        let voices = serde_json::json!([
            {"id": "default", "name": "Default", "language": "en"},
            {"id": "en-us", "name": "English (US)", "language": "en-US"},
            {"id": "en-uk", "name": "English (UK)", "language": "en-GB"},
            {"id": "en-sc", "name": "English (Scottish)", "language": "en-GB"},
            {"id": "fr", "name": "French", "language": "fr-FR"},
            {"id": "de", "name": "German", "language": "de-DE"},
            {"id": "es", "name": "Spanish", "language": "es-ES"},
            {"id": "it", "name": "Italian", "language": "it-IT"},
        ]);
        let filtered: Vec<&serde_json::Value> = if let Some(ref lang) = language {
            voices
                .as_array()
                .expect("voices json! literal is always an array")
                .iter()
                .filter(|v| {
                    v["language"]
                        .as_str()
                        .unwrap_or("")
                        .starts_with(lang.as_str())
                })
                .collect()
        } else {
            voices
                .as_array()
                .expect("voices json! literal is always an array")
                .iter()
                .collect()
        };
        span.ok_json(
            serde_json::json!({"voices": filtered, "total": filtered.len(), "engine": "espeak"}),
        )
    }

    #[tool(description = "Send a message to a Matrix room.")]
    async fn send_message(
        &self,
        Parameters(SendMessageRequest { room_id, body }): Parameters<SendMessageRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("send_message", &self.webid);
        match self
            .matrix
            .send_message(&RoomId::new(&room_id), &body, None)
            .await
        {
            Ok(()) => span.ok_json(serde_json::json!({"sent": true, "room_id": room_id})),
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Failed to send message: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(description = "Create a threaded conversation (Matrix room).")]
    async fn create_thread(
        &self,
        Parameters(CreateThreadRequest { title, topic }): Parameters<CreateThreadRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("create_thread", &self.webid);
        match self.matrix.create_room(&title, topic.as_deref()).await {
            Ok(room_id) => {
                span.ok_json(serde_json::json!({"room_id": room_id.as_str(), "title": title}))
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Failed to create thread: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(description = "Invite another replicant to a Matrix room.")]
    async fn invite_agent(
        &self,
        Parameters(InviteAgentRequest {
            room_id,
            replicant_id,
        }): Parameters<InviteAgentRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("invite_agent", &self.webid);
        let webid = match WebID::from_str(&replicant_id) {
            Ok(w) => w,
            Err(_) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "Invalid replicant ID: {}",
                        replicant_id
                    ))
                    .to_json_string(),
                );
            }
        };
        let user_id = match self.registry.resolve(&webid).await {
            Some(uid) => uid,
            None => {
                return span.error(
                    McpErrorKind::PermissionDenied,
                    McpToolError::permission_denied(format!(
                        "Replicant {} not registered",
                        replicant_id
                    ))
                    .to_json_string(),
                );
            }
        };
        match self.matrix.invite_user(&RoomId::new(&room_id), &user_id).await {
            Ok(()) => span.ok_json(serde_json::json!({"invited": true, "room_id": room_id, "replicant_id": replicant_id})),
            Err(e) => span.error(McpErrorKind::Unavailable, McpToolError::unavailable(format!("Failed to invite agent: {}", e)).to_json_string()),
        }
    }

    #[tool(description = "List active communication threads.")]
    async fn list_threads(&self) -> String {
        let span = ToolSpanGuard::new("list_threads", &self.webid);
        match self.matrix.list_rooms().await {
            Ok(threads) => {
                let thread_list: Vec<serde_json::Value> = threads.iter().map(|t| serde_json::json!({
                    "room_id": t.room_id.as_str(), "title": t.title,
                    "participants": t.participants.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
                    "monitored": t.monitored_by.len(), "escalated": t.escalated,
                })).collect();
                span.ok_json(
                    serde_json::json!({"threads": thread_list, "total": thread_list.len()}),
                )
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Failed to list threads: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(description = "Assign a thread to an agent's watchlist for monitoring.")]
    async fn monitor_thread(
        &self,
        Parameters(MonitorThreadRequest {
            room_id,
            replicant_id,
        }): Parameters<MonitorThreadRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("monitor_thread", &self.webid);
        let webid = match WebID::from_str(&replicant_id) {
            Ok(w) => w,
            Err(_) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "Invalid replicant ID: {}",
                        replicant_id
                    ))
                    .to_json_string(),
                );
            }
        };
        match self.registry.monitor_thread(&webid, &RoomId::new(&room_id)).await {
            Ok(()) => span.ok_json(serde_json::json!({"monitored": true, "room_id": room_id, "replicant_id": replicant_id})),
            Err(e) => span.error(McpErrorKind::PermissionDenied, McpToolError::permission_denied(format!("Failed to monitor thread: {}", e)).to_json_string()),
        }
    }

    #[tool(description = "Pull an agent into a discussion by sending them a tagged message.")]
    async fn tag_agent(
        &self,
        Parameters(TagAgentRequest {
            room_id,
            replicant_id,
            body,
        }): Parameters<TagAgentRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("tag_agent", &self.webid);
        let webid = match WebID::from_str(&replicant_id) {
            Ok(w) => w,
            Err(_) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "Invalid replicant ID: {}",
                        replicant_id
                    ))
                    .to_json_string(),
                );
            }
        };
        let user_id = match self.registry.resolve(&webid).await {
            Some(uid) => uid,
            None => {
                return span.error(
                    McpErrorKind::PermissionDenied,
                    McpToolError::permission_denied(format!(
                        "Replicant {} not registered",
                        replicant_id
                    ))
                    .to_json_string(),
                );
            }
        };
        let mention = format!("@{} {}", user_id.as_str(), body);
        let structured = serde_json::json!({"tag": {"target": replicant_id, "type": "mention"}});
        match self.matrix.send_message(&RoomId::new(&room_id), &mention, Some(structured)).await {
            Ok(()) => span.ok_json(serde_json::json!({"tagged": true, "room_id": room_id, "replicant_id": replicant_id})),
            Err(e) => span.error(McpErrorKind::Unavailable, McpToolError::unavailable(format!("Failed to tag agent: {}", e)).to_json_string()),
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────

/// Run the communication MCP server (used by binary target).
pub async fn run(
    replicant: String,
    _daemon_client: Option<hkask_mcp::DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
    let homeserver_url =
        std::env::var("HKASK_MATRIX_URL").unwrap_or_else(|_| "http://localhost:8008".to_string());

    let mut transport = MatrixTransport::new(&homeserver_url);
    transport
        .health_check()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if let (Ok(username), Ok(password)) = (
        std::env::var("HKASK_MATRIX_AGENT_USERNAME"),
        std::env::var("HKASK_MATRIX_AGENT_PASSWORD"),
    ) {
        transport
            .login(&username, &password)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    let matrix = Arc::new(transport);
    let registry = Arc::new(AgentRegistry::new());

    // Note: 7R7 listener is started by the daemon, not here.
    // The MCP binary is a thin wrapper — infrastructure lives in the daemon.

    hkask_mcp::run_server(
        "hkask-mcp-communication",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| {
            Ok(CommunicationServer::new(
                ctx.webid,
                Arc::clone(&matrix),
                Arc::clone(&registry),
            ))
        },
        vec![],
    )
    .await
}
