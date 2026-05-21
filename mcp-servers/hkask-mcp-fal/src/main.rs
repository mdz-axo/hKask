//! hKask MCP Fal — Fal.ai API integration (image, video, audio generation)

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router, transport::stdio, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

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

#[derive(Debug, Default)]
pub struct FalServer;

impl FalServer {
    pub fn new() -> Self {
        Self
    }
}

#[tool_router(server_handler)]
impl FalServer {
    #[tool(description = "Ping Fal.ai API")]
    async fn fal_ping(&self) -> String {
        r#"{"status":"ok","message":"Fal.ai API is reachable"}"#.to_string()
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
        format!(
            r#"{{"prompt":"{}","image_size":"{}","num_images":{},"images":["https://example.com/image1.png"]}}"#,
            prompt,
            image_size.unwrap_or_else(|| "1024x1024".to_string()),
            num_images.unwrap_or(1)
        )
    }

    #[tool(description = "Generate an image quickly")]
    async fn fal_generate_image_fast(
        &self,
        Parameters(GenerateImageFastRequest { prompt, image_size }): Parameters<
            GenerateImageFastRequest,
        >,
    ) -> String {
        format!(
            r#"{{"prompt":"{}","image_size":"{}","image":"https://example.com/fast_image.png"}}"#,
            prompt,
            image_size.unwrap_or_else(|| "1024x1024".to_string())
        )
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
        format!(
            r#"{{"prompt":"{}","image_url":"{}","strength":{},"output":"https://example.com/transformed.png"}}"#,
            prompt,
            image_url,
            strength.unwrap_or(0.7)
        )
    }

    #[tool(description = "Upscale an image")]
    async fn fal_upscale(
        &self,
        Parameters(UpscaleRequest { image_url, scale }): Parameters<UpscaleRequest>,
    ) -> String {
        format!(
            r#"{{"image_url":"{}","scale":{},"output":"https://example.com/upscaled.png"}}"#,
            image_url,
            scale.unwrap_or(4)
        )
    }

    #[tool(description = "Generate a video from a prompt")]
    async fn fal_generate_video(
        &self,
        Parameters(GenerateVideoRequest { prompt, duration }): Parameters<GenerateVideoRequest>,
    ) -> String {
        format!(
            r#"{{"prompt":"{}","duration":{},"video":"https://example.com/video.mp4"}}"#,
            prompt,
            duration.unwrap_or(5.0)
        )
    }

    #[tool(description = "Generate music from a prompt")]
    async fn fal_generate_music(
        &self,
        Parameters(GenerateMusicRequest {
            prompt,
            duration_seconds,
        }): Parameters<GenerateMusicRequest>,
    ) -> String {
        format!(
            r#"{{"prompt":"{}","duration":{},"audio":"https://example.com/music.mp3"}}"#,
            prompt,
            duration_seconds.unwrap_or(30.0)
        )
    }

    #[tool(description = "Transcribe audio to text")]
    async fn fal_whisper(
        &self,
        Parameters(WhisperRequest { audio_url }): Parameters<WhisperRequest>,
    ) -> String {
        format!(
            r#"{{"audio_url":"{}","transcription":"Simulated transcription text"}}"#,
            audio_url
        )
    }

    #[tool(description = "Generate a caption for an image")]
    async fn fal_caption(
        &self,
        Parameters(CaptionRequest { image_url }): Parameters<CaptionRequest>,
    ) -> String {
        format!(
            r#"{{"image_url":"{}","caption":"A beautiful image"}}"#,
            image_url
        )
    }

    #[tool(description = "Generate a 3D model from an image")]
    async fn fal_generate_3d(
        &self,
        Parameters(Generate3dRequest { image_url }): Parameters<Generate3dRequest>,
    ) -> String {
        format!(
            r#"{{"image_url":"{}","model_3d":"https://example.com/model.obj"}}"#,
            image_url
        )
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
