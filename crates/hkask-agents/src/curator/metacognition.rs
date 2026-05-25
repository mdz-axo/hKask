//! Metacognition Loop — Curator's periodic system governance
//!
//! The Curator performs metacognition on system performance:
//! - Queries CNS spans for health metrics
//! - Checks variety counters (algedonic alerts if deficit > 100)
//! - Collects bot status reports from standing session
//! - Synthesizes system state updates
//! - Triggers escalations when thresholds are exceeded
//! - Posts summaries to standing session

use crate::curator::escalation::EscalationQueue;
use crate::ports::CnsQueryPort;
use hkask_storage::{MetacognitionStore, StoredSnapshot};
use hkask_types::BotID;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum MetacognitionError {
    #[error("Escalation error: {0}")]
    Escalation(String),
    #[error("CNS query error: {0}")]
    CnsQuery(String),
}

/// Escalation trigger thresholds
#[derive(Debug, Clone)]
pub struct EscalationThresholds {
    /// Variety deficit threshold (default: 100)
    pub variety_deficit: u64,
    /// Critical alert count threshold (default: 3)
    pub critical_alerts: usize,
    /// Bot failure count threshold (default: 2)
    pub bot_failures: usize,
}

impl Default for EscalationThresholds {
    fn default() -> Self {
        Self {
            variety_deficit: 100,
            critical_alerts: 3,
            bot_failures: 2,
        }
    }
}

/// System health snapshot
#[derive(Debug, Clone)]
pub struct SystemHealthSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cns_health: String,
    pub variety_counters: Vec<(String, u64)>,
    pub critical_alerts: usize,
    pub total_alerts: usize,
    pub bot_status_reports: Vec<BotStatusReport>,
}

/// Bot status report from standing session
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BotStatusReport {
    pub bot_name: String,
    pub status: BotHealthStatus,
    pub last_report: Option<chrono::DateTime<chrono::Utc>>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BotHealthStatus {
    Healthy,
    Degraded,
    Critical,
    Unresponsive,
}

impl std::fmt::Display for BotHealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotHealthStatus::Healthy => write!(f, "healthy"),
            BotHealthStatus::Degraded => write!(f, "degraded"),
            BotHealthStatus::Critical => write!(f, "critical"),
            BotHealthStatus::Unresponsive => write!(f, "unresponsive"),
        }
    }
}

/// Metacognition loop configuration
#[derive(Debug, Clone)]
pub struct MetacognitionConfig {
    /// Interval between metacognition cycles (default: 1 hour)
    pub interval: Duration,
    /// Escalation thresholds
    pub thresholds: EscalationThresholds,
    /// Expected variety per domain (for deficit calculation)
    pub expected_variety_per_domain: u64,
}

impl Default for MetacognitionConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(3600), // 1 hour
            thresholds: EscalationThresholds::default(),
            expected_variety_per_domain: 50,
        }
    }
}

/// Metacognition loop — Curator's system governance mechanism
pub struct MetacognitionLoop {
    cns: Arc<dyn CnsQueryPort>,
    escalation_queue: tokio::sync::Mutex<Arc<EscalationQueue>>,
    config: MetacognitionConfig,
    bot_reports: Arc<RwLock<Vec<BotStatusReport>>>,
    /// Persistent storage for snapshots (R6: Persist Metacognition State)
    store: Option<Arc<MetacognitionStore>>,
}

impl MetacognitionLoop {
    pub fn new(
        cns: Arc<dyn CnsQueryPort>,
        escalation_queue: Arc<EscalationQueue>,
        config: MetacognitionConfig,
    ) -> Self {
        Self {
            cns,
            escalation_queue: tokio::sync::Mutex::new(escalation_queue),
            config,
            bot_reports: Arc::new(RwLock::new(Vec::new())),
            store: None,
        }
    }

