//! Git command handlers for `kask git`
//!
//! Implements the CLI display logic for registry git archival operations
//! and local CAS operations (verify, diff, log, snapshot).

use std::sync::Arc;

use crate::block_on;
use crate::cli::GitAction;
use crate::commands;
use hkask_mcp::GixCasAdapter;
use hkask_types::ports::git_cas::{GitCASPort, RepoId, TreeEntryKind};

/// Resolve the hexagonal `GitCASPort` from the environment.
///
/// Returns `Arc<dyn GitCASPort>` so CLI commands share the same trait boundary
/// as API and MCP servers (MCP ≡ CLI ≡ API parity).
fn resolve_git_cas_port() -> Arc<dyn GitCASPort> {
    let adapter = super::helpers::or_exit(
        GixCasAdapter::from_env(),
        "Failed to initialize CAS adapter",
    );
    Arc::new(adapter) as Arc<dyn GitCASPort>
}

/// Parse a RepoId from a string, returning Registry as default.
fn parse_repo_id(repo: &str) -> RepoId {
    match repo {
        "registry" | "" => RepoId::Registry,
        "memory" => RepoId::Memory,
        "cns-audit" => RepoId::CnsAudit,
        "sovereignty" => RepoId::Sovereignty,
        "goals-specs" => RepoId::GoalsSpecs,
        "sessions" => RepoId::Sessions,
        "vault" => RepoId::Vault,
        _ => {
            eprintln!("Unknown repo '{}', defaulting to 'registry'", repo);
            RepoId::Registry
        }
    }
}

pub fn run(rt: &tokio::runtime::Runtime, action: GitAction) {
    match action {
        // ── GitHub API operations (existing) ──────────────────────────────
        GitAction::Archive {
            owner,
            repo,
            branch,
            path,
            content,
            file,
        } => {
            let runtime = hkask_mcp::runtime::McpRuntime::new();
            let acp_secret = super::helpers::or_exit(
                super::config::resolve_acp_secret(),
                "Failed to resolve ACP secret for capability tokens",
            );
            let checker = hkask_types::CapabilityChecker::new(acp_secret.as_bytes());

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
            let runtime = hkask_mcp::runtime::McpRuntime::new();
            let acp_secret = super::helpers::or_exit(
                super::config::resolve_acp_secret(),
                "Failed to resolve ACP secret for capability tokens",
            );
            let checker = hkask_types::CapabilityChecker::new(acp_secret.as_bytes());

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
            let runtime = hkask_mcp::runtime::McpRuntime::new();
            let acp_secret = super::helpers::or_exit(
                super::config::resolve_acp_secret(),
                "Failed to resolve ACP secret for capability tokens",
            );
            let checker = hkask_types::CapabilityChecker::new(acp_secret.as_bytes());

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
            let runtime = hkask_mcp::runtime::McpRuntime::new();
            let acp_secret = super::helpers::or_exit(
                super::config::resolve_acp_secret(),
                "Failed to resolve ACP secret for capability tokens",
            );
            let checker = hkask_types::CapabilityChecker::new(acp_secret.as_bytes());

            println!(
                "{}",
                block_on!(
                    rt,
                    commands::create_registry_snapshot(&runtime, &checker, &owner, &repo, &message,),
                    "Snapshot failed"
                )
            );
        }

        // ── Local CAS operations (Phase 5) ──────────────────────────────
        GitAction::CasVerify { repo } => {
            let port = resolve_git_cas_port();
            let repo_id = parse_repo_id(&repo);
            let report = block_on!(rt, port.verify(&repo_id), "Verify failed");

            println!("Verification report for '{}':", report.repo.dir_name());
            println!("  Total blobs:   {}", report.total_blobs);
            println!("  Verified:      {}", report.verified_blobs);
            if report.corrupt_hashes.is_empty() {
                println!("  Integrity:     ✓ OK");
            } else {
                println!("  Integrity:    ✗ CORRUPT");
                for hash in &report.corrupt_hashes {
                    println!("    Corrupt: {}", hash);
                }
            }
        }

        GitAction::CasDiff { repo, from, to } => {
            let port = resolve_git_cas_port();
            let repo_id = parse_repo_id(&repo);
            let diffs = block_on!(rt, port.diff(&repo_id, &from, &to), "Diff failed");

            println!("Diff for '{}' ({} → {}):", repo_id.dir_name(), from, to);
            if diffs.is_empty() {
                println!("  No changes.");
            } else {
                for d in &diffs {
                    println!("  {:?} {}", d.kind, d.path);
                }
            }
        }

        GitAction::CasLog { repo, max_count } => {
            let port = resolve_git_cas_port();
            let repo_id = parse_repo_id(&repo);
            let entries = block_on!(rt, port.log(&repo_id, max_count), "Log failed");

            if entries.is_empty() {
                println!("No snapshots found for '{}'.", repo_id.dir_name());
            } else {
                println!("Snapshots for '{}':", repo_id.dir_name());
                for (i, entry) in entries.iter().enumerate() {
                    println!("  {}. {} {}", i + 1, entry.commit, entry.message,);
                }
            }
        }

        GitAction::CasSnapshot { repo, message } => {
            let port = resolve_git_cas_port();
            let repo_id = parse_repo_id(&repo);
            let commit = block_on!(rt, port.snapshot(&repo_id, &message), "Snapshot failed");

            println!("Snapshot created for '{}': {}", repo_id.dir_name(), commit);
        }

        GitAction::CasRestore {
            repo,
            r#ref,
            prefix,
        } => {
            let port = resolve_git_cas_port();
            let repo_id = parse_repo_id(&repo);
            let reference = r#ref.as_deref().unwrap_or("HEAD");
            let prefix_str = prefix.as_deref().unwrap_or("");

            let entries = block_on!(
                rt,
                port.list_tree(&repo_id, reference, prefix_str),
                "List tree failed"
            );

            if entries.is_empty() {
                println!(
                    "No entries found for '{}' at '{}'.",
                    repo_id.dir_name(),
                    reference
                );
            } else {
                println!(
                    "Restoring {} entries from '{}' at '{}':",
                    entries.len(),
                    repo_id.dir_name(),
                    reference
                );
                for entry in &entries {
                    if entry.kind == TreeEntryKind::Blob {
                        let content = block_on!(
                            rt,
                            port.get_blob(&repo_id, &entry.content_hash),
                            "Get blob failed"
                        );
                        println!(
                            "  {} ({} bytes, hash: {})",
                            entry.path,
                            content.len(),
                            entry.content_hash
                        );
                    } else {
                        println!("  {} (tree)", entry.path);
                    }
                }
            }
        }
    }
}
