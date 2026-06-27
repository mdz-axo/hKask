//! Backup command handlers for `kask backup`
//!
//! Implements CLI display logic for backup operations. All business logic
//! delegates to `hkask_services::BackupService`.

use std::sync::Arc;

use hkask_ports::git_cas::GitCASPort;
use hkask_services::RetentionPolicy;
use hkask_services::{ArtifactType, BackupScope, BackupService, ListFilter, RestoreScope};
use std::str::FromStr;

use crate::block_on;
use crate::cli::BackupAction;
use hkask_services::load_backup_config;

/// Resolve the hexagonal `GitCASPort` from the environment.
fn resolve_git_cas_port() -> Arc<dyn GitCASPort> {
    let adapter = super::helpers::or_exit(
        hkask_mcp::GixCasAdapter::from_env(),
        "Failed to initialize CAS adapter",
    );
    Arc::new(adapter) as Arc<dyn GitCASPort>
}

/// Parse a comma-separated list of artifact types.
fn parse_artifact_types(s: &str) -> Vec<ArtifactType> {
    s.split(',')
        .map(|s| s.trim())
        .filter_map(|s| ArtifactType::from_str(s).ok())
        .collect()
}

/// Parse a backup scope from a CLI string.
fn parse_scope(s: &str) -> BackupScope {
    match s {
        "full" | "" => BackupScope::Full,
        other => {
            if let Ok(at) = ArtifactType::from_str(other) {
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
            if let Ok(at) = ArtifactType::from_str(other) {
                RestoreScope::ByType(at)
            } else {
                eprintln!("Unknown scope '{}', defaulting to full", other);
                RestoreScope::Full
            }
        }
    }
}

/// Run a backup operation.
///
/// expect: "I can access all hKask functionality through the kask CLI"
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is valid, action is valid
/// post: backup operation executed
pub fn run(rt: &tokio::runtime::Runtime, action: BackupAction) {
    // P9: CNS span
    tracing::info!(target: "cns.cli", operation = "backup", action = ?action, "CNS");
    match action {
        BackupAction::Snapshot { scope } => {
            let port = resolve_git_cas_port();
            let svc = BackupService::new(port, load_backup_config());
            let _backup_scope = parse_scope(&scope);

            // Manual snapshots commit whatever blobs are already in CAS repos
            // (populated by the daemon's BackupLoop artifact producers).
            // If no blobs exist yet, run `kask backup status` to check daemon health.
            let result = block_on!(rt, svc.run_daily_snapshot(), "Snapshot failed");
            if result.commits.is_empty() {
                println!("No tracked repos configured or no blobs to snapshot.");
                println!("Configure tracking with: kask backup config set --types TYPE,...");
                println!("Check daemon status with: kask backup status");
                return;
            }
            println!("Snapshot created:");
            for (repo, commit) in &result.commits {
                println!("  {}: {}", repo.dir_name(), commit);
            }
            println!("  Artifacts: {}", result.artifact_count.unwrap_or(0));
            println!("  Timestamp: {}", result.timestamp);
        }

        BackupAction::Restore {
            commit,
            scope,
            output,
        } => {
            let port = resolve_git_cas_port();
            let svc = BackupService::new(port, load_backup_config());
            let restore_scope = parse_restore_scope(&scope);

            let commit_hash: hkask_ports::git_cas::CommitHash =
                commit
                    .parse()
                    .unwrap_or_else(|e: hkask_ports::git_cas::ParseHashError| {
                        eprintln!("Invalid commit hash '{}': {}", commit, e);
                        std::process::exit(1);
                    });

            let artifacts = block_on!(
                rt,
                svc.scoped_restore(&commit_hash, restore_scope),
                "Restore failed"
            );

            println!("Restored {} artifacts:", artifacts.len());
            for (at, id, bytes) in &artifacts {
                println!("  {}/{} ({} bytes)", at.label(), id, bytes.len());
            }

            // Write artifacts to output directory if specified
            if let Some(ref out_dir) = output {
                std::fs::create_dir_all(out_dir).unwrap_or_else(|e| {
                    eprintln!("Failed to create output directory '{}': {}", out_dir, e);
                    std::process::exit(1);
                });
                for (at, id, bytes) in &artifacts {
                    let filename = format!("{}-{}.json", at.label(), id);
                    let path = std::path::Path::new(out_dir).join(&filename);
                    std::fs::write(&path, bytes).unwrap_or_else(|e| {
                        eprintln!("Failed to write '{}': {}", path.display(), e);
                    });
                    println!("  → wrote {}", path.display());
                }
                println!("\nArtifacts written to: {}", out_dir);
                println!(
                    "Each file is a JSON envelope. Restore to stores requires store-specific logic."
                );
            } else {
                println!("\nUse --output <dir> to write restored artifacts to disk.");
            }
        }

        BackupAction::List { r#type, limit } => {
            let port = resolve_git_cas_port();
            let svc = BackupService::new(port, load_backup_config());

            let filter = ListFilter {
                artifact_type: r#type
                    .as_deref()
                    .and_then(|s| ArtifactType::from_str(s).ok()),
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
            let svc = BackupService::new(port, load_backup_config());

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

        BackupAction::Status => {
            let port = resolve_git_cas_port();
            let svc = BackupService::new(port, load_backup_config());
            let config = svc.config();

            println!("Backup health:");
            println!(
                "  Auto-snapshot: {}",
                if config.auto_snapshot {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            println!(
                "  Tracked types: {}",
                if config.tracked_types.is_empty() {
                    "(none — configure with 'kask backup config set')".to_string()
                } else {
                    config
                        .tracked_types
                        .iter()
                        .map(|t| t.label())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            );
            println!(
                "  Retention: {}",
                match &config.retention {
                    Some(rp) => format!("{}d daily, {}w weekly", rp.daily_days, rp.weekly_weeks),
                    None => "forever".to_string(),
                }
            );
            println!(
                "  Encryption: {}",
                if config.encryption.is_some() {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            println!("  Verify after snapshot: {}", config.verify_after_snapshot);

            // Show most recent snapshot
            let filter = ListFilter {
                artifact_type: None,
                limit: Some(1),
            };
            let snapshots = block_on!(rt, svc.list(filter), "Status check failed");
            if !snapshots.is_empty() {
                let last = &snapshots[0];
                println!(
                    "\n  Last snapshot: {}",
                    last.timestamp.format("%Y-%m-%d %H:%M:%S")
                );
                for (repo, commit) in &last.commits {
                    println!("    {}: {}", repo.dir_name(), commit);
                }
            } else {
                println!("\n  Last snapshot: (none — daemon may not have run yet)");
            }
        }

        BackupAction::Verify => {
            let port = resolve_git_cas_port();
            let svc = BackupService::new(port, load_backup_config());

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
                let svc = BackupService::new(port, load_backup_config());
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
                let mut svc = BackupService::new(port, load_backup_config());

                let mut config = svc.config().clone();
                config.tracked_types = parse_artifact_types(&types);

                if let Some(dur_str) = _retention {
                    config.retention = Some(
                        RetentionPolicy::from_duration_str(&dur_str).unwrap_or_else(|e| {
                            eprintln!("Invalid retention duration '{}': {}", dur_str, e);
                            std::process::exit(1);
                        }),
                    );
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