    /// Set persistent storage for snapshots (R6: Persist Metacognition State)
    pub fn with_store(mut self, store: Arc<MetacognitionStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Submit a bot status report
    pub async fn submit_bot_report(&self, report: BotStatusReport) {
        let mut reports = self.bot_reports.write().await;
        // Replace existing report for this bot
        if let Some(existing) = reports.iter_mut().find(|r| r.bot_name == report.bot_name) {
            *existing = report;
        } else {
            reports.push(report);
        }
    }

    /// Get current bot status reports
    pub async fn get_bot_reports(&self) -> Vec<BotStatusReport> {
        self.bot_reports.read().await.clone()
    }

    /// Run a single metacognition cycle
    pub async fn run_cycle(&self) -> Result<SystemHealthSnapshot, MetacognitionError> {
        info!(target: "curator.metacognition", "Starting metacognition cycle");

        // Query CNS health
        let cns_health = self.cns.health().await;
        let cns_health_str = format!("{:?}", cns_health);

        // Query variety counters
        let variety_counters = self.cns.variety().await;

        // Query alerts
        let all_alerts = self.cns.alerts().await;
        let critical_alerts = self.cns.critical_alerts().await;

        // Get bot reports
        let bot_reports = self.get_bot_reports().await;

        let snapshot = SystemHealthSnapshot {
            timestamp: chrono::Utc::now(),
            cns_health: cns_health_str,
            variety_counters: variety_counters.clone(),
            critical_alerts: critical_alerts.len(),
            total_alerts: all_alerts.len(),
            bot_status_reports: bot_reports.clone(),
        };

        // Check escalation triggers
        self.check_escalation_triggers(&snapshot).await?;

        // R6: Persist snapshot to storage
        if let Some(ref store) = self.store {
            let stored = StoredSnapshot {
                id: 0,
                timestamp: snapshot.timestamp.to_rfc3339(),
                cns_health: snapshot.cns_health.clone(),
                critical_alerts: snapshot.critical_alerts as i32,
                total_alerts: snapshot.total_alerts as i32,
                variety_counters_json: serde_json::to_string(&snapshot.variety_counters)
                    .unwrap_or_else(|_| "{}".to_string()),
                bot_reports_json: serde_json::to_string(&snapshot.bot_status_reports)
                    .unwrap_or_else(|_| "[]".to_string()),
            };
            if let Err(e) = store.save_snapshot(&stored) {
                warn!(
                    target: "curator.metacognition",
                    error = %e,
                    "Failed to persist metacognition snapshot"
                );
            }
        }

        info!(
            target: "curator.metacognition",
            health = %snapshot.cns_health,
            critical_alerts = snapshot.critical_alerts,
            bot_reports = snapshot.bot_status_reports.len(),
            "Metacognition cycle complete"
        );

        Ok(snapshot)
    }

    /// Check escalation triggers and post escalations if needed
    async fn check_escalation_triggers(
        &self,
        snapshot: &SystemHealthSnapshot,
    ) -> Result<(), MetacognitionError> {
        // Check variety deficit
        let mut _total_variety_deficit = 0u64;
        for (domain, variety) in &snapshot.variety_counters {
            let deficit = self
                .config
                .expected_variety_per_domain
                .saturating_sub(*variety);
            if deficit > 0 {
                _total_variety_deficit += deficit;
                if deficit > self.config.thresholds.variety_deficit {
                    warn!(
                        target: "curator.metacognition",
                        domain = %domain,
                        variety = variety,
                        deficit = deficit,
                        "Variety deficit exceeds threshold"
                    );
                }
            }
        }

        // Check critical alerts
        if snapshot.critical_alerts >= self.config.thresholds.critical_alerts {
            warn!(
                target: "curator.metacognition",
                critical_alerts = snapshot.critical_alerts,
                threshold = self.config.thresholds.critical_alerts,
                "Critical alert count exceeds threshold"
            );

            // Post escalation for critical alerts
            let template_id = hkask_types::TemplateID::new();
            let bot_id = BotID::new(); // System-level escalation
            let error_context = format!(
                "Critical alert count ({}) exceeds threshold ({})",
                snapshot.critical_alerts, self.config.thresholds.critical_alerts
            );

            let queue = self.escalation_queue.lock().await;
            queue
                .add(
                    template_id,
                    bot_id,
                    format!("System has {} critical alerts", snapshot.critical_alerts),
                    0.3, // Low confidence — needs human review
                    0,
                    error_context,
                )
                .map_err(|e| MetacognitionError::Escalation(e.to_string()))?;
        }

        // Check bot failures
        let failed_bots: Vec<_> = snapshot
            .bot_status_reports
            .iter()
            .filter(|r| {
                r.status == BotHealthStatus::Critical || r.status == BotHealthStatus::Unresponsive
            })
            .collect();

        if failed_bots.len() >= self.config.thresholds.bot_failures {
            warn!(
                target: "curator.metacognition",
                failed_bots = failed_bots.len(),
                threshold = self.config.thresholds.bot_failures,
                "Bot failure count exceeds threshold"
            );

            // Post escalation for bot failures
            let template_id = hkask_types::TemplateID::new();
            let bot_id = BotID::new();
            let error_context = format!(
                "{} bots in critical/unresponsive state: {}",
                failed_bots.len(),
                failed_bots
                    .iter()
                    .map(|b| b.bot_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            let queue = self.escalation_queue.lock().await;
            queue
                .add(
                    template_id,
                    bot_id,
                    format!("{} bots require attention", failed_bots.len()),
                    0.4,
                    0,
                    error_context,
                )
                .map_err(|e| MetacognitionError::Escalation(e.to_string()))?;
        }

        Ok(())
    }

    /// Generate a system state summary for posting to standing session
    pub fn generate_summary(&self, snapshot: &SystemHealthSnapshot) -> String {
        let mut summary = String::new();
        summary.push_str("## Metacognition Update\n\n");
        summary.push_str(&format!(
            "**Timestamp:** {}\n",
            snapshot.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        summary.push_str(&format!("**CNS Health:** {}\n", snapshot.cns_health));
        summary.push_str(&format!(
            "**Critical Alerts:** {}\n",
            snapshot.critical_alerts
        ));
        summary.push_str(&format!("**Total Alerts:** {}\n\n", snapshot.total_alerts));

        if !snapshot.variety_counters.is_empty() {
            summary.push_str("### Variety Counters\n");
            for (domain, variety) in &snapshot.variety_counters {
                summary.push_str(&format!("- {}: {}\n", domain, variety));
            }
            summary.push('\n');
        }

        if !snapshot.bot_status_reports.is_empty() {
            summary.push_str("### Bot Status\n");
            for report in &snapshot.bot_status_reports {
                summary.push_str(&format!("- **{}**: {}", report.bot_name, report.status));
                if !report.issues.is_empty() {
                    summary.push_str(&format!(" ({})", report.issues.join(", ")));
                }
                summary.push('\n');
            }
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::CnsRuntimeAdapter;
    use hkask_cns::CnsRuntime;
    use rusqlite::Connection;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_metacognition_cycle() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("escalations.db");
        let conn = Arc::new(Connection::open(db_path).unwrap());
        let queue = Arc::new(EscalationQueue::new(conn).unwrap());
        let cns = Arc::new(CnsRuntimeAdapter::new(Arc::new(CnsRuntime::new())));

        let config = MetacognitionConfig::default();
        let loop_instance = MetacognitionLoop::new(cns, queue, config);

        // Run a single cycle
        let snapshot = loop_instance.run_cycle().await.unwrap();

        assert_eq!(snapshot.critical_alerts, 0);
        assert_eq!(snapshot.total_alerts, 0);
        assert!(snapshot.bot_status_reports.is_empty());
    }

    #[tokio::test]
    async fn test_escalation_on_critical_alerts() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("escalations.db");
        let conn = Arc::new(Connection::open(db_path).unwrap());
        let queue = Arc::new(EscalationQueue::new(conn).unwrap());
        let cns = Arc::new(CnsRuntimeAdapter::new(Arc::new(CnsRuntime::new())));

        let config = MetacognitionConfig {
            thresholds: EscalationThresholds {
                critical_alerts: 3,
                variety_deficit: 100,
                bot_failures: 2,
            },
            ..Default::default()
        };

        let loop_instance = MetacognitionLoop::new(cns, queue.clone(), config);

        let snapshot = SystemHealthSnapshot {
            timestamp: chrono::Utc::now(),
            cns_health: "Degraded".to_string(),
            variety_counters: vec![],
            critical_alerts: 5,
            total_alerts: 10,
            bot_status_reports: vec![],
        };

        // Check escalation triggers
        loop_instance
            .check_escalation_triggers(&snapshot)
            .await
            .unwrap();

        // Verify escalation was posted
        let stats = queue.stats().unwrap();
        assert_eq!(stats.pending, 1);
    }

    #[tokio::test]
    async fn test_summary_generation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("escalations.db");
        let conn = Arc::new(Connection::open(db_path).unwrap());
        let queue = Arc::new(EscalationQueue::new(conn).unwrap());
        let cns = Arc::new(CnsRuntimeAdapter::new(Arc::new(CnsRuntime::new())));

        let config = MetacognitionConfig::default();
        let loop_instance = MetacognitionLoop::new(cns, queue, config);

        let snapshot = SystemHealthSnapshot {
            timestamp: chrono::Utc::now(),
            cns_health: "Healthy".to_string(),
            variety_counters: vec![("domain1".to_string(), 42)],
            critical_alerts: 0,
            total_alerts: 2,
            bot_status_reports: vec![BotStatusReport {
                bot_name: "test-bot".to_string(),
                status: BotHealthStatus::Healthy,
                last_report: None,
                issues: vec![],
            }],
        };

        let summary = loop_instance.generate_summary(&snapshot);

        assert!(summary.contains("Metacognition Update"));
        assert!(summary.contains("Healthy"));
        assert!(summary.contains("domain1"));
        assert!(summary.contains("test-bot"));
    }
}
