//! MCP command handlers for `kask mcp`
//!
//! Implements the CLI display logic for MCP tool invocation and listing.

use crate::cli::McpAction;

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

            let input_value: serde_json::Value =
                super::helpers::or_exit(serde_json::from_str(&input), "parse JSON input");

            let (dispatcher, token) = crate::commands::config::create_mcp_dispatcher()
                .unwrap_or_else(|e| {
                    eprintln!("Failed to create MCP dispatcher: {}", e);
                    std::process::exit(1);
                });

            let tools = rt.block_on(dispatcher.list_tools());
            if tools.is_empty() {
                eprintln!("Warning: No tools registered in MCP runtime.");
            } else {
                eprintln!("Available tools: {:?}", tools);
            }

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
