//! hKask MCP Fal — Fal.ai API integration (image, video, audio generation)

use hkask_mcp::server::{
    McpToolError, McpToolOutput, ToolSpanGuard, classify_http_error, resolve_credential,
    validate_tool_url,
};
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

fn build_client() -> Result<reqwest::Client, McpToolError> {
    let key = resolve_credential("HKASK_FAL_API_KEY").map_err(|_| {
        McpToolError::failed_precondition("HKASK_FAL_API_KEY not found in keychain or environment")
    })?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Key {key}")
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
pub struct GenerateImageFastRequest {
    pub prompt: String,
    pub image_size: Option<String>,
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
    client: reqwest::Client,
}

impl FalServer {
    pub fn new(webid: WebID) -> Result<Self, anyhow::Error> {
        let client = build_client()?;
        Ok(Self { webid, client })
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
        let span = ToolSpanGuard::new("fal:ping", &self.webid);
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
                    span.ok(McpToolOutput::new(serde_json::json!({
                        "status": "ok",
                        "message": "Fal.ai API is reachable and authenticated",
                        "http_status": status.as_u16(),
                    }))
                    .to_json_string())
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
        let span = ToolSpanGuard::new("fal:generate_image", &self.webid);
        let body = serde_json::json!({
            "prompt": prompt,
            "image_size": image_size.unwrap_or_else(|| "1024x1024".to_string()),
            "num_images": num_images.unwrap_or(1),
        });
        match fal_post(&self.client, "fal-ai/flux/schnell", body).await {
            Ok(v) => span.ok(McpToolOutput::new(v).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Generate an image quickly")]
    async fn fal_generate_image_fast(
        &self,
        Parameters(GenerateImageFastRequest { prompt, image_size }): Parameters<
            GenerateImageFastRequest,
        >,
    ) -> String {
        let span = ToolSpanGuard::new("fal:generate_image_fast", &self.webid);
        let body = serde_json::json!({
            "prompt": prompt,
            "image_size": image_size.unwrap_or_else(|| "1024x1024".to_string()),
        });
        match fal_post(&self.client, "fal-ai/flux/schnell", body).await {
            Ok(v) => span.ok(McpToolOutput::new(v).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
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
        let span = ToolSpanGuard::new("fal:image_to_image", &self.webid);
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
        match fal_post(&self.client, "fal-ai/flux/dev/image-to-image", body).await {
            Ok(v) => span.ok(McpToolOutput::new(v).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Upscale an image")]
    async fn fal_upscale(
        &self,
        Parameters(UpscaleRequest { image_url, scale }): Parameters<UpscaleRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal:upscale", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let body = serde_json::json!({
            "image_url": image_url,
            "scale": scale.unwrap_or(4),
        });
        match fal_post(&self.client, "fal-ai/imageutils/u2net", body).await {
            Ok(v) => span.ok(McpToolOutput::new(v).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Generate a video from a prompt")]
    async fn fal_generate_video(
        &self,
        Parameters(GenerateVideoRequest { prompt, duration }): Parameters<GenerateVideoRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal:generate_video", &self.webid);
        let mut body = serde_json::json!({
            "prompt": prompt,
        });
        if let Some(d) = duration {
            body["duration"] = serde_json::json!(d);
        }
        match self.queue_post("fal-ai/minimax/video-01-live", body).await {
            Ok(v) => span.ok(McpToolOutput::new(v).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Generate music from a prompt")]
    async fn fal_generate_music(
        &self,
        Parameters(GenerateMusicRequest {
            prompt,
            duration_seconds,
        }): Parameters<GenerateMusicRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal:generate_music", &self.webid);
        let mut body = serde_json::json!({
            "prompt": prompt,
        });
        if let Some(d) = duration_seconds {
            body["duration"] = serde_json::json!(d);
        }
        match self.queue_post("fal-ai/stable-audio", body).await {
            Ok(v) => span.ok(McpToolOutput::new(v).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Transcribe audio to text")]
    async fn fal_whisper(
        &self,
        Parameters(WhisperRequest { audio_url }): Parameters<WhisperRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal:whisper", &self.webid);
        if let Err(e) = validate_tool_url(&audio_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let body = serde_json::json!({
            "audio_url": audio_url,
        });
        match fal_post(&self.client, "fal-ai/whisper", body).await {
            Ok(v) => span.ok(McpToolOutput::new(v).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Generate a caption for an image")]
    async fn fal_caption(
        &self,
        Parameters(CaptionRequest { image_url }): Parameters<CaptionRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal:caption", &self.webid);
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
        match fal_post(&self.client, "fal-ai/any-llm", body).await {
            Ok(v) => span.ok(McpToolOutput::new(v).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }

    #[tool(description = "Generate a 3D model from an image")]
    async fn fal_generate_3d(
        &self,
        Parameters(Generate3dRequest { image_url }): Parameters<Generate3dRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("fal:generate_3d", &self.webid);
        if let Err(e) = validate_tool_url(&image_url) {
            return span.error(e.kind, e.to_json_string());
        }
        let body = serde_json::json!({
            "image_url": image_url,
        });
        match self.queue_post("fal-ai/hunyuan3d", body).await {
            Ok(v) => span.ok(McpToolOutput::new(v).to_json_string()),
            Err(e) => span.error(e.kind, e.to_json_string()),
        }
    }
}

hkask_mcp::mcp_server_main!(
    "hkask-mcp-fal",
    FalServer,
    credentials: vec![hkask_mcp::CredentialRequirement::required(
        "HKASK_FAL_API_KEY",
        "Fal.ai API key for AI image generation",
    )]
);
