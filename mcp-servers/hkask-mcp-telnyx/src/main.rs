//! hKask MCP Telnyx — Telnyx API v2 integration (SMS, voice, WhatsApp)

use rmcp::{
    ServiceExt,
    handler::server::wrapper::Parameters,
    tool, tool_router, transport::stdio,
};
use schemars::JsonSchema;
use serde::Deserialize;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const BASE_URL: &str = "https://api.telnyx.com/v2";

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

#[derive(Debug)]
pub struct TelnyxServer {
    client: reqwest::Client,
}

impl Default for TelnyxServer {
    fn default() -> Self {
        Self::new()
    }
}

impl TelnyxServer {
    pub fn new() -> Self {
        let api_key = std::env::var("HKASK_TELNYX_API_KEY").unwrap_or_default();
        let mut headers = reqwest::header::HeaderMap::new();
        if !api_key.is_empty()
            && let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}"))
        {
            headers.insert(reqwest::header::AUTHORIZATION, val);
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }

    fn api_url(path: &str) -> String {
        format!("{BASE_URL}{path}")
    }

}

#[tool_router(server_handler)]
impl TelnyxServer {
    #[tool(description = "Ping Telnyx API")]
    async fn telnyx_ping(&self) -> String {
        let client = self.client.clone();
        let url = Self::api_url("/phone_numbers?page_size=1");

        match client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => {
                        serde_json::json!({
                            "status": if status.is_success() { "ok" } else { "error" },
                            "http_status": status.as_u16(),
                            "message": if status.is_success() {
                                "Telnyx API is reachable"
                            } else {
                                "Telnyx API returned an error"
                            },
                            "data": body,
                        })
                        .to_string()
                    }
                    Err(e) => serde_json::json!({
                        "status": "error",
                        "http_status": status.as_u16(),
                        "error": format!("Failed to parse response: {e}")
                    })
                    .to_string(),
                }
            }
            Err(e) => serde_json::json!({
                "status": "error",
                "error": format!("Failed to connect: {e}")
            })
            .to_string(),
        }
    }

    #[tool(description = "List phone numbers")]
    async fn telnyx_list_numbers(&self) -> String {
        let client = self.client.clone();
        let url = Self::api_url("/phone_numbers");

        match client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => {
                        if status.is_success() {
                            body.to_string()
                        } else {
                            serde_json::json!({
                                "error": "Telnyx API error",
                                "http_status": status.as_u16(),
                                "details": body,
                            })
                            .to_string()
                        }
                    }
                    Err(e) => serde_json::json!({
                        "error": format!("Failed to parse response: {e}"),
                        "http_status": status.as_u16(),
                    })
                    .to_string(),
                }
            }
            Err(e) => serde_json::json!({
                "error": format!("Request failed: {e}")
            })
            .to_string(),
        }
    }

    #[tool(description = "Buy a phone number")]
    async fn telnyx_buy_number(
        &self,
        Parameters(BuyNumberRequest {
            phone_number,
            messaging_profile_id,
        }): Parameters<BuyNumberRequest>,
    ) -> String {
        let client = self.client.clone();
        let url = Self::api_url("/number_orders");
        let body = serde_json::json!({
            "phone_numbers": [{"phone_number": phone_number}],
            "messaging_profile_id": messaging_profile_id,
        });

        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(resp_body) => {
                        if status.is_success() {
                            resp_body.to_string()
                        } else {
                            serde_json::json!({
                                "error": "Failed to purchase number",
                                "http_status": status.as_u16(),
                                "details": resp_body,
                            })
                            .to_string()
                        }
                    }
                    Err(e) => serde_json::json!({
                        "error": format!("Failed to parse response: {e}"),
                        "http_status": status.as_u16(),
                    })
                    .to_string(),
                }
            }
            Err(e) => serde_json::json!({
                "error": format!("Request failed: {e}")
            })
            .to_string(),
        }
    }

    #[tool(description = "Send an SMS")]
    async fn telnyx_send_sms(
        &self,
        Parameters(SendSmsRequest { from, to, text }): Parameters<SendSmsRequest>,
    ) -> String {
        let client = self.client.clone();
        let url = Self::api_url("/messages");
        let body = serde_json::json!({
            "from": from,
            "to": to,
            "text": text,
        });

        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(resp_body) => {
                        if status.is_success() {
                            resp_body.to_string()
                        } else {
                            serde_json::json!({
                                "error": "Failed to send SMS",
                                "http_status": status.as_u16(),
                                "details": resp_body,
                            })
                            .to_string()
                        }
                    }
                    Err(e) => serde_json::json!({
                        "error": format!("Failed to parse response: {e}"),
                        "http_status": status.as_u16(),
                    })
                    .to_string(),
                }
            }
            Err(e) => serde_json::json!({
                "error": format!("Request failed: {e}")
            })
            .to_string(),
        }
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
        let client = self.client.clone();
        let url = Self::api_url("/calls");
        let body = serde_json::json!({
            "from": from,
            "to": to,
            "webhook_url": webhook_url,
        });

        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(resp_body) => {
                        if status.is_success() {
                            resp_body.to_string()
                        } else {
                            serde_json::json!({
                                "error": "Failed to initiate call",
                                "http_status": status.as_u16(),
                                "details": resp_body,
                            })
                            .to_string()
                        }
                    }
                    Err(e) => serde_json::json!({
                        "error": format!("Failed to parse response: {e}"),
                        "http_status": status.as_u16(),
                    })
                    .to_string(),
                }
            }
            Err(e) => serde_json::json!({
                "error": format!("Request failed: {e}")
            })
            .to_string(),
        }
    }

    #[tool(description = "Send a WhatsApp message")]
    async fn telnyx_send_whatsapp(
        &self,
        Parameters(SendWhatsAppRequest {
            from,
            to,
            content_type,
            content,
        }): Parameters<SendWhatsAppRequest>,
    ) -> String {
        let client = self.client.clone();
        let url = Self::api_url("/messages");
        let body = serde_json::json!({
            "from": from,
            "to": to,
            "type": "whatsapp",
            "whatsapp": {
                "content_type": content_type,
                "content": content,
            },
        });

        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(resp_body) => {
                        if status.is_success() {
                            resp_body.to_string()
                        } else {
                            serde_json::json!({
                                "error": "Failed to send WhatsApp message",
                                "http_status": status.as_u16(),
                                "details": resp_body,
                            })
                            .to_string()
                        }
                    }
                    Err(e) => serde_json::json!({
                        "error": format!("Failed to parse response: {e}"),
                        "http_status": status.as_u16(),
                    })
                    .to_string(),
                }
            }
            Err(e) => serde_json::json!({
                "error": format!("Request failed: {e}")
            })
            .to_string(),
        }
    }

    #[tool(description = "Generate text-to-speech audio")]
    async fn telnyx_tts(
        &self,
        Parameters(TtsRequest { text, voice }): Parameters<TtsRequest>,
    ) -> String {
        let client = self.client.clone();
        let url = Self::api_url("/calls");
        let voice_name = voice.unwrap_or_else(|| "female".to_string());
        let body = serde_json::json!({
            "from": "+18001234567",
            "to": "+18007654321",
            "webhook_url": "https://example.com/tts-webhook",
            "tts": {
                "text": text,
                "voice": voice_name,
            },
        });

        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(resp_body) => {
                        if status.is_success() {
                            resp_body.to_string()
                        } else {
                            serde_json::json!({
                                "error": "TTS requires an active call via Call Control API",
                                "http_status": status.as_u16(),
                                "note": "Use telnyx_make_call to establish a call first, then use the call_control_id with the Call Control speak command to play TTS audio",
                                "details": resp_body,
                            })
                            .to_string()
                        }
                    }
                    Err(e) => serde_json::json!({
                        "error": "TTS requires an active call via Call Control API",
                        "note": "Use telnyx_make_call to establish a call first, then use the call_control_id with the Call Control speak command",
                        "parse_error": format!("Failed to parse response: {e}"),
                    })
                    .to_string(),
                }
            }
            Err(e) => serde_json::json!({
                "error": format!("Request failed: {e}"),
                "note": "TTS requires an active call via Call Control API",
            })
            .to_string(),
        }
    }

    #[tool(description = "List available voices")]
    async fn telnyx_list_voices(&self) -> String {
        serde_json::json!({
            "voices": [
                {"id": "female", "name": "Female", "language": "en-US", "gender": "female"},
                {"id": "male", "name": "Male", "language": "en-US", "gender": "male"},
                {"id": "Alice", "name": "Alice", "language": "en-US", "gender": "female"},
                {"id": "Bob", "name": "Bob", "language": "en-US", "gender": "male"},
                {"id": "Eva", "name": "Eva", "language": "en-US", "gender": "female"},
                {"id": "Adam", "name": "Adam", "language": "en-GB", "gender": "male"},
                {"id": "Bridget", "name": "Bridget", "language": "en-GB", "gender": "female"},
                {"id": "Chloe", "name": "Chloe", "language": "fr-FR", "gender": "female"},
                {"id": "Denise", "name": "Denise", "language": "de-DE", "gender": "female"},
                {"id": "Ellen", "name": "Ellen", "language": "es-ES", "gender": "female"},
            ]
        })
        .to_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if std::env::var("HKASK_TELNYX_API_KEY").is_err() {
        tracing::warn!("HKASK_TELNYX_API_KEY not set — API calls will fail");
    }

    let server = TelnyxServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-telnyx started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
