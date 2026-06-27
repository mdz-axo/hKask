//! BackupLoop — Cybernetic loop for scheduled backup snapshots.
//! # REQ: P7-svc-backup-loop-f4 — daily automatic snapshots via the daemon loop system.
//! expect: "Automatic daily snapshots run via the daemon loop system"
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

use hkask_cns::types::loops::{
    ActionType, Deviation, HkaskLoop, LoopAction, LoopId, Signal, SignalMetric,
};
use parking_lot::RwLock;
use tracing::{info, warn};

use crate::producers::ArtifactProducer;
use crate::service::BackupService;

/// State tracked by the BackupLoop across cycles.
#[derive(Debug, Clone, Default)]
struct BackupLoopState {
    /// When the last daily snapshot completed successfully.
    last_snapshot: Option<Instant>,
    /// When the last prune completed.
    last_prune: Option<Instant>,
    /// When the last snapshot attempt failed (for dampening).
    last_failure: Option<Instant>,
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
    producers: RwLock<Vec<Arc<dyn ArtifactProducer>>>,
}

impl BackupLoop {
    /// Create a new BackupLoop wrapping a BackupService.
    pub fn new(service: Arc<BackupService>) -> Self {
        Self {
            service,
            state: Arc::new(RwLock::new(BackupLoopState::default())),
            producers: RwLock::new(Vec::new()),
        }
    }

    /// Access the inner BackupService for read-only queries (TUI, status).
    pub fn service(&self) -> &Arc<BackupService> {
        &self.service
    }

    /// Register an artifact producer to be called before each daily snapshot.
    /// Can be called after construction — thread-safe via interior mutability.
    pub fn add_producer(&self, producer: Arc<dyn ArtifactProducer>) {
        self.producers.write().push(producer);
    }

    /// Check if auto-snapshot is enabled (delegates to BackupService).
    fn auto_snapshot_enabled(&self) -> bool {
        self.service.auto_snapshot_enabled()
    }

    /// Check if a daily snapshot is due (24h since last success).
    /// Dampener: if last attempt failed within the past hour, skip.
    fn is_snapshot_due(&self) -> bool {
        let state = self.state.read();
        // Dampener: don't retry for 1 hour after a failure
        if let Some(fail) = state.last_failure
            && fail.elapsed().as_secs() < 3600
        {
            return false;
        }
        match state.last_snapshot {
            Some(instant) => instant.elapsed().as_secs() >= 86400,
            None => true,
        }
    }

    /// Record a failed snapshot attempt (for dampening).
    fn record_failure(&self) {
        let mut state = self.state.write();
        state.last_failure = Some(Instant::now());
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
    /// Dampener: skip if last attempt failed within the past hour.
    async fn compare(&self, signals: &[Signal]) -> Vec<Deviation> {
        // Dampener: don't produce deviations after a recent failure
        if !self.is_snapshot_due() {
            return Vec::new();
        }
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
    /// Dampener is applied in compare() — actions produced by compute() are always executed.
    async fn act(&self, actions: &[LoopAction]) {
        if actions.is_empty() || !self.auto_snapshot_enabled() {
            return;
        }

        info!(
            target: "cns.backup",
            "Daily backup: starting snapshot + optional verify/prune"
        );

        // 1. Produce: push current subsystem state into CAS
        // Clone Arc'd producers to avoid holding RwLockReadGuard across .await
        let producers: Vec<Arc<dyn ArtifactProducer>> =
            self.producers.read().iter().cloned().collect();
        let mut total_produced = 0usize;
        for producer in &producers {
            match producer.produce(self.service.cas().as_ref()).await {
                Ok(count) => {
                    if count > 0 {
                        info!(
                            target: "cns.backup",
                            produced = count,
                            types = ?producer.artifact_types(),
                            "Backup produce: {} artifacts of types {:?}",
                            count,
                            producer.artifact_types()
                        );
                    }
                    total_produced += count;
                }
                Err(e) => {
                    warn!(
                        target: "cns.backup",
                        error = %e,
                        types = ?producer.artifact_types(),
                        "Backup produce failed for {:?}: {}",
                        producer.artifact_types(),
                        e
                    );
                }
            }
        }
        if total_produced > 0 {
            info!(
                target: "cns.backup",
                total_produced = total_produced,
                "Backup produce: {} total artifacts produced",
                total_produced
            );
        }

        // 2. Snapshot: commit all CAS repos
        match self.service.run_daily_snapshot().await {
            Ok(metadata) => {
                info!(
                    target: "cns.backup",
                    artifact_count = metadata.artifact_count.unwrap_or(0),
                    repos = metadata.commits.len(),
                    "Backup snapshot completed: {} repos",
                    metadata.commits.len()
                );
                self.record_snapshot();

                // Optionally verify integrity after snapshot.
                if self.service.verify_after_snapshot_enabled() {
                    match self.service.verify().await {
                        Ok(reports) => {
                            let corrupt: usize =
                                reports.iter().map(|r| r.corrupt_hashes.len()).sum();
                            if corrupt > 0 {
                                warn!(
                                    target: "cns.backup",
                                    corrupt_blobs = corrupt,
                                    "Backup verify: {} corrupt blobs detected",
                                    corrupt
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                target: "cns.backup",
                                error = %e,
                                "CNS"
                            );
                        }
                    }
                }

                // Run prune if retention is configured.
                if self.service.retention_configured() {
                    match self.service.prune(false).await {
                        Ok(report) => {
                            info!(
                                target: "cns.backup",
                                evaluated = report.evaluated,
                                retained = report.retained,
                                removed = report.removed.len(),
                                "CNS"
                            );
                            self.record_prune();
                        }
                        Err(e) => {
                            warn!(
                                target: "cns.backup",
                                error = %e,
                                "Backup prune failed: {}",
                                e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                warn!(
                    target: "cns.backup",
                    error = %e,
                    "Backup snapshot failed: {}",
                    e
                );
                self.record_failure();
            }
        }
    }
}
