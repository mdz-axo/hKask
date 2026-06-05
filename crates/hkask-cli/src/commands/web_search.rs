//! Web search command handlers for `kask web-search`
//!
//! Implements the CLI display logic for web search via MCP.

use hkask_mcp::runtime::McpRuntime;

pub fn run(rt: &tokio::runtime::Runtime, query: String, max_results: usize) {
    use hkask_templates::McpPort;
    use hkask_types::WebID;

    let runtime = McpRuntime::new();
    let mcp_secret = crate::commands::config::resolve_mcp_secret().unwrap_or_else(|_| {
        tracing::warn!("Using dev fallback for MCP secret");
        "hkask-insecure-dev-fallback".to_string()
    });
    let (dispatcher, _) = crate::commands::config::create_disconnected_governed_dispatcher(
        runtime,
        mcp_secret.as_bytes(),
    );
    let from = WebID::new();
    let to = WebID::new();
    let token = dispatcher.issue_capability("web".to_string(), from, to);

    match rt.block_on(dispatcher.invoke(
        "web:search",
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
}
