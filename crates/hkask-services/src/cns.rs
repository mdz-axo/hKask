//! CNS health, alerts, and variety queries.
//!
//! `CnsService` wraps the shared `CnsRuntime` from `ServiceContext`,
//! hiding the `Arc<RwLock<>>` pattern so callers don't repeat
//! `cns_runtime.read().await.xxx().await` at every call site.

use std::sync::Arc;
use tokio::sync::RwLock;

use hkask_cns::{CnsRuntime, RuntimeAlert};
use hkask_types::cns::CnsHealth;

/// Service for CNS health checks, algedonic alerts, and variety counters.
///
/// Wraps the shared `CnsRuntime` behind a clean async interface.
/// Constructed during `ServiceContext::build()` — never created directly.
#[derive(Clone)]
pub struct CnsService {
    runtime: Arc<RwLock<CnsRuntime>>,
}

impl CnsService {
    /// Create from the shared CNS runtime.
    pub fn new(runtime: Arc<RwLock<CnsRuntime>>) -> Self {
        Self { runtime }
    }

    /// Current CNS health snapshot.
    pub async fn health(&self) -> CnsHealth {
        self.runtime.read().await.health().await
    }

    /// Active algedonic alerts.
    pub async fn alerts(&self) -> Vec<RuntimeAlert> {
        self.runtime.read().await.alerts().await
    }

    /// Variety counter snapshots: (domain_name, variety_count).
    pub async fn variety(&self) -> Vec<(String, u64)> {
        self.runtime.read().await.variety().await
    }
}
