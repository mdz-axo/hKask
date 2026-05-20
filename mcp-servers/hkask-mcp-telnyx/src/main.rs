//! hKask MCP Telnyx — Unified communications (SMS, voice, WhatsApp, Voice AI)

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter},
    model::*,
    transport::stdio,
    schemars, tool, tool_router, tool_handler,
};
use rmcp::handler::server::wrapper::Parameters;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use secrecy::Secret;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const TELNYX_API_BASE: &str = "https://api.telnyx.com/v2";

/// Telnyx server implementation
pub struct TelnyxServer {
    tool_router: ToolRouter<TelnyxServer>,
    client: Client,
    api_key: Option<Secret<String>>,
}

impl TelnyxServer {
    pub fn new() -> Self {
        let api_key = std::env::var("TELNYX_API_KEY").ok().map(Secret::new);
        let client = Client::builder().build().unwrap_or_default();

        Self {
            tool_router: Self::tool_router(),
            client,
            api_key,
        }
    }

    fn get_headers(&self) -> std::collections::HashMap<&str, String> {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Content-Type", "application/json".to_string());
        if let Some(key) = &self.api_key {
            headers.insert("Authorization", format!("Bearer {}", key.expose_secret()));
        }
        headers
    }
}

#[tool_router(server_handler)]
impl TelnyxServer {
    #[tool(description = "Ping the Telnyx server")]
    async fn telnyx_ping(&self) -> String {
        serde_json::json!({
            "status": "ok",
            "server": "hkask-mcp-telnyx",
            "version": SERVER_VERSION,
            "api_key_configured": self.api_key.is_some()
        }).to_string()
    }

    #[tool(description = "List available phone numbers")]
    async fn telnyx_list_numbers(&self) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "TELNYX_API_KEY not configured" }).to_string();
        }

        let url = format!("{}/phone_numbers", TELNYX_API_BASE);
        match self.client.get(&url).headers(self.get_headers().into()).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Buy a phone number")]
    async fn telnyx_buy_number(&self, phone_number: String, messaging_profile_id: Option<String>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "TELNYX_API_KEY not configured" }).to_string();
        }

        let url = format!("{}/phone_numbers/actions/buy", TELNYX_API_BASE);
        let payload = serde_json::json!({
            "phone_number": phone_number,
            "messaging_profile_id": messaging_profile_id
        });

        match self.client.post(&url).headers(self.get_headers().into()).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Send an SMS message")]
    async fn telnyx_send_sms(&self, from: String, to: String, text: String) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "TELNYX_API_KEY not configured" }).to_string();
        }

        let url = format!("{}/messages", TELNYX_API_BASE);
        let payload = serde_json::json!({
            "from": from,
            "to": to,
            "text": text
        });

        match self.client.post(&url).headers(self.get_headers().into()).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Make a voice call")]
    async fn telnyx_make_call(&self, from: String, to: String, webhook_url: Option<String>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "TELNYX_API_KEY not configured" }).to_string();
        }

        let url = format!("{}/calls", TELNYX_API_BASE);
        let mut payload = serde_json::json!({
            "from": from,
            "to": to
        });
        if let Some(webhook) = webhook_url {
            payload["webhook_url"] = serde_json::json!(webhook);
        }

        match self.client.post(&url).headers(self.get_headers().into()).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Send WhatsApp message")]
    async fn telnyx_send_whatsapp(&self, from: String, to: String, content_type: String, content: String) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "TELNYX_API_KEY not configured" }).to_string();
        }

        let url = format!("{}/messages/whatsapp", TELNYX_API_BASE);
        let payload = serde_json::json!({
            "from": from,
            "to": to,
            "content_type": content_type,
            "content": content
        });

        match self.client.post(&url).headers(self.get_headers().into()).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "Text-to-speech using Telnyx Voice AI")]
    async fn telnyx_tts(&self, text: String, voice: Option<String>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "TELNYX_API_KEY not configured" }).to_string();
        }

        let voice = voice.unwrap_or_else(|| "default".to_string());
        let url = format!("{}/audio/tts", TELNYX_API_BASE);
        let payload = serde_json::json!({
            "text": text,
            "voice": voice
        });

        match self.client.post(&url).headers(self.get_headers().into()).json(&payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }

    #[tool(description = "List available TTS voices")]
    async fn telnyx_list_voices(&self) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "TELNYX_API_KEY not configured" }).to_string();
        }

        let url = format!("{}/audio/voices", TELNYX_API_BASE);
        match self.client.get(&url).headers(self.get_headers().into()).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.text().await {
                        Ok(body) => body,
                        Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                } else {
                    serde_json::json!({ "error": format!("API returned {}", resp.status()) }).to_string()
                }
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }).to_string()
        }
    }
}

impl TelnyxServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = TelnyxServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-telnyx MCP server started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
