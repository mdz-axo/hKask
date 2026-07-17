//! Tool-augmented chat — parse model responses for tool call directives
//! and invoke them through GovernedTool or the Communication Loop.
//
//! The REPL turn loop (`turn.rs::run_turn_loop`) calls `extract_tool_calls`
//! to parse model responses, then `invoke_tool_call` for each tool, and
//! `format_tool_results` to build the feedback string for the next
//! iteration. Display is handled by the turn loop via the `TurnSink` trait,
//! not in this module.
//
//! Tool call sources (in priority order):
//! 1. **Structured**: `InferenceResult.tool_calls` when `finish_reason == "tool_calls"`
//! 2. **Text fallback**: `<<tool:server/tool_name\n{args}\n>>` directives in text

use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;

/// Common prefix for the tool-call instruction section of the system prompt.
///
/// Used by both `format_tool_prompt_section` (dynamic) and the hardcoded
/// fallback in `chat_with_agent` (when no MCP runtime is available).
pub const TOOL_CALL_FORMAT_INTRO: &str = "\n## Tool Calls\n\
         You have access to MCP tools. When you need to invoke a tool, include a \
         tool call directive in your response using this format:\n\
         \n\
         <<tool:server/tool_name\n\
         {\"key\": \"value\"}\n\
         >>\n\
         \n";
use hkask_capability::derive_signing_key;
use hkask_capability::{DelegationAction, DelegationResource, DelegationToken};
use hkask_ports::{ChatToolDefinition, ChatToolFunction, StructuredToolCall, ToolInfo, ToolPort};
use hkask_types::WebID;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Convert MCP-discovered tools to OpenAI-compatible `ChatToolDefinition`s.
///
/// Each tool becomes a `{"type": "function", "function": {...}}` definition
/// that models supporting native function calling can use to return structured
/// `tool_calls` instead of relying on `<<tool:...>>` text directives.
///
/// The tool name uses `server_id/name` convention so that `map_tool_calls()`
/// in `chat_protocol.rs` can parse it back into `StructuredToolCall`.
pub fn tools_to_definitions(tools: &[ToolInfo]) -> Vec<ChatToolDefinition> {
    tools
        .iter()
        .map(|tool| ChatToolDefinition {
            tool_type: "function".to_string(),
            function: ChatToolFunction {
                name: format!("{}/{}", tool.server_id, tool.name),
                description: tool.description.clone(),
                parameters: tool.input_schema.clone(),
            },
        })
        .collect()
}

/// Format available MCP tools into a system prompt section.
///
/// Derives the tool list from runtime discovery, grouped by server.
/// This replaces the hardcoded tool format string in `chat.rs` — the
/// LLM now sees only tools that are actually running.
///
/// GovernedTool still enforces authorization at invocation time.
/// This section is advisory — it tells the LLM what's available.
pub fn format_tool_prompt_section(tools: &[ToolInfo]) -> String {
    if tools.is_empty() {
        return String::new();
    }

    // Group tools by server_id for readable formatting
    let mut by_server: BTreeMap<&str, Vec<&ToolInfo>> = BTreeMap::new();
    for tool in tools {
        by_server.entry(&tool.server_id).or_default().push(tool);
    }

    let mut section = TOOL_CALL_FORMAT_INTRO.to_string();

    for (server_id, server_tools) in &by_server {
        section.push_str(&format!("**{}:**\n", server_id));
        for tool in server_tools {
            if tool.description.is_empty() {
                section.push_str(&format!("- {}\n", tool.name));
            } else {
                section.push_str(&format!("- {} — {}\n", tool.name, tool.description));
            }
        }
        section.push('\n');
    }

    section.push_str(
        "You may include multiple tool calls in a single response. After the tool \
         executes, the system will feed the results back to you for a follow-up response. \
         Use tools when they would provide better or more current information than your training data.\
         ",
    );

    // Self-correction pattern: when filesystem write/edit tools are available,
    // instruct the agent to verify changes before claiming success.
    let has_write_tools = tools
        .iter()
        .any(|t| t.name == "fs.write" || t.name == "fs.edit" || t.name == "shell.exec");
    if has_write_tools {
        section.push_str(
            "## Self-Correction Pattern\n\
             After writing or editing a file, ALWAYS verify your change by running \
             the appropriate check command (e.g., `cargo check` for Rust). If the \
             check fails, read the error output, fix the issue, and re-verify. \
             Do not report success until verification passes.\n\
             ",
        );

        // Error recovery strategies — abridged inline reference. For the full
        // diagnostic pipeline, invoke `skill_execute("heal.self-heal", ...)`.
        section.push_str(
            "## Error Recovery\n\
             When a command or file operation fails:\n\
             - Classify the error (auth, permission, not-found, network, timeout).\n\
             - Retry transient failures up to 3 times with increasing delay.\n\
             - For persistent errors, invoke the self-heal skill: \
               `skill_execute(\"heal.self-heal\", {error_message, operation})`.\n\
             - If unhealable, report the error clearly with full output and \
               suggest next steps.\n\
             ",
        );
    }

    section
}

/// A parsed tool call directive from a model response.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// The MCP server ID (e.g., "hkask-mcp-condenser")
    pub server: String,
    /// The tool name (e.g., "condenser_compress")
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
/// Mints a DelegationToken from the session's A2A secret for OCAP
/// authorization, then routes through GovernedTool (energy budgets, CNS).
pub async fn invoke_tool_call(
    call: &ToolCall,
    governed_tool: &Arc<GovernedTool<RawMcpToolPort>>,
    agent_webid: &WebID,
    a2a_secret: &[u8],
    host: &dyn crate::host::ReplHost,
) -> anyhow::Result<serde_json::Value> {
    let token = DelegationToken::new(
        DelegationResource::Tool,
        call.tool.clone(),
        DelegationAction::Execute,
        host.resolve_user_webid(),
        *agent_webid,
        &derive_signing_key(a2a_secret),
    );

    governed_tool
        .invoke(&call.server, &call.tool, call.args.clone(), &token)
        .await
        .map_err(|e| anyhow::anyhow!("{}: {}", call.tool, e))
}

/// Build the tool results context to feed back to the model.
pub fn format_tool_results(calls: &[(ToolCall, anyhow::Result<serde_json::Value>)]) -> String {
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

// `process_response` was removed. Tool-call extraction is now pure
// (`extract_tool_calls`), and invocation/display is handled by the
// unified turn loop in `turn.rs` via the `TurnSink` trait. This fixes:
// - println side effects leaking to stdout in TUI mode
// - duplicate preamble printing
// - double parse_tool_calls computation
// - the missing final-response display when iteration > 1

/// Extract tool calls from a model response, checking structured calls first.
///
/// Priority:
/// 1. Structured tool calls from native function calling (`finish_reason == "tool_calls"`)
/// 2. `<<tool:...>>` text directives as fallback
///
/// Pure — no I/O side effects. The caller is responsible for invoking
/// tools (via `invoke_tool_call`) and displaying results.
pub fn extract_tool_calls(
    response_text: &str,
    structured_tool_calls: Option<&[StructuredToolCall]>,
) -> ParsedResponse {
    if let Some(calls) = structured_tool_calls
        && !calls.is_empty()
    {
        return ParsedResponse {
            text: response_text.to_string(),
            tool_calls: calls.iter().cloned().map(ToolCall::from).collect(),
        };
    }
    parse_tool_calls(response_text)
}
