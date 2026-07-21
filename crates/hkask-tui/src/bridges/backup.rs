//! BackupDataBridge — trait for backup status in the TUI.
//!
//! Provides the Backup window with live snapshot data. Read-only —
//! destructive operations (snapshot, restore, prune) remain CLI-only.

use std::sync::Arc;

/// Summary of the most recent snapshot for display.
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Number of artifacts in this snapshot
    pub artifact_count: usize,
    /// How the snapshot was triggered (Manual / Auto / Safety)
    pub trigger: String,
    /// Total repository commits tracked
    pub commit_count: usize,
}

/// Summary of backup configuration.
#[derive(Debug, Clone)]
pub struct BackupConfigSummary {
    pub auto_snapshot: bool,
    pub verify_after_snapshot: bool,
    pub encryption_enabled: bool,
    pub tracked_types_count: usize,
    pub retention_daily_days: u32,
    pub retention_weekly_weeks: u32,
}

#[derive(Debug, Clone)]
pub struct BackupReady {
    pub last_snapshot: Option<SnapshotInfo>,
    pub snapshot_count: usize,
    pub config: BackupConfigSummary,
    pub verified: bool,
    pub verification_detail: String,
}

#[derive(Debug, Clone)]
pub enum BackupSnapshot {
    Unavailable { reason: String },
    Ready(BackupReady),
    Failed { error: String },
}

/// Trait for querying backup subsystem state as one coherent snapshot.
pub trait BackupDataBridge: Send + Sync {
    fn snapshot(&self) -> BackupSnapshot;
}

/// Mock implementation for TUI development and testing.
pub struct MockBackupBridge {
    pub last: Option<SnapshotInfo>,
    pub count: usize,
    pub cfg: BackupConfigSummary,
    pub verified: bool,
}

impl Default for MockBackupBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBackupBridge {
    pub fn new() -> Self {
        Self {
            last: None,
            count: 0,
            cfg: BackupConfigSummary {
                auto_snapshot: true,
                verify_after_snapshot: false,
                encryption_enabled: false,
                tracked_types_count: 18,
                retention_daily_days: 21,
                retention_weekly_weeks: 12,
            },
            verified: false,
        }
    }

    pub fn with_snapshot(mut self, timestamp: &str, artifacts: usize) -> Self {
        self.count = 1;
        self.last = Some(SnapshotInfo {
            timestamp: timestamp.into(),
            artifact_count: artifacts,
            trigger: "Manual".into(),
            commit_count: artifacts,
        });
        self
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl BackupDataBridge for MockBackupBridge {
    fn snapshot(&self) -> BackupSnapshot {
        BackupSnapshot::Ready(BackupReady {
            last_snapshot: self.last.clone(),
            snapshot_count: self.count,
            config: self.cfg.clone(),
            verified: self.verified,
            verification_detail: if self.verified {
                "All repos healthy".into()
            } else {
                "No verification run".into()
            },
        })
    }
}
