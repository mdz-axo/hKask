// G2 Justification: 6 public items — CNS boundary ports (circuit breaker, consolidation, depletion, backpressure, observer). Each is a distinct architectural seam; merging would violate interface segregation. ≤7 cap met.

use crate::cns::CircuitState;
use crate::event::{NuEvent, SpanNamespace};
use crate::id::WebID;
use crate::loops::LoopId;

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
