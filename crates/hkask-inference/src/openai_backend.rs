//! Shared OpenAI-compatible chat completion logic.
//!
//! All five backends (DeepInfra, Together, OpenRouter, KiloCode, Fal)
//! use [`openai_compatible_generate`] for their `generate()` method.
//! The function parameterizes the chat endpoint path and auth header
//! prefix to accommodate provider-specific differences:
//!
//! | Provider   | Chat path               | Auth header     |
//! |------------|-------------------------|-----------------|
//! | DeepInfra  | `/v1/chat/completions`  | `Bearer`        |
//! | Together   | `/v1/chat/completions`  | `Bearer`        |
//! | OpenRouter | `/v1/chat/completions`  | `Bearer`        |
//! | KiloCode   | `/chat/completions`     | `Bearer`        |
//! | Fal        | `/v1/chat/completions`  | `Key`           |

use crate::chat_protocol::build_chat_request;
use crate::chat_protocol::{ChatResponse, FusionPlugin, chat_response_to_result, validate_prompt};
use crate::config::ProviderConfig;
use hkask_ports::{ChatToolDefinition, InferenceError, InferenceResult};
use hkask_types::template::LLMParameters;
use reqwest::Client;

/// Parameterized OpenAI-compatible chat completion.
///
/// `chat_path` is the URL path appended to `config.base_url` (e.g., `/v1/chat/completions`).
/// `auth_prefix` is the `Authorization` header prefix (e.g., `"Bearer"` or `"Key"`).
/// `provider_code` is the short provider identifier used in logs and error messages.
///
/// expect: "The system regulates text/image/speech generation through provider membranes"
/// \[P9\] Motivating: Homeostatic Self-Regulation — shared regulated generation for OpenAI-compatible backends
/// pre:  model is a valid provider model name
/// pre:  prompt is non-empty (validated by validate_prompt)
/// pre:  params is a valid LLMParameters
/// post: returns Ok(InferenceResult) with generated text, model, usage stats
/// post: if connection fails → Err(InferenceError::Connection)
/// post: if prompt is empty → Err(InferenceError::Generation)
#[allow(clippy::too_many_arguments)]
pub async fn openai_compatible_generate(
    client: &Client,
    config: &ProviderConfig,
    model: &str,
    prompt: &str,
    params: &LLMParameters,
    tools: Option<&[ChatToolDefinition]>,
    chat_path: &str,
    auth_prefix: &str,
    provider_code: &str,
) -> Result<InferenceResult, InferenceError> {
    validate_prompt(prompt)?;
    let tools = tools.map(|t| t.to_vec());
    let request = build_chat_request(
        model,
        prompt,
        params,
        Some(false),
        None,
        tools,
        None<Vec<FusionPlugin>>,
    );

    let response = client
        .post(format!("{}{}", config.base_url, chat_path))
        .header(
            "Authorization",
            format!("{} {}", auth_prefix, config.api_key),
        )
        .json(&request)
        .send()
        .await
        .map_err(|e| InferenceError::Connection(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(InferenceError::Connection(format!(
            "{} status {}: {}",
            provider_code, status, error_text
        )));
    }

    let chat_response: ChatResponse = response
        .json()
        .await
        .map_err(|e| InferenceError::Json(format!("{} JSON parse: {}", provider_code, e)))?;

    let result = chat_response_to_result(chat_response)?;
    tracing::info!(
        target: "cns.inference",
        provider = provider_code,
        model = %result.model,
        tokens = result.usage.total_tokens,
        finish_reason = %result.finish_reason,
        "{} inference completed",
        provider_code
    );
    Ok(result)
}
