//! hKask MCP Fal.ai — Media generation (image, video, audio, 3D)

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter},
    model::*,
    schemars, tool, tool_router, tool_handler,
};
use rmcp::handler::server::wrapper::parameters::Parameters;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use secrecy::Secret;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const FAL_API_BASE: &str = "https://fal.rest";

/// Fal.ai server implementation
pub struct FalServer {
    tool_router: ToolRouter<FalServer>,
    client: Client,
    api_key: Option<Secret<String>>,
}

impl FalServer {
    pub fn new() -> Self {
        let api_key = std::env::var("FAL_KEY").ok().map(Secret::new);
        let client = Client::builder().build().unwrap_or_default();

        Self {
            tool_router: Self::tool_router(),
            client,
            api_key,
        }
    }

    fn get_headers(&self) -> std::collections::HashMap<&str, String> {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization", "Key ".to_string() + self.api_key.as_ref().map(|k| k.expose_secret()).unwrap_or(&"".to_string()));
        headers
    }
}

#[tool_router]
impl FalServer {
    #[tool(description = "Ping the Fal.ai server")]
    async fn fal_ping(&self) -> String {
        serde_json::json!({
            "status": "ok",
            "server": "hkask-mcp-fal",
            "version": SERVER_VERSION,
            "api_key_configured": self.api_key.is_some()
        }).to_string()
    }

    #[tool(description = "Generate image from text prompt using Flux")]
    async fn fal_generate_image(&self, prompt: String, image_size: Option<String>, num_images: Option<u32>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FAL_KEY not configured" }).to_string();
        }

        let image_size = image_size.unwrap_or_else(|| "landscape_4_3".to_string());
        let num_images = num_images.unwrap_or(1);
        let url = format!("{}/fal-ai/flux/dev", FAL_API_BASE);
        let payload = serde_json::json!({
            "prompt": prompt,
            "image_size": image_size,
            "num_images": num_images
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

    #[tool(description = "Generate image fast using Flux Schnell")]
    async fn fal_generate_image_fast(&self, prompt: String, image_size: Option<String>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FAL_KEY not configured" }).to_string();
        }

        let image_size = image_size.unwrap_or_else(|| "landscape_4_3".to_string());
        let url = format!("{}/fal-ai/flux/schnell", FAL_API_BASE);
        let payload = serde_json::json!({
            "prompt": prompt,
            "image_size": image_size
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

    #[tool(description = "Image-to-image transformation")]
    async fn fal_image_to_image(&self, prompt: String, image_url: String, strength: Option<f64>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FAL_KEY not configured" }).to_string();
        }

        let strength = strength.unwrap_or(0.75);
        let url = format!("{}/fal-ai/flux/dev/image-to-image", FAL_API_BASE);
        let payload = serde_json::json!({
            "prompt": prompt,
            "image_url": image_url,
            "strength": strength
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

    #[tool(description = "Upscale image using Creative Upscaler")]
    async fn fal_upscale(&self, image_url: String, scale: Option<f64>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FAL_KEY not configured" }).to_string();
        }

        let scale = scale.unwrap_or(2.0);
        let url = format!("{}/fal-ai/creative-upscaler", FAL_API_BASE);
        let payload = serde_json::json!({
            "image_url": image_url,
            "scale": scale
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

    #[tool(description = "Generate video from text using Kling")]
    async fn fal_generate_video(&self, prompt: String, duration: Option<String>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FAL_KEY not configured" }).to_string();
        }

        let duration = duration.unwrap_or_else(|| "5".to_string());
        let url = format!("{}/fal-ai/kling-video/v1/standard/text-to-video", FAL_API_BASE);
        let payload = serde_json::json!({
            "prompt": prompt,
            "duration": duration
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

    #[tool(description = "Generate music from text using MusicGen")]
    async fn fal_generate_music(&self, prompt: String, duration_seconds: Option<f64>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FAL_KEY not configured" }).to_string();
        }

        let duration = duration_seconds.unwrap_or(10.0);
        let url = format!("{}/fal-ai/musicgen", FAL_API_BASE);
        let payload = serde_json::json!({
            "prompt": prompt,
            "duration": duration
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

    #[tool(description = "Speech-to-text using Whisper")]
    async fn fal_whisper(&self, audio_url: String) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FAL_KEY not configured" }).to_string();
        }

        let url = format!("{}/fal-ai/whisper", FAL_API_BASE);
        let payload = serde_json::json!({
            "audio_url": audio_url
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

    #[tool(description = "Image caption using LLaVA Next")]
    async fn fal_caption(&self, image_url: String) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FAL_KEY not configured" }).to_string();
        }

        let url = format!("{}/fal-ai/llava-next", FAL_API_BASE);
        let payload = serde_json::json!({
            "image_url": image_url,
            "task": "caption"
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

    #[tool(description = "Text-to-speech using F5 TTS")]
    async fn fal_tts(&self, text: String, voice: Option<String>) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FAL_KEY not configured" }).to_string();
        }

        let voice = voice.unwrap_or_else(|| "default".to_string());
        let url = format!("{}/fal-ai/f5-tts", FAL_API_BASE);
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

    #[tool(description = "Generate 3D model using TRELLIS")]
    async fn fal_generate_3d(&self, image_url: String) -> String {
        if self.api_key.is_none() {
            return serde_json::json!({ "error": "FAL_KEY not configured" }).to_string();
        }

        let url = format!("{}/fal-ai/trellis", FAL_API_BASE);
        let payload = serde_json::json!({
            "image_url": image_url
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
}

#[tool_handler]
impl ServerHandler for FalServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = FalServer::new();
    let service = server.serve_stdio();
    tracing::info!("hkask-mcp-fal MCP server started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
