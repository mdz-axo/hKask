//! Shared OpenAI-compatible chat completion protocol types and helpers.
//!
//! All backends (DeepInfra, Together AI, fal.ai) speak the same
//! `/v1/chat/completions` wire format. This module provides the shared
//! request/response types and helper functions used by all backends.
//!
//! These are free functions, not a trait — two backends don't justify
//! an abstraction layer. Each backend owns its HTTP client, auth, and
//! model listing endpoint independently.

use futures_util::StreamExt;
use hkask_ports::{
    InferenceError, InferenceResult, InferenceStreamChunk, InferenceUsage, StructuredToolCall,
    TokenProb, TokenProbability,
};
use hkask_types::template::LLMParameters;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
fn default_enable_thinking() -> bool {
    true
}

#[allow(dead_code)]
fn is_true(b: &bool) -> bool {
    *b
}

// ── Request types ────────────────────────────────────────────────────────────

/// OpenAI-compatible chat completion request body.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: i32,
    pub min_p: f32,
    pub typical_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    pub max_tokens: i32,
    pub seed: Option<u64>,
    pub n_probs: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Enable thinking/reasoning mode.
    /// Default true. Set to false for condenser/summarization tasks to prevent
    /// the model from spending output tokens on internal reasoning.
    /// Skipped from serialization when true (most models don't need it).
    #[serde(default = "default_enable_thinking", skip_serializing_if = "is_true")]
    pub enable_thinking: bool,
}

/// A single message in the chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// Base64-encoded images for multimodal/vision requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

/// Build an OpenAI-compatible chat completion request from hKask parameters.
///
/// `stream: false` is explicit in non-streaming calls to prevent chunked
/// transfer encoding from confusing JSON parsers.
///
/// expect: "The system constructs and validates regulated LLM requests"
/// \[P9\] Motivating: Homeostatic Self-Regulation — constructs regulated LLM request payload
/// pre:  model is non-empty, prompt is non-empty
/// post: returns serde_json::Value with model, messages, and parameters
pub fn build_chat_request(
    model: &str,
    prompt: &str,
    images: Option<Vec<String>>,
    params: &LLMParameters,
    stream: Option<bool>,
    n_probs: Option<i32>,
) -> ChatRequest {
    ChatRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            images,
        }],
        temperature: params.temperature,
        top_p: params.top_p,
        top_k: params.top_k as i32,
        min_p: params.min_p,
        typical_p: params.typical_p,
        frequency_penalty: params.frequency_penalty,
        presence_penalty: params.presence_penalty,
        max_tokens: params.max_tokens as i32,
        seed: params.seed,
        n_probs,
        stream,
        enable_thinking: !params.disable_thinking,
    }
}

// ── Response types ───────────────────────────────────────────────────────────

/// OpenAI-compatible chat completion response.
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: ChatUsage,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ChatResponseMessage,
    pub finish_reason: String,
    #[serde(default, rename = "token_probs")]
    pub token_probs: Option<Vec<RawTokenProb>>,
    #[serde(default)]
    pub tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponseMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ── Token probability types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RawTokenProb {
    pub token: String,
    pub prob: f64,
    #[serde(default)]
    pub top_k: Vec<RawTokenProbTopK>,
}

#[derive(Debug, Deserialize)]
pub struct RawTokenProbTopK {
    pub token: String,
    pub prob: f64,
}

// ── Tool call types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RawToolCall {
    pub id: Option<String>,
    #[serde(rename = "function")]
    pub function: RawFunctionCall,
}

#[derive(Debug, Deserialize)]
pub struct RawFunctionCall {
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

// ── SSE streaming types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StreamChunk {
    pub choices: Vec<StreamChoice>,
    pub model: String,
    #[serde(default)]
    pub usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct StreamDelta {
    #[serde(default)]
    pub content: Option<String>,
}

// ── Conversion helpers ──────────────────────────────────────────────────────

/// Map raw tool calls to `StructuredToolCall`.
///
/// Tool call names use `server/tool` convention (e.g., `memory/recall`).
/// If no `/` separator, server is empty and the full name is the tool.
/// Map raw tool calls from API response to structured ToolCall format.
///
/// expect: "The system constructs and validates regulated LLM requests"
/// \[P9\] Motivating: Homeostatic Self-Regulation — structured tool-call results for routing
/// pre:  calls is a valid slice of RawToolCall
/// post: returns `Vec<StructuredToolCall>` with parsed arguments
pub fn map_tool_calls(calls: &[RawToolCall]) -> Vec<StructuredToolCall> {
    calls
        .iter()
        .map(|tc| {
            let (server, tool) = tc
                .function
                .name
                .split_once('/')
                .map(|(s, t)| (s.to_string(), t.to_string()))
                .unwrap_or_else(|| (String::new(), tc.function.name.clone()));
            StructuredToolCall {
                server,
                tool,
                args: tc.function.arguments.clone(),
                call_id: Some(tc.id.clone().unwrap_or_default()),
            }
        })
        .collect()
}

/// Convert raw token probabilities to `TokenProbability`.
/// Map raw token probabilities to structured TokenProbability format.
///
/// expect: "The system constructs and validates regulated LLM requests"
/// \[P9\] Motivating: Homeostatic Self-Regulation — token probability metadata for monitoring
/// pre:  probs is a valid slice of RawTokenProb
/// post: returns `Vec<TokenProbability>` with mapped fields
pub fn map_token_probs(probs: &[RawTokenProb]) -> Vec<TokenProbability> {
    probs
        .iter()
        .map(|p| TokenProbability {
            token: p.token.clone(),
            prob: p.prob,
            top_k: p
                .top_k
                .iter()
                .map(|tk| TokenProb {
                    token: tk.token.clone(),
                    prob: tk.prob,
                })
                .collect(),
        })
        .collect()
}

/// Convert a `ChatResponse` into an `InferenceResult`.
/// Convert a chat completion response to InferenceResult.
///
/// expect: "The system normalizes provider responses for monitoring"
/// \[P9\] Motivating: Homeostatic Self-Regulation — normalizes provider response for monitoring
/// pre:  response is a valid ChatResponse
/// post: returns Ok(InferenceResult) with text, usage, finish_reason
/// post: returns Err if no choices in response
pub fn chat_response_to_result(response: ChatResponse) -> Result<InferenceResult, InferenceError> {
    let choice = response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| InferenceError::Generation("Empty response".to_string()))?;

