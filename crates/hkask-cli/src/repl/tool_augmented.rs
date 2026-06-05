//! Tool-augmented chat — parse model responses for tool call directives
//! and invoke them through GovernedTool or the Communication Loop.
//!
//! Both single-agent REPL and ensemble turns call the same `process_response`
//! function. Any agent response that contains tool calls (either structured
//! `InferenceResult.tool_calls` from native function calling or `<<tool:...>>`
//! text directives) gets parsed, invoked, and optionally fed back to the model
//! for a followup.
//!
//! Tool call sources (in priority order):
//! 1. **Structured**: `InferenceResult.tool_calls` when `finish_reason == "tool_calls"`
//! 2. **Text fallback**: `<<tool:server/tool_name\n{args}\n>>` directives in text

use hkask_cns::GovernedTool;
use hkask_mcp::raw_tool_port::RawMcpToolPort;
use hkask_types::ports::{StructuredToolCall, ToolPort};
use hkask_types::{DelegationAction, DelegationResource, DelegationToken, WebID};
use std::sync::Arc;

/// A parsed tool call directive from a model response.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// The MCP server ID (e.g., "hkask-mcp-inference")
    pub server: String,
    /// The tool name (e.g., "inference_generate")
    pub tool: String,
    /// The JSON arguments for the tool call
    pub args: serde_json::Value,
}

impl From<StructuredToolCall> for ToolCall {
    fn from(stc: StructuredToolCall) -> Self {
        Self {
            server: stc.server,
            tool: stc.tool,
            args: stc.args,
        }
    }
}

/// Result of parsing a model response for tool calls.
pub struct ParsedResponse {
    /// Text content (everything that isn't a tool call directive)
    pub text: String,
    /// Parsed tool call directives (in order of appearance)
    pub tool_calls: Vec<ToolCall>,
}

/// Parse a model response for `<<tool:...>>` text directives.
///
/// Tool calls are embedded in the response using the format:
/// `<<tool:server/tool_name\n{"key": "value"}\n>>`
///
/// The parser is forgiving — if a directive is malformed, it's treated as
/// plain text and not extracted.
pub fn parse_tool_calls(response: &str) -> ParsedResponse {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();
    let mut remaining = response;

    while let Some(start) = remaining.find("<<tool:") {
        text_parts.push(remaining[..start].to_string());
        remaining = &remaining[start + 7..]; // skip "<<tool:"

        let end_pos = match remaining.find(">>") {
            Some(pos) => pos,
            None => {
                text_parts.push(format!("<<tool:{}", remaining));
                remaining = "";
                break;
            }
        };

        let directive = &remaining[..end_pos];
        remaining = &remaining[end_pos + 2..];

        let (header, json_body) = match directive.find('\n') {
            Some(pos) => (&directive[..pos], directive[pos + 1..].trim()),
            None => (directive, "{}"),
        };

        let (server, tool) = match header.find('/') {
            Some(pos) => (
                header[..pos].trim().to_string(),
                header[pos + 1..].trim().to_string(),
            ),
            None => ("".to_string(), header.trim().to_string()),
        };

        if tool.is_empty() {
            text_parts.push(format!("<<tool:{}>>", directive));
            continue;
        }

        let args: serde_json::Value = match serde_json::from_str(json_body) {
            Ok(v) => v,
            Err(_) => {
                text_parts.push(format!("<<tool:{}>>", directive));
                continue;
            }
        };

        tool_calls.push(ToolCall { server, tool, args });
    }

    if !remaining.is_empty() {
        text_parts.push(remaining.to_string());
    }

    let full_text = text_parts
        .iter()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    ParsedResponse {
        text: full_text,
        tool_calls,
    }
}

/// Invoke a parsed tool call through the GovernedTool membrane.
///
/// Mints a DelegationToken from the session's ACP secret for OCAP
/// authorization, then routes through GovernedTool (gas budgets, CNS).
pub async fn invoke_tool_call(
    call: &ToolCall,
    governed_tool: &Arc<GovernedTool<RawMcpToolPort>>,
    agent_webid: &WebID,
    acp_secret: &[u8],
) -> Result<serde_json::Value, String> {
    let token = DelegationToken::new(
        DelegationResource::Tool,
        call.tool.clone(),
        DelegationAction::Execute,
        WebID::new(),
        *agent_webid,
        acp_secret,
    );

    governed_tool
        .invoke(&call.server, &call.tool, call.args.clone(), &token)
        .await
        .map_err(|e| format!("{}: {}", call.tool, e))
}

