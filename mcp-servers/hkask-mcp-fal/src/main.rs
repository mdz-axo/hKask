//! hKask MCP Fal — Fal.ai API integration (image, video, audio generation)

use hkask_mcp::server::{McpToolError, ToolSpanGuard, classify_http_error, validate_tool_url};
use hkask_mcp::{DaemonClient, DaemonResponse};
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

const SYNC_BASE: &str = "https://fal.run";
const QUEUE_BASE: &str = "https://queue.fal.run";
const MAX_POLL_SECS: u64 = 60;
const POLL_INTERVAL_SECS: u64 = 2;

fn build_client(api_key: &str) -> Result<reqwest::Client, McpToolError> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Key {api_key}")
            .parse()
            .map_err(|e| McpToolError::internal(format!("Invalid header: {e}")))?,
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| McpToolError::internal(format!("Failed to build HTTP client: {e}")))
}

async fn fal_post(
    client: &reqwest::Client,
    endpoint: &str,
    body: Value,
) -> Result<Value, McpToolError> {
    let url = format!("{SYNC_BASE}/{endpoint}");
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| McpToolError::unavailable(format!("Request failed: {e}")))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(classify_http_error("Fal", status, &text));
    }
    serde_json::from_str(&text)
        .map_err(|e| McpToolError::internal(format!("Failed to parse response: {e}")))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateImageRequest {
    pub prompt: String,
    pub image_size: Option<String>,
    pub num_images: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImageToImageRequest {
    pub prompt: String,
    pub image_url: String,
    pub strength: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpscaleRequest {
    pub image_url: String,
    pub scale: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateVideoRequest {
    pub prompt: String,
    pub duration: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateMusicRequest {
    pub prompt: String,
    pub duration_seconds: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WhisperRequest {
    pub audio_url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CaptionRequest {
    pub image_url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Generate3dRequest {
    pub image_url: String,
}

pub struct FalServer {
    webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    daemon: Option<DaemonClient>,
    client: reqwest::Client,
}

impl FalServer {
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<DaemonClient>,
        api_key: String,
    ) -> Result<Self, anyhow::Error> {
        let client = build_client(&api_key)?;
        Ok(Self {
            webid,
            replicant,
            daemon,
            client,
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
                        tracing::debug!(target: "hkask.mcp.fal.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.fal.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.fal.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }

    async fn queue_post(&self, endpoint: &str, body: Value) -> Result<Value, McpToolError> {
        let submit_url = format!("{QUEUE_BASE}/{endpoint}");

        let resp = self
            .client
            .post(&submit_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| McpToolError::unavailable(format!("Submit failed: {e}")))?;

        let status = resp.status();
        let v: Value = resp
            .json()
            .await
            .map_err(|e| McpToolError::internal(format!("Failed to parse submission: {e}")))?;

        if !status.is_success() {
            return Err(classify_http_error("Fal", status, &v.to_string()));
        }

        let request_id = v
            .get("request_id")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown")
            .to_string();

        let status_url = format!("{QUEUE_BASE}/{endpoint}/requests/{request_id}/status");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(MAX_POLL_SECS);

        loop {
            if tokio::time::Instant::now() > deadline {
                return Err(McpToolError::timeout(format!(
                    "Polling timed out after {MAX_POLL_SECS}s (request_id: {request_id})"
                )));
            }

            match self.client.get(&status_url).send().await {
                Ok(resp) => {
                    let v: Value = resp.json().await.map_err(|e| {
                        McpToolError::internal(format!("Failed to parse status: {e}"))
                    })?;
                    match v.get("status").and_then(|s| s.as_str()) {
                        Some("COMPLETED") => break,
                        Some("FAILED") => {
                            return Err(McpToolError::internal(format!("Job failed: {v}")));
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    return Err(McpToolError::unavailable(format!(
                        "Status check failed: {e}"
                    )));
                }
            }

            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }

        let result_url = format!("{QUEUE_BASE}/{endpoint}/requests/{request_id}");
        let resp = self
            .client
            .get(&result_url)
            .send()
            .await
            .map_err(|e| McpToolError::unavailable(format!("Result fetch failed: {e}")))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_http_error("Fal", status, &text));
        }

        serde_json::from_str(&text)
            .map_err(|e| McpToolError::internal(format!("Failed to parse result: {e}")))
    }
}

#[tool_router(server_handler)]
impl FalServer {
    #[tool(description = "Ping Fal.ai API to verify connectivity and authentication")]
    async fn fal_ping(&self) -> String {
        let span = ToolSpanGuard::new("fal_ping", &self.webid);
        let url = format!("{SYNC_BASE}/fal-ai/flux/schnell");
        match self
            .client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status == reqwest::StatusCode::UNAUTHORIZED
                    || status == reqwest::StatusCode::FORBIDDEN
                {
                    let err = McpToolError::permission_denied(
                        "Fal.ai API key is invalid or unauthorized",
                    );
                    span.error(err.kind, err.to_json_string())
                } else {
                    span.ok_json(serde_json::json!({
                        "status": "ok",
                        "message": "Fal.ai API is reachable and authenticated",
                        "http_status": status.as_u16(),
                    }))
                }
            }
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Connection failed: {e}")).to_json_string(),
            ),
        }
    }

    #[tool(description = "Generate an image from a prompt")]
    async fn fal_generate_image(
        &self,
        Parameters(GenerateImageRequest {
            prompt,
            image_size,
            num_images,
        }): Parameters<GenerateImageRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_generate_image", &self.webid);
        let size = image_size.clone();
        let body = serde_json::json!({
            "prompt": prompt,
            "image_size": image_size.unwrap_or_else(|| "1024x1024".to_string()),
            "num_images": num_images.unwrap_or(1),
        });
        let result = fal_post(&self.client, "fal-ai/flux/schnell", body).await;
        self.record_experience(
            "fal_generate_image",
            &prompt,
            if result.is_ok() { "success" } else { "error" },
            serde_json::json!({"image_size": size, "num_images": num_images}),
        );
        span.finish(result)
    }

    #[tool(description = "Transform an image with a prompt")]
    async fn fal_image_to_image(
        &self,
        Parameters(ImageToImageRequest {
            prompt,
            image_url,
            strength,
        }): Parameters<ImageToImageRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_image_to_image", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let mut body = serde_json::json!({
            "prompt": prompt,
            "image_url": image_url,
        });
        if let Some(s) = strength {
            body["strength"] = serde_json::json!(s);
        }
        span.finish(fal_post(&self.client, "fal-ai/flux/dev/image-to-image", body).await)
    }

    #[tool(description = "Upscale an image")]
    async fn fal_upscale(
        &self,
        Parameters(UpscaleRequest { image_url, scale }): Parameters<UpscaleRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_upscale", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let body = serde_json::json!({
            "image_url": image_url,
            "scale": scale.unwrap_or(4),
        });
        span.finish(fal_post(&self.client, "fal-ai/imageutils/u2net", body).await)
    }

    #[tool(description = "Generate a video from a prompt")]
    async fn fal_generate_video(
        &self,
        Parameters(GenerateVideoRequest { prompt, duration }): Parameters<GenerateVideoRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_generate_video", &self.webid);
        let mut body = serde_json::json!({
            "prompt": prompt,
        });
        if let Some(d) = duration {
            body["duration"] = serde_json::json!(d);
        }
        span.finish(self.queue_post("fal-ai/minimax/video-01-live", body).await)
    }

    #[tool(description = "Generate music from a prompt")]
    async fn fal_generate_music(
        &self,
        Parameters(GenerateMusicRequest {
            prompt,
            duration_seconds,
        }): Parameters<GenerateMusicRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_generate_music", &self.webid);
        let mut body = serde_json::json!({
            "prompt": prompt,
        });
        if let Some(d) = duration_seconds {
            body["duration"] = serde_json::json!(d);
        }
        span.finish(self.queue_post("fal-ai/stable-audio", body).await)
    }

    #[tool(description = "Transcribe audio to text")]
    async fn fal_whisper(
        &self,
        Parameters(WhisperRequest { audio_url }): Parameters<WhisperRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_whisper", &self.webid);
        if let Err(e) = validate_tool_url(&audio_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let body = serde_json::json!({
            "audio_url": audio_url,
        });
        span.finish(fal_post(&self.client, "fal-ai/whisper", body).await)
    }

    #[tool(description = "Generate a caption for an image")]
    async fn fal_caption(
        &self,
        Parameters(CaptionRequest { image_url }): Parameters<CaptionRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_caption", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let body = serde_json::json!({
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "Provide a detailed caption for this image."},
                        {"type": "image_url", "image_url": {"url": image_url}}
                    ]
                }
            ]
        });
        span.finish(fal_post(&self.client, "fal-ai/any-llm", body).await)
    }

    #[tool(description = "Generate a 3D model from an image")]
    async fn fal_generate_3d(
        &self,
        Parameters(Generate3dRequest { image_url }): Parameters<Generate3dRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal_generate_3d", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let body = serde_json::json!({
            "image_url": image_url,
        });
        span.finish(self.queue_post("fal-ai/hunyuan3d", body).await)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.fal", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(DaemonClient::new())
    } else {
        None
    };

    hkask_mcp::run_server(
        "hkask-mcp-fal",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let api_key = ctx
                .credentials
                .get("HKASK_FAL_API_KEY")
                .expect("required credential checked by run_stdio_server")
                .clone();
            FalServer::new(ctx.webid, replicant.clone(), daemon_client.clone(), api_key)
        },
        vec![hkask_mcp::CredentialRequirement::required(
            "HKASK_FAL_API_KEY",
            "Fal.ai API key for AI image generation",
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
            tracing::info!(target: "hkask.mcp.fal", replicant = %replicant, webid = %webid, "Replicant authenticated via daemon");
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

    let assignment = client.assignment_query(replicant, "fal").await?;
    match assignment {
        DaemonResponse::AssignmentResponse { assigned: true } => {
            tracing::info!(target: "hkask.mcp.fal", replicant = %replicant, "Replicant assigned to fal role");
        }
        DaemonResponse::AssignmentResponse { assigned: false } => {
            anyhow::bail!(
                "Replicant '{}' is not assigned to the fal MCP role. Use 'kask replicant assign {} fal' to grant this role.",
                replicant,
                replicant
            );
        }
        other => anyhow::bail!("Unexpected assignment response: {:?}", other),
    }

    tracing::info!(target: "hkask.mcp.fal", replicant = %replicant, "P4 dual-gate verification complete");
    Ok(())
}
