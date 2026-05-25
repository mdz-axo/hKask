//! CNS Runtime Adapter — Implements CnsQueryPort for CnsRuntime
//!
//! This adapter bridges the concrete CnsRuntime implementation to the
//! CnsQueryPort trait, mapping `hkask_cns` types to domain-native port types.

use crate::ports::{AlertInfo, AlertLevel, CnsQueryPort, HealthStatus};
use async_trait::async_trait;
use hkask_cns::CnsRuntime;
use std::sync::Arc;

fn map_severity(s: hkask_cns::AlertSeverity) -> AlertLevel {
    match s {
        hkask_cns::AlertSeverity::Info => AlertLevel::Info,
        hkask_cns::AlertSeverity::Warning => AlertLevel::Warning,
        hkask_cns::AlertSeverity::Critical => AlertLevel::Critical,
    }
}

fn map_alert(a: hkask_cns::RuntimeAlert) -> AlertInfo {
    AlertInfo {
        domain: a.domain,
        deficit: a.deficit,
        threshold: a.threshold,
        severity: map_severity(a.severity),
        escalated: a.escalated,
        message: a.message,
    }
}

fn map_health(h: hkask_cns::CnsHealth) -> HealthStatus {
    HealthStatus {
        overall_deficit: h.overall_deficit,
        critical_count: h.critical_count,
        warning_count: h.warning_count,
        healthy: h.healthy,
    }
}

pub struct CnsRuntimeAdapter {
    runtime: Arc<CnsRuntime>,
}

impl CnsRuntimeAdapter {
    pub fn new(runtime: Arc<CnsRuntime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl CnsQueryPort for CnsRuntimeAdapter {
    async fn health(&self) -> HealthStatus {
        map_health(self.runtime.health().await)
    }

    async fn variety(&self) -> Vec<(String, u64)> {
        self.runtime.variety().await
    }

    async fn alerts(&self) -> Vec<AlertInfo> {
        self.runtime.alerts().await.into_iter().map(map_alert).collect()
    }

    async fn critical_alerts(&self) -> Vec<AlertInfo> {
        self.runtime
            .critical_alerts()
            .await
            .into_iter()
            .map(map_alert)
            .collect()
    }

    async fn variety_for_domain(&self, domain: &str) -> u64 {
        self.runtime.variety_for_domain(domain).await
    }
}
