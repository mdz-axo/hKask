//! CLI adapter command handlers — thin wrappers that delegate to AdapterPort.
//!
//! The adapter lifecycle operations (list, deploy, status, teardown) are
//! canonical `hkask_adapter::AdapterPort` trait methods. These CLI commands
//! print usage guidance pointing users at the adapter crate's direct surface,
//! since the training MCP server no longer wraps these (deleted 2026-07-19).
//!
//! To call these programmatically, use `AdapterPort::{list_adapters,
//! create_endpoint, endpoint_status, teardown_endpoint}` via the
//! `hkask-adapter` crate directly.

use crate::cli::AdapterAction;

/// Dispatch adapter subcommands.
pub fn run(action: AdapterAction) {
    match action {
        AdapterAction::List { skill } => {
            println!("Adapter list — delegates to AdapterPort::list_adapters.");
            println!();
            if let Some(s) = skill {
                println!("  kask adapter list --skill \"{}\"", s);
            } else {
                println!("  kask adapter list");
            }
        }
        AdapterAction::Deploy { adapter, provider } => {
            println!(
                "Deploy adapter '{}' to {} — delegates to AdapterPort::create_endpoint.",
                adapter, provider
            );
            println!();
            println!(
                "  kask adapter deploy --adapter \"{}\" --provider \"{}\"",
                adapter, provider
            );
        }
        AdapterAction::Status { deployment_id } => {
            println!(
                "Deployment status '{}' — delegates to AdapterPort::endpoint_status.",
                deployment_id
            );
            println!();
            println!(
                "  kask adapter status --deployment-id \"{}\"",
                deployment_id
            );
        }
        AdapterAction::Teardown { deployment_id } => {
            println!(
                "Teardown deployment '{}' — delegates to AdapterPort::teardown_endpoint.",
                deployment_id
            );
            println!();
            println!(
                "  kask adapter teardown --deployment-id \"{}\"",
                deployment_id
            );
        }
    }
}
