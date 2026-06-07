//! SnapshotLoop — Cybernetic loop for scheduled CAS snapshots
//!
//! Implements the HkaskLoop (sense → compare → compute → act) cycle to
//! take snapshots of CAS repositories based on their retention policies.
//!
//! Per-repo policies determine when a snapshot is needed:
//! - If the repo's policy is disabled, no snapshot is taken
//! - If the time since the last snapshot exceeds the policy interval, a snapshot is due
//! - If no previous snapshot exists, one is always taken
//!
//! State tracking uses `parking_lot::RwLock` for interior mutability so
//! the `HkaskLoop` trait's `&self` methods can update snapshot timestamps
//! after successful snapshot actions.

use hkask_types::loops::{
    ActionType, Deviation, HkaskLoop, LoopAction, LoopId, Signal, SignalMetric,
};
use hkask_types::ports::git_cas::{
    CommitHash, GitCASPort, RepoId, RepoSnapshotPolicy, RetentionPolicy, RetentionTier,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Configuration for the SnapshotLoop.
#[derive(Debug, Clone)]
pub struct SnapshotLoopConfig {
    /// Per-repo snapshot policies. Defaults are used for repos not listed.
    pub repo_policies: Vec<RepoSnapshotPolicy>,
    /// Global default retention policy.
    pub default_policy: RetentionPolicy,
}

impl Default for SnapshotLoopConfig {
    fn default() -> Self {
        Self {
            repo_policies: RepoId::all()
                .iter()
                .map(|id| RepoSnapshotPolicy::default_for(id.clone()))
                .collect(),
            default_policy: RetentionPolicy::default(),
        }
    }
}

/// The last known snapshot timestamp per repo.
#[derive(Debug, Clone, Default)]
struct SnapshotState {
    /// Instant of the last successful snapshot for this repo.
    last_snapshot: Option<Instant>,
    /// Commit hash of the last successful snapshot.
    last_commit: Option<CommitHash>,
}

/// Cybernetic loop that takes scheduled snapshots of CAS repositories.
///
/// Implements the sense → compare → compute → act cycle:
/// - **Sense**: Check time elapsed since last snapshot per repo
/// - **Compare**: Detect deviations from RetentionPolicy intervals
/// - **Compute**: Produce snapshot actions for repos that need them
/// - **Act**: Call `snapshot()` on the GitCASPort for each due repo
///
/// Interior mutability via `parking_lot::RwLock<HashMap<String, SnapshotState>>`
/// allows `act()` to record successful snapshot timestamps despite the
/// `&self` signature required by `HkaskLoop`.
pub struct SnapshotLoop {
    port: Arc<dyn GitCASPort>,
    config: SnapshotLoopConfig,
    /// Per-repo snapshot state, keyed by `RepoId::dir_name()`.
    /// Uses `RwLock` for interior mutability — `act()` writes, `sense()` reads.
    state: Arc<parking_lot::RwLock<HashMap<String, SnapshotState>>>,
}

impl SnapshotLoop {
    /// Create a new SnapshotLoop with the given CAS port and default config.
    pub fn new(port: Arc<dyn GitCASPort>) -> Self {
        Self {
            port,
            config: SnapshotLoopConfig::default(),
            state: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    /// Create a SnapshotLoop with custom configuration.
    pub fn with_config(port: Arc<dyn GitCASPort>, config: SnapshotLoopConfig) -> Self {
        Self {
            port,
            config,
            state: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    /// Get the policy for a specific repo, falling back to default.
    fn policy_for(&self, repo: &RepoId) -> RepoSnapshotPolicy {
        self.config
            .repo_policies
            .iter()
            .find(|p| &p.repo == repo)
            .cloned()
            .unwrap_or_else(|| RepoSnapshotPolicy::default_for(repo.clone()))
    }

    /// Determine which retention tier applies given the elapsed seconds.
    fn applicable_tier(policy: &RetentionPolicy, elapsed_secs: u64) -> Option<&RetentionTier> {
        for tier in &policy.tiers {
            if elapsed_secs <= tier.max_age_secs {
                return Some(tier);
            }
        }
        // Elapsed exceeds all tier max-age thresholds → use the last (forever) tier
        policy.tiers.last()
    }

    /// Check if a repo needs a snapshot based on its policy and time since last snapshot.
    fn needs_snapshot(&self, repo: &RepoId) -> bool {
        let policy = self.policy_for(repo);
        if !policy.enabled {
            return false;
        }
        let retention = policy.effective_policy();
        let state = self.state.read();

        match state.get(repo.dir_name()).and_then(|s| s.last_snapshot) {
            Some(instant) => {
                let elapsed = instant.elapsed().as_secs();
                let tier = match Self::applicable_tier(&retention, elapsed) {
                    Some(t) => t,
                    None => return true, // No tier = no constraint = snapshot needed
                };
                elapsed >= tier.interval_secs
            }
            None => true, // No previous snapshot — always take one
        }
    }

    /// Record a successful snapshot for a repo.
    fn record_snapshot(&self, repo: &RepoId, commit: CommitHash) {
        let mut state = self.state.write();
        let entry = state.entry(repo.dir_name().to_string()).or_default();
        entry.last_snapshot = Some(Instant::now());
        entry.last_commit = Some(commit);
    }
}

#[async_trait::async_trait]
impl HkaskLoop for SnapshotLoop {
    fn id(&self) -> LoopId {
        LoopId::Snapshot
    }

    /// Sense: measure time since last snapshot per repo.
    async fn sense(&self) -> Vec<Signal> {
        let mut signals = Vec::new();
        let state = self.state.read();

        for repo in RepoId::all() {
            let policy = self.policy_for(repo);
            if !policy.enabled {
                continue;
            }
            let retention = policy.effective_policy();

            let elapsed_secs = state
                .get(repo.dir_name())
                .and_then(|s| s.last_snapshot)
                .map(|instant| instant.elapsed().as_secs())
                .unwrap_or(u64::MAX);

            // The applicable interval for this elapsed duration
            let interval = match Self::applicable_tier(&retention, elapsed_secs) {
                Some(tier) => tier.interval_secs as f64,
                None => 0.0, // No tier → snapshot immediately
            };

            // Signal: elapsed time vs. expected interval
            // If elapsed >= interval, we're "above set-point" (snapshot is due)
            signals.push(Signal::new(
                LoopId::Snapshot,
                SignalMetric::SnapshotInterval,
                elapsed_secs as f64,
                interval,
            ));
        }

        drop(state);
        signals
    }

    /// Compare: detect repos where snapshot interval has been exceeded.
    async fn compare(&self, signals: &[Signal]) -> Vec<Deviation> {
        // Only propagate signals where snapshot is due (elapsed >= interval)
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

    /// Compute: produce a single Calibrate action for snapshot scheduling.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        if deviations.is_empty() {
            return Vec::new();
        }
        // One action covers all due repos — act() iterates them
        vec![LoopAction::new(
            LoopId::Snapshot,
            ActionType::Calibrate,
            serde_json::json!({"action": "snapshot", "repos": "all_due"}),
        )]
    }

    /// Act: take snapshots for repos that need them.
    async fn act(&self, actions: &[LoopAction]) {
        if actions.is_empty() {
            return;
        }
        for repo in RepoId::all() {
            if self.needs_snapshot(repo) {
                match self.port.snapshot(repo, "scheduled snapshot").await {
                    Ok(commit) => {
                        tracing::info!(
                            target: "cns.snapshot_loop",
                            repo = repo.dir_name(),
                            "Scheduled snapshot taken"
                        );
                        self.record_snapshot(repo, commit);
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "cns.snapshot_loop",
                            repo = repo.dir_name(),
                            error = %e,
                            "Scheduled snapshot failed"
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::ports::git_cas::MockGitCas;

    fn all_policies_enabled() -> Vec<RepoSnapshotPolicy> {
        RepoId::all()
            .iter()
            .map(|id| RepoSnapshotPolicy::default_for(id.clone()))
            .collect()
    }

    #[tokio::test]
    async fn first_tick_takes_snapshot_for_all_enabled_repos() {
        let mock = Arc::new(MockGitCas::new());
        let loop_ = SnapshotLoop::new(mock.clone());

        // No previous state — every enabled repo should get a snapshot
        loop_.tick().await;

        let history = mock.snapshot_history();
        assert_eq!(
            history.len(),
            RepoId::all().len(),
            "first tick should snapshot all enabled repos"
        );

        // Each repo should appear exactly once
        for repo in RepoId::all() {
            let count = history.iter().filter(|(r, _, _)| r == repo).count();
            assert_eq!(
                count,
                1,
                "{} should have exactly one snapshot",
                repo.dir_name()
            );
        }
    }

    #[tokio::test]
    async fn disabled_repo_is_skipped() {
        let mut policies = all_policies_enabled();
        // Disable Vault
        policies.retain(|p| p.repo != RepoId::Vault);
        policies.push(RepoSnapshotPolicy::disabled(RepoId::Vault));

        let mock = Arc::new(MockGitCas::new());
        let config = SnapshotLoopConfig {
            repo_policies: policies,
            default_policy: RetentionPolicy::default(),
        };
        let loop_ = SnapshotLoop::with_config(mock.clone(), config);

        loop_.tick().await;

        let history = mock.snapshot_history();
        for (repo, _, _) in &history {
            assert_ne!(
                *repo,
                RepoId::Vault,
                "disabled repo should not be snapshotted"
            );
        }
        assert_eq!(
            history.len(),
            RepoId::all().len() - 1,
            "one repo disabled, so one fewer snapshot"
        );
    }

    #[tokio::test]
    async fn repo_within_interval_is_not_snapshotted_again() {
        let mock = Arc::new(MockGitCas::new());
        let loop_ = SnapshotLoop::new(mock.clone());

        // First tick: all repos get snapshotted
        loop_.tick().await;
        let after_first = mock.snapshot_history().len();
        assert!(after_first > 0, "first tick should produce snapshots");

        // Second immediate tick: repos are within their interval, no new snapshots
        loop_.tick().await;
        let after_second = mock.snapshot_history().len();
        assert_eq!(
            after_second, after_first,
            "no new snapshots when within interval"
        );
    }

    #[tokio::test]
    async fn sense_produces_signals_for_all_enabled_repos() {
        let mock = Arc::new(MockGitCas::new());
        let loop_ = SnapshotLoop::new(mock);

        let signals = loop_.sense().await;
        // Default config has all repos enabled → 7 signals
        assert_eq!(signals.len(), RepoId::all().len());

        // All signals should use SnapshotInterval metric
        for signal in &signals {
            assert_eq!(signal.metric, SignalMetric::SnapshotInterval);
        }
    }

    #[tokio::test]
    async fn compare_filters_to_due_repos_on_first_tick() {
        let mock = Arc::new(MockGitCas::new());
        let loop_ = SnapshotLoop::new(mock);

        let signals = loop_.sense().await;
        let deviations = loop_.compare(&signals).await;

        // First tick: all repos have no previous snapshot → all due
        assert_eq!(
            deviations.len(),
            RepoId::all().len(),
            "all repos should deviate on first tick"
        );
    }

    #[tokio::test]
    async fn needs_snapshot_returns_false_for_disabled_repo() {
        let mut policies = all_policies_enabled();
        policies.retain(|p| p.repo != RepoId::CnsAudit);
        policies.push(RepoSnapshotPolicy::disabled(RepoId::CnsAudit));

        let mock = Arc::new(MockGitCas::new());
        let config = SnapshotLoopConfig {
            repo_policies: policies,
            default_policy: RetentionPolicy::default(),
        };
        let loop_ = SnapshotLoop::with_config(mock, config);

        assert!(
            !loop_.needs_snapshot(&RepoId::CnsAudit),
            "disabled repo should not need a snapshot"
        );
    }

    #[tokio::test]
    async fn applicable_tier_selects_correct_tier() {
        let policy = RetentionPolicy::default();

        // 0 seconds elapsed → first tier (30min interval, 3h max)
        let tier = SnapshotLoop::applicable_tier(&policy, 0).unwrap();
        assert_eq!(tier.interval_secs, 30 * 60);

        // 3 hours + 1 second → second tier (daily interval, 3d max)
        let tier = SnapshotLoop::applicable_tier(&policy, 3 * 3600 + 1).unwrap();
        assert_eq!(tier.interval_secs, 86400);

        // u64::MAX → last tier (monthly interval, forever)
        let tier = SnapshotLoop::applicable_tier(&policy, u64::MAX).unwrap();
        assert_eq!(tier.interval_secs, 30 * 86400);
    }

    /// Behavioral property: SnapshotLoop implements HkaskLoop with LoopId::Snapshot,
    /// so it can be registered in the LoopSystem and ticks the full sense→compare→compute→act cycle.
    #[tokio::test]
    async fn snapshot_loop_ticks_full_cycle_with_correct_loop_id() {
        let mock = Arc::new(MockGitCas::new());
        let loop_ = SnapshotLoop::new(mock.clone());

        assert_eq!(
            loop_.id(),
            LoopId::Snapshot,
            "SnapshotLoop must identify as LoopId::Snapshot"
        );

        // A full tick should complete without panic and produce snapshots
        loop_.tick().await;

        let history = mock.snapshot_history();
        assert!(
            !history.is_empty(),
            "tick should produce at least one snapshot"
        );
    }
}
