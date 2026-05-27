//! hKask MCP Fal — Fal.ai API integration (image, video, audio generation)

use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const SYNC_BASE: &str = "https://fal.run";
const QUEUE_BASE: &str = "https://queue.fal.run";
const MAX_POLL_SECS: u64 = 60;
const POLL_INTERVAL_SECS: u64 = 2;

fn err_json(msg: impl std::fmt::Display) -> String {
    serde_json::json!({"error": msg.to_string()}).to_string()
}

fn build_client() -> Result<reqwest::Client, anyhow::Error> {
    let key = std::env::var("HKASK_FAL_API_KEY")
        .map_err(|_| anyhow::anyhow!("HKASK_FAL_API_KEY environment variable is not set"))?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Key {key}").parse()?,
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(300))
        .build()?;

    Ok(client)
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
    client: reqwest::Client,
}

impl FalServer {
    pub fn new() -> Self {
        let client = build_client().expect("Failed to create Fal.ai API client");
        Self { client }
    }

    async fn sync_post(&self, endpoint: &str, body: Value) -> String {
        let url = format!("{SYNC_BASE}/{endpoint}");
        match self.client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(text) if status.is_success() => text,
                    Ok(text) => err_json(format!("Fal API error ({status}): {text}")),
                    Err(e) => err_json(format!("Failed to read response: {e}")),
                }
            }
            Err(e) => err_json(format!("Request failed: {e}")),
        }
    }

    async fn queue_post(&self, endpoint: &str, body: Value) -> String {
        let submit_url = format!("{QUEUE_BASE}/{endpoint}");

        let request_id = match self.client.post(&submit_url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<Value>().await {
                    Ok(v) if status.is_success() => v
                        .get("request_id")
                        .and_then(|r| r.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    Ok(v) => return v.to_string(),
                    Err(e) => return err_json(format!("Failed to parse submission: {e}")),
                }
            }
            Err(e) => return err_json(format!("Submit failed: {e}")),
        };

        let status_url = format!("{QUEUE_BASE}/{endpoint}/requests/{request_id}/status");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(MAX_POLL_SECS);

        loop {
            if tokio::time::Instant::now() > deadline {
                return err_json(format!(
                    "Polling timed out after {MAX_POLL_SECS}s (request_id: {request_id})"
                ));
            }

            match self.client.get(&status_url).send().await {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(v) => match v.get("status").and_then(|s| s.as_str()) {
                        Some("COMPLETED") => break,
                        Some("FAILED") => return err_json(format!("Job failed: {v}")),
                        _ => {}
                    },
                    Err(e) => return err_json(format!("Failed to parse status: {e}")),
                },
                Err(e) => return err_json(format!("Status check failed: {e}")),
            }

            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }

        let result_url = format!("{QUEUE_BASE}/{endpoint}/requests/{request_id}");
        match self.client.get(&result_url).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(text) if status.is_success() => text,
                    Ok(text) => err_json(format!("Result fetch error ({status}): {text}")),
                    Err(e) => err_json(format!("Failed to read result: {e}")),
                }
            }
            Err(e) => err_json(format!("Result fetch failed: {e}")),
        }
    }
}

impl Default for FalServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(server_handler)]
impl FalServer {
    #[tool(description = "Ping Fal.ai API to verify connectivity and authentication")]
    async fn fal_ping(&self) -> String {
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
                    err_json("Fal.ai API key is invalid or unauthorized")
                } else {
                    serde_json::json!({
                        "status": "ok",
                        "message": "Fal.ai API is reachable and authenticated",
                        "http_status": status.as_u16(),
                    })
                    .to_string()
                }
            }
            Err(e) => err_json(format!("Connection failed: {e}")),
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
        let body = serde_json::json!({
            "prompt": prompt,
            "image_size": image_size.unwrap_or_else(|| "1024x1024".to_string()),
            "num_images": num_images.unwrap_or(1),
        });
        self.sync_post("fal-ai/flux/schnell", body).await
    }

    #[tool(description = "Generate an image quickly")]
    async fn fal_generate_image_fast(
        &self,
        Parameters(GenerateImageFastRequest { prompt, image_size }): Parameters<
            GenerateImageFastRequest,
        >,
    ) -> String {
        let body = serde_json::json!({
            "prompt": prompt,
            "image_size": image_size.unwrap_or_else(|| "1024x1024".to_string()),
        });
        self.sync_post("fal-ai/flux/schnell", body).await
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
        let mut body = serde_json::json!({
            "prompt": prompt,
            "image_url": image_url,
        });
        if let Some(s) = strength {
            body["strength"] = serde_json::json!(s);
        }
        self.sync_post("fal-ai/flux/dev/image-to-image", body).await
    }

    #[tool(description = "Upscale an image")]
    async fn fal_upscale(
        &self,
        Parameters(UpscaleRequest { image_url, scale }): Parameters<UpscaleRequest>,
    ) -> String {
        let body = serde_json::json!({
            "image_url": image_url,
            "scale": scale.unwrap_or(4),
        });
        self.sync_post("fal-ai/imageutils/u2net", body).await
    }

    #[tool(description = "Generate a video from a prompt")]
    async fn fal_generate_video(
        &self,
        Parameters(GenerateVideoRequest { prompt, duration }): Parameters<GenerateVideoRequest>,
    ) -> String {
        let mut body = serde_json::json!({
            "prompt": prompt,
        });
        if let Some(d) = duration {
            body["duration"] = serde_json::json!(d);
        }
        self.queue_post("fal-ai/minimax/video-01-live", body).await
    }

    #[tool(description = "Generate music from a prompt")]
    async fn fal_generate_music(
        &self,
        Parameters(GenerateMusicRequest {
            prompt,
            duration_seconds,
        }): Parameters<GenerateMusicRequest>,
    ) -> String {
        let mut body = serde_json::json!({
            "prompt": prompt,
        });
        if let Some(d) = duration_seconds {
            body["duration"] = serde_json::json!(d);
        }
        self.queue_post("fal-ai/stable-audio", body).await
    }

    #[tool(description = "Transcribe audio to text")]
    async fn fal_whisper(
        &self,
        Parameters(WhisperRequest { audio_url }): Parameters<WhisperRequest>,
    ) -> String {
        let body = serde_json::json!({
            "audio_url": audio_url,
        });
        self.sync_post("fal-ai/whisper", body).await
    }

    #[tool(description = "Generate a caption for an image")]
    async fn fal_caption(
        &self,
        Parameters(CaptionRequest { image_url }): Parameters<CaptionRequest>,
    ) -> String {
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
        self.sync_post("fal-ai/any-llm", body).await
    }

    #[tool(description = "Generate a 3D model from an image")]
    async fn fal_generate_3d(
        &self,
        Parameters(Generate3dRequest { image_url }): Parameters<Generate3dRequest>,
    ) -> String {
        let body = serde_json::json!({
            "image_url": image_url,
        });
        self.queue_post("fal-ai/hunyuan3d", body).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = FalServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-fal started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
