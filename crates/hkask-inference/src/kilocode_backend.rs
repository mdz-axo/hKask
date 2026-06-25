//! KiloCode backend — cloud inference via Kilo Gateway OpenAI-compatible API.
//!
//! Kilo Gateway exposes `/chat/completions` and `/models` at
//! `https://api.kilo.ai/api/gateway`. Requires Bearer token
//! authentication via `KILOCODE_API_KEY`.
//!
//! Kilo Gateway provides a unified API to 500+ models from
//! multiple providers through a single endpoint, with server-side
//! auto-routing via `kilo-auto/*` virtual models.

use crate::chat_protocol::{
    ChatResponse, FusionPlugin, build_chat_request, chat_response_to_result, parse_sse_stream,
    validate_prompt,
};
use crate::config::InferenceConfig;
use futures_util::StreamExt;
use hkask_ports::{ChatToolDefinition, InferenceError, InferenceResult, InferenceStreamChunk};
use hkask_types::template::LLMParameters;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// KiloCode backend for chat completions and model listing via Kilo Gateway.
pub struct KiloCodeBackend {
    base_url: String,
    api_key: String,
    client: Arc<reqwest::Client>,
    /// Optional mode header for Kilo auto-routing (x-kilocode-mode).
    /// Set from FusionConfig.kilo_mode when fusion provider is KiloCode.
    kilocode_mode: Option<String>,
}

/// A model entry returned by Kilo Gateway's `/models` endpoint.
#[derive(Debug, Deserialize, Serialize)]
pub struct KiloCodeModel {
    pub id: String,
    pub object: Option<String>,
    pub created: Option<u64>,
    #[serde(default)]
    pub owned_by: Option<String>,
    /// Model display name (Kilo Gateway extension).
    #[serde(default)]
    pub name: Option<String>,
    /// Context length in tokens (Kilo Gateway extension).
    #[serde(default)]
    pub context_length: Option<u64>,
    /// Pricing information (Kilo Gateway extension).
    #[serde(default)]
    pub pricing: Option<KiloCodePricing>,
}

/// Pricing info from Kilo Gateway model listing.
#[derive(Debug, Deserialize, Serialize)]
pub struct KiloCodePricing {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub completion: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KiloCodeModelList {
    data: Vec<KiloCodeModel>,
}

impl KiloCodeBackend {
    /// Create a new KiloCode backend from inference config.
    ///
    /// Returns an error if `kilocode_api_key` is empty.
    ///
    /// expect: "The system creates provider membranes requiring valid API keys"
    /// \[P4\] Motivating: Clear Boundaries — KiloCode provider membrane requires valid API key
    /// pre:  config.kilocode_api_key is set
    /// post: returns KiloCodeBackend with configured HTTP client
    pub fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        if config.kilocode_api_key.is_empty() {
            return Err(InferenceError::Connection(
                "KiloCode API key not configured (set KILOCODE_API_KEY)".into(),
            ));
        }
        let client = config
            .build_client()
            .map(Arc::new)
            .map_err(InferenceError::Connection)?;
        // Extract Kilo auto-routing mode from fusion config
        let kilocode_mode = config
            .fusion
            .as_ref()
            .and_then(|f| f.kilo_mode_header().map(|s| s.to_string()));
        Ok(Self {
            base_url: config.kilocode_base_url.clone(),
            api_key: config.kilocode_api_key.clone(),
            client,
            kilocode_mode,
        })
    }

