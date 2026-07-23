//! hkask-mcp-communication — MCP server for Matrix communication and TTS.
//!
//! Provides 9 MCP tools for communication:
//!   tts_speak       — speak text aloud via system TTS (espeak)
//!   tts_generate    — generate TTS audio file (espeak)
//!   tts_list_voices — list available system TTS voices
//!   send_message    — send a message to a Matrix room
//!   create_thread   — create a threaded conversation
//!   invite_agent    — invite another userpod to a room
//!   list_threads    — list active communication threads
//!   monitor_thread  — assign a thread to an agent's watchlist
//!   tag_agent       — pull an agent into a discussion
//!
//! All Matrix operations delegate to `hkask-communication`. The daemon owns
//! the Matrix connection and 7R7 listener. This binary is a thin MCP wrapper.
//!
//! The CommunicationServer struct and tool methods are exported from the library
//! target to enable testability (P5 Testing Discipline).

#![allow(unused_crate_dependencies)] // Bin target — deps used in main.rs, lint checks lib target only

pub mod types;

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use types::*;

// Re-export core communication types for backward compatibility
pub use hkask_communication::agent_registration;
pub use hkask_communication::listener;
pub use hkask_communication::matrix;

use hkask_communication::agent_registration::AgentRegistry;
use hkask_communication::matrix::{MatrixTransport, RoomId};
use hkask_mcp_server::server::{McpToolError, ServerContext, execute_tool};
use hkask_types::WebID;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use std::str::FromStr;
use std::sync::Arc;

// ── Server ───────────────────────────────────────────────────────────────

hkask_mcp_server::mcp_server!(
    pub struct CommunicationServer {
        pub matrix: Arc<MatrixTransport>,
        pub registry: Arc<AgentRegistry>,
    }
);

