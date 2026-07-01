//! SLO Manager — Service Level Objective evaluation for the CNS
//!
//! SloManager loads SLO definitions, evaluates them against ν-event data,
//! computes compliance rates and error budgets, emits `cns.slo.evaluated`
//! spans, and produces breach alerts for the algedonic pathway.
//!
//! # Epistemic grounding
//! - **crt:certainty** = Probabilistic (SLO compliance is sampled, not total)
//! - **crt:force** = Evidence (IS statement, computed from ν-event data)
//! - **mode** = IS
//!
//! # Cybernetic role
//! - Sensor: queries ν-event store for operation counts
//! - Comparator: compares compliance rate to SLO target
//! - Effector: emits `cns.slo.evaluated` spans; feeds breaches to algedonic

use hkask_types::cns::{CnsSpan, SloDefinition, SloEvaluation, SloSeverity};
use std::time::{SystemTime, UNIX_EPOCH};

/// Result of querying the ν-event store for SLO data.
///
/// The SloManager is decoupled from the storage layer — it accepts
/// a closure that provides these counts. This enables unit testing
/// without a database and supports multiple storage backends.
#[derive(Debug, Clone)]
pub struct SloDataPoint {
    /// Total operations in the SLO's window
    pub total_operations: u64,
    /// Successful operations in the SLO's window
    pub successful_operations: u64,
}

/// Trait for providing ν-event data to the SloManager.
///
/// Implementations query the ν-event store for operation counts
/// within a time window for a given CNS span namespace.
pub trait SloDataProvider: Send + Sync {
    /// Query operation counts for the given span namespace within
    /// the specified time window (in seconds before now).
    fn query(
        &self,
        span_namespace: &str,
        window_seconds: u64,
    ) -> Result<SloDataPoint, SloManagerError>;
}

/// Errors that can occur during SLO evaluation.
#[derive(Debug, thiserror::Error)]
pub enum SloManagerError {
    #[error("Data provider error: {0}")]
    DataProvider(String),

    #[error("SLO not found: {0}")]
    NotFound(String),

    #[error("Invalid SLO configuration: {0}")]
    InvalidConfig(String),
}

/// Manages Service Level Objective definitions, evaluation, and breach detection.
///
/// ## Example
///
/// ```rust,ignore
/// use hkask_cns::slo_manager::SloManager;
/// use hkask_types::cns::seed_slos;
///
/// let mut manager = SloManager::new();
/// for slo in seed_slos() {
///     manager.register(slo);
/// }
///
/// let evaluations = manager.evaluate(&data_provider)?;
/// for eval in &evaluations {
///     if eval.in_breach {
///         // escalate via algedonic pathway
///     }
/// }
/// ```
pub struct SloManager {
    slos: Vec<SloDefinition>,
}

impl SloManager {
    /// Create an empty SLO manager.
    ///
    /// expect: "The system initializes SLO management with no definitions"
    /// [P9] Motivating: Homeostatic Self-Regulation — SLOs are the contract layer
    /// post: returns SloManager with empty SLO list
    pub fn new() -> Self {
        Self { slos: Vec::new() }
    }

    /// Create a manager pre-loaded with seed SLOs.
    ///
    /// expect: "The system initializes with baseline SLO definitions"
    /// [P9] Motivating: Homeostatic Self-Regulation — seed SLOs establish baseline
    /// post: returns SloManager with 3 seed SLOs
    pub fn with_seed_slos() -> Self {
        Self {
            slos: hkask_types::cns::seed_slos(),
        }
    }

    /// Register an SLO definition.
    ///
    /// expect: "The system accepts SLO definitions for evaluation"
    /// [P9] Motivating: Homeostatic Self-Regulation — extensible SLO registry
    /// pre:  slo.slo_id is unique (caller's responsibility)
    /// post: SLO is added to the registry
    pub fn register(&mut self, slo: SloDefinition) {
        self.slos.push(slo);
    }

    /// Register multiple SLO definitions.
    pub fn register_all(&mut self, slos: impl IntoIterator<Item = SloDefinition>) {
        self.slos.extend(slos);
    }

