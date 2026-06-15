//! BackupLoop — Cybernetic loop for scheduled backup snapshots.
//! # REQ: F4 — daily automatic snapshots via the daemon loop system.
//!
//! Implements `HkaskLoop` (sense → compare → compute → act) to run
//! daily backup snapshots through `BackupService`. Respects the
//! `auto_snapshot` config flag and optionally runs verify + prune
//! after each snapshot.
//!
//! Registered in `AgentService::build()` alongside the existing
//! `SnapshotLoop` (which handles raw CAS-level snapshots).

use std::sync::Arc;
use std::time::Instant;

use hkask_types::loops::{
    ActionType, Deviation, HkaskLoop, LoopAction, LoopId, Signal, SignalMetric,
};
use parking_lot::RwLock;
use tracing::{info, warn};

use super::BackupService;

/// State tracked by the BackupLoop across cycles.
#[derive(Debug, Clone, Default)]
struct BackupLoopState {
    /// When the last daily snapshot completed.
    last_snapshot: Option<Instant>,
    /// When the last prune completed.
    last_prune: Option<Instant>,
}

/// Cybernetic loop that runs daily backup snapshots via `BackupService`.
///
/// Cycle:
/// - **Sense**: Check time since last snapshot (target: 24h)
/// - **Compare**: Detect if snapshot is overdue
/// - **Compute**: Produce snapshot + optional prune actions
/// - **Act**: Call `BackupService::run_daily_snapshot()`, then verify/prune
pub struct BackupLoop {
    service: Arc<BackupService>,
    state: Arc<RwLock<BackupLoopState>>,
}

impl BackupLoop {
    /// Create a new BackupLoop wrapping a BackupService.
    pub fn new(service: Arc<BackupService>) -> Self {
        Self {
            service,
            state: Arc::new(RwLock::new(BackupLoopState::default())),
        }
    }

    /// Check if auto-snapshot is enabled.
    fn auto_snapshot_enabled(&self) -> bool {
        self.service.config().auto_snapshot
    }

    /// Check if a daily snapshot is due (24h since last).
    fn is_snapshot_due(&self) -> bool {
        let state = self.state.read();
        match state.last_snapshot {
            Some(instant) => instant.elapsed().as_secs() >= 86400, // 24 hours
            None => true,                                          // Never snapshotted — do it now
        }
    }

    /// Record a successful snapshot.
    fn record_snapshot(&self) {
        let mut state = self.state.write();
        state.last_snapshot = Some(Instant::now());
    }

    /// Record a successful prune.
    fn record_prune(&self) {
        let mut state = self.state.write();
        state.last_prune = Some(Instant::now());
    }
}

#[async_trait::async_trait]
impl HkaskLoop for BackupLoop {
    fn id(&self) -> LoopId {
        LoopId::Cybernetics
    }

    /// Sense: measure time since last snapshot.
    async fn sense(&self) -> Vec<Signal> {
        if !self.auto_snapshot_enabled() {
            return Vec::new();
        }

        let state = self.state.read();
        let elapsed_secs = state
            .last_snapshot
            .map(|i| i.elapsed().as_secs())
            .unwrap_or(u64::MAX);

        vec![Signal::new(
            LoopId::Cybernetics,
            SignalMetric::SnapshotInterval,
            elapsed_secs as f64,
            86400.0, // 24-hour set-point
        )]
    }

    /// Compare: detect if snapshot is overdue.
    async fn compare(&self, signals: &[Signal]) -> Vec<Deviation> {
        signals
            .iter()
            .filter(|s| {
                s.metric == SignalMetric::SnapshotInterval
                    && s.value >= s.set_point
                    && s.set_point > 0.0
            })
            .filter_map(Deviation::from_signal)
            .collect()
    }

    /// Compute: produce snapshot + optional prune actions.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        if deviations.is_empty() {
            return Vec::new();
        }
        vec![LoopAction::new(
            LoopId::Cybernetics,
            ActionType::Calibrate,
            serde_json::json!({"action": "daily_backup"}),
        )]
    }

    /// Act: run daily snapshot, then optionally verify and prune.
    async fn act(&self, actions: &[LoopAction]) {
        if actions.is_empty() || !self.auto_snapshot_enabled() {
            return;
        }

        if !self.is_snapshot_due() {
            return;
        }

        info!(target: "hkask.backup.loop", "Running scheduled daily backup snapshot");

        match self.service.run_daily_snapshot().await {
            Ok(metadata) => {
                info!(
                    target: "hkask.backup.loop",
                    artifact_count = metadata.artifact_count,
                    repos = metadata.commits.len(),
                    "Daily backup snapshot complete"
                );
                self.record_snapshot();

                // Optionally verify integrity after snapshot.
                if self.service.config().verify_after_snapshot {
                    match self.service.verify().await {
                        Ok(reports) => {
                            let corrupt: usize =
                                reports.iter().map(|r| r.corrupt_hashes.len()).sum();
                            if corrupt > 0 {
                                warn!(
                                    target: "hkask.backup.loop",
                                    corrupt_blobs = corrupt,
                                    "Post-snapshot verification found corruption"
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                target: "hkask.backup.loop",
                                error = %e,
                                "Post-snapshot verification failed"
                            );
                        }
                    }
                }

                // Run prune if retention is configured.
                if self.service.config().retention.is_some() {
                    match self.service.prune(false).await {
                        Ok(report) => {
                            info!(
                                target: "hkask.backup.loop",
                                evaluated = report.evaluated,
                                retained = report.retained,
                                removed = report.removed.len(),
                                "Post-snapshot prune complete"
                            );
                            self.record_prune();
                        }
                        Err(e) => {
                            warn!(
                                target: "hkask.backup.loop",
                                error = %e,
                                "Post-snapshot prune failed"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                warn!(
                    target: "hkask.backup.loop",
                    error = %e,
                    "Daily backup snapshot failed"
                );
            }
        }
    }
}
