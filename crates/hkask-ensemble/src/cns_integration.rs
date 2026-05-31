//! CNS Integration - Variety tracking and algedonic alert handling

use hkask_cns::algedonic::RuntimeAlert;
use hkask_cns::variety::VarietyMonitor;
use hkask_types::WebID;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// CNS integration manager
pub struct CnsIntegration {
    variety_monitor: Arc<RwLock<VarietyMonitor>>,
    observer_webid: WebID,
}

impl CnsIntegration {
    pub fn new(observer_webid: WebID) -> Self {
        Self {
            variety_monitor: Arc::new(RwLock::new(VarietyMonitor::new())),
            observer_webid,
        }
    }

    /// Track variety for a category
    pub async fn track_variety(&self, category: &str, count: u64, threshold: u64) {
        let mut variety_monitor = self.variety_monitor.write().await;
        let counter = variety_monitor.counter(category);
        for _ in 0..count {
            counter.increment("state_active");
        }

        let deficit = counter.deficit(threshold);
        if deficit > 0 {
            let alert = RuntimeAlert::new(category, deficit, threshold);
            if alert.should_escalate() {
                drop(variety_monitor);
                self.handle_algedonic_alert(alert).await;
            }
        }
    }

    /// Handle algedonic alert
    pub async fn handle_algedonic_alert(&self, alert: RuntimeAlert) {
        warn!(
            target: "hkask.cns.algedonic",
            severity = ?alert.severity,
            message = %alert.message,
            "Algedonic alert triggered"
        );

        info!(
            target: "hkask.cns.algedonic",
            domain = %alert.domain,
            deficit = alert.deficit,
            threshold = alert.threshold,
            "Alert recorded"
        );
    }

    /// Get observer WEBID
    pub fn observer(&self) -> WebID {
        self.observer_webid
    }
}

/// CNS integration builder
pub struct CnsIntegrationBuilder {
    observer_webid: WebID,
    variety_threshold: u64,
}

impl CnsIntegrationBuilder {
    pub fn new(observer_webid: WebID) -> Self {
        Self {
            observer_webid,
            variety_threshold: 100,
        }
    }

    pub fn with_variety_threshold(mut self, threshold: u64) -> Self {
        self.variety_threshold = threshold;
        self
    }

    pub fn build(self) -> CnsIntegration {
        CnsIntegration::new(self.observer_webid)
    }
}