    let token_probabilities = choice.token_probs.as_ref().map(|p| map_token_probs(p));

    let tool_calls = choice
        .tool_calls
        .as_ref()
        .map(|calls| map_tool_calls(calls))
        .unwrap_or_default();

    Ok(InferenceResult {
        text: choice.message.content,
        model: response.model,
        usage: InferenceUsage {
            prompt_tokens: response.usage.prompt_tokens,
            completion_tokens: response.usage.completion_tokens,
            total_tokens: response.usage.total_tokens,
        },
        finish_reason: choice.finish_reason,
        token_probabilities,
        tool_calls,
    })
}

/// Parse SSE stream lines into `InferenceStreamChunk` vec.
/// Parse an SSE stream into InferenceStreamChunks.
///
/// expect: "The system constructs and validates regulated LLM requests"
/// \[P9\] Motivating: Homeostatic Self-Regulation — parses streaming response chunks for regulated output
/// pre:  stream is a valid SSE byte stream
/// post: returns stream of InferenceStreamChunk parsed from SSE data lines
pub fn parse_sse_stream(
    body: &str,
    model_id: &str,
) -> Vec<Result<InferenceStreamChunk, InferenceError>> {
    let mut chunks: Vec<Result<InferenceStreamChunk, InferenceError>> = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line == "data: [DONE]" {
            continue;
        }
        let json_str = line.strip_prefix("data: ").unwrap_or(line);
        let chunk: StreamChunk = match serde_json::from_str(json_str) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let choice = match chunk.choices.first() {
            Some(c) => c,
            None => continue,
        };

        let text_delta = choice.delta.content.clone().unwrap_or_default();
        let finish_reason = choice.finish_reason.clone();
        let tool_calls = choice
            .tool_calls
            .as_ref()
            .map(|calls| map_tool_calls(calls))
            .unwrap_or_default();
        let usage = chunk.usage.map(|u| InferenceUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        chunks.push(Ok(InferenceStreamChunk {
            text_delta,
            model: chunk.model.clone(),
            finish_reason: finish_reason.clone(),
            usage: if finish_reason.is_some() { usage } else { None },
            tool_calls: if finish_reason.is_some() {
                tool_calls
            } else {
                vec![]
            },
        }));
    }

    if chunks.is_empty() {
        chunks.push(Ok(InferenceStreamChunk {
            text_delta: String::new(),
            model: model_id.to_string(),
            finish_reason: Some("stop".to_string()),
            usage: None,
            tool_calls: vec![],
        }));
    }

    chunks
}

/// Validate a prompt string.
///
/// expect: "The system constructs and validates regulated LLM requests"
/// \[P9\] Motivating: Homeostatic Self-Regulation — input validation prevents token overconsumption
/// pre:  prompt is a valid &str
/// post: returns Err(Generation) if prompt is empty
/// post: returns Err(Generation) if prompt.len() > 1_000_000
pub fn validate_prompt(prompt: &str) -> Result<(), InferenceError> {
    if prompt.is_empty() {
        return Err(InferenceError::Generation("Prompt is empty".to_string()));
    }
    if prompt.len() > 1_000_000 {
        return Err(InferenceError::Generation("Prompt too long".to_string()));
    }
    Ok(())
}

/// Stream a chat completion from an OpenAI-compatible endpoint via SSE.
///
/// Shared helper used by all backends. Manages HTTP request, status handling,
/// and SSE parsing. Backends differ only in their Authorization header value
/// (Bearer vs Key) and base URL.
///
/// expect: "The system constructs and validates regulated LLM requests"
/// \[P9\] Motivating: Homeostatic Self-Regulation — shared streaming helper for all providers
/// pre:  client is a configured reqwest::Client
/// pre:  base_url and auth_header_value are non-empty
/// pre:  model and prompt are non-empty
/// post: returns Pin<Box<Stream<Item = `Result<InferenceStreamChunk, InferenceError>`> + Send>>
pub fn stream_chat_completion(
    client: std::sync::Arc<reqwest::Client>,
    base_url: String,
    auth_header_value: String,
    model: String,
    prompt: String,
    params: LLMParameters,
) -> std::pin::Pin<
    Box<dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send>,
