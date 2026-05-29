//! Algedonic Escalation Adapter — Routes CNS alerts to Curator via standing session
//!
//! Connects AlgedonicManager's escalation callback to the Curator metacognition loop.
//! Every AlertSeverity::Critical alert is delivered to the Curator as an ACP message.
//! Every AlertSeverity::Warning increments the bot's evaluation score deficit counter.
//! Calibration actions emit cns.energy.calibrate spans recording before/after thresholds.

use crate::algedonic::{AlertSeverity, DEFAULT_THRESHOLD, RuntimeAlert};
use crate::spans::CnsEmit;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Trait for sending ACP messages to the Curator.
/// Implemented by ACP runtime or test doubles.
pub trait AcpSender: Send + Sync {
    /// Send an algedonic alert notification to the Curator via ACP.
    fn send_alert(
        &self,
        domain: &str,
        severity: &str,
        deficit: u64,
        drift_magnitude: f64,
        message: &str,
    );
}

/// Calibration record — tracks threshold changes made by Curator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationRecord {
    /// Domain being calibrated
    pub domain: String,
    /// Threshold before calibration
    pub threshold_before: u64,
    /// Threshold after calibration
    pub threshold_after: u64,
    /// Curator WebID who authorized the calibration
    pub calibrated_by: WebID,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Reason for calibration
    pub reason: String,
}

/// Escalation action produced by the Curator's metacognition loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscalationAction {
    /// Adjust a threshold (Curator directive)
    CalibrateThreshold {
        domain: String,
        new_threshold: u64,
        reason: String,
    },
    /// Trigger a Kata coaching cycle (Curator coaching directive)
    TriggerKata {
        bot_id: WebID,
        kata_type: String,
        gap_description: String,
    },
    /// Escalate to human administrator (deficit > 500, persistent Critical)
    EscalateToHuman {
        domain: String,
        deficit: u64,
        alert_message: String,
    },
}

/// Result of processing an escalated alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationResult {
    /// The original alert
    pub alert: RuntimeAlert,
    /// The action taken
    pub action: EscalationAction,
    /// Whether the action was successfully applied
    pub applied: bool,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Compute the magnitude of divergence between specification goals and
/// actual implementation state.
///
/// Returns a f64 between 0.0 (perfect alignment) and 1.0 (complete drift).
/// The drift is computed as the ratio of unmet goals to total goals.
pub fn compute_spec_drift(goals_total: u64, goals_met: u64) -> f64 {
    if goals_total == 0 {
        return 0.0; // No goals = no drift possible
    }
    let unmet = goals_total.saturating_sub(goals_met);
    (unmet as f64) / (goals_total as f64)
}

/// Algedonic escalation adapter
///
/// Routes algedonic alerts from CNS to the Curator via the standing session.
/// Implements the EscalationCallback trait pattern used by AlgedonicManager.
pub struct AlgedonicEscalationAdapter {
    /// Curator's WebID for directing escalations
    curator_webid: WebID,
    /// Pending escalation results (for CLI/API visibility)
    escalation_results: Arc<RwLock<Vec<EscalationResult>>>,
    /// Calibration history
    calibration_history: Arc<RwLock<Vec<CalibrationRecord>>>,
    /// Pending alerts not yet processed
    pending_alerts: Arc<RwLock<Vec<RuntimeAlert>>>,
    /// Default variety deficit threshold
    default_threshold: u64,
    /// ACP sender for delivering alerts to Curator
    acp_sender: Option<Arc<dyn AcpSender + Send + Sync>>,
    /// CNS emitter for spec drift spans
    cns_emitter: Option<Arc<dyn CnsEmit + Send + Sync>>,
}

impl AlgedonicEscalationAdapter {
    /// Create a new escalation adapter
    pub fn new(curator_webid: WebID) -> Self {
        Self {
            curator_webid,
            escalation_results: Arc::new(RwLock::new(Vec::new())),
            calibration_history: Arc::new(RwLock::new(Vec::new())),
            pending_alerts: Arc::new(RwLock::new(Vec::new())),
            default_threshold: DEFAULT_THRESHOLD,
            acp_sender: None,
            cns_emitter: None,
        }
    }

    /// Create with custom default threshold
    pub fn with_threshold(mut self, threshold: u64) -> Self {
        self.default_threshold = threshold;
        self
    }

    /// Wire in an ACP sender for delivering alerts to Curator
    pub fn with_acp_sender(mut self, sender: Arc<dyn AcpSender + Send + Sync>) -> Self {
        self.acp_sender = Some(sender);
        self
    }

    /// Wire in a CNS emitter for spec drift spans
    pub fn with_cns_emitter(mut self, emitter: Arc<dyn CnsEmit + Send + Sync>) -> Self {
        self.cns_emitter = Some(emitter);
        self
    }

