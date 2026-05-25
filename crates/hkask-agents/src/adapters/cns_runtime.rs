//! CNS Runtime Adapter — Implements CnsQueryPort for CnsRuntime
//!
//! This adapter bridges the concrete CnsRuntime implementation to the
//! CnsQueryPort trait, following hexagonal architecture principles (R7).

use crate::ports::CnsQueryPort;
use async_trait::async_trait;
use hkask_cns::{CnsHealth, CnsRuntime, RuntimeAlert};
use std::sync::Arc;

/// Adapter that implements CnsQueryPort for CnsRuntime
pub struct CnsRuntimeAdapter {
    runtime: Arc<CnsRuntime>,
}

impl CnsRuntimeAdapter {
    /// Create a new adapter wrapping a CnsRuntime instance
    pub fn new(runtime: Arc<CnsRuntime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl CnsQueryPort for CnsRuntimeAdapter {
    async fn health(&self) -> CnsHealth {
        self.runtime.health().await
    }

    async fn variety(&self) -> Vec<(String, u64)> {
        self.runtime.variety().await
    }

    async fn alerts(&self) -> Vec<RuntimeAlert> {
        self.runtime.alerts().await
    }

    async fn critical_alerts(&self) -> Vec<RuntimeAlert> {
        self.runtime.critical_alerts().await
    }

    async fn variety_for_domain(&self, domain: &str) -> u64 {
        self.runtime.variety_for_domain(domain).await
    }
}
