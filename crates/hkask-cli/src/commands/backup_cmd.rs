//! Backup command handlers for `kask backup`
//!
//! Implements CLI display logic for backup operations. All business logic
//! delegates to `hkask_services::BackupService`.

use std::sync::Arc;

use hkask_services::backup::config::RetentionPolicy;
use hkask_services::{ArtifactType, BackupScope, BackupService, ListFilter, RestoreScope};
use hkask_types::ports::git_cas::GitCASPort;

use crate::block_on;
use crate::cli::BackupAction;

/// Resolve the hexagonal `GitCASPort` from the environment.
fn resolve_git_cas_port() -> Arc<dyn GitCASPort> {
    let adapter = super::helpers::or_exit(
        hkask_mcp::GixCasAdapter::from_env(),
        "Failed to initialize CAS adapter",
    );
    Arc::new(adapter) as Arc<dyn GitCASPort>
}

/// Parse an artifact type from a CLI string.
fn parse_artifact_type(s: &str) -> Option<ArtifactType> {
    match s {
        "template" => Some(ArtifactType::Template),
        "style" => Some(ArtifactType::Style),
        "goal" => Some(ArtifactType::Goal),
        "spec" => Some(ArtifactType::Spec),
        "memory" | "memory_triple" => Some(ArtifactType::MemoryTriple),
        "embedding" => Some(ArtifactType::Embedding),
        "registry" | "registry_entry" => Some(ArtifactType::RegistryEntry),
        "cns" | "cns_audit" => Some(ArtifactType::CnsAudit),
        "sovereignty" | "sovereignty_manifest" => Some(ArtifactType::SovereigntyManifest),
        "session" => Some(ArtifactType::Session),
        "wallet" | "wallet_state" => Some(ArtifactType::WalletState),
        "settings" => Some(ArtifactType::Settings),
        _ => None,
    }
}

/// Parse a comma-separated list of artifact types.
fn parse_artifact_types(s: &str) -> Vec<ArtifactType> {
    s.split(',')
        .map(|s| s.trim())
        .filter_map(parse_artifact_type)
        .collect()
}

/// Parse a backup scope from a CLI string.
fn parse_scope(s: &str) -> BackupScope {
    match s {
        "full" | "" => BackupScope::Full,
        other => {
            if let Some(at) = parse_artifact_type(other) {
                BackupScope::ByType(at)
            } else {
                eprintln!("Unknown scope '{}', defaulting to full", other);
                BackupScope::Full
            }
        }
    }
}

/// Parse a restore scope from a CLI string.
fn parse_restore_scope(s: &str) -> RestoreScope {
    match s {
        "full" | "" => RestoreScope::Full,
        other => {
            if let Some(at) = parse_artifact_type(other) {
                RestoreScope::ByType(at)
            } else {
                eprintln!("Unknown scope '{}', defaulting to full", other);
                RestoreScope::Full
            }
        }
    }
}

