//! MCP command handlers for `kask mcp`
//!
//! Implements the CLI display logic for MCP tool invocation and listing.

use crate::cli::McpAction;

/// Built-in MCP servers to start for CLI commands.
const BUILTIN_SERVERS: &[(&str, &str)] = &[
    ("inference", "hkask-mcp-inference"),
    ("cns", "hkask-mcp-cns"),
    ("condenser", "hkask-mcp-condenser"),
    ("episodic", "hkask-mcp-episodic"),
    ("semantic", "hkask-mcp-semantic"),
    ("ocap", "hkask-mcp-ocap"),
    ("keystore", "hkask-mcp-keystore"),
    ("git", "hkask-mcp-git"),
    ("registry", "hkask-mcp-registry"),
    ("goal", "hkask-mcp-goal"),
    ("doc-knowledge", "hkask-mcp-doc-knowledge"),
    ("spec", "hkask-mcp-spec"),
];

pub fn run(rt: &tokio::runtime::Runtime, action: McpAction) {
    match action {
        McpAction::ListServers => {
            let runtime = hkask_mcp::runtime::McpRuntime::new();
            for (server_id, command) in BUILTIN_SERVERS {
                match rt.block_on(runtime.start_server(server_id, command)) {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::warn!(
                            target: "hkask.cli",
                            server_id = %server_id,
                            error = %e,
                            "Failed to start MCP server"
                        );
                    }
                }
            }

            let servers = rt.block_on(runtime.list_servers());
            println!("MCP servers:");
            if servers.is_empty() {
                println!("  (no servers registered)");
            } else {
                for server in &servers {
                    println!("  {} ({} tools)", server.id, server.tools.len());
                }
            }
        }
        McpAction::ListTools => {
            let runtime = hkask_mcp::runtime::McpRuntime::new();
            for (server_id, command) in BUILTIN_SERVERS {
                match rt.block_on(runtime.start_server(server_id, command)) {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::warn!(
                            target: "hkask.cli",
                            server_id = %server_id,
                            error = %e,
                            "Failed to start MCP server"
                        );
                    }
                }
            }

            let tools = rt.block_on(runtime.discover_tools());
            println!("Available tools:");
            if tools.is_empty() {
                println!("  (no tools registered)");
            } else {
                for tool_name in &tools {
                    println!("  {}", tool_name);
                }
            }
        }
        McpAction::GetTool { name } => {
            let runtime = hkask_mcp::runtime::McpRuntime::new();
            for (server_id, command) in BUILTIN_SERVERS {
                match rt.block_on(runtime.start_server(server_id, command)) {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::warn!(
                            target: "hkask.cli",
                            server_id = %server_id,
                            error = %e,
                            "Failed to start MCP server"
                        );
                    }
                }
            }

            match rt.block_on(runtime.get_tool_info(&name)) {
                Some(info) => {
                    println!("Tool: {}", info.name);
                    println!("  Description: {}", info.description);
                    println!("  Server: {}", info.server_id);
                    if let Some(cap) = &info.required_capability {
                        println!("  Required capability: {}", cap);
                    }
                    println!(
                        "  Input schema: {}",
                        serde_json::to_string_pretty(&info.input_schema)
                            .unwrap_or_else(|_| info.input_schema.to_string())
                    );
                }
                None => {
                    eprintln!("Tool '{}' not found", name);
                    std::process::exit(1);
                }
            }
        }
        McpAction::Invoke {
            server: _server,
            tool,
            input,
        } => {
            use hkask_templates::McpPort;

            let input_value: serde_json::Value =
                super::helpers::or_exit(serde_json::from_str(&input), "parse JSON input");

            let (dispatcher, token) =
                crate::commands::config::create_mcp_dispatcher_with_servers(rt, BUILTIN_SERVERS)
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to create MCP dispatcher: {}", e);
                        std::process::exit(1);
                    });

            let result = match rt.block_on(dispatcher.invoke(&tool, input_value, &token)) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Tool invocation error: {}", e);
                    rt.block_on(dispatcher.shutdown_all());
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
            );

            rt.block_on(dispatcher.shutdown_all());
        }
    }
}
