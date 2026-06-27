//! Backup command handlers for `kask backup`
//!
//! Implements CLI display logic for backup operations. All business logic
//! delegates to `hkask_services::BackupService`.

use std::sync::Arc;

use hkask_ports::git_cas::GitCASPort;
use hkask_services::RetentionPolicy;
use hkask_services::{ArtifactType, BackupScope, BackupService, ListFilter};
use std::str::FromStr;

use crate::block_on;
use crate::cli::BackupAction;
use hkask_services::load_backup_config;

/// Resolve the concrete `GixCasAdapter` for pod-directory backup operations.
fn resolve_gix_adapter() -> hkask_mcp::GixCasAdapter {
    super::helpers::or_exit(
        hkask_mcp::GixCasAdapter::from_env(),
        "Failed to initialize CAS adapter",
    )
}

/// Resolve the hexagonal `GitCASPort` from the environment (for old BackupService ops).
fn resolve_git_cas_port() -> Arc<dyn GitCASPort> {
    Arc::new(resolve_gix_adapter()) as Arc<dyn GitCASPort>
}

/// Parse a date string like "2026-06-27" or "2026-06-27T08:00:00" to Unix seconds.
fn parse_date(s: &str) -> u64 {
    // Try ISO 8601 with time first
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.timestamp() as u64;
    }
    // Try date-only: append T00:00:00Z
    let with_time = format!("{}T00:00:00Z", s);
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&with_time) {
        return dt.timestamp() as u64;
    }
    eprintln!(
        "Invalid date '{}'. Use YYYY-MM-DD or YYYY-MM-DDTHH:MM:SS",
        s
    );
    std::process::exit(1);
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

/// Parse a backup scope from a CLI string.
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

        BackupAction::Restore { pod, date, commit } => {
            let adapter = resolve_gix_adapter();

            // Resolve pod directory
            let sanitized = hkask_types::agent_paths::sanitize_name(&pod);
            let base = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            let pod_dir = base.join("hkask").join("agents").join(&sanitized);
            if !pod_dir.join("pod.db").exists() {
                eprintln!("Pod '{}' not found at {}", pod, pod_dir.display());
                std::process::exit(1);
            }

            // Resolve commit from date or hash
            let commit_hash = if let Some(ref date_str) = date {
                let target = parse_date(date_str);
                match block_on!(
                    rt,
                    adapter.resolve_date(&pod_dir, target),
                    "Date lookup failed"
                ) {
                    Some(hash) => hash,
                    None => {
                        eprintln!("No snapshots found before {}", date_str);
                        std::process::exit(1);
                    }
                }
            } else if let Some(ref hash_str) = commit {
                hash_str
                    .parse()
                    .unwrap_or_else(|e: hkask_ports::git_cas::ParseHashError| {
                        eprintln!("Invalid commit hash '{}': {}", hash_str, e);
                        std::process::exit(1);
                    })
            } else {
                eprintln!("Specify --date YYYY-MM-DD or --commit HASH");
                std::process::exit(1);
            };

            // Restore pod.db from the commit
            block_on!(
                rt,
                adapter.restore_file_from_commit(
                    &pod_dir,
                    &commit_hash,
                    "pod.db",
                    &pod_dir.join("pod.db")
                ),
                "Restore failed"
            );

            println!("Pod '{}' restored to commit {}", pod, commit_hash);
            println!("Restart the pod to apply the restored state:");
            println!("  kask pod deactivate {} && kask pod activate {}", pod, pod);
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
