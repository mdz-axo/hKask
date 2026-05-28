//! CNS Runtime Adapter — Wraps CnsRuntime with domain-native types
//!
//! Bridges the concrete CnsRuntime implementation to domain-native types,
//! mapping `hkask_cns` types to the types defined in the ports module.

use crate::ports::{AlertInfo, AlertLevel, HealthStatus};
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

    pub async fn health(&self) -> HealthStatus {
        map_health(self.runtime.health().await)
    }

    pub async fn variety(&self) -> Vec<(String, u64)> {
        self.runtime.variety().await
    }

    pub async fn alerts(&self) -> Vec<AlertInfo> {
        self.runtime
            .alerts()
            .await
            .into_iter()
            .map(map_alert)
            .collect()
    }

    pub async fn critical_alerts(&self) -> Vec<AlertInfo> {
        self.runtime
            .critical_alerts()
            .await
            .into_iter()
            .map(map_alert)
            .collect()
    }

    pub async fn variety_for_domain(&self, domain: &str) -> u64 {
        self.runtime.variety_for_domain(domain).await
    }
}
