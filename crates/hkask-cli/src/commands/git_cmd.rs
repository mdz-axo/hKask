//! Git command handlers for `kask git`
//!
//! Implements the CLI display logic for registry git archival operations.

use crate::block_on;
use crate::cli::GitAction;
use crate::commands;

pub fn run(rt: &tokio::runtime::Runtime, action: GitAction) {
    let runtime = hkask_mcp::runtime::McpRuntime::new();

    // Resolve ACP secret and create CapabilityChecker for token minting (G9)
    let acp_secret = super::helpers::or_exit(
        super::config::resolve_acp_secret(),
        "Failed to resolve ACP secret for capability tokens",
    );
    let checker = hkask_types::CapabilityChecker::new(acp_secret.as_bytes());

    match action {
        GitAction::Archive {
            owner,
            repo,
            branch,
            path,
            content,
            file,
        } => {
            let content_str = if let Some(c) = content {
                c
            } else if let Some(f) = file {
                super::helpers::or_exit(std::fs::read_to_string(&f), "Failed to read file")
            } else {
                eprintln!("Either --content or --file must be provided");
                std::process::exit(1);
            };
            println!(
                "{}",
                block_on!(
                    rt,
                    commands::archive_registry_to_git(
                        &runtime,
                        &checker,
                        &owner,
                        &repo,
                        &branch,
                        &path,
                        &content_str,
                    ),
                    "Archive failed"
                )
            );
        }
        GitAction::Restore {
            owner,
            repo,
            r#ref,
            target,
        } => {
            println!(
                "{}",
                block_on!(
                    rt,
                    commands::restore_registry_from_git(
                        &runtime, &checker, &owner, &repo, &r#ref, &target,
                    ),
                    "Restore failed"
                )
            );
        }
        GitAction::List { owner, repo } => {
            let commits = block_on!(
                rt,
                commands::list_registry_archives(&runtime, &checker, &owner, &repo,),
                "List failed"
            );
            println!("Archived versions for {}/{}:", owner, repo);
            for (i, sha) in commits.iter().enumerate() {
                println!("  {}. {}", i + 1, sha);
            }
        }
        GitAction::Snapshot {
            owner,
            repo,
            message,
        } => {
            println!(
                "{}",
                block_on!(
                    rt,
                    commands::create_registry_snapshot(&runtime, &checker, &owner, &repo, &message,),
                    "Snapshot failed"
                )
            );
        }
    }
}
