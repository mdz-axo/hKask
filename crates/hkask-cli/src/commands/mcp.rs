//! MCP command handlers for `kask mcp`

use std::sync::Arc;

use crate::cli::McpAction;

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

fn build_mcp(
    rt: &tokio::runtime::Runtime,
    servers: &[(&str, &str)],
) -> (
    Arc<hkask_mcp::runtime::McpRuntime>,
    Arc<hkask_mcp::McpDispatcher>,
) {
    let config = super::helpers::or_exit(
        hkask_services::ServiceConfig::from_env(),
        "Failed to resolve config",
    );
    let mcp = hkask_mcp::runtime::McpRuntime::new();
    let dispatcher = Arc::new(hkask_mcp::McpDispatcher::with_secret(
        mcp.clone(),
        &config.mcp_secret,
    ));
    for (server_id, command) in servers {
        match rt.block_on(mcp.start_server(server_id, command)) {
            Ok(()) => {
                tracing::info!(target: "hkask.cli", server_id = %server_id, "MCP server started")
            }
            Err(e) => {
                tracing::warn!(target: "hkask.cli", server_id = %server_id, error = %e, "Failed to start MCP server")
            }
        }
    }
    (Arc::new(mcp), dispatcher)
}

pub fn run(rt: &tokio::runtime::Runtime, action: McpAction) {
    match action {
        McpAction::ListServers => {
            let (mcp, _) = build_mcp(rt, BUILTIN_SERVERS);
            let servers = rt.block_on(mcp.list_servers());
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
            let (mcp, _) = build_mcp(rt, BUILTIN_SERVERS);
            let tools = rt.block_on(mcp.discover_tools());
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
            let (mcp, _) = build_mcp(rt, BUILTIN_SERVERS);
            match rt.block_on(mcp.get_tool_info(&name)) {
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
            server: _,
            tool,
            input,
        } => {
            use hkask_templates::McpPort;
            let input_value: serde_json::Value =
                super::helpers::or_exit(serde_json::from_str(&input), "parse JSON input");
            let (_mcp, dispatcher) = build_mcp(rt, BUILTIN_SERVERS);
            let from = hkask_types::WebID::new();
            let to = hkask_types::WebID::new();
            let token = dispatcher.issue_capability("tools".to_string(), from, to);
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