/// Build the tool results context to feed back to the model.
pub fn format_tool_results(calls: &[(ToolCall, Result<serde_json::Value, String>)]) -> String {
    if calls.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();
    parts.push("Tool results:".to_string());
    parts.push(String::new());

    for (call, result) in calls {
        match result {
            Ok(value) => {
                let formatted = match serde_json::to_string_pretty(value) {
                    Ok(s) => s,
                    Err(_) => format!("{:?}", value),
                };
                parts.push(format!("✓ {} → {}", call.tool, formatted));
            }
            Err(err) => {
                parts.push(format!("✗ {} → ERROR: {}", call.tool, err));
            }
        }
    }

    parts.join("\n")
}

/// Process tool calls in a response: parse, invoke, display results.
///
/// This is the single shared async function called by both single-agent REPL
/// and ensemble turns. It:
/// 1. Checks `InferenceResult.tool_calls` for structured native function calls
/// 2. Falls back to parsing `<<tool:...>>` text directives if no structured calls
/// 3. Invokes each through GovernedTool
/// 4. Prints results to the terminal
/// 5. Returns the final response text (with tool calls stripped) and
///    the formatted tool results (for followup inference if needed)
///
/// The `agent_name` parameter is used for display prefix (e.g. "Curator").
pub async fn process_response(
    response_text: &str,
    agent_name: &str,
    governed_tool: &Arc<GovernedTool<RawMcpToolPort>>,
    agent_webid: &WebID,
    acp_secret: &[u8],
    structured_tool_calls: Option<&[StructuredToolCall]>,
) -> ProcessedResponse {
    // Priority 1: Use structured tool calls from native function calling
    // (when the model returned finish_reason == "tool_calls")
    // Priority 2: Parse <<tool:...>> text directives as a fallback
    let tool_calls: Vec<ToolCall> = if let Some(calls) = structured_tool_calls {
        if !calls.is_empty() {
            calls.iter().cloned().map(ToolCall::from).collect()
        } else {
            // No structured calls — try text parsing
            let parsed = parse_tool_calls(response_text);
            parsed.tool_calls
        }
    } else {
        let parsed = parse_tool_calls(response_text);
        parsed.tool_calls
    };

    // Determine the text content (strip text directives if we parsed any)
    let text_content = if structured_tool_calls.is_some_and(|c| !c.is_empty()) {
        // When we have structured calls, use the full response text (no directives to strip)
        response_text.to_string()
    } else {
        let parsed = parse_tool_calls(response_text);
        if parsed.tool_calls.is_empty() {
            response_text.to_string()
        } else {
            parsed.text
        }
    };

    if tool_calls.is_empty() {
        // No tool calls — return response as-is
        return ProcessedResponse {
            text: response_text.to_string(),
            tool_results_formatted: String::new(),
            had_tool_calls: false,
        };
    }

    // Display the text portion of the response (before tool calls)
    if !text_content.trim().is_empty() {
        println!("{}: {}", agent_name, text_content.trim());
    }

    println!(
        "  \x1b[2m⟐ {} tool call(s) from {}\x1b[0m",
        tool_calls.len(),
        agent_name
    );

    // Invoke each tool call through GovernedTool
    let mut tool_results = Vec::new();
    for call in &tool_calls {
        print!("  \x1b[2m  Invoking {}\x1b[0m", call.tool);
        if !call.server.is_empty() {
            print!(" on \x1b[36m{}\x1b[0m", call.server);
        }
        println!("...");

        let result = invoke_tool_call(call, governed_tool, agent_webid, acp_secret).await;

        match &result {
            Ok(value) => {
                println!("  \x1b[32m  ✓\x1b[0m {}", call.tool);
                if let Ok(formatted) = serde_json::to_string_pretty(value) {
                    for line in formatted.lines().take(5) {
                        println!("    {}", line);
                    }
                    if formatted.lines().count() > 5 {
                        println!("    ...");
                    }
                }
            }
            Err(err) => {
                println!("  \x1b[31m  ✗\x1b[0m {} — {}", call.tool, err);
            }
        }

        tool_results.push((call.clone(), result));
    }

    let tool_results_formatted = format_tool_results(&tool_results);

    // Final text = text content if non-empty,
    // otherwise a summary of what was invoked
    let final_text = if text_content.trim().is_empty() {
        format!(
            "[{} invoked: {}]",
            agent_name,
            tool_calls
                .iter()
                .map(|c| c.tool.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        text_content.trim().to_string()
    };

    ProcessedResponse {
        text: final_text,
        tool_results_formatted,
        had_tool_calls: true,
    }
}

/// Result of processing a response for tool calls.
pub struct ProcessedResponse {
    /// The response text with tool call directives stripped out.
    /// If the response was only tool calls (no text), this is a summary.
    pub text: String,
    /// Formatted tool results suitable for feeding back to the model
    /// as context in a followup inference turn.
    pub tool_results_formatted: String,
    /// Whether any tool calls were found and invoked.
    pub had_tool_calls: bool,
}

/// Maximum number of tool-augmented followup loops to prevent infinite recursion.
#[allow(dead_code)]
pub const MAX_TOOL_LOOPS: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_tool_calls() {
        let response = "Hello! I can help you with that.";
        let parsed = parse_tool_calls(response);
        assert!(parsed.tool_calls.is_empty());
        assert_eq!(parsed.text, "Hello! I can help you with that.");
    }

    #[test]
    fn test_parse_single_tool_call() {
        let response = "Let me look that up.\n\n<<tool:hkask-mcp-semantic/semantic_recall\n{\"entity\": \"rust\"}\n>>\n\nHere's what I found.";
        let parsed = parse_tool_calls(response);
        assert_eq!(parsed.tool_calls.len(), 1);
        assert_eq!(parsed.tool_calls[0].server, "hkask-mcp-semantic");
        assert_eq!(parsed.tool_calls[0].tool, "semantic_recall");
        assert_eq!(parsed.tool_calls[0].args["entity"], "rust");
    }

    #[test]
    fn test_parse_tool_call_no_server() {
        let response = "<<tool:semantic_recall\n{\"entity\": \"rust\"}\n>>";
        let parsed = parse_tool_calls(response);
        assert_eq!(parsed.tool_calls.len(), 1);
        assert_eq!(parsed.tool_calls[0].server, "");
        assert_eq!(parsed.tool_calls[0].tool, "semantic_recall");
    }

    #[test]
    fn test_parse_multiple_tool_calls() {
        let response = "<<tool:hkask-mcp-cns/cns_health\n{}\n>>\n\n<<tool:hkask-mcp-semantic/semantic_count\n{}\n>>";
        let parsed = parse_tool_calls(response);
        assert_eq!(parsed.tool_calls.len(), 2);
        assert_eq!(parsed.tool_calls[0].tool, "cns_health");
        assert_eq!(parsed.tool_calls[1].tool, "semantic_count");
    }

    #[test]
    fn test_parse_malformed_no_close() {
        let response = "<<tool:hkask-mcp-cns/cns_health\n{}";
        let parsed = parse_tool_calls(response);
        assert!(parsed.tool_calls.is_empty());
        assert!(parsed.text.contains("<<tool:"));
    }

    #[test]
    fn test_parse_malformed_bad_json() {
        let response = "<<tool:cns_health\n{not json}\n>>";
        let parsed = parse_tool_calls(response);
        assert!(parsed.tool_calls.is_empty());
    }

    #[test]
    fn test_format_tool_results() {
        let call = ToolCall {
            server: "hkask-mcp-cns".into(),
            tool: "cns_health".into(),
            args: serde_json::json!({}),
        };
        let results = vec![(call, Ok(serde_json::json!({"status": "healthy"})))];
        let formatted = format_tool_results(&results);
        assert!(formatted.contains("✓ cns_health"));
        assert!(formatted.contains("healthy"));
    }

    #[test]
    fn test_structured_tool_call_conversion() {
        let stc = StructuredToolCall {
            server: "hkask-mcp-cns".to_string(),
            tool: "cns_health".to_string(),
            args: serde_json::json!({}),
            call_id: Some("call_abc123".to_string()),
        };
        let tc: ToolCall = stc.into();
        assert_eq!(tc.server, "hkask-mcp-cns");
        assert_eq!(tc.tool, "cns_health");
    }
}
