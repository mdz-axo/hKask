//! MCP command handlers for `kask mcp`
//!
//! Implements the CLI display logic for MCP tool invocation and listing.

use crate::cli::McpAction;
use hkask_mcp::runtime::McpRuntime;

pub fn run(rt: &tokio::runtime::Runtime, action: McpAction) {
    match action {
        McpAction::ListServers => {
            println!("MCP servers:");
            println!("  (no servers registered)");
        }
        McpAction::ListTools => {
            println!("Available tools:");
            println!("  (no tools registered)");
        }
        McpAction::GetTool { name } => {
            println!("Get tool: {}", name);
            println!("Note: Tool lookup requires MCP runtime integration.");
        }
        McpAction::Invoke {
            server: _server,
            tool,
            input,
        } => {
            use hkask_templates::McpPort;
            use hkask_types::WebID;

            let input_value: serde_json::Value =
                super::helpers::or_exit(serde_json::from_str(&input), "parse JSON input");

            let runtime = McpRuntime::new();
            let mcp_secret = crate::commands::config::resolve_mcp_secret().unwrap_or_else(|_| {
                tracing::warn!("Using dev fallback for MCP secret");
                "hkask-insecure-dev-fallback".to_string()
            });
            let (dispatcher, _) = crate::commands::config::create_disconnected_governed_dispatcher(
                runtime,
                mcp_secret.as_bytes(),
            );

            let tools = rt.block_on(dispatcher.list_tools());
            if tools.is_empty() {
                eprintln!("Warning: No tools registered in MCP runtime.");
            } else {
                eprintln!("Available tools: {:?}", tools);
            }

            let from = WebID::new();
            let to = WebID::new();
            let token = dispatcher.issue_capability(tool.clone(), from, to);

            let result = match rt.block_on(dispatcher.invoke(&tool, input_value, &token)) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Tool invocation error: {}", e);
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
            );
        }
    }
}
