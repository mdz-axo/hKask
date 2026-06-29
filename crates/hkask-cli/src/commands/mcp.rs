//! MCP command handlers for `kask mcp`

use crate::cli::McpAction;
use hkask_mcp::BUILTIN_SERVERS;

fn build_service_context(rt: &tokio::runtime::Runtime) -> hkask_services::AgentService {
    let ctx = super::helpers::build_service_context();
    let replicant_name = ctx.config().agent_name.clone();
    super::helpers::start_mcp_servers_with_env(rt, &ctx, BUILTIN_SERVERS, &replicant_name);
    ctx
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; action is a valid McpAction variant
/// post: dispatches to list_servers, list_tools, get_tool, or invoke tool operations
pub fn run(rt: &tokio::runtime::Runtime, action: McpAction) {
    match action {
        McpAction::ListServers => {
            let ctx = build_service_context(rt);
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
            let ctx = build_service_context(rt);
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
            let ctx = build_service_context(rt);
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
            let ctx = build_service_context(rt);
            let from = super::helpers::resolve_user_webid();
            let to = super::helpers::resolve_user_webid();
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