    /// Remove an SLO by ID.
    ///
    /// expect: "The system supports SLO lifecycle management"
    /// post: if slo_id exists, it is removed; returns true if removed
    pub fn deregister(&mut self, slo_id: &str) -> bool {
        let len_before = self.slos.len();
        self.slos.retain(|s| s.slo_id != slo_id);
        self.slos.len() < len_before
    }

    /// Get all registered SLOs.
    pub fn slos(&self) -> &[SloDefinition] {
        &self.slos
    }

    /// Get active SLOs only.
    pub fn active_slos(&self) -> Vec<&SloDefinition> {
        self.slos.iter().filter(|s| s.active).collect()
    }

    /// Evaluate all active SLOs against the data provider.
    ///
    /// expect: "The system evaluates SLOs against measured data"
    /// [P9] Motivating: Homeostatic Self-Regulation — SLO evaluation drives feedback
    /// [P8] Constraining: Semantic Grounding — evaluations are computed, not guessed
    /// pre:  data_provider is operational
    /// post: returns one SloEvaluation per active SLO; errors are logged per-SLO,
    ///       not failing the entire batch
    pub fn evaluate(&self, provider: &dyn SloDataProvider) -> Vec<SloEvaluation> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.active_slos()
            .iter()
            .filter_map(|slo| {
                match provider.query(&slo.span_namespace, slo.window_seconds) {
                    Ok(data) => {
                        let compliance = if data.total_operations > 0 {
                            data.successful_operations as f64 / data.total_operations as f64
                        } else {
                            // No data = perfect compliance (no failures observed)
                            1.0
                        };

                        let error_budget_total = slo.error_budget(data.total_operations);
                        let failures = data
                            .total_operations
                            .saturating_sub(data.successful_operations);
                        let error_budget_remaining = if error_budget_total > 0.0 {
                            (error_budget_total - failures as f64).max(0.0) / error_budget_total
                        } else {
                            1.0
                        };

                        // Burn rate: fraction of error budget consumed per hour.
                        // Approximated as failures / (window_hours × error_budget_total)
                        let window_hours = slo.window_seconds as f64 / 3600.0;
                        let burn_rate = if error_budget_total > 0.0 && window_hours > 0.0 {
                            (failures as f64 / error_budget_total) / window_hours
                        } else {
                            0.0
                        };

                        let in_breach = slo.is_breached(compliance);

                        // Emit CNS span
                        CnsSpan::SloEvaluated.emit("evaluated");
                        tracing::info!(
                            target: "cns",
                            cns_domain = "cns.slo.evaluated",
                            slo_id = %slo.slo_id,
                            compliance = %compliance,
                            error_budget_pct = %(error_budget_remaining * 100.0),
                            burn_rate = %burn_rate,
                            in_breach = %in_breach,
                            total_ops = %data.total_operations,
                            "SLO evaluated",
                        );

                        Some(SloEvaluation {
                            slo_id: slo.slo_id.clone(),
                            current_compliance: compliance,
                            error_budget_remaining,
                            burn_rate,
                            in_breach,
                            evaluated_at: now,
                        })
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "cns",
                            cns_domain = "cns.slo.evaluated",
                            slo_id = %slo.slo_id,
                            error = %e,
                            "SLO evaluation failed — data provider error",
                        );
                        // Don't fail the entire batch for one SLO's data error
                        None
                    }
                }
            })
            .collect()
    }

    /// Evaluate SLOs and return only breached evaluations.
    pub fn evaluate_breaches(&self, provider: &dyn SloDataProvider) -> Vec<SloEvaluation> {
        self.evaluate(provider)
            .into_iter()
            .filter(|e| e.in_breach)
            .collect()
    }

    /// Evaluate SLOs and return evaluations sorted by severity (Critical first).
    pub fn evaluate_sorted(&self, provider: &dyn SloDataProvider) -> Vec<SloEvaluation> {
        let mut results = self.evaluate(provider);
        results.sort_by(|a, b| {
            let severity_a = self
                .slos
                .iter()
                .find(|s| s.slo_id == a.slo_id)
                .map(|s| slo_severity_rank(s.severity))
                .unwrap_or(2);
            let severity_b = self
                .slos
                .iter()
                .find(|s| s.slo_id == b.slo_id)
                .map(|s| slo_severity_rank(s.severity))
                .unwrap_or(2);
            severity_a.cmp(&severity_b).then_with(|| {
                // Within same severity, sort by worst compliance first
                a.current_compliance
                    .partial_cmp(&b.current_compliance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        results
    }
}

impl Default for SloManager {
    fn default() -> Self {
        Self::with_seed_slos()
    }
}

/// Rank SloSeverity for sorting (lower = more severe).
fn slo_severity_rank(s: SloSeverity) -> u8 {
    match s {
        SloSeverity::Critical => 0,
        SloSeverity::High => 1,
        SloSeverity::Medium => 2,
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::cns::SloSeverity;

    /// A test data provider that returns configurable counts.
    struct TestDataProvider {
        total: u64,
        successful: u64,
        /// If set, return an error instead
        error: Option<String>,
    }

    impl TestDataProvider {
        fn new(total: u64, successful: u64) -> Self {
            Self {
                total,
                successful,
                error: None,
            }
        }

        fn with_error(msg: &str) -> Self {
            Self {
                total: 0,
                successful: 0,
                error: Some(msg.to_string()),
            }
        }
    }

    impl SloDataProvider for TestDataProvider {
        fn query(
            &self,
            _span_namespace: &str,
            _window_seconds: u64,
        ) -> Result<SloDataPoint, SloManagerError> {
            if let Some(ref err) = self.error {
                return Err(SloManagerError::DataProvider(err.clone()));
            }
            Ok(SloDataPoint {
                total_operations: self.total,
                successful_operations: self.successful,
            })
        }
    }

    // ── SloManager lifecycle ──────────────────────────────────────────────

    #[test]
    fn slo_manager_starts_empty() {
        let manager = SloManager::new();
        assert!(manager.slos().is_empty());
    }

    #[test]
    fn slo_manager_with_seeds_has_three() {
        let manager = SloManager::with_seed_slos();
        assert_eq!(manager.slos().len(), 3);
    }

    #[test]
    fn register_and_deregister_slo() {
        let mut manager = SloManager::new();
        let slo = SloDefinition::new("test", "test", "cns.test", 0.99, 3600, SloSeverity::Medium);
        manager.register(slo);
        assert_eq!(manager.slos().len(), 1);
        assert!(manager.deregister("test"));
        assert!(manager.slos().is_empty());
    }

    #[test]
    fn deregister_nonexistent_returns_false() {
        let mut manager = SloManager::new();
        assert!(!manager.deregister("nonexistent"));
    }

    #[test]
    fn active_slos_filters_inactive() {
        let mut manager = SloManager::new();
        let mut slo = SloDefinition::new(
            "active",
            "active",
            "cns.test",
            0.99,
            3600,
            SloSeverity::High,
        );
        slo.active = true;
        let mut slo2 = SloDefinition::new(
            "inactive",
            "inactive",
            "cns.test",
            0.99,
            3600,
            SloSeverity::High,
        );
        slo2.active = false;
        manager.register(slo);
        manager.register(slo2);
        assert_eq!(manager.active_slos().len(), 1);
    }

    // ── SLO evaluation ────────────────────────────────────────────────────

    #[test]
    fn evaluate_perfect_compliance() {
        let manager = SloManager::with_seed_slos();
        let provider = TestDataProvider::new(1000, 1000); // 100% success
        let results = manager.evaluate(&provider);
        assert_eq!(results.len(), 3);
        for eval in &results {
            assert!(
                !eval.in_breach,
                "SLO {} should not be breached",
                eval.slo_id
            );
            assert!((eval.current_compliance - 1.0).abs() < 0.001);
            assert!((eval.error_budget_remaining - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn evaluate_below_target_breaches() {
        let mut manager = SloManager::new();
        manager.register(SloDefinition::new(
            "test",
            "test",
            "cns.test",
            0.99,
            3600,
            SloSeverity::Critical,
        ));
        // 98% success when target is 99%
        let provider = TestDataProvider::new(1000, 980);
        let results = manager.evaluate(&provider);
        assert_eq!(results.len(), 1);
        assert!(results[0].in_breach);
    }

    #[test]
    fn evaluate_no_data_is_perfect() {
        let mut manager = SloManager::new();
        manager.register(SloDefinition::new(
            "test",
            "test",
            "cns.test",
            0.99,
            3600,
            SloSeverity::High,
        ));
        // Zero operations — no failures observed, assume perfect
        let provider = TestDataProvider::new(0, 0);
        let results = manager.evaluate(&provider);
        assert_eq!(results.len(), 1);
        assert!(!results[0].in_breach);
        assert!((results[0].current_compliance - 1.0).abs() < 0.001);
    }

    #[test]
    fn evaluate_error_budget_burn_rate() {
        let mut manager = SloManager::new();
        // 99% target over 1 hour window
        manager.register(SloDefinition::new(
            "test",
            "test",
            "cns.test",
            0.99,
            3600,
            SloSeverity::Critical,
        ));
        // 1000 ops, 985 success = 1.5% failure rate, budget = 10 failures
        let provider = TestDataProvider::new(1000, 985);
        let results = manager.evaluate(&provider);
        assert_eq!(results.len(), 1);
        assert!(results[0].in_breach);
        // Burn rate should be positive (consuming budget)
        assert!(results[0].burn_rate > 0.0);
        // Error budget should be partially consumed
        assert!(results[0].error_budget_remaining < 1.0);
    }

    #[test]
    fn evaluate_data_provider_error_is_non_fatal() {
        let mut manager = SloManager::new();
        manager.register(SloDefinition::new(
            "test1",
            "test",
            "cns.test",
            0.99,
            3600,
            SloSeverity::Medium,
        ));
        manager.register(SloDefinition::new(
            "test2",
            "test",
            "cns.test",
            0.99,
            3600,
            SloSeverity::Medium,
        ));
        // Mixed provider: one succeeds, one errors — batch must not fail
        // We can't test this with a single provider easily, so we test
        // that a failing provider returns empty results (graceful degradation)
        let failing = TestDataProvider::with_error("connection refused");
        let results = manager.evaluate(&failing);
        assert!(
            results.is_empty(),
            "failing provider should yield no evaluations"
        );
    }

    #[test]
    fn evaluate_breaches_returns_only_breached() {
        let mut manager = SloManager::new();
        // SLO at 99% target
        manager.register(SloDefinition::new(
            "passing",
            "passing",
            "cns.test",
            0.99,
            3600,
            SloSeverity::High,
        ));
        // SLO at 99.9% target — tighter
        manager.register(SloDefinition::new(
            "failing",
            "failing",
            "cns.test",
            0.999,
            3600,
            SloSeverity::Critical,
        ));
        // 995/1000 = 99.5% — passes the first, fails the second
        let provider = TestDataProvider::new(1000, 995);
        let breaches = manager.evaluate_breaches(&provider);
        assert_eq!(breaches.len(), 1);
        assert_eq!(breaches[0].slo_id, "failing");
    }

    #[test]
    fn evaluate_sorted_critical_first() {
        let mut manager = SloManager::new();
        manager.register(SloDefinition::new(
            "medium",
            "medium",
            "cns.test",
            0.99,
            3600,
            SloSeverity::Medium,
        ));
        manager.register(SloDefinition::new(
            "critical",
            "critical",
            "cns.test",
            0.99,
            3600,
            SloSeverity::Critical,
        ));
        manager.register(SloDefinition::new(
            "high",
            "high",
            "cns.test",
            0.99,
            3600,
            SloSeverity::High,
        ));
        // All breached at 90% compliance
        let provider = TestDataProvider::new(1000, 900);
        let results = manager.evaluate_sorted(&provider);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].slo_id, "critical");
        assert_eq!(results[1].slo_id, "high");
        assert_eq!(results[2].slo_id, "medium");
    }

    #[test]
    fn seed_slos_evaluate_correctly() {
        let manager = SloManager::with_seed_slos();
        // All seed SLOs target 99.5% or 99.9%
        // 999/1000 = 99.9% — passes all
        let provider = TestDataProvider::new(1000, 999);
        let results = manager.evaluate(&provider);
        assert_eq!(results.len(), 3);
        // SLO-INF-001 and SLO-API-001 target 99.9% — 99.9% passes
        // SLO-SKL-001 targets 99.5% — 99.9% passes
        for eval in &results {
            assert!(
                !eval.in_breach,
                "Seed SLO {} should not be breached at 99.9% compliance",
                eval.slo_id
            );
        }
    }
}
