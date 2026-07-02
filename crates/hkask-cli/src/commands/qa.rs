//! QA command handlers for `kask qa`
//!
//! Runs QA script manifests through `hkask_test_harness::qa_script::run_script()`.

use crate::cli::QaAction;
use std::path::Path;

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

            // If script path is absolute or relative to cwd, resolve it
            let manifest_path = if script.is_absolute() {
                script
                    .strip_prefix(&workspace_root)
                    .unwrap_or(&script)
                    .to_path_buf()
            } else if script.starts_with("registry/") {
                script
            } else {
                // Relative path — make it relative to workspace root
                script
            };

            println!("Running QA script: {}", manifest_path.display());

            match rt.block_on(hkask_test_harness::qa_script::run_script(
                workspace,
                &manifest_path,
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