> {
    Box::pin(
        futures_util::stream::once(async move {
            let request = build_chat_request(&model, &prompt, None, &params, Some(true), None);

            let response = match client
                .post(format!("{}/v1/chat/completions", base_url))
                .header("Authorization", &auth_header_value)
                .json(&request)
                .send()
                .await
                .map_err(|e| InferenceError::Connection(e.to_string()))
            {
                Ok(r) => r,
                Err(e) => return vec![Err(e)],
            };

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return vec![Err(InferenceError::Connection(format!(
                    "streaming status {}: {}",
                    status, error_text
                )))];
            }

            let body = match response
                .text()
                .await
                .map_err(|e| InferenceError::Connection(e.to_string()))
            {
                Ok(b) => b,
                Err(e) => return vec![Err(e)],
            };

            parse_sse_stream(&body, &model)
        })
        .map(futures_util::stream::iter)
        .flatten(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// expect: "Inference chat response deserialization works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates response normalization
    #[test]
    fn chat_response_deserializes_openai_format() {
        let raw = r#"{
            "id": "chatcmpl-847",
            "object": "chat.completion",
            "created": 1781219013,
            "model": "qwen3:4b",
            "system_fingerprint": "fp_deepinfra",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "The sun beat down."
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 5,
                "total_tokens": 16
            }
        }"#;

        let resp: ChatResponse =
            serde_json::from_str(raw).expect("ChatResponse must deserialize OpenAI format");

        assert_eq!(resp.model, "qwen3:4b");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content, "The sun beat down.");
        assert_eq!(resp.choices[0].finish_reason, "stop");
        assert_eq!(resp.usage.prompt_tokens, 11);
        assert_eq!(resp.usage.completion_tokens, 5);
        assert_eq!(resp.usage.total_tokens, 16);
    }

    /// expect: "Inference chat request building works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates non-streaming request payload
    #[test]
    fn build_chat_request_stream_false() {
        let params = LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.0,
            typical_p: 0.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 512,
            seed: None,
            disable_thinking: false,
            adapter: None,
        };
        let req = build_chat_request(
            "qwen3:4b",
            "Write a sentence.",
            None,
            &params,
            Some(false),
            None,
        );
        let json = serde_json::to_value(&req).expect("serialization must succeed");
        assert_eq!(json["stream"], serde_json::json!(false));
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Write a sentence.");
    }

    /// expect: "Inference prompt validation works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates prompt guardrails
    #[test]
    fn validate_prompt_rejects_invalid() {
        assert!(validate_prompt("").is_err());
        assert!(validate_prompt("hello").is_ok());
    }

    /// expect: "Inference thinking mode wire format works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates reasoning-mode suppression
    #[test]
    fn disable_thinking_maps_to_wire_format() {
        let params = LLMParameters {
            temperature: 0.3,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.0,
            typical_p: 0.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 256,
            seed: None,
            disable_thinking: true,
            adapter: None,
        };
        let req = build_chat_request("qwen3:8b", "Summarize.", None, &params, Some(false), None);
        let json = serde_json::to_value(&req).expect("serialization must succeed");
        assert_eq!(json["enable_thinking"], serde_json::json!(false));
    }

    /// expect: "Inference thinking mode omission works correctly under test conditions"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — validates default reasoning-mode omission
    #[test]
    fn enable_thinking_omitted_when_true() {
        let params = LLMParameters {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.0,
            typical_p: 0.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            max_tokens: 512,
            seed: None,
            disable_thinking: false,
            adapter: None,
        };
        let req = build_chat_request("qwen3:4b", "Hello.", None, &params, Some(false), None);
        let json = serde_json::to_value(&req).expect("serialization must succeed");
        // enable_thinking should NOT appear in JSON when true (skip_serializing_if)
        assert!(json.get("enable_thinking").is_none());
    }

    // [P9] Motivating: Homeostatic Self-Regulation — input validation prevents token overconsumption
    // For any non-empty string ≤ 1_000_000 chars, validate_prompt returns Ok(()).
    // For empty string, returns Err. For strings > 1_000_000, returns Err.
    #[test]
    fn validate_prompt_contract() {
        // Empty → error
        assert!(validate_prompt("").is_err());

        // Normal prompts → ok
        assert!(validate_prompt("hello").is_ok());
        assert!(validate_prompt("a").is_ok());
        assert!(validate_prompt(&"x".repeat(1000)).is_ok());
        assert!(validate_prompt(&"x".repeat(1_000_000)).is_ok());

        // Overlong → error
        assert!(validate_prompt(&"x".repeat(1_000_001)).is_err());
    }
}
