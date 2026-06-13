//! hKask MCP Communication — local TTS/STT.
//!
//! Tools:
//!   tts_speak       — speak text aloud via system TTS (espeak)
//!   tts_generate    — generate TTS audio file (espeak)
//!   tts_list_voices — list available system TTS voices

use hkask_mcp::server::ToolSpanGuard;
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TtsSpeakRequest {
    pub text: String,
    #[serde(default = "default_espeak_voice")]
    pub voice: String,
}

fn default_espeak_voice() -> String {
    "default".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TtsGenerateRequest {
    pub text: String,
    #[serde(default = "default_espeak_voice")]
    pub voice: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListVoicesRequest {
    pub language: Option<String>,
}

pub struct CommunicationServer {
    webid: WebID,
}

impl CommunicationServer {
    pub fn new(webid: WebID) -> Self {
        Self { webid }
    }
}

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
            Ok(status) if status.success() => span.ok_json(serde_json::json!({
                "spoken": true,
                "text_length": text.len(),
                "engine": "espeak",
            })),
            Ok(_) => span.error(
                McpErrorKind::Internal,
                serde_json::json!({"error": "espeak exited with error"}).to_string(),
            ),
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                serde_json::json!({"error": format!("espeak not available: {e}")}).to_string(),
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

        match std::process::Command::new("espeak")
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .and_then(|mut c| c.wait())
        {
            Ok(status) if status.success() => span.ok_json(serde_json::json!({
                "audio_path": path_str,
                "voice": voice,
                "text_length": text.len(),
                "engine": "espeak",
            })),
            Ok(_) => span.error(
                McpErrorKind::Internal,
                serde_json::json!({"error": "espeak exited with error"}).to_string(),
            ),
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                serde_json::json!({"error": format!("espeak not available: {e}")}).to_string(),
            ),
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
                .unwrap()
                .iter()
                .filter(|v| {
                    v["language"]
                        .as_str()
                        .unwrap_or("")
                        .starts_with(lang.as_str())
                })
                .collect()
        } else {
            voices.as_array().unwrap().iter().collect()
        };
        span.ok_json(serde_json::json!({
            "voices": filtered,
            "total": filtered.len(),
            "engine": "espeak",
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-communication",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| Ok(CommunicationServer::new(ctx.webid)),
        vec![],
    )
    .await
}
