//! CNS span emission with full categorization

use hkask_types::{NuEvent, Span, WebID};
use serde_json::Value;
use tracing::info;

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
}

impl SpanCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpanCategory::Connector => "cns.connector",
            SpanCategory::Pipeline => "cns.pipeline",
            SpanCategory::Tool => "cns.tool",
            SpanCategory::Prompt => "cns.prompt",
            SpanCategory::AgentPod => "cns.agent_pod",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "connector" | "cns.connector" => Some(SpanCategory::Connector),
            "pipeline" | "cns.pipeline" => Some(SpanCategory::Pipeline),
            "tool" | "cns.tool" => Some(SpanCategory::Tool),
            "prompt" | "cns.prompt" => Some(SpanCategory::Prompt),
            "agent_pod" | "cns.agent_pod" => Some(SpanCategory::AgentPod),
            _ => None,
        }
    }
}

/// CNS span emitter
pub struct SpanEmitter {
    observer_webid: WebID,
}

impl SpanEmitter {
    pub fn new(observer_webid: WebID) -> Self {
        Self { observer_webid }
    }

    /// Emit a CNS span event
    pub fn emit(&self, span: Span, phase: hkask_types::Phase, observation: Value) {
        let event = NuEvent::new(self.observer_webid, span, phase, observation, 0);

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
        self.emit(
            Span::Connector(action.to_string()),
            hkask_types::Phase::Observe,
            observation,
        );
    }

    /// Emit pipeline span (multi-stage processing)
    pub fn emit_pipeline(&self, stage: &str, observation: Value) {
        self.emit(
            Span::Pipeline(stage.to_string()),
            hkask_types::Phase::Observe,
            observation,
        );
    }

    /// Emit tool span (tool invocation)
    pub fn emit_tool(&self, tool_name: &str, observation: Value) {
        self.emit(
            Span::Tool(tool_name.to_string()),
            hkask_types::Phase::Observe,
            observation,
        );
    }

    /// Emit prompt span (template rendering/execution)
    pub fn emit_prompt(&self, phase: &str, observation: Value) {
        let span = Span::Prompt(phase.to_string());
        let event_phase = match phase {
            "select" => hkask_types::Phase::Observe,
            "render" => hkask_types::Phase::Orient,
            "execute" => hkask_types::Phase::Decide,
            "outcome" => hkask_types::Phase::Act,
            _ => hkask_types::Phase::Observe,
        };
        self.emit(span, event_phase, observation);
    }

    /// Emit agent pod span (lifecycle event)
    pub fn emit_agent_pod(&self, lifecycle_event: &str, observation: Value) {
        self.emit(
            Span::AgentPod(lifecycle_event.to_string()),
            hkask_types::Phase::Observe,
            observation,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_category_as_str() {
        assert_eq!(SpanCategory::Connector.as_str(), "cns.connector");
        assert_eq!(SpanCategory::Pipeline.as_str(), "cns.pipeline");
        assert_eq!(SpanCategory::Tool.as_str(), "cns.tool");
        assert_eq!(SpanCategory::Prompt.as_str(), "cns.prompt");
        assert_eq!(SpanCategory::AgentPod.as_str(), "cns.agent_pod");
    }

    #[test]
    fn test_span_category_from_str() {
        assert_eq!(
            SpanCategory::from_str("connector"),
            Some(SpanCategory::Connector)
        );
        assert_eq!(
            SpanCategory::from_str("cns.pipeline"),
            Some(SpanCategory::Pipeline)
        );
        assert_eq!(SpanCategory::from_str("invalid"), None);
    }

    #[test]
    fn test_span_emitter_new() {
        let webid = WebID::new();
        let emitter = SpanEmitter::new(webid);
        // Just verify it constructs without error
        drop(emitter);
    }
}
