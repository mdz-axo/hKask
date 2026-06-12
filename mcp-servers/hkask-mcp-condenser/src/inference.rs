//! Inference-backed summarization — pure functions for thread summary
//!
//! Extracts the testable logic from `condenser_thread_summary` so it can be
//! verified without a live inference endpoint.

use hkask_mcp::server::McpToolError;

use crate::types::ThreadSummaryOutput;

/// Inference API format — detected from the INFERENCE_URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiFormat {
    /// Ollama format: POST /api/chat with `think`, `options.num_ctx`, `options.num_predict`
    Ollama,
    /// OpenAI-compatible format (OpenRouter, LiteLLM, etc.): POST /v1/chat/completions
    OpenAi,
}

/// Detect the API format from the inference URL.
///
/// - URLs containing `openrouter.ai` → `OpenAi`
/// - URLs whose path ends with `/v1` (common for OpenAI-compatible proxies) → `OpenAi`
/// - Everything else → `Ollama` (existing behavior, backward compatible)
pub fn detect_format(url: &str) -> ApiFormat {
    let lower = url.to_lowercase();
    if lower.contains("openrouter.ai") {
        return ApiFormat::OpenAi;
    }
    // Heuristic: /v1 as the final path segment indicates OpenAI-compatible base URL
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        return ApiFormat::OpenAi;
    }
    ApiFormat::Ollama
}

/// Format a parsed messages array into a conversation text string.
///
/// Each message becomes `[role]: content\n\n`. Messages missing `role` or
/// `content` fields use `"unknown"` and `""` respectively.
pub fn format_conversation_text(messages: &[serde_json::Value]) -> String {
    let mut text = String::new();
    for msg in messages {
        let role = msg
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown");
        let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
        text.push_str(&format!("[{role}]: {content}\n\n"));
    }
    text
}

/// Build the summarization prompt from conversation text and current query.
pub fn build_summarization_prompt(conversation_text: &str, current_query: &str) -> String {
    format!(
        "Summarize this conversation history for context compaction. \
         Preserve: key decisions, file paths mentioned, error states encountered, \
         code changes made, and the current task goal. \
         Discard: verbose tool output, intermediate file reads, repeated information, \
         and anything not directly relevant to the current task.\n\n\
         Current task: {current_query}\n\n\
         Conversation history:\n{conversation_text}"
    )
}

/// Validate and extract summary content from an inference response.
///
/// Parses the response according to the detected API format:
/// - Ollama: `resp.message.content`
/// - OpenAI: `resp.choices[0].message.content`
///
/// Returns `Ok(summary)` if the content is non-empty, `Err` otherwise.
pub fn extract_summary(
    format: ApiFormat,
    resp_body: &serde_json::Value,
) -> Result<String, McpToolError> {
    let content = match format {
        ApiFormat::Ollama => resp_body
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str()),
        ApiFormat::OpenAi => resp_body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str()),
    };
    match content {
        Some(c) if !c.trim().is_empty() => Ok(c.to_string()),
        Some(_) => Err(McpToolError::internal(
            "Inference engine returned an empty summary",
        )),
        None => Err(McpToolError::internal(format!(
            "Inference engine response missing content field ({:?} format)",
            format
        ))),
    }
}

/// Approximate token count using whitespace splitting.
pub fn approx_token_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Build the JSON body for a chat inference request.
///
/// Pure function — no HTTP, no async. Testable in isolation. The caller
/// (`condenser_thread_summary`) is responsible for sending the request.
///
/// Produces Ollama-style or OpenAI-style JSON depending on `format`.
pub fn build_chat_request(
    format: ApiFormat,
    model: &str,
    summarization_prompt: &str,
    system_prompt: &str,
    _num_ctx: u32,
    max_predict: u32,
) -> serde_json::Value {
    match format {
        ApiFormat::Ollama => serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system",  "content": system_prompt},
                {"role": "user",    "content": summarization_prompt}
            ],
            "stream": false,
            "think":  false,
            "options": {
                "num_ctx":     _num_ctx,
                "num_predict": max_predict
            }
        }),
        ApiFormat::OpenAi => serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system",  "content": system_prompt},
                {"role": "user",    "content": summarization_prompt}
            ],
            "stream": false,
            "max_tokens": max_predict
        }),
    }
}

/// Build a `ThreadSummaryOutput` from extracted data.
pub fn build_summary_output(
    summary: String,
    msg_count: usize,
    inference_model: String,
    inference_url: String,
) -> ThreadSummaryOutput {
    ThreadSummaryOutput {
        summary_tokens_approx: approx_token_count(&summary),
        summary,
        original_message_count: msg_count,
        inference_model,
        inference_url,
    }
}
