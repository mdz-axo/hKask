//! Models command handlers for `kask models`
//!
//! Lists available models from the inference backend via `InferenceService`.
//! Inference is an internal cognition layer, not an MCP server — the `kask models`
//! command queries the inference backend directly through the service layer.

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; inference backend must be reachable
/// post: lists available models grouped by family with name, size, and quantization info
pub fn run(rt: &tokio::runtime::Runtime) {
    let ctx = super::helpers::build_agent_service();

    let inf_ctx = hkask_services_core::InferenceContext::from(&ctx);
    match rt.block_on(hkask_services_core::InferenceService::list_models(&inf_ctx)) {
        Ok(models) => {
            if models.is_empty() {
                println!("No models available.");
                return;
            }
            // Group by family for tiered display
            use std::collections::BTreeMap;
            let mut by_family: BTreeMap<String, Vec<&hkask_services_core::ModelInfo>> =
                BTreeMap::new();
            for m in &models {
                let family = m.family.as_deref().unwrap_or("uncategorized");
                by_family.entry(family.to_string()).or_default().push(m);
            }
            println!("\n=== Available Models ===");
            for (family, group) in &by_family {
                println!("  {}: {} model(s)", family, group.len());
                for m in group {
                    let size_label = m
                        .parameter_size
                        .as_deref()
                        .unwrap_or(m.quantization_level.as_deref().unwrap_or(""));
                    let bytes = m
                        .size_bytes
                        .map(|b| {
                            if b >= 1_000_000_000 {
                                format!("{:.1} GB", b as f64 / 1_000_000_000.0)
                            } else if b >= 1_000_000 {
                                format!("{:.1} MB", b as f64 / 1_000_000.0)
                            } else {
                                format!("{} B", b)
                            }
                        })
                        .unwrap_or_default();
                    println!("    - {}  {}  {}", m.name, size_label, bytes);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to list models: {}", e);
            std::process::exit(1);
        }
    }
}
