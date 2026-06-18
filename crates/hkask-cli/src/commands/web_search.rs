//! Web search command — MCP web search.

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

/// expect: "I can access all hKask functionality through the kask CLI" [P3]
/// pre:  rt is a valid tokio Runtime; query is a non-empty search string; max_results > 0
/// post: performs MCP web search; prints results with title, URL, and snippet; exits on failure
pub fn run(rt: &tokio::runtime::Runtime, query: String, max_results: usize) {
    use hkask_templates::McpPort;
    let ctx = build_service_context(rt, &[("research", "hkask-mcp-research")]);
    let from = super::helpers::resolve_user_webid();
    let to = super::helpers::resolve_user_webid();
    let token = ctx
        .mcp_dispatcher()
        .issue_capability("tools".to_string(), from, to);
    match rt.block_on(ctx.mcp_dispatcher().invoke(
        "web_search",
        serde_json::json!({"query": query, "max_results": max_results}),
        &token,
    )) {
        Ok(result) => {
            if let Some(results) = result.get("results").and_then(|r| r.as_array()) {
                println!("\n=== Web Search: {} ===\n", query);
                for (i, item) in results.iter().enumerate() {
                    let title = item
                        .get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Untitled");
                    let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("");
                    let snippet = item.get("snippet").and_then(|s| s.as_str()).unwrap_or("");
                    println!("{}. {}", i + 1, title);
                    println!("   URL: {}", url);
                    if !snippet.is_empty() {
                        println!("   {}", snippet);
                    }
                    println!();
                }
            } else if let Some(error) = result.get("error") {
                eprintln!("Search error: {}", error);
                std::process::exit(1);
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            }
        }
        Err(e) => {
            eprintln!("Web search failed: {}", e);
            std::process::exit(1);
        }
    }
    rt.block_on(ctx.mcp_dispatcher().shutdown_all());
}
