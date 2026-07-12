//! CLI adapter command handlers — thin wrappers that delegate to MCP tools.
//!
//! The adapter lifecycle tools live in the training MCP server.
//! These CLI commands print usage guidance pointing users to the MCP gateway,
//! since `kask tool call` is the universal dispatch mechanism.

use crate::cli::AdapterAction;

/// Dispatch adapter subcommands.
pub fn run(action: AdapterAction) {
    match action {
        AdapterAction::List { skill } => {
            println!("Adapter list — delegates to training MCP.");
            println!();
            if let Some(s) = skill {
                println!(
                    "  kask tool call training_list_adapters --params '{{\"skill_name\": \"{}\"}}'",
                    s
                );
            } else {
                println!("  kask tool call training_list_adapters");
            }
        }
        AdapterAction::Deploy { adapter, provider } => {
            println!(
                "Deploy adapter '{}' to {} — delegates to training MCP.",
                adapter, provider
            );
            println!();
            println!(
                "  kask tool call training_deploy --params '{{\"adapter_name\": \"{}\", \"provider\": \"{}\"}}'",
                adapter, provider
            );
            println!();
            println!(
                "Or via REPL: /training_deploy adapter_name=\"{}\" provider=\"{}\"",
                adapter, provider
            );
        }
        AdapterAction::Status { deployment_id } => {
            println!(
                "Deployment status '{}' — delegates to training MCP.",
                deployment_id
            );
            println!();
            println!(
                "  kask tool call training_deployment_status --params '{{\"deployment_id\": \"{}\"}}'",
                deployment_id
            );
        }
        AdapterAction::Teardown { deployment_id } => {
            println!(
                "Teardown deployment '{}' — delegates to training MCP.",
                deployment_id
            );
            println!();
            println!(
                "  kask tool call training_teardown --params '{{\"deployment_id\": \"{}\"}}'",
                deployment_id
            );
        }
    }
}
