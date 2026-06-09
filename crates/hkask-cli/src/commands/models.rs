//! Models command handlers for `kask models`

use std::sync::Arc;

fn build_mcp(
    rt: &tokio::runtime::Runtime,
    servers: &[(&str, &str)],
) -> Arc<hkask_mcp::McpDispatcher> {
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
    dispatcher
}

pub fn run(rt: &tokio::runtime::Runtime) {
    use hkask_templates::McpPort;
    let dispatcher = build_mcp(rt, &[("inference", "hkask-mcp-inference")]);
    let from = hkask_types::WebID::new();
    let to = hkask_types::WebID::new();
    let token = dispatcher.issue_capability("tools".to_string(), from, to);
    match rt.block_on(dispatcher.invoke("inference_models", serde_json::json!({}), &token)) {
        Ok(result) => {
            if let Some(tiers) = result.get("model_tiers").and_then(|t| t.as_array()) {
                println!("\n=== Available Model Tiers ===");
                for tier in tiers {
                    let label = tier
                        .get("tier")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown");
                    let count = tier.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
                    println!("  {}: {} models", label, count);
                    if let Some(models) = tier.get("models").and_then(|m| m.as_array()) {
                        for model in models {
                            let name = model.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                            let size = model.get("size").and_then(|s| s.as_str()).unwrap_or("");
                            println!("    - {}  {}", name, size);
                        }
                    }
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to list models: {}", e);
            rt.block_on(dispatcher.shutdown_all());
            std::process::exit(1);
        }
    }
    rt.block_on(dispatcher.shutdown_all());
}
