//! hKask MCP Telnyx — Telnyx API v2 integration (SMS, voice, WhatsApp)

use hkask_mcp::server::{
    McpToolOutput, ToolSpanGuard, api_get, api_post, resolve_credential, validate_tool_url,
};
use hkask_types::WebID;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

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
pub struct ListVoicesRequest {
    /// Filter voices by language prefix (e.g., "en", "fr", "de")
    pub language: Option<String>,
}

pub struct TelnyxServer {
    webid: WebID,
    client: reqwest::Client,
}

impl TelnyxServer {
    pub fn new(webid: WebID) -> Result<Self, anyhow::Error> {
        let api_key = resolve_credential("HKASK_TELNYX_API_KEY").map_err(|_| {
            anyhow::anyhow!("HKASK_TELNYX_API_KEY not found in keychain or environment")
        })?;

        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}")) {
            headers.insert(reqwest::header::AUTHORIZATION, val);
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Ok(Self { webid, client })
    }
}

#[tool_router(server_handler)]
impl TelnyxServer {
    #[tool(description = "Ping Telnyx API")]
    async fn telnyx_ping(&self) -> String {
        let span = ToolSpanGuard::new("telnyx_ping", &self.webid);
        let url = format!("{BASE_URL}/phone_numbers?page_size=1");
        match api_get(&self.client, "Telnyx", &url).await {
            Ok(body) => span.ok(McpToolOutput::new(serde_json::json!({
                "status": "ok",
                "message": "Telnyx API is reachable",
                "data": body,
            }))
            .to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "List phone numbers")]
    async fn telnyx_list_numbers(&self) -> String {
        let span = ToolSpanGuard::new("telnyx_list_numbers", &self.webid);
        let url = format!("{BASE_URL}/phone_numbers");
        match api_get(&self.client, "Telnyx", &url).await {
            Ok(body) => span.ok(McpToolOutput::new(body).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
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
        let span = ToolSpanGuard::new("telnyx_buy_number", &self.webid);
        let url = format!("{BASE_URL}/number_orders");
        let payload = serde_json::json!({
            "phone_numbers": [{"phone_number": phone_number}],
            "messaging_profile_id": messaging_profile_id,
        });
        match api_post(&self.client, "Telnyx", &url, &payload).await {
            Ok(resp_body) => span.ok(McpToolOutput::new(resp_body).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Send an SMS")]
    async fn telnyx_send_sms(
        &self,
        Parameters(SendSmsRequest { from, to, text }): Parameters<SendSmsRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("telnyx_send_sms", &self.webid);
        let url = format!("{BASE_URL}/messages");
        let payload = serde_json::json!({
            "from": from,
            "to": to,
            "text": text,
        });
        match api_post(&self.client, "Telnyx", &url, &payload).await {
            Ok(resp_body) => span.ok(McpToolOutput::new(resp_body).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
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
        let span = ToolSpanGuard::new("telnyx_make_call", &self.webid);
        if let Err(e) = validate_tool_url(&webhook_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let url = format!("{BASE_URL}/calls");
        let payload = serde_json::json!({
            "from": from,
            "to": to,
            "webhook_url": webhook_url,
        });
        match api_post(&self.client, "Telnyx", &url, &payload).await {
            Ok(resp_body) => span.ok(McpToolOutput::new(resp_body).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
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
        let span = ToolSpanGuard::new("telnyx_send_whatsapp", &self.webid);
        let url = format!("{BASE_URL}/messages");
        let payload = serde_json::json!({
            "from": from,
            "to": to,
            "type": "whatsapp",
            "whatsapp": {
                "content_type": content_type,
                "content": content,
            },
        });
        match api_post(&self.client, "Telnyx", &url, &payload).await {
            Ok(resp_body) => span.ok(McpToolOutput::new(resp_body).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "List available TTS voices (static catalog from Telnyx docs)")]
    async fn telnyx_list_voices(
        &self,
        Parameters(ListVoicesRequest { language }): Parameters<ListVoicesRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("telnyx_list_voices", &self.webid);
        let all_voices = serde_json::json!([
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
        ]);
        let voices: Vec<&serde_json::Value> = if let Some(ref lang) = language {
            all_voices
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
            all_voices.as_array().unwrap().iter().collect()
        };
        span.ok(McpToolOutput::new(serde_json::json!({
            "voices": voices,
            "total": voices.len(),
            "source": "static catalog (Telnyx Call Control API docs)",
        }))
        .to_json_string())
    }
}

hkask_mcp::mcp_server_main!(
    "hkask-mcp-telnyx",
    TelnyxServer,
    credentials: vec![hkask_mcp::CredentialRequirement::required(
        "HKASK_TELNYX_API_KEY",
        "Telnyx API key for messaging and number management",
    )]
);
