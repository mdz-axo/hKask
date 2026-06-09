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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::McpErrorKind;
    use serde_json::json;

    // ── format_conversation_text ──

    // REQ: format_conversation_text renders [role]: content for each message
    #[test]
    fn format_conversation_basic() {
        let messages = vec![
            json!({"role": "user", "content": "hello"}),
            json!({"role": "assistant", "content": "hi there"}),
        ];
        let text = format_conversation_text(&messages);
        assert_eq!(text, "[user]: hello\n\n[assistant]: hi there\n\n");
    }

    // REQ: format_conversation_text uses "unknown" for missing role
    #[test]
    fn format_conversation_missing_role() {
        let messages = vec![json!({"content": "hello"})];
        let text = format_conversation_text(&messages);
        assert!(text.contains("[unknown]: hello"));
    }

    // REQ: format_conversation_text uses "" for missing content
    #[test]
    fn format_conversation_missing_content() {
        let messages = vec![json!({"role": "user"})];
        let text = format_conversation_text(&messages);
        assert!(text.contains("[user]: "));
    }

    // REQ: format_conversation_text handles empty array
    #[test]
    fn format_conversation_empty() {
        let text = format_conversation_text(&[]);
        assert_eq!(text, "");
    }

    // ── extract_summary ──

    // REQ: extract_summary returns content for valid response
    #[test]
    fn extract_summary_valid() {
        let resp = json!({"message": {"content": "Summary of decisions"}});
        let result = extract_summary(&resp);
        assert_eq!(result.unwrap(), "Summary of decisions");
    }

    // REQ: extract_summary returns error for missing message field
    #[test]
    fn extract_summary_missing_message() {
        let resp = json!({"choices": []});
        let err = extract_summary(&resp).unwrap_err();
        assert_eq!(err.kind, McpErrorKind::Internal);
        assert!(err.to_json_string().contains("missing message.content"));
    }

    // REQ: extract_summary returns error for missing content field
    #[test]
    fn extract_summary_missing_content() {
        let resp = json!({"message": {"role": "assistant"}});
        let err = extract_summary(&resp).unwrap_err();
        assert_eq!(err.kind, McpErrorKind::Internal);
        assert!(err.to_json_string().contains("missing message.content"));
    }

    // REQ: extract_summary returns error for empty content
    #[test]
    fn extract_summary_empty_content() {
        let resp = json!({"message": {"content": ""}});
        let err = extract_summary(&resp).unwrap_err();
        assert_eq!(err.kind, McpErrorKind::Internal);
        assert!(err.to_json_string().contains("empty summary"));
    }

    // REQ: extract_summary returns error for whitespace-only content
    #[test]
    fn extract_summary_whitespace_content() {
        let resp = json!({"message": {"content": "   \n\t  "}});
        let result = extract_summary(&resp);
        assert!(result.is_err());
    }

    // REQ: extract_summary accepts content with leading/trailing whitespace
    #[test]
    fn extract_summary_content_with_whitespace() {
        let resp = json!({"message": {"content": "  valid summary  "}});
        assert_eq!(extract_summary(&resp).unwrap(), "  valid summary  ");
    }

    // ── approx_token_count ──

    // REQ: approx_token_count counts whitespace-separated words
    #[test]
    fn approx_token_count_basic() {
        assert_eq!(approx_token_count("hello world"), 2);
        assert_eq!(approx_token_count(""), 0);
        assert_eq!(approx_token_count("   "), 0);
    }

    // ── build_chat_request ──

    // REQ: build_chat_request sets model, stream=false, think=false
    #[test]
    fn build_chat_request_top_level_fields() {
        let req = build_chat_request("qwen3:8b", "summarize this", "be concise", 8192, 500);
        assert_eq!(req["model"], "qwen3:8b");
        assert_eq!(req["stream"], false);
        assert_eq!(req["think"], false);
    }

    // REQ: build_chat_request places system prompt as first message
    #[test]
    fn build_chat_request_system_message_first() {
        let req = build_chat_request("m", "user prompt", "system prompt", 4096, 200);
        let msgs = req["messages"].as_array().unwrap();
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[0]["content"], "system prompt");
    }

    // REQ: build_chat_request places summarization prompt as second message
    #[test]
    fn build_chat_request_user_message_second() {
        let req = build_chat_request("m", "user prompt", "sys", 4096, 200);
        let msgs = req["messages"].as_array().unwrap();
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "user prompt");
    }

    // REQ: build_chat_request sets num_ctx and num_predict in options
    #[test]
    fn build_chat_request_options() {
        let req = build_chat_request("m", "p", "s", 8192, 500);
        assert_eq!(req["options"]["num_ctx"], 8192);
        assert_eq!(req["options"]["num_predict"], 500);
    }

    // ── build_summarization_prompt ──

    // REQ: build_summarization_prompt includes current query and conversation text
    #[test]
    fn summarization_prompt_includes_query_and_conversation() {
        let prompt = build_summarization_prompt("some conversation", "fix the bug");
        assert!(prompt.contains("fix the bug"));
        assert!(prompt.contains("some conversation"));
        assert!(prompt.contains("key decisions"));
    }

    // ── build_summary_output ──

    // REQ: build_summary_output populates all fields correctly
    #[test]
    fn build_summary_output_fields() {
        let output = build_summary_output(
            "Summary text here".to_string(),
            5,
            "qwen3:8b".to_string(),
            "http://localhost:11435/api/chat".to_string(),
        );
        assert_eq!(output.summary, "Summary text here");
        assert_eq!(output.original_message_count, 5);
        assert_eq!(output.summary_tokens_approx, 3);
        assert_eq!(output.inference_model, "qwen3:8b");
        assert_eq!(output.inference_url, "http://localhost:11435/api/chat");
    }
}
