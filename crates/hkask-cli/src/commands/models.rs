//! Models command handlers for `kask models`
//!
//! Implements the CLI display logic for listing available model tiers.

pub fn run(rt: &tokio::runtime::Runtime) {
    use hkask_templates::McpPort;

    let (dispatcher, token) =
        crate::commands::config::create_mcp_dispatcher().unwrap_or_else(|e| {
            eprintln!("Failed to create MCP dispatcher: {}", e);
            std::process::exit(1);
        });

    match rt.block_on(dispatcher.invoke("inference:models", serde_json::json!({}), &token)) {
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
            std::process::exit(1);
        }
    }
}
