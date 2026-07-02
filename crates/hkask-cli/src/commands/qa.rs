//! QA command handlers for `kask qa`
//!
//! Runs QA script manifests through `hkask_test_harness::qa_script::run_script()`.
//! When MCP tool dispatch is needed, starts the relevant MCP servers and wires
//! McpRuntime via the McpDispatchFn callback.

use crate::cli::QaAction;
use std::path::Path;
use std::sync::Arc;

/// pre:  action is a valid QaAction
/// post: runs the specified QA operation, prints results to stdout
pub fn run(action: QaAction) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    match action {
        QaAction::Run { script } => {
            let workspace_root = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());

            let workspace = Path::new(&workspace_root);
            if !workspace.join("Cargo.toml").exists() {
                eprintln!("Error: not in a Cargo workspace. Run from the hKask workspace root.");
                std::process::exit(1);
            }

            let manifest_path = if script.is_absolute() {
                script
                    .strip_prefix(&workspace_root)
                    .unwrap_or(&script)
                    .to_path_buf()
            } else if script.starts_with("registry/") {
                script
            } else {
                script
            };

            println!("Running QA script: {}", manifest_path.display());

            // Try to set up MCP dispatch for manifests that use mcp_tool steps.
            // Gracefully degrades if binaries aren't built or config is missing.
            let mcp_dispatch = setup_mcp_dispatch(&rt);

            match rt.block_on(hkask_test_harness::qa_script::run_script(
                workspace,
                &manifest_path,
                mcp_dispatch,
            )) {
                Ok(output) => {
                    println!();
                    println!("QA Script Complete");
                    println!("==================");
                    println!("  Manifest:   {}", output.manifest_id);
                    println!("  Terminal:   step {}", output.terminal_ordinal);
                    println!("  Message:    {}", output.terminal_message.trim());
                    println!("  Steps run:  {}", output.steps_executed);
                    println!("  Gas used:   {}", output.gas_used);
                    if output.terminal_message.contains("FAIL") {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!();
                    eprintln!("QA Script Failed");
                    eprintln!("================");
                    eprintln!("  Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        QaAction::List => {
            let workspace_root = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());

            let manifest_dir = Path::new(&workspace_root).join("registry/manifests");
            match std::fs::read_dir(&manifest_dir) {
                Ok(entries) => {
                    let mut manifests: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_name().to_string_lossy().starts_with("qa-"))
                        .collect();
                    manifests.sort_by_key(|e| e.file_name());

                    if manifests.is_empty() {
                        println!("No QA manifests found in {}", manifest_dir.display());
                        return;
                    }

                    println!("QA Manifests");
                    println!("============");
                    for entry in manifests {
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy();
                        let desc = std::fs::read_to_string(entry.path())
                            .ok()
                            .and_then(|content| {
                                serde_yaml_neo::from_str::<serde_yaml_neo::Value>(&content)
                                    .ok()
                                    .and_then(|v| {
                                        v.get("manifest")?
                                            .get("description")?
                                            .as_str()
                                            .map(|s| s.trim().to_string())
                                    })
                            })
                            .unwrap_or_else(|| "(no description)".to_string());
                        println!("  {} — {}", name_str, desc);
                    }
                }
                Err(_) => {
                    println!("No registry/manifests directory found.");
                }
            }
        }
    }
}

/// Try to set up MCP dispatch for QA manifests with mcp_tool steps.
/// Returns None if MCP infrastructure isn't available (graceful degradation).
fn setup_mcp_dispatch(
    rt: &tokio::runtime::Runtime,
) -> Option<hkask_test_harness::qa_script::McpDispatchFn> {
    use hkask_mcp::BUILTIN_SERVERS;

    // Build AgentService and start MCP servers (same pattern as kask mcp)
    // Build AgentService and start MCP servers
    let ctx = crate::commands::helpers::build_agent_service();

    let replicant_name = ctx.config().agent_name.clone();
    crate::commands::helpers::start_mcp_servers_with_env(
        rt,
        &ctx,
        BUILTIN_SERVERS,
        &replicant_name,
    );

    let mcp = ctx.infra().mcp.clone();

    // Build dispatch closure: maps (tool_name, tool_params) → Result<output, error>
    let dispatch: hkask_test_harness::qa_script::McpDispatchFn =
        Arc::new(move |tool_name: String, tool_params: String| {
            let mcp = mcp.clone();
            Box::pin(async move {
                // Parse params JSON
                let args: serde_json::Map<String, serde_json::Value> =
                    match serde_json::from_str(&tool_params) {
                        Ok(serde_json::Value::Object(map)) => map,
                        Ok(_) => serde_json::Map::new(),
                        Err(_) => serde_json::Map::new(),
                    };

                // Resolve server_id from tool registry
                let server_id = match mcp.get_tool_info(&tool_name).await {
                    Some(info) => info.server_id,
                    None => {
                        return Err(format!(
                            "tool '{}' not found in any registered server",
                            tool_name
                        ));
                    }
                };

                // Dispatch via McpRuntime (bypasses OCAP governance — QA is internal audit)
                match mcp.call_tool(&server_id, &tool_name, args).await {
                    Ok(result) => {
                        let text: String = result
                            .content
                            .iter()
                            .filter_map(|c| match &**c {
                                rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        Ok(text)
                    }
                    Err(e) => Err(format!("MCP dispatch error: {}", e)),
                }
            })
        });

    Some(dispatch)
}