// ── Tools ────────────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl CommunicationServer {
    #[tool(description = "Speak text aloud using the system TTS engine (espeak)")]
    pub async fn tts_speak(
        &self,
        Parameters(TtsSpeakRequest { text, voice }): Parameters<TtsSpeakRequest>,
    ) -> String {
        execute_tool(self, "tts_speak", async {
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
                Ok(status) if status.success() => Ok(serde_json::json!({
                    "spoken": true, "text_length": text.len(), "engine": "espeak"
                })),
                Ok(_) => Err(McpToolError::internal("espeak exited with error")),
                Err(e) => Err(McpToolError::unavailable(format!(
                    "espeak not available: {e}"
                ))),
            }
        })
        .await
    }

    #[tool(description = "Generate TTS audio file using system TTS. Returns path to WAV file.")]
    pub async fn tts_generate(
        &self,
        Parameters(TtsGenerateRequest { text, voice }): Parameters<TtsGenerateRequest>,
    ) -> String {
        execute_tool(self, "tts_generate", async {
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
            match std::process::Command::new("espeak")
                .args(&args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .and_then(|mut c| c.wait())
            {
                Ok(status) if status.success() => Ok(serde_json::json!({
                    "audio_path": path_str, "voice": voice, "text_length": text.len(),
                    "engine": "espeak"
                })),
                Ok(_) => Err(McpToolError::internal("espeak exited with error")),
                Err(e) => Err(McpToolError::unavailable(format!(
                    "espeak not available: {e}"
                ))),
            }
        })
        .await
    }

    #[tool(description = "List available system TTS voices (espeak)")]
    pub async fn tts_list_voices(
        &self,
        Parameters(ListVoicesRequest { language }): Parameters<ListVoicesRequest>,
    ) -> String {
        execute_tool(self, "tts_list_voices", async {
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
            Ok(serde_json::json!({
                "voices": filtered, "total": filtered.len(), "engine": "espeak"
            }))
        })
        .await
    }

    #[tool(description = "Send a message to a Matrix room.")]
    pub async fn send_message(
        &self,
        Parameters(SendMessageRequest { room_id, body }): Parameters<SendMessageRequest>,
    ) -> String {
        execute_tool(self, "send_message", async {
            match self
                .matrix
                .send_message(&RoomId::new(&room_id), &body, None)
                .await
            {
                Ok(()) => Ok(serde_json::json!({"sent": true, "room_id": room_id})),
                Err(e) => Err(McpToolError::unavailable(format!(
                    "Failed to send message: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(description = "Create a threaded conversation (Matrix room).")]
    pub async fn create_thread(
        &self,
        Parameters(CreateThreadRequest { title, topic }): Parameters<CreateThreadRequest>,
    ) -> String {
        execute_tool(self, "create_thread", async {
            match self.matrix.create_room(&title, topic.as_deref()).await {
                Ok(room_id) => Ok(serde_json::json!({"room_id": room_id.as_str(), "title": title})),
                Err(e) => Err(McpToolError::unavailable(format!(
                    "Failed to create thread: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(description = "Invite another userpod to a Matrix room.")]
    pub async fn invite_agent(
        &self,
        Parameters(InviteAgentRequest {
            room_id,
            userpod_id,
        }): Parameters<InviteAgentRequest>,
    ) -> String {
        execute_tool(self, "invite_agent", async {
            let webid = match WebID::from_str(&userpod_id) {
                Ok(w) => w,
                Err(_) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "Invalid userpod ID: {}",
                        userpod_id
                    )));
                }
            };
            let user_id = match self.registry.resolve(&webid).await {
                Some(uid) => uid,
                None => {
                    return Err(McpToolError::permission_denied(format!(
                        "UserPod {} not registered",
                        userpod_id
                    )));
                }
            };
            match self
                .matrix
                .invite_user(&RoomId::new(&room_id), &user_id)
                .await
            {
                Ok(()) => Ok(serde_json::json!({
                    "invited": true, "room_id": room_id, "userpod_id": userpod_id
                })),
                Err(e) => Err(McpToolError::unavailable(format!(
                    "Failed to invite agent: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(description = "List active communication threads.")]
    pub async fn list_threads(&self) -> String {
        execute_tool(self, "list_threads", async {
            match self.matrix.list_rooms().await {
                Ok(threads) => {
                    let thread_list: Vec<serde_json::Value> = threads
                        .iter()
                        .map(|t| {
                            serde_json::json!({
                                "room_id": t.room_id.as_str(), "title": t.title,
                                "participants": t.participants.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
                                "monitored": t.monitored_by.len(), "escalated": t.escalated,
                            })
                        })
                        .collect();
                    Ok(serde_json::json!({
                        "threads": thread_list, "total": thread_list.len()
                    }))
                }
                Err(e) => Err(McpToolError::unavailable(format!(
                    "Failed to list threads: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(description = "Assign a thread to an agent's watchlist for monitoring.")]
    pub async fn monitor_thread(
        &self,
        Parameters(MonitorThreadRequest {
            room_id,
            userpod_id,
        }): Parameters<MonitorThreadRequest>,
    ) -> String {
        execute_tool(self, "monitor_thread", async {
            let webid = match WebID::from_str(&userpod_id) {
                Ok(w) => w,
                Err(_) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "Invalid userpod ID: {}",
                        userpod_id
                    )));
                }
            };
            match self
                .registry
                .monitor_thread(&webid, &RoomId::new(&room_id))
                .await
            {
                Ok(()) => Ok(serde_json::json!({
                    "monitored": true, "room_id": room_id, "userpod_id": userpod_id
                })),
                Err(e) => Err(McpToolError::permission_denied(format!(
                    "Failed to monitor thread: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(description = "Pull an agent into a discussion by sending them a tagged message.")]
    pub async fn tag_agent(
        &self,
        Parameters(TagAgentRequest {
            room_id,
            userpod_id,
            body,
        }): Parameters<TagAgentRequest>,
    ) -> String {
        execute_tool(self, "tag_agent", async {
            let webid = match WebID::from_str(&userpod_id) {
                Ok(w) => w,
                Err(_) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "Invalid userpod ID: {}",
                        userpod_id
                    )));
                }
            };
            let user_id = match self.registry.resolve(&webid).await {
                Some(uid) => uid,
                None => {
                    return Err(McpToolError::permission_denied(format!(
                        "UserPod {} not registered",
                        userpod_id
                    )));
                }
            };
            let mention = format!("@{} {}", user_id.as_str(), body);
            let structured = serde_json::json!({"tag": {"target": userpod_id, "type": "mention"}});
            match self
                .matrix
                .send_message(&RoomId::new(&room_id), &mention, Some(structured))
                .await
            {
                Ok(()) => Ok(serde_json::json!({
                    "tagged": true, "room_id": room_id, "userpod_id": userpod_id
                })),
                Err(e) => Err(McpToolError::unavailable(format!(
                    "Failed to tag agent: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(
        description = "Upload a file to the Matrix homeserver. Returns an mxc:// URI for use in messages."
    )]
    async fn upload_file(
        &self,
        Parameters(UploadFileRequest {
            filename,
            mime_type,
            data_base64,
        }): Parameters<UploadFileRequest>,
    ) -> String {
        use base64::Engine;
        execute_tool(self, "upload_file", async {
            let data = base64::engine::general_purpose::STANDARD
                .decode(&data_base64)
                .map_err(|e| McpToolError::invalid_argument(format!("Invalid base64: {}", e)))?;
            match self.matrix.upload_file(&filename, &mime_type, &data).await {
                Ok(uri) => Ok(serde_json::json!({"uri": uri, "filename": filename, "mime_type": mime_type, "size": data.len()})),
                Err(e) => Err(McpToolError::unavailable(format!("Upload failed: {}", e))),
            }
        })
        .await
    }

    #[tool(
        description = "Upload a file and send it as an attachment to a Matrix room. Supports images, video, audio, and generic files."
    )]
    async fn send_file(
        &self,
        Parameters(SendFileRequest {
            room_id,
            filename,
            mime_type,
            data_base64,
            caption,
        }): Parameters<SendFileRequest>,
    ) -> String {
        use base64::Engine;
        execute_tool(self, "send_file", async {
            let data = base64::engine::general_purpose::STANDARD
                .decode(&data_base64)
                .map_err(|e| McpToolError::invalid_argument(format!("Invalid base64: {}", e)))?;
            match self
                .matrix
                .send_file(&RoomId::new(&room_id), &filename, &mime_type, &data, caption.as_deref())
                .await
            {
                Ok(()) => Ok(serde_json::json!({"sent": true, "room_id": room_id, "filename": filename, "size": data.len()})),
                Err(e) => Err(McpToolError::unavailable(format!("Send file failed: {}", e))),
            }
        })
        .await
    }
}

// ── Entry point ───────────────────────────────────────────────────────────

/// Run the communication MCP server (used by binary target).
pub async fn run(
    userpod: String,
    daemon_client: Option<hkask_mcp_server::DaemonClient>,
) -> Result<(), hkask_mcp_server::McpError> {
    let homeserver_url =
        std::env::var("HKASK_MATRIX_URL").unwrap_or_else(|_| "http://localhost:8008".to_string());

    let mut transport = MatrixTransport::new(&homeserver_url);
    transport
        .health_check()
        .await
        .map_err(|e| hkask_mcp_server::McpError::UnexpectedResponse {
            context: "matrix health check".into(),
            detail: e.to_string(),
        })?;

    if let (Ok(username), Ok(password)) = (
        std::env::var("HKASK_MATRIX_AGENT_USERNAME"),
        std::env::var("HKASK_MATRIX_AGENT_PASSWORD"),
    ) {
        transport.login(&username, &password).await.map_err(|e| {
            hkask_mcp_server::McpError::UnexpectedResponse {
                context: "matrix login".into(),
                detail: e.to_string(),
            }
        })?;
    }

    let matrix = Arc::new(transport);
    let registry = Arc::new(AgentRegistry::new());

    // Note: 7R7 listener is started by the daemon, not here.
    // The MCP binary is a thin wrapper — infrastructure lives in the daemon.

    hkask_mcp_server::run_server(
        "hkask-mcp-communication",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| {
            Ok(CommunicationServer::new(
                ctx.webid,
                userpod.clone(),
                daemon_client.clone(),
                Arc::clone(&matrix),
                Arc::clone(&registry),
            ))
        },
        vec![],
    )
    .await
}
