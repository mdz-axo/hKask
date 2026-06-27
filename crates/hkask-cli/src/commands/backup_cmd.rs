//! Backup command handlers for `kask backup`
//!
//! Implements CLI display logic for backup operations. All business logic
//! delegates to `hkask_services::BackupService`.

use std::sync::Arc;

use hkask_ports::git_cas::GitCASPort;
use hkask_services::RetentionPolicy;
use hkask_services::{ArtifactType, BackupService, ListFilter};
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

/// Parse a date string "YYYY-MM-DD" or "YYYY-MM-DDTHH:MM:SS" to Unix seconds.
fn parse_date(s: &str) -> u64 {
    // Split date from optional time
    let (date_part, time_part) = if let Some(t_idx) = s.find('T') {
        (&s[..t_idx], Some(&s[t_idx + 1..]))
    } else {
        (s, None)
    };

    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        eprintln!("Invalid date '{}'. Use YYYY-MM-DD.", s);
        std::process::exit(1);
    }
    let year: i32 = parts[0].parse().unwrap_or(0);
    let month: u32 = parts[1].parse().unwrap_or(1);
    let day: u32 = parts[2].parse().unwrap_or(1);

    let (hour, min, sec) = if let Some(t) = time_part {
        let tp: Vec<&str> = t.split(':').collect();
        (
            tp.first().and_then(|v| v.parse().ok()).unwrap_or(0),
            tp.get(1).and_then(|v| v.parse().ok()).unwrap_or(0),
            tp.get(2).and_then(|v| v.parse().ok()).unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    };

    // Days since Unix epoch for the given date (approximate, good enough for backup lookup)
    let mut days = 0i64;
    for y in 1970..year as i64 {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month as usize {
        days += month_days[m - 1] as i64;
        if m == 2 && is_leap(year as i64) {
            days += 1;
        }
    }
    days += (day as i64) - 1;

    (days * 86400 + hour as i64 * 3600 + min as i64 * 60 + sec as i64) as u64
}

fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

/// Convert days since Unix epoch to "YYYY-MM-DD" string.
fn unix_days_to_date(days: i64) -> String {
    let mut remaining = days;
    let mut year = 1970i64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let month_days = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1usize;
    for md in &month_days {
        if remaining < *md as i64 {
            break;
        }
        remaining -= *md as i64;
        month += 1;
    }
    let day = remaining + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Parse a comma-separated list of artifact types.
fn parse_artifact_types(s: &str) -> Vec<ArtifactType> {
    s.split(',')
        .map(|s| s.trim())
        .filter_map(|s| ArtifactType::from_str(s).ok())
        .collect()
}

/// Parse a date string
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
        BackupAction::Snapshot { scope: _ } => {
            let adapter = resolve_gix_adapter();

            // Manual snapshot: snapshot all pod directories immediately.
            // ActivePods isn't available in CLI context, so we scan the agents dir.
            let base = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            let agents_dir = base.join("hkask").join("agents");
            if !agents_dir.exists() {
                println!(
                    "No agents directory found at {}. Nothing to snapshot.",
                    agents_dir.display()
                );
                return;
            }

            let mut count = 0usize;
            if let Ok(entries) = std::fs::read_dir(&agents_dir) {
                for entry in entries.flatten() {
                    let pod_dir = entry.path();
                    if !pod_dir.is_dir() || !pod_dir.join("pod.db").exists() {
                        continue;
                    }
                    let pod_name = pod_dir
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let commit = block_on!(
                        rt,
                        adapter.snapshot_pod_dir(&pod_dir, &format!("manual: {}", pod_name)),
                        "Snapshot failed"
                    );
                    println!("  {} → {}", pod_name, commit);
                    count += 1;
                }
            }

            println!("Snapshot {} pods.", count);
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

        BackupAction::List { limit } => {
            let adapter = resolve_gix_adapter();
            let base = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            let agents_dir = base.join("hkask").join("agents");

            if !agents_dir.exists() {
                println!("No agents directory found.");
                return;
            }

            println!("Pod backup snapshots:");
            let mut found = false;
            if let Ok(entries) = std::fs::read_dir(&agents_dir) {
                for entry in entries.flatten() {
                    let pod_dir = entry.path();
                    if !pod_dir.is_dir() || !pod_dir.join(".git").exists() {
                        continue;
                    }
                    let pod_name = pod_dir
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let commits = block_on!(rt, adapter.log_pod(&pod_dir, limit), "List failed");
                    if commits.is_empty() {
                        continue;
                    }
                    found = true;
                    println!("\n  {}:", pod_name);
                    for (i, entry) in commits.iter().enumerate() {
                        let ts = entry.timestamp_secs;
                        let days = ts / 86400;
                        let time = ts % 86400;
                        let h = time / 3600;
                        let m = (time % 3600) / 60;
                        // Days since epoch → approximate date
                        let date = unix_days_to_date(days as i64);
                        println!(
                            "    {}. {} {:02}:{:02}  {}",
                            i + 1,
                            date,
                            h,
                            m,
                            entry.commit
                        );
                    }
                }
            }
            if !found {
                println!("  (no snapshots found)");
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
