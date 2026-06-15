//! Pure formatting functions for thread summary — prompt building, text formatting,
//! token estimation, and output construction.
//!
//! Inference is handled by the centralized `InferencePort` (hkask-inference router).
//! This module contains only the testable pure logic with no HTTP or async.

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

/// Build a `ThreadSummaryOutput` from extracted data.
pub fn build_summary_output(
    summary: String,
    original_text: &str,
    msg_count: usize,
    inference_model: String,
) -> ThreadSummaryOutput {
    ThreadSummaryOutput {
        original_tokens_approx: approx_token_count(original_text),
        summary_tokens_approx: approx_token_count(&summary),
        summary,
        original_message_count: msg_count,
        inference_model,
    }
}

/// Approximate token count using whitespace splitting.
pub fn approx_token_count(text: &str) -> usize {
    text.split_whitespace().count()
}
