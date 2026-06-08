//! Models command handlers for `kask models`
//!
//! Implements the CLI display logic for listing available model tiers.

fn build_service_context(
    rt: &tokio::runtime::Runtime,
    servers: &[(&str, &str)],
) -> hkask_services::ServiceContext {
    let config = super::helpers::or_exit(
        hkask_services::ServiceConfig::from_env(),
        "Failed to resolve config",
    );
    let ctx = super::helpers::or_exit(
        rt.block_on(hkask_services::ServiceContext::build(config)),
        "Failed to build ServiceContext",
    );
    for (server_id, command) in servers {
        match rt.block_on(ctx.mcp_runtime.start_server(server_id, command)) {
            Ok(()) => {
                tracing::info!(target: "hkask.cli", server_id = %server_id, "MCP server started");
            }
            Err(e) => {
                tracing::warn!(target: "hkask.cli", server_id = %server_id, error = %e, "Failed to start MCP server");
            }
        }
    }
    ctx
}

pub fn run(rt: &tokio::runtime::Runtime) {
    use hkask_templates::McpPort;

    let ctx = build_service_context(rt, &[("inference", "hkask-mcp-inference")]);

    let from = hkask_types::WebID::new();
    let to = hkask_types::WebID::new();
    let token = ctx
        .mcp_dispatcher
        .issue_capability("tools".to_string(), from, to);

    match rt.block_on(
        ctx.mcp_dispatcher
            .invoke("inference_models", serde_json::json!({}), &token),
    ) {
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
            rt.block_on(ctx.mcp_dispatcher.shutdown_all());
            std::process::exit(1);
        }
    }

    rt.block_on(ctx.mcp_dispatcher.shutdown_all());
}
