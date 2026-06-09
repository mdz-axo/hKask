//! Inference-backed summarization — pure functions for thread summary
//!
//! Extracts the testable logic from `condenser_thread_summary` so it can be
//! verified without a live inference endpoint.

use hkask_mcp::server::McpToolError;

use crate::types::ThreadSummaryOutput;

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
/// Returns `Ok(summary)` if the response contains non-empty `message.content`.
/// Returns `Err(McpToolError)` if the field is missing or the content is empty/whitespace-only.
pub fn extract_summary(resp_body: &serde_json::Value) -> Result<String, McpToolError> {
    match resp_body
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
    {
        Some(content) if !content.trim().is_empty() => Ok(content.to_string()),
        Some(_) => Err(McpToolError::internal(
            "Inference engine returned an empty summary",
        )),
        None => Err(McpToolError::internal(
            "Inference engine response missing message.content field",
        )),
    }
}

/// Approximate token count using whitespace splitting.
pub fn approx_token_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Build the JSON body for an `/api/chat` inference request.
///
/// Pure function — no HTTP, no async. Testable in isolation. The caller
/// (`condenser_thread_summary`) is responsible for sending the request.
pub fn build_chat_request(
    model: &str,
    summarization_prompt: &str,
    system_prompt: &str,
    num_ctx: u32,
    max_predict: u32,
) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system",  "content": system_prompt},
            {"role": "user",    "content": summarization_prompt}
        ],
        "stream": false,
        "think":  false,
        "options": {
            "num_ctx":     num_ctx,
            "num_predict": max_predict
        }
    })
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

