//! CNS span emission with full categorization

use hkask_types::{NuEvent, Span, WebID};
use serde_json::Value;
use tracing::info;

/// CnsEmit — Canonical CNS event emission trait
///
/// Unified trait for all CNS event emission across hKask subsystems.
/// Replaces `CnsPort` (hkask-templates) and `CNSSpanPort` (hkask-agents).
pub trait CnsEmit {
    /// Emit a CNS span event with full context
    ///
    /// # Arguments
    /// * `span` — Span name (e.g., "cns.agent_pod.registered")
    /// * `phase` — Event phase (e.g., "registered", "activated", "observe")
    /// * `observation` — Event observation as JSON
    /// * `confidence` — Confidence score (0.0 to 1.0)
    fn emit_event(&self, span: &str, phase: &str, observation: &Value, confidence: f64);

    /// Emit a CNS span event with default phase ("observe")
    ///
    /// Convenience method for callers that don't need explicit phase tracking.
    fn emit(&self, span: &str, outcome: Value, confidence: f64) {
        self.emit_event(span, "observe", &outcome, confidence);
    }
}

/// CNS span categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanCategory {
    /// External I/O (LLM dispatch, OCR, embeddings)
    Connector,
    /// Multi-stage processing flows
    Pipeline,
    /// Tool governance and invocation
    Tool,
    /// Prompt feedback loop (rendered, validated, outcome)
    Prompt,
    /// Agent lifecycle (populated, registered, activated, delegation)
    AgentPod,
    /// Energy cost tracking (allocate, consume, opportunity, deficit)
    Energy,
    /// User sovereignty monitoring (boundary, acquisition, kill-zone)
    Sovereignty,
    /// Goal primitive (create, transition, verify, complete, subgoal)
    Goal,
}

impl SpanCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpanCategory::Connector => "cns.connector",
            SpanCategory::Pipeline => "cns.pipeline",
            SpanCategory::Tool => "cns.tool",
            SpanCategory::Prompt => "cns.prompt",
            SpanCategory::AgentPod => "cns.agent_pod",
            SpanCategory::Energy => "cns.energy",
            SpanCategory::Sovereignty => "cns.sovereignty",
            SpanCategory::Goal => "cns.goal",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "connector" | "cns.connector" => Some(SpanCategory::Connector),
            "pipeline" | "cns.pipeline" => Some(SpanCategory::Pipeline),
            "tool" | "cns.tool" => Some(SpanCategory::Tool),
            "prompt" | "cns.prompt" => Some(SpanCategory::Prompt),
            "agent_pod" | "cns.agent_pod" => Some(SpanCategory::AgentPod),
            "energy" | "cns.energy" => Some(SpanCategory::Energy),
            "sovereignty" | "cns.sovereignty" => Some(SpanCategory::Sovereignty),
            "goal" | "cns.goal" => Some(SpanCategory::Goal),
            _ => None,
        }
    }
}

/// CNS span emitter
pub struct SpanEmitter {
    observer_webid: WebID,
}

impl Default for SpanEmitter {
    fn default() -> Self {
        Self {
            observer_webid: WebID::new(),
        }
    }
}

impl SpanEmitter {
    pub fn new(observer_webid: WebID) -> Self {
        Self { observer_webid }
    }

    /// Emit a CNS span event
    pub fn emit(&self, span: Span, observation: Value) {
        let event = NuEvent::new(
            self.observer_webid,
            span,
            hkask_types::Phase::Observe,
            observation,
            0,
        );

        info!(
            target: "cns",
            event = ?event.id,
            span = ?event.span,
            phase = ?event.phase,
            "CNS event emitted"
        );
    }

    /// Emit connector span (external I/O)
    pub fn emit_connector(&self, action: &str, observation: Value) {
        self.emit(Span::Connector(action.to_string()), observation);
    }

    /// Emit pipeline span (multi-stage processing)
    pub fn emit_pipeline(&self, stage: &str, observation: Value) {
        self.emit(Span::Pipeline(stage.to_string()), observation);
    }

    /// Emit tool span (tool invocation)
    pub fn emit_tool(&self, tool_name: &str, observation: Value) {
        self.emit(Span::Tool(tool_name.to_string()), observation);
    }

    /// Emit prompt span (template rendering/execution)
    pub fn emit_prompt(&self, phase: &str, observation: Value) {
        self.emit(Span::Prompt(phase.to_string()), observation);
    }

    /// Emit agent pod span (lifecycle event)
    pub fn emit_agent_pod(&self, lifecycle_event: &str, observation: Value) {
        self.emit(Span::AgentPod(lifecycle_event.to_string()), observation);
    }

    /// Emit energy span (cost tracking)
    pub fn emit_energy(&self, energy_event: &str, observation: Value) {
        self.emit(Span::Energy(energy_event.to_string()), observation);
    }

    /// Emit sovereignty span (boundary, acquisition, kill-zone)
    pub fn emit_sovereignty(&self, sovereignty_event: &str, observation: Value) {
        self.emit(
            Span::Sovereignty(sovereignty_event.to_string()),
            observation,
        );
    }

    /// Emit sovereignty alert (kill-zone detected)
    pub fn emit_sovereignty_alert(&self, alert_type: &str, observation: Value) {
        self.emit(
            Span::Sovereignty(format!("alert.{}", alert_type)),
            observation,
        );
    }

    /// Emit goal span (lifecycle event)
    pub fn emit_goal(&self, goal_event: &str, observation: Value) {
        self.emit(Span::Goal(goal_event.to_string()), observation);
    }

    /// Emit goal alert (variety deficit, algedonic)
    pub fn emit_goal_alert(&self, alert_type: &str, observation: Value) {
        self.emit(Span::Goal(format!("alert.{}", alert_type)), observation);
    }
}
