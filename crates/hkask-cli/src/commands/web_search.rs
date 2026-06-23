//! Web search command — MCP web search.

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; query is a non-empty search string; max_results > 0
/// post: performs MCP web search; prints results with title, URL, and snippet; exits on failure
pub fn run(rt: &tokio::runtime::Runtime, query: String, max_results: usize) {
    use hkask_templates::McpPort;
    let ctx = super::helpers::build_service_context();
    super::helpers::start_mcp_server(rt, &ctx, "research", "hkask-mcp-research");
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
