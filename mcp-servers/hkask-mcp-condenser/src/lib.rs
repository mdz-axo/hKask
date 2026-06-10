//! hKask MCP Condenser — Library API
//!
//! Direct library interface for context condensation, bypassing the MCP server.
//! Callers can use `thread_summary()` without a running condenser server.

mod inference;
pub mod types;

use inference::ApiFormat;
pub use inference::{
    approx_token_count, build_chat_request, build_summarization_prompt, build_summary_output,
    detect_format, extract_summary, format_conversation_text,
};
use types::ThreadSummaryOutput;

/// System prompt for the thread-summary inference request.
const THREAD_SUMMARY_SYSTEM_PROMPT: &str = "You are a context condensation assistant. Produce structured summaries that \
     preserve technical details (file paths, error messages, decisions) while \
     eliminating verbosity. Use bullet points. Be concise.";

/// Default context window size for the inference engine.
const DEFAULT_NUM_CTX: u32 = 8192;

/// Summarize conversation history using an inference engine.
///
/// Pure library function — no MCP server, no tool dispatch. Callers
/// provide their own HTTP client and inference endpoint configuration.
///
/// Returns `ThreadSummaryOutput` on success, or an error string on failure.
pub async fn thread_summary(
    client: &reqwest::Client,
    messages: &[serde_json::Value],
    current_query: &str,
    max_tokens: Option<u32>,
    model: &str,
    inference_url: &str,
) -> Result<ThreadSummaryOutput, String> {
    let msg_count = messages.len();
    if msg_count == 0 {
        return Err("messages array is empty".to_string());
    }

    let api_format = detect_format(inference_url);
    let conversation_text = format_conversation_text(messages);
    let max_tok = max_tokens.unwrap_or(500);
    let summarization_prompt = build_summarization_prompt(&conversation_text, current_query);

    let chat_request = build_chat_request(
        api_format,
        model,
        &summarization_prompt,
        THREAD_SUMMARY_SYSTEM_PROMPT,
        DEFAULT_NUM_CTX,
        max_tok,
    );

    let url = match api_format {
        ApiFormat::Ollama => format!("{}/api/chat", inference_url.trim_end_matches('/')),
        ApiFormat::OpenAi => format!("{}/chat/completions", inference_url.trim_end_matches('/')),
    };

    let resp = client
        .post(&url)
        .json(&chat_request)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Inference engine returned HTTP {}", resp.status()));
    }

    let resp_body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse inference response: {e}"))?;

    let summary = extract_summary(api_format, &resp_body).map_err(|e| format!("{e}"))?;

    Ok(build_summary_output(
        summary,
        msg_count,
        model.to_string(),
        url,
    ))
}