    /// Process an algedonic alert — this is the callback function
    ///
    /// Called by AlgedonicManager when a variety deficit exceeds threshold.
    /// Routes the alert to the Curator based on severity.
    pub async fn process_alert(&self, alert: &RuntimeAlert) {
        match alert.severity {
            AlertSeverity::Critical => {
                // Critical alerts: escalate to Curator with action recommendation
                let action = if alert.deficit > 500 {
                    // Deficit > 500: escalate to human
                    EscalationAction::EscalateToHuman {
                        domain: alert.domain.clone(),
                        deficit: alert.deficit,
                        alert_message: alert.message.clone(),
                    }
                } else {
                    // Deficit 100-500: trigger calibration
                    let new_threshold = (alert.threshold * 3) / 2; // 1.5x current threshold
                    EscalationAction::CalibrateThreshold {
                        domain: alert.domain.clone(),
                        new_threshold,
                        reason: format!(
                            "Variety deficit {} exceeds threshold {}. Curator-directed calibration.",
                            alert.deficit, alert.threshold
                        ),
                    }
                };

                info!(
                    target: "cns.algedonic.escalation",
                    domain = %alert.domain,
                    deficit = alert.deficit,
                    severity = "critical",
                    "Escalating alert to Curator"
                );

                // Compute spec drift as deficit-to-threshold ratio
                let drift = compute_spec_drift(
                    alert.threshold,
                    alert.threshold.saturating_sub(alert.deficit),
                );

                // Deliver ACP notification to Curator
                if let Some(ref sender) = self.acp_sender {
                    sender.send_alert(
                        &alert.domain,
                        "critical",
                        alert.deficit,
                        drift,
                        &alert.message,
                    );
                }

                // Emit cns.spec.drift span for critical severity
                if let Some(ref emitter) = self.cns_emitter {
                    emitter.emit(
                        "cns.spec.drift",
                        serde_json::json!({
                            "domain": alert.domain,
                            "drift_magnitude": drift,
                            "severity": "critical",
                            "deficit": alert.deficit,
                            "threshold": alert.threshold,
                            "message": alert.message,
                        }),
                        drift,
                    );
                }

                let result = EscalationResult {
                    alert: alert.clone(),
                    action,
                    applied: false, // Will be marked true when Curator processes it
                    timestamp: chrono::Utc::now(),
                };

                let mut results = self.escalation_results.write().await;
                results.push(result);
            }
            AlertSeverity::Warning => {
                // Warning alerts: queue for next metacognition cycle
                warn!(
                    target: "cns.algedonic.escalation",
                    domain = %alert.domain,
                    deficit = alert.deficit,
                    "Warning: variety deficit approaching threshold"
                );
                let mut pending = self.pending_alerts.write().await;
                pending.push(alert.clone());
            }
            AlertSeverity::Info => {
                // Info alerts: log only, no escalation
                info!(
                    target: "cns.algedonic.escalation",
                    domain = %alert.domain,
                    deficit = alert.deficit,
                    "Info: variety deficit detected"
                );
            }
        }
    }

    /// Get pending alerts for metacognition cycle
    pub async fn get_pending_alerts(&self) -> Vec<RuntimeAlert> {
        let mut pending = self.pending_alerts.write().await;
        std::mem::take(&mut *pending)
    }

    /// Get all escalation results
    pub async fn get_escalation_results(&self) -> Vec<EscalationResult> {
        self.escalation_results.read().await.clone()
    }

    /// Record a calibration (Curator-directed threshold change)
    pub async fn record_calibration(
        &self,
        domain: &str,
        threshold_before: u64,
        threshold_after: u64,
        reason: &str,
    ) {
        let record = CalibrationRecord {
            domain: domain.to_string(),
            threshold_before,
            threshold_after,
            calibrated_by: self.curator_webid,
            timestamp: chrono::Utc::now(),
            reason: reason.to_string(),
        };

        info!(
            target: "cns.energy.calibrate",
            domain = %domain,
            threshold_before,
            threshold_after,
            reason = %reason,
            "Calibration recorded"
        );

        let mut history = self.calibration_history.write().await;
        history.push(record);
    }

    /// Get calibration history
    pub async fn get_calibration_history(&self) -> Vec<CalibrationRecord> {
        self.calibration_history.read().await.clone()
    }

    /// Mark an escalation result as applied
    pub async fn mark_applied(&self, index: usize) {
        let mut results = self.escalation_results.write().await;
        if let Some(result) = results.get_mut(index) {
            result.applied = true;
        }
    }

    /// Get the Curator's WebID
    pub fn curator_webid(&self) -> &WebID {
        &self.curator_webid
    }
}

/// Create an escalation callback function that sends alerts through a channel
///
/// This creates a closure suitable for use with `AlgedonicManager::with_escalation_callback()`.
/// The callback queues alerts for async processing via the adapter.
pub fn create_escalation_callback(
    adapter: Arc<AlgedonicEscalationAdapter>,
) -> Box<dyn Fn(&RuntimeAlert) + Send + Sync> {
    // Note: The AlgedonicManager callback is synchronous, so we can't await here.
    // Instead, we queue the alert for processing in the next metacognition cycle.
    Box::new(move |alert: &RuntimeAlert| {
        // Log the alert immediately (synchronous)
        match alert.severity {
            AlertSeverity::Critical => {
                error!(
                    target: "cns.algedonic.callback",
                    domain = %alert.domain,
                    deficit = alert.deficit,
                    "CRITICAL algedonic alert received — queuing for Curator"
                );
            }
            AlertSeverity::Warning => {
                warn!(
                    target: "cns.algedonic.callback",
                    domain = %alert.domain,
                    deficit = alert.deficit,
                    "Warning algedonic alert received"
                );
            }
            AlertSeverity::Info => {
                info!(
                    target: "cns.algedonic.callback",
                    domain = %alert.domain,
                    deficit = alert.deficit,
                    "Info algedonic alert received"
                );
            }
        }

        // Queue for async processing
        let alert_clone = alert.clone();
        let adapter = adapter.clone();
        tokio::spawn(async move {
            adapter.process_alert(&alert_clone).await;
        });
    })
}
