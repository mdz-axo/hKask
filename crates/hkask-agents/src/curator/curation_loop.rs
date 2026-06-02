//! Curation Loop — metacognitive observer (Loop 5)
//!
//! observe → evaluate → compose → regulate
//!
//! The Curation Loop is the ONLY loop that can override Cybernetics.
//! It observes system state and intervenes when Cybernetics
//! can't self-stabilize (e.g., alert cascade).

use crate::curator::metacognition::MetacognitionLoop;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::{Deviation, HkaskLoop, LoopAction, LoopId, Signal};
use std::sync::Arc;

/// Curation Loop — metacognitive observer.
///
/// Wraps `MetacognitionLoop` and expresses its regulation cycle
/// through the `Loop` trait. The `CuratorContext` provides
/// capability-disciplined access to CNS, dispatch, and escalation.
pub struct CurationLoop {
    metacognition: Arc<MetacognitionLoop>,
}

impl CurationLoop {
    /// Create a new Curation Loop wrapping a MetacognitionLoop.
    pub fn new(metacognition: Arc<MetacognitionLoop>) -> Self {
        Self { metacognition }
    }

    /// Access the underlying MetacognitionLoop for domain operations
    /// (evaluate_bot, generate_summary, etc.).
    pub fn metacognition(&self) -> &Arc<MetacognitionLoop> {
        &self.metacognition
    }
}

#[async_trait::async_trait]
impl HkaskLoop for CurationLoop {
    fn id(&self) -> LoopId {
        LoopId::Curation
    }

    /// Sense: read CNS health snapshot, variety counters, bot reports.
    ///
    /// Produces signals for:
    /// - Variety deficit (total across all domains)
    /// - Critical alert count
    /// - Bot failure count
    async fn sense(&self) -> Vec<Signal> {
        let context = self.metacognition.context();

        let _cns_health = context.cns().health().await;
        let variety = context.cns().variety().await;
        let critical_alerts = context.cns().critical_alerts().await;
        let bot_reports = self.metacognition.get_bot_reports().await;

        let total_variety_deficit: u64 = variety
            .iter()
            .map(|(_, count)| *count)
            .fold(0u64, |acc, v| acc.saturating_add(v));

        let failed_bots = bot_reports
            .iter()
            .filter(|r| r.status == crate::curator::bot_metrics::BotHealthStatus::Critical)
            .count();

        vec![
            Signal::new(
                LoopId::Curation,
                "variety_deficit",
                total_variety_deficit as f64,
                self.metacognition.config().thresholds.variety_deficit as f64,
            ),
            Signal::new(
                LoopId::Curation,
                "critical_alerts",
                critical_alerts.len() as f64,
                self.metacognition.config().thresholds.critical_alerts as f64,
            ),
            Signal::new(
                LoopId::Curation,
                "bot_failures",
                failed_bots as f64,
                self.metacognition.config().thresholds.bot_failures as f64,
            ),
        ]
    }

    /// Compare: detect thresholds exceeded.
    /// Uses the default implementation from the trait.
    /// Compute: produce CuratorDirectives as LoopActions.
    async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        let mut actions = Vec::new();

        for dev in deviations {
            match dev.signal.metric.as_str() {
                "variety_deficit" if dev.signal.value > dev.signal.set_point => {
                    actions.push(LoopAction::new(
                        LoopId::Cybernetics,
                        hkask_types::loops::ActionType::Calibrate,
                        serde_json::json!({
                            "reason": "variety_deficit_exceeded",
                            "deficit": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ));
                }
                "critical_alerts" if dev.signal.value > dev.signal.set_point => {
                    actions.push(LoopAction::new(
                        LoopId::Cybernetics,
                        hkask_types::loops::ActionType::Escalate,
                        serde_json::json!({
                            "reason": "critical_alerts_exceeded",
                            "count": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ));
                }
                "bot_failures" if dev.signal.value > dev.signal.set_point => {
                    actions.push(LoopAction::new(
                        LoopId::Cybernetics,
                        hkask_types::loops::ActionType::Escalate,
                        serde_json::json!({
                            "reason": "bot_failures_exceeded",
                            "count": dev.signal.value,
                            "threshold": dev.signal.set_point,
                        }),
                    ));
                }
                _ => {}
            }
        }

        actions
    }

    /// Act: issue directives through CuratorContext with DAMPEN filtering.
    ///
    /// Converts `LoopAction`s to `CuratorDirective`s and issues them
    /// through the dispatch. Dampening is applied automatically by
    /// `CuratorContext::issue_directive()`.
    async fn act(&self, actions: &[LoopAction]) {
        for action in actions {
            tracing::info!(
                target: "curation.loop",
                action_type = ?action.action_type,
                target_loop = %action.target,
                "Curation Loop regulatory action"
            );

            // Convert LoopAction to CuratorDirective and issue
            let directive = match action.action_type {
                hkask_types::loops::ActionType::Calibrate
                    if action.parameters.get("reason").and_then(|v| v.as_str())
                        == Some("variety_deficit_exceeded") =>
                {
                    let deficit = action
                        .parameters
                        .get("deficit")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let threshold = action
                        .parameters
                        .get("threshold")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(100);
                    Some(CuratorDirective::CalibrateThreshold {
                        domain: "variety".to_string(),
                        new_threshold: deficit.saturating_add(threshold),
                    })
                }
                hkask_types::loops::ActionType::Escalate => {
                    // Escalations go through the escalation queue
                    // (handled by CuratorContext internally)
                    None
                }
                _ => None,
            };

            if let Some(directive) = directive
                && let Some(trace_id) = self
                    .metacognition
                    .context()
                    .issue_directive(directive)
                    .await
            {
                tracing::info!(
                    target: "curation.loop",
                    trace_id = %trace_id,
                    "Directive issued through dispatch"
                );
            }
            // None means directive was dampened or issuance failed
        }
    }
}
