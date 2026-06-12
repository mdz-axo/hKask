//! hKask MCP Telnyx — Telnyx API v2 integration (SMS, voice, WhatsApp)

use hkask_mcp::server::{ToolSpanGuard, api_get, api_post, validate_tool_url};
use hkask_mcp::{DaemonClient, DaemonResponse};
use hkask_types::{McpErrorKind, WebID};
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NotifyUserRequest {
    /// Message content to send to the human user
    pub message: String,
    /// Preferred channel: "sms", "whatsapp", or "call". Defaults to "sms".
    #[serde(default = "default_channel")]
    pub channel: String,
}

fn default_channel() -> String {
    "sms".to_string()
}

pub struct TelnyxServer {
    webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    daemon: Option<DaemonClient>,
    client: reqwest::Client,
    /// Human user's phone number (from HKASK_USER_PHONE env var)
    user_phone: Option<String>,
    /// Human user's name (from HKASK_USER_NAME env var)
    user_name: Option<String>,
}

impl TelnyxServer {
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<DaemonClient>,
        api_key: String,
        user_phone: Option<String>,
        user_name: Option<String>,
    ) -> Result<Self, anyhow::Error> {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}")) {
            headers.insert(reqwest::header::AUTHORIZATION, val);
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;
        Ok(Self {
            webid,
            replicant,
            daemon,
            client,
            user_phone,
            user_name,
        })
    }

    /// Record a tool call as a narrative experience in the agent's memory.
    fn record_experience(
        &self,
        tool: &str,
        input_summary: &str,
        outcome: &str,
        detail: serde_json::Value,
    ) {
        if let Some(ref daemon) = self.daemon {
            let value = serde_json::json!({
                "tool": tool,
                "input": input_summary,
                "outcome": outcome,
                "detail": detail,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            let daemon_clone = daemon.clone();
            let replicant = self.replicant.clone();
            let tool_name = tool.to_string();
            tokio::spawn(async move {
                match daemon_clone
                    .store_experience(&replicant, "mcp_session", "observed", &value, Some(0.85))
                    .await
                {
                    Ok(DaemonResponse::StoreResponse { stored: true, .. }) => {
                        tracing::debug!(target: "hkask.mcp.telnyx.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.telnyx.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.telnyx.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }
}

#[tool_router(server_handler)]
impl TelnyxServer {
    #[tool(description = "Ping Telnyx API")]
    async fn telnyx_ping(&self) -> String {
        let span = ToolSpanGuard::new("telnyx_ping", &self.webid);
        let url = format!("{BASE_URL}/phone_numbers?page_size=1");
        span.finish(api_get(&self.client, "Telnyx", &url).await.map(|body| {
            serde_json::json!({
                "status": "ok",
                "message": "Telnyx API is reachable",
                "data": body,
            })
        }))
    }

    #[tool(description = "List phone numbers")]
    async fn telnyx_list_numbers(&self) -> String {
        let span = ToolSpanGuard::new("telnyx_list_numbers", &self.webid);
        let url = format!("{BASE_URL}/phone_numbers");
        span.finish(api_get(&self.client, "Telnyx", &url).await)
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
        span.finish(api_post(&self.client, "Telnyx", &url, &payload).await)
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
        let result = api_post(&self.client, "Telnyx", &url, &payload).await;
        self.record_experience(
            "telnyx_send_sms",
            &format!("{} -> {}", from, to),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"text_length": text.len()}),
        );
        span.finish(result)
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
        let result = api_post(&self.client, "Telnyx", &url, &payload).await;
        self.record_experience(
            "telnyx_make_call",
            &format!("{} -> {}", from, to),
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({}),
        );
        span.finish(result)
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
        span.finish(api_post(&self.client, "Telnyx", &url, &payload).await)
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
        span.ok_json(serde_json::json!({
            "voices": voices,
            "total": voices.len(),
            "source": "static catalog (Telnyx Call Control API docs)",
        }))
    }

    #[tool(
        description = "Send a message to the human user via the best available channel (SMS, WhatsApp, or call). Uses the replicant's own number as sender and the user's phone as recipient."
    )]
    async fn telnyx_notify_user(
        &self,
        Parameters(NotifyUserRequest { message, channel }): Parameters<NotifyUserRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("telnyx_notify_user", &self.webid);

        let user_phone = match &self.user_phone {
            Some(p) if !p.is_empty() => p.clone(),
            _ => {
                return span.error(
                    McpErrorKind::FailedPrecondition,
                    serde_json::json!({
                        "error": "User phone not configured. Set HKASK_USER_PHONE or complete onboarding."
                    }).to_string(),
                );
            }
        };

        let replicant_phone = std::env::var("HKASK_REPLICANT_PHONE").ok();

        match channel.as_str() {
            "whatsapp" => {
                let from = replicant_phone.unwrap_or_else(|| user_phone.clone());
                let url = format!("{BASE_URL}/messages");
                let payload = serde_json::json!({
                    "from": from,
                    "to": user_phone,
                    "type": "whatsapp",
                    "whatsapp": {
                        "content_type": "text",
                        "content": message,
                    },
                });
                span.finish(api_post(&self.client, "Telnyx", &url, &payload).await)
            }
            "call" => {
                let from = match replicant_phone {
                    Some(ref p) => p.clone(),
                    None => {
                        return span.error(
                            McpErrorKind::FailedPrecondition,
                            serde_json::json!({
                                "error": "Replicant phone not configured. Set HKASK_REPLICANT_PHONE or assign a number via onboarding."
                            }).to_string(),
                        );
                    }
                };
                let url = format!("{BASE_URL}/calls");
                let payload = serde_json::json!({
                    "from": from,
                    "to": user_phone,
                    "connection_id": "default",
                });
                span.finish(api_post(&self.client, "Telnyx", &url, &payload).await)
            }
            _ => {
                // Default: SMS
                let from = replicant_phone.unwrap_or_else(|| user_phone.clone());
                let url = format!("{BASE_URL}/messages");
                let payload = serde_json::json!({
                    "from": from,
                    "to": user_phone,
                    "text": message,
                });
                let result = api_post(&self.client, "Telnyx", &url, &payload).await;
                self.record_experience(
                    "telnyx_notify_user",
                    &format!("sms to user"),
                    if result.is_ok() { "success" } else { "error" },
                    serde_json::json!({"message_length": message.len()}),
                );
                span.finish(result)
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.telnyx", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(DaemonClient::new())
    } else {
        None
    };

    let user_phone = std::env::var("HKASK_USER_PHONE").ok();
    let user_name = std::env::var("HKASK_USER_NAME").ok();

    hkask_mcp::run_server(
        "hkask-mcp-telnyx",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let api_key = ctx
                .credentials
                .get("HKASK_TELNYX_API_KEY")
                .expect("required credential checked by run_stdio_server")
                .clone();
            TelnyxServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                api_key,
                user_phone.clone(),
                user_name.clone(),
            )
        },
        vec![hkask_mcp::CredentialRequirement::required(
            "HKASK_TELNYX_API_KEY",
            "Telnyx API key for messaging and number management",
        )],
    )
    .await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = DaemonClient::new();

    let auth = client.auth_query(replicant).await?;
    match auth {
        DaemonResponse::AuthResponse {
            authenticated: true,
            webid: Some(ref webid),
            ..
        } => {
            tracing::info!(target: "hkask.mcp.telnyx", replicant = %replicant, webid = %webid, "Replicant authenticated via daemon");
        }
        DaemonResponse::AuthResponse {
            authenticated: false,
            action: Some(ref action),
            ..
        } if action == "prompt_user" => {
            anyhow::bail!(
                "Replicant '{}' is not authenticated. Enter the replicant's passphrase in the hKask terminal.",
                replicant
            );
        }
        other => anyhow::bail!("Unexpected auth response: {:?}", other),
    }

    let assignment = client.assignment_query(replicant, "telnyx").await?;
    match assignment {
        DaemonResponse::AssignmentResponse { assigned: true } => {
            tracing::info!(target: "hkask.mcp.telnyx", replicant = %replicant, "Replicant assigned to telnyx role");
        }
        DaemonResponse::AssignmentResponse { assigned: false } => {
            anyhow::bail!(
                "Replicant '{}' is not assigned to the telnyx MCP role. Use 'kask replicant assign {} telnyx' to grant this role.",
                replicant,
                replicant
            );
        }
        other => anyhow::bail!("Unexpected assignment response: {:?}", other),
    }

    tracing::info!(target: "hkask.mcp.telnyx", replicant = %replicant, "P4 dual-gate verification complete");
    Ok(())
}
