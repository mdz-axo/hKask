//! CNS Runtime Adapter — maps CnsRuntime to domain-native types

use crate::ports::{AlertInfo, AlertLevel, HealthStatus};
use hkask_cns::CnsRuntime;
use hkask_types::WebID;
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

/// CNS adapter — wraps CnsRuntime for domain-native types
pub struct CnsAdapter {
    runtime: Arc<CnsRuntime>,
}

impl CnsAdapter {
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

    pub async fn calibrate_threshold(&self, domain: &str, new_threshold: u64) {
        self.runtime
            .calibrate_threshold(domain, new_threshold)
            .await
    }

    pub async fn increment_and_check(&self, domain: &str, state_name: &str) -> Option<AlertInfo> {
        self.runtime.increment_variety(domain, state_name).await;
        self.runtime.check_variety(domain).await.map(map_alert)
    }
}
