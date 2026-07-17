//! QA command handlers for `kask qa`
//!
//! Runs QA script manifests through `hkask_test_harness::qa_script::run_script()`.
//! When MCP tool dispatch is needed, starts the relevant MCP servers and wires
//! McpRuntime via the McpDispatchFn callback.

use crate::cli::QaAction;
use hkask_test_harness::qa_script::QaStatus;
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
            } else {
                script
            };

            println!("Running QA script: {}", manifest_path.display());

            // Only start MCP servers if the manifest uses mcp_tool steps; avoids
            // spawning servers for run_command/classify-only manifests. (F10 — the
            // full tool_name->server_id mapping is circular: the registry requires
            // servers running, so we gate on mcp_tool presence instead.)
            let manifest_uses_mcp = std::fs::read_to_string(&manifest_path)
                .map(|c| c.contains("mcp_tool"))
                .unwrap_or(false);
            let mcp_dispatch = if manifest_uses_mcp {
                setup_mcp_dispatch(&rt)
            } else {
                None
            };

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
                    if output.status == QaStatus::Fail {
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
/// Creates McpRuntime directly (no AgentService/database needed) and starts
/// the specific servers referenced by mcp_tool manifests.
fn setup_mcp_dispatch(
    rt: &tokio::runtime::Runtime,
) -> Option<hkask_test_harness::qa_script::McpDispatchFn> {
    use hkask_mcp::McpRuntime;
    use hkask_test_harness::qa_script::QaDispatchError;

    let runtime = McpRuntime::new();

    // Resolve replicant identity from the environment (set by
    // propagate_replicant_env in the REPL, or manually for CLI use).
    // Without HKASK_MCP_HOST, MCP servers fail with MissingHostIdentity.
    let replicant_name = std::env::var("HKASK_MCP_HOST").unwrap_or_else(|_| "qa-agent".to_string());
    let env = super::helpers::replicant_env_map(&replicant_name);

    // Start the MCP servers that QA dispatch smoke tests need.
    // These are the ones referenced in qa-mcp-dispatch-smoke.yaml.
    let servers: &[(&str, &str)] = &[
        ("skill", "hkask-mcp-skill"),
        ("kanban", "hkask-mcp-kata-kanban"),
        ("condenser", "hkask-mcp-condenser"),
        ("media", "hkask-mcp-media"),
    ];

    for (server_id, binary) in servers {
        if let Err(e) = rt.block_on(runtime.start_server_with_env(server_id, binary, env.clone())) {
            eprintln!(
                "Note: MCP server '{}' unavailable ({}) — mcp_tool steps will use stub",
                server_id, e
            );
            return None;
        }
    }

    let mcp = Arc::new(runtime);

    // Build dispatch closure: maps (tool_name, tool_params) → text output
    let dispatch: hkask_test_harness::qa_script::McpDispatchFn =
        Arc::new(move |tool_name: String, tool_params: String| {
            let mcp = mcp.clone();
            Box::pin(async move {
                let args: serde_json::Map<String, serde_json::Value> =
                    match serde_json::from_str(&tool_params) {
                        Ok(serde_json::Value::Object(map)) => map,
                        _ => serde_json::Map::new(),
                    };

                // Resolve server_id from tool registry
                let server_id = match mcp.get_tool_info(&tool_name).await {
                    Some(info) => info.server_id,
                    None => {
                        return Err(QaDispatchError::ToolNotFound { tool: tool_name });
                    }
                };

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
                    Err(e) => Err(QaDispatchError::DispatchError {
                        message: e.to_string(),
                    }),
                }
            })
        });

    Some(dispatch)
}