pub fn run(rt: &tokio::runtime::Runtime, action: BackupAction) {
    match action {
        BackupAction::Snapshot { scope } => {
            let port = resolve_git_cas_port();
            let svc = BackupService::new(port);
            let backup_scope = parse_scope(&scope);

            // Manual snapshots require the caller to provide artifact data.
            // For now, manual snapshots snapshot whatever is already in the CAS repos.
            // Full auto-snapshot on mutation is deferred to F4.
            let result = block_on!(rt, svc.snapshot(backup_scope, &[]), "Snapshot failed");
            println!("Snapshot created:");
            for (repo, commit) in &result.commits {
                println!("  {}: {}", repo.dir_name(), commit);
            }
            println!("  Artifacts: {}", result.artifact_count);
            println!("  Timestamp: {}", result.timestamp);
        }

        BackupAction::Restore { commit, scope } => {
            let port = resolve_git_cas_port();
            let svc = BackupService::new(port);
            let restore_scope = parse_restore_scope(&scope);

            let commit_hash: hkask_types::ports::git_cas::CommitHash =
                commit.parse().unwrap_or_else(|e: String| {
                    eprintln!("Invalid commit hash '{}': {}", commit, e);
                    std::process::exit(1);
                });

            let artifacts = block_on!(
                rt,
                svc.restore(&commit_hash, restore_scope),
                "Restore failed"
            );

            println!("Restored {} artifacts:", artifacts.len());
            for (at, id, _bytes) in &artifacts {
                println!("  {}: {}", at.label(), id);
            }
        }

        BackupAction::List { r#type, limit } => {
            let port = resolve_git_cas_port();
            let svc = BackupService::new(port);

            let filter = ListFilter {
                artifact_type: r#type.as_deref().and_then(parse_artifact_type),
                limit: Some(limit),
            };

            let snapshots = block_on!(rt, svc.list(filter), "List failed");

            println!("Backup snapshots:");
            for (i, snap) in snapshots.iter().enumerate() {
                println!(
                    "  {}. {} — {}",
                    i + 1,
                    snap.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    snap.commits
                        .first()
                        .map(|(_, c)| c.to_string())
                        .unwrap_or_default()
                );
            }
        }

        BackupAction::Prune { execute } => {
            let port = resolve_git_cas_port();
            let svc = BackupService::new(port);

            let dry_run = !execute;
            let report = block_on!(rt, svc.prune(dry_run), "Prune failed");

            if report.evaluated == 0 {
                println!("No retention policy configured — nothing to prune.");
                return;
            }

            if dry_run {
                println!("Prune dry-run report:");
            } else {
                println!("Prune report:");
            }
            println!("  Evaluated: {}", report.evaluated);
            println!("  Retained:  {}", report.retained);
            println!("  Removed:   {}", report.removed.len());
            for (repo, commit) in &report.removed {
                println!("    {}: {}", repo.dir_name(), commit);
            }
        }

        BackupAction::Verify => {
            let port = resolve_git_cas_port();
            let svc = BackupService::new(port);

            let reports = block_on!(rt, svc.verify(), "Verify failed");

            println!("Backup integrity report:");
            for report in &reports {
                let status = if report.corrupt_hashes.is_empty() {
                    "✓ OK"
                } else {
                    "✗ CORRUPT"
                };
                println!(
                    "  {}: {} ({} blobs, {} verified)",
                    report.repo.dir_name(),
                    status,
                    report.total_blobs,
                    report.verified_blobs
                );
                for hash in &report.corrupt_hashes {
                    println!("    Corrupt: {}", hash);
                }
            }
        }

        BackupAction::Config { action } => match action {
            crate::cli::ConfigAction::Show => {
                let port = resolve_git_cas_port();
                let svc = BackupService::new(port);
                let config = svc.config();

                println!("Backup configuration:");
                println!("  Tracked types:");
                if config.tracked_types.is_empty() {
                    println!("    (none)");
                } else {
                    for at in &config.tracked_types {
                        println!("    - {}", at.label());
                    }
                }
                println!("  Auto-snapshot: {}", config.auto_snapshot);
                println!("  Verify after snapshot: {}", config.verify_after_snapshot);
                match &config.retention {
                    Some(rp) => {
                        println!(
                            "  Retention: {}d daily, {}w weekly",
                            rp.daily_days, rp.weekly_weeks
                        );
                    }
                    None => println!("  Retention: forever"),
                }
            }

            crate::cli::ConfigAction::Set {
                types,
                retention: _retention,
                no_auto,
            } => {
                let port = resolve_git_cas_port();
                let mut svc = BackupService::new(port);

                let mut config = svc.config().clone();
                config.tracked_types = parse_artifact_types(&types);

                if let Some(dur_str) = _retention {
                    let days: u32 = dur_str.trim_end_matches('d').parse().unwrap_or(21);
                    config.retention = Some(RetentionPolicy {
                        daily_days: days,
                        weekly_weeks: 12,
                    });
                }

                if no_auto {
                    config.auto_snapshot = false;
                }

                svc.update_config(config)
                    .map_err(|e| {
                        eprintln!("Config update failed: {}", e);
                        std::process::exit(1);
                    })
                    .ok();
                println!("Backup configuration updated.");
            }
        },
    }
}
