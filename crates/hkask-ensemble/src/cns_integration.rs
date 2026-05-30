//! CNS Integration - Full wiring of CNS spans across ensemble
//!
//! Integrates CNS monitoring across all ensemble components:
//! - Chat coordination spans
//! - Deliberation tracking
//! - Confidence escalation
//! - Variety monitoring
//! - Algedonic alerts

use hkask_cns::{RuntimeAlert, SpanEmitter, VarietyMonitor};
use hkask_types::{Phase, Span, WebID};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::cns_spans::OkapiCnsSpan;

/// CNS integration manager
///
/// Central hub for CNS span emission and algedonic alert handling.
pub struct CnsIntegration {
    span_emitter: SpanEmitter,
    variety_monitor: Arc<RwLock<VarietyMonitor>>,
    observer_webid: WebID,
}

impl CnsIntegration {
    pub fn new(observer_webid: WebID) -> Self {
        Self {
            span_emitter: SpanEmitter::new(observer_webid),
            variety_monitor: Arc::new(RwLock::new(VarietyMonitor::new())),
            observer_webid,
        }
    }

    /// Emit chat coordination span
    pub fn emit_chat_span(&self, action: &str, data: serde_json::Value) {
        self.span_emitter
            .emit_with_phase(Span::connector(action), Phase::Observe, data);
        info!(target: "hkask.cns.chat", action = %action, "Chat span emitted");
    }

    /// Emit deliberation span
    pub fn emit_deliberation_span(&self, session_id: &str, status: &str, data: serde_json::Value) {
        self.span_emitter.emit_with_phase(
            Span::pipeline(&format!("deliberation.{}", session_id)),
            Phase::Observe,
            json!({
                "session_id": session_id,
                "status": status,
                "data": data
            }),
        );
        info!(target: "hkask.cns.deliberation", session_id = %session_id, "Deliberation span emitted");
    }

    /// Emit confidence escalation span
    pub fn emit_confidence_escalation(
        &self,
        initial_confidence: f64,
        threshold: f64,
        primary_model: &str,
        escalated_model: &str,
    ) {
        let span = OkapiCnsSpan::ConfidenceEscalation {
            initial_confidence,
            threshold,
            primary_model: primary_model.to_string(),
            escalated_model: escalated_model.to_string(),
        };

        self.span_emitter
            .emit_with_phase(Span::prompt("escalation"), Phase::Observe, json!(span));

        info!(
            target: "hkask.cns.confidence",
            initial_confidence = %initial_confidence,
            threshold = %threshold,
            "Confidence escalation triggered"
        );
    }

    /// Emit tool invocation span
    pub fn emit_tool_span(&self, tool_name: &str, success: bool, data: serde_json::Value) {
        let span_name = if success {
            "tool.invoked"
        } else {
            "tool.failed"
        };

        self.span_emitter.emit_with_phase(
            Span::tool(span_name),
            Phase::Observe,
            json!({
                "tool_name": tool_name,
                "success": success,
                "data": data
            }),
        );
    }

    /// Emit template render span
    pub fn emit_template_render_span(
        &self,
        template_id: &str,
        success: bool,
        data: serde_json::Value,
    ) {
        let span_name = if success {
            "template.rendered"
        } else {
            "template.failed"
        };

        self.span_emitter.emit_with_phase(
            Span::prompt(span_name),
            Phase::Observe,
            json!({
                "template_id": template_id,
                "success": success,
                "data": data
            }),
        );
    }

    /// Track variety for a category
    pub async fn track_variety(&self, category: &str, count: u64, threshold: u64) {
        // VarietyMonitor doesn't have track() method - use counter().increment()
        let mut variety_monitor = self.variety_monitor.write().await;
        let counter = variety_monitor.counter(category);
        for _ in 0..count {
            counter.increment("state_active");
        }

        // Check for deficit and generate alert if needed
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

        // AlgedonicManager doesn't have handle() - just log the alert
        // The alert is already logged above
        info!(
            target: "hkask.cns.algedonic",
            domain = %alert.domain,
            deficit = alert.deficit,
            threshold = alert.threshold,
            "Alert recorded"
        );
    }

    /// Emit agent pod lifecycle span
    pub fn emit_pod_lifecycle_span(
        &self,
        pod_id: &str,
        lifecycle_event: &str,
        data: serde_json::Value,
    ) {
        self.span_emitter.emit_with_phase(
            Span::agent_pod(lifecycle_event),
            Phase::Observe,
            json!({
                "pod_id": pod_id,
                "data": data
            }),
        );
        info!(target: "hkask.cns.pod", pod_id = %pod_id, lifecycle_event = %lifecycle_event, "Pod lifecycle span emitted");
    }

    /// Emit goal span
    pub fn emit_goal_span(&self, goal_id: &str, goal_event: &str, data: serde_json::Value) {
        self.span_emitter.emit_with_phase(
            Span::goal(goal_event),
            Phase::Observe,
            json!({
                "goal_id": goal_id,
                "data": data
            }),
        );
    }

    /// Emit sovereignty boundary span
    pub fn emit_sovereignty_span(&self, boundary_type: &str, data: serde_json::Value) {
        self.span_emitter
            .emit_with_phase(Span::sovereignty(boundary_type), Phase::Observe, data);
    }

    /// Emit energy consumption span
    pub fn emit_energy_span(
        &self,
        energy_event: &str,
        tokens: u64,
        cost: f64,
        data: serde_json::Value,
    ) {
        self.span_emitter.emit_with_phase(
            Span::energy(energy_event),
            Phase::Observe,
            json!({
                "tokens": tokens,
                "estimated_cost": cost,
                "data": data
            }),
        );
    }

    /// Get observer WebID
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
