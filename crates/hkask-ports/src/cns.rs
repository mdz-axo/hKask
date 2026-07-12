use chrono::{DateTime, Utc};
use hkask_types::InfrastructureError;
use hkask_types::cns::CircuitState;
use hkask_types::event::{NuEvent, SpanNamespace};
use hkask_types::id::WebID;
use hkask_types::loops::LoopId;

use async_trait::async_trait;

/// Circuit breaker boundary for the Cybernetics membrane.
///
/// Allows the Inference loop to use circuit breaking without depending on hkask-cns.
/// Impl: `CircuitBreaker` (in hkask-cns)
pub trait CircuitBreakerPort: Send + Sync {
    fn allow_request(&self) -> bool;
    fn record_success(&self);
    fn record_failure(&self);
    fn state(&self) -> CircuitState;
}

/// Parameters for consolidation. All fields except `limit` optional.
#[derive(Debug, Clone)]
pub struct ConsolidationRequest {
    pub limit: usize,
    pub confidence_floor: Option<f64>,
    pub max_semantic_triples: Option<usize>,
}

impl Default for ConsolidationRequest {
    fn default() -> Self {
        Self {
            limit: 100,
            confidence_floor: None,
            max_semantic_triples: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConsolidationOutcome {
    pub consolidated_count: usize,
    pub deleted_count: usize,
    pub failed_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepletionSignal {
    pub agent: WebID,
    pub remaining: u64,
    pub cap: u64,
    pub usage_ratio: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackpressureSignal {
    pub source: LoopId,
    pub reason: String,
    pub severity: f64,
}

/// Subscribes to CNS events by span namespace.

#[async_trait]
pub trait CnsObserver: Send + Sync {
    fn interest_mask(&self) -> Vec<SpanNamespace>;

    async fn on_event(&self, event: &NuEvent);

    async fn on_depletion(&self, signal: &DepletionSignal);

    async fn on_backpressure(&self, signal: &BackpressureSignal);
}

/// Storage port for CNS event queries.
///
/// Abstracts the NuEventStore behind a trait so the cybernetic regulation
/// layer (GasReport, CalibratedEnergyEstimator, WalletGasCalibrator) can be
/// tested without a real SQLite database.
///
/// Concrete impl: `NuEventStore` in `hkask-storage`.
pub trait CnsStoragePort: Send + Sync {
    fn query_algedonic(
        &self,
        since: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<NuEvent>, InfrastructureError>;

    /// Replay events with temporal decay weighting. Events older than
    /// the lookback window or below the weight threshold are excluded.
    fn replay_weighted(
        &self,
        since: DateTime<Utc>,
        limit: u64,
        config: &DecayConfig,
    ) -> Result<Vec<WeightedEvent>, InfrastructureError>;

    /// Persist a loop cursor key-value pair for crash recovery.
    fn persist_cursor(&self, key: &str, value: i64) -> Result<(), InfrastructureError>;

    /// Load a persisted loop cursor. Returns `None` if no cursor exists.
    fn load_cursor(&self, key: &str) -> Result<Option<i64>, InfrastructureError>;
}

/// A NuEvent with its computed replay weight.
#[derive(Debug, Clone)]
pub struct WeightedEvent {
    pub event: NuEvent,
    pub weight: f64,
}

/// Per-domain decay constants for weighted replay.
///
/// Each loop domain has its own lambda for exponential decay.
/// Half-life = ln(2)/lambda.
#[derive(Debug, Clone)]
pub struct DecayConfig {
    pub cybernetics_lambda: f64,
    pub curation_lambda: f64,
    pub inference_lambda: f64,
    pub episodic_lambda: f64,
    pub weight_threshold: f64,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            cybernetics_lambda: std::f64::consts::LN_2 / 300.0,
            curation_lambda: std::f64::consts::LN_2 / 900.0,
            inference_lambda: std::f64::consts::LN_2 / 120.0,
            episodic_lambda: std::f64::consts::LN_2 / 600.0,
            weight_threshold: 0.001,
        }
    }
}