    /// Send a chat completion request to Kilo Gateway.
    ///
    /// expect: "The system regulates text/image/speech generation through provider membranes"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated text generation
    /// pre:  model is a valid Kilo Gateway model name
    /// pre:  prompt is non-empty (validated by validate_prompt)
    /// pre:  params is a valid LLMParameters
    /// post: returns Ok(InferenceResult) with generated text, model, usage stats
    /// post: if connection fails → Err(InferenceError::Connection)
    /// post: if prompt is empty → Err(InferenceError::Generation)
    pub async fn generate(
        &self,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
        plugins: Option<Vec<FusionPlugin>>,
    ) -> Result<InferenceResult, InferenceError> {
        validate_prompt(prompt)?;
        let tools = tools.map(|t| t.to_vec());
        let request = build_chat_request(
            model,
            prompt,
            None,
            params,
            Some(false),
            Some(5),
            tools,
            plugins,
        );

        let mut req = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key));
        if let Some(ref mode) = self.kilocode_mode {
            req = req.header("x-kilocode-mode", mode);
        }
        let response =
            req.json(&request).send().await.map_err(|e| {
                InferenceError::Connection(format!("KiloCode request failed: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "KiloCode returned {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("KiloCode JSON parse: {}", e)))?;

        let result = chat_response_to_result(chat_response)?;
        info!(
            target: "cns.inference",
            provider = "KC",
            model = %result.model,
            tokens = result.usage.total_tokens,
            finish_reason = %result.finish_reason,
            "KiloCode inference completed"
        );
        Ok(result)
    }

    /// Stream a chat completion from Kilo Gateway via SSE.
    ///
    /// expect: "The system regulates text/image/speech generation through provider membranes"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated streaming text generation
    /// pre:  model is a valid Kilo Gateway model name
    /// post: returns stream of inference chunks
    pub fn generate_stream(
        &self,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
    ) -> std::pin::Pin<
        Box<
            dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>>
                + Send
                + '_,
        >,
    > {
        let auth = format!("Bearer {}", self.api_key);
        let base_url = self.base_url.clone();
        let client = Arc::clone(&self.client);
        let model = model.to_string();
        let prompt = prompt.to_string();
        let params = params.clone();
        let mode = self.kilocode_mode.clone();

        Box::pin(
            Box::pin(futures_util::stream::once(async move {
                let request = build_chat_request(
                    &model,
                    &prompt,
                    None,
                    &params,
                    Some(true),
                    None,
                    None::<Vec<ChatToolDefinition>>,
                    None::<Vec<FusionPlugin>>,
                );

                let mut req = client
                    .post(format!("{base_url}/chat/completions"))
                    .header("Authorization", &auth);
                if let Some(ref m) = mode {
                    req = req.header("x-kilocode-mode", m);
                }

                let response = match req.json(&request).send().await.map_err(|e| {
                    InferenceError::Connection(format!("KiloCode stream request failed: {e}"))
                }) {
                    Ok(r) => r,
                    Err(e) => return vec![Err(e)],
                };

                let status = response.status();
                if !status.is_success() {
                    let error_text = response.text().await.unwrap_or_default();
                    return vec![Err(InferenceError::Connection(format!(
                        "KiloCode streaming status {status}: {error_text}"
                    )))];
                }

                let body = match response.text().await {
                    Ok(b) => b,
                    Err(e) => {
                        return vec![Err(InferenceError::Connection(format!(
                            "KiloCode stream read error: {e}"
                        )))];
                    }
                };

                parse_sse_stream(&body, &model)
            }))
            .flat_map(|chunks| futures_util::stream::iter(chunks)),
        )
    }

    /// Vision/multimodal inference with base64-encoded images.
    ///
    /// expect: "The system regulates text/image/speech generation through provider membranes"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — regulated multimodal generation
    /// pre:  model is a valid Kilo Gateway vision-capable model name
    /// pre:  prompt is non-empty
    /// pre:  images is non-empty (at least one base64-encoded image)
    /// pre:  params is a valid LLMParameters
    /// post: returns Ok(InferenceResult) with vision-generated text
    /// post: if connection fails → Err(InferenceError::Connection)
    pub async fn generate_vision(
        &self,
        model: &str,
        prompt: &str,
        images: &[String],
        params: &LLMParameters,
    ) -> Result<InferenceResult, InferenceError> {
        validate_prompt(prompt)?;
        let request = build_chat_request(
            model,
            prompt,
            Some(images.to_vec()),
            params,
            Some(false),
            Some(5),
            None::<Vec<ChatToolDefinition>>,
            None::<Vec<FusionPlugin>>,
        );

        let mut req = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key));
        if let Some(ref mode) = self.kilocode_mode {
            req = req.header("x-kilocode-mode", mode);
        }
        let response = req.json(&request).send().await.map_err(|e| {
            InferenceError::Connection(format!("KiloCode vision request failed: {}", e))
        })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "KiloCode returned {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| InferenceError::Json(format!("KiloCode vision JSON parse: {}", e)))?;

        chat_response_to_result(chat_response)
    }

    /// List available models from Kilo Gateway.
    ///
    /// expect: "I can discover available models across providers"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — model variety discovery
    /// pre:  self.client and self.base_url are initialized
    /// post: returns Ok(`Vec<KiloCodeModel>`) with all available models
    /// post: if API returns non-success → Err(InferenceError::Connection)
    /// post: if connection fails → Err(InferenceError::Connection)
    pub async fn list_models(&self) -> Result<Vec<KiloCodeModel>, InferenceError> {
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                InferenceError::Connection(format!("KiloCode models request failed: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(InferenceError::Connection(format!(
                "KiloCode models error {}: {}",
                status, body
            )));
        }

        let list: KiloCodeModelList = response.json().await.map_err(|e| {
            InferenceError::Connection(format!("KiloCode models parse error: {}", e))
        })?;

        info!(
            target: "hkask.inference.kilocode",
            count = list.data.len(),
            "Fetched KiloCode model list"
        );

        Ok(list.data)
    }
}
