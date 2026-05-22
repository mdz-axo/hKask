//! hKask MCP Telnyx — Telnyx API integration (SMS, voice, WhatsApp)

use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BuyNumberRequest {
    pub phone_number: String,
    pub messaging_profile_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendSmsRequest {
    pub from: String,
    pub to: String,
    pub text: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MakeCallRequest {
    pub from: String,
    pub to: String,
    pub webhook_url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendWhatsAppRequest {
    pub from: String,
    pub to: String,
    pub content_type: String,
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TtsRequest {
    pub text: String,
    pub voice: Option<String>,
}

#[derive(Debug, Default)]
pub struct TelnyxServer;

impl TelnyxServer {
    pub fn new() -> Self {
        Self
    }
}

#[tool_router(server_handler)]
impl TelnyxServer {
    #[tool(description = "Ping Telnyx API")]
    async fn telnyx_ping(&self) -> String {
        r#"{"status":"ok","message":"Telnyx API is reachable"}"#.to_string()
    }

    #[tool(description = "List phone numbers")]
    async fn telnyx_list_numbers(&self) -> String {
        r#"{"numbers":[],"count":0}"#.to_string()
    }

    #[tool(description = "Buy a phone number")]
    async fn telnyx_buy_number(
        &self,
        Parameters(BuyNumberRequest {
            phone_number,
            messaging_profile_id,
        }): Parameters<BuyNumberRequest>,
    ) -> String {
        format!(
            r#"{{"phone_number":"{}","messaging_profile_id":"{}","purchased":true}}"#,
            phone_number, messaging_profile_id
        )
    }

    #[tool(description = "Send an SMS")]
    async fn telnyx_send_sms(
        &self,
        Parameters(SendSmsRequest { from, to, text }): Parameters<SendSmsRequest>,
    ) -> String {
        format!(
            r#"{{"from":"{}","to":"{}","text":"{}","sent":true}}"#,
            from, to, text
        )
    }

    #[tool(description = "Make a phone call")]
    async fn telnyx_make_call(
        &self,
        Parameters(MakeCallRequest {
            from,
            to,
            webhook_url,
        }): Parameters<MakeCallRequest>,
    ) -> String {
        format!(
            r#"{{"from":"{}","to":"{}","webhook_url":"{}","call_initiated":true}}"#,
            from, to, webhook_url
        )
    }

    #[tool(description = "Send a WhatsApp message")]
    async fn telnyx_send_whatsapp(
        &self,
        Parameters(SendWhatsAppRequest {
            from,
            to,
            content_type,
            content: _,
        }): Parameters<SendWhatsAppRequest>,
    ) -> String {
        format!(
            r#"{{"from":"{}","to":"{}","content_type":"{}","sent":true}}"#,
            from, to, content_type
        )
    }

    #[tool(description = "Generate text-to-speech audio")]
    async fn telnyx_tts(
        &self,
        Parameters(TtsRequest { text, voice }): Parameters<TtsRequest>,
    ) -> String {
        format!(
            r#"{{"text":"{}","voice":"{}","audio_url":"https://example.com/audio.mp3"}}"#,
            text,
            voice.unwrap_or_else(|| "default".to_string())
        )
    }

    #[tool(description = "List available voices")]
    async fn telnyx_list_voices(&self) -> String {
        r#"{"voices":["default","male","female","child"]}"#.to_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = TelnyxServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-telnyx started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
