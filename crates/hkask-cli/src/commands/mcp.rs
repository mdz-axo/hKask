//! MCP command handlers for `kask mcp`

use crate::cli::McpAction;

const BUILTIN_SERVERS: &[(&str, &str)] = &[
    ("memory", "hkask-mcp-memory"),
    ("condenser", "hkask-mcp-condenser"),
    ("spec", "hkask-mcp-spec"),
    ("docproc", "hkask-mcp-docproc"),
    ("media", "hkask-mcp-media"),
];

fn build_service_context(
    rt: &tokio::runtime::Runtime,
    servers: &[(&str, &str)],
) -> hkask_services::AgentService {
    let config = super::helpers::or_exit(
        hkask_services::ServiceConfig::from_env(),
        "Failed to resolve config",
    );
    let ctx = super::helpers::or_exit(
        rt.block_on(hkask_services::AgentService::build(config)),
        "Failed to build AgentService",
    );
    for (server_id, command) in servers {
        match rt.block_on(ctx.mcp_runtime().start_server(server_id, command)) {
            Ok(()) => {
                tracing::info!(target: "hkask.cli", server_id = %server_id, "MCP server started")
            }
            Err(e) => {
                tracing::warn!(target: "hkask.cli", server_id = %server_id, error = %e, "Failed to start MCP server")
            }
        }
    }
    ctx
}

pub fn run(rt: &tokio::runtime::Runtime, action: McpAction) {
    match action {
        McpAction::ListServers => {
            let ctx = build_service_context(rt, BUILTIN_SERVERS);
            let servers = rt.block_on(ctx.mcp_runtime().list_servers());
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
            let ctx = build_service_context(rt, BUILTIN_SERVERS);
            let tools = rt.block_on(ctx.mcp_runtime().discover_tools());
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
            let ctx = build_service_context(rt, BUILTIN_SERVERS);
            match rt.block_on(ctx.mcp_runtime().get_tool_info(&name)) {
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
            let ctx = build_service_context(rt, BUILTIN_SERVERS);
            let from = hkask_types::WebID::new();
            let to = hkask_types::WebID::new();
            let token = ctx
                .mcp_dispatcher()
                .issue_capability("tools".to_string(), from, to);
            let result = match rt.block_on(ctx.mcp_dispatcher().invoke(&tool, input_value, &token))
            {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Tool invocation error: {}", e);
                    rt.block_on(ctx.mcp_dispatcher().shutdown_all());
                    std::process::exit(1);
                }
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
            );
            rt.block_on(ctx.mcp_dispatcher().shutdown_all());
        }
    }
}
