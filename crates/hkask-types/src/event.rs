//! ν-event (NuEvent) — Cybernetic audit trail events

use crate::id::{EventID, WebID};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// ν-event — Cybernetic observation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuEvent {
    pub id: EventID,
    pub timestamp: DateTime<Utc>,
    pub observer_webid: WebID,
    pub span: Span,
    pub phase: Phase,
    pub observation: Value,
    pub regulation: Option<Value>,
    pub outcome: Option<Value>,
    pub recursion_depth: u8,
    pub parent_event: Option<EventID>,
    pub visibility: String,
}

impl NuEvent {
    pub fn new(
        observer_webid: WebID,
        span: Span,
        phase: Phase,
        observation: Value,
        recursion_depth: u8,
    ) -> Self {
        Self {
            id: EventID::new(),
            timestamp: Utc::now(),
            observer_webid,
            span,
            phase,
            observation,
            regulation: None,
            outcome: None,
            recursion_depth,
            parent_event: None,
            visibility: "private".to_string(),
        }
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_outcome(mut self, outcome: Value) -> Self {
        self.outcome = Some(outcome);
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_regulation(mut self, regulation: Value) -> Self {
        self.regulation = Some(regulation);
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_parent(mut self, parent: EventID) -> Self {
        self.parent_event = Some(parent);
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_visibility(mut self, visibility: &str) -> Self {
        self.visibility = visibility.to_string();
        self
    }
}

/// Span namespace for CNS events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "category", content = "path")]
pub enum Span {
    Prompt(String),
    Tool(String),
    AgentPod(String),
    Connector(String),
    Pipeline(String),
    Energy(String),
    Review(String),
}

impl Span {
    pub fn prompt(path: &str) -> Self {
        Span::Prompt(format!("cns.prompt.{}", path))
    }

    pub fn tool(path: &str) -> Self {
        Span::Tool(format!("cns.tool.{}", path))
    }

    pub fn agent_pod(path: &str) -> Self {
        Span::AgentPod(format!("cns.agent_pod.{}", path))
    }

    pub fn connector(path: &str) -> Self {
        Span::Connector(format!("cns.connector.{}", path))
    }

    pub fn pipeline(path: &str) -> Self {
        Span::Pipeline(format!("cns.pipeline.{}", path))
    }

    pub fn energy(path: &str) -> Self {
        Span::Energy(format!("cns.energy.{}", path))
    }

    pub fn review(path: &str) -> Self {
        Span::Review(format!("cns.review.{}", path))
    }

    pub fn as_str(&self) -> &str {
        match self {
            Span::Prompt(s) => s,
            Span::Tool(s) => s,
            Span::AgentPod(s) => s,
            Span::Connector(s) => s,
            Span::Pipeline(s) => s,
            Span::Energy(s) => s,
            Span::Review(s) => s,
        }
    }
}

/// Phase of cybernetic cycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Observe,
    Regulate,
    Outcome,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Observe => "observe",
            Phase::Regulate => "regulate",
            Phase::Outcome => "outcome",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_nu_event_new() {
        let event = NuEvent::new(
            WebID::new(),
            Span::prompt("select"),
            Phase::Observe,
            json!({"test": "data"}),
            0,
        );

        assert_eq!(event.recursion_depth, 0);
        assert_eq!(event.visibility, "private");
        assert!(event.outcome.is_none());
    }

    #[test]
    fn test_nu_event_with_outcome() {
        let event = NuEvent::new(
            WebID::new(),
            Span::prompt("select"),
            Phase::Observe,
            json!({"test": "data"}),
            0,
        )
        .with_outcome(json!({"result": "success"}));

        assert!(event.outcome.is_some());
    }

    #[test]
    fn test_nu_event_with_parent() {
        let parent_id = EventID::new();
        let event = NuEvent::new(
            WebID::new(),
            Span::prompt("select"),
            Phase::Observe,
            json!({"test": "data"}),
            0,
        )
        .with_parent(parent_id);

        assert_eq!(event.parent_event, Some(parent_id));
    }

    #[test]
    fn test_span_prompt() {
        let span = Span::prompt("select");
        assert_eq!(span.as_str(), "cns.prompt.select");
    }

    #[test]
    fn test_span_tool() {
        let span = Span::tool("invocation");
        assert_eq!(span.as_str(), "cns.tool.invocation");
    }

    #[test]
    fn test_span_agent_pod() {
        let span = Span::agent_pod("populated");
        assert_eq!(span.as_str(), "cns.agent_pod.populated");
    }

    #[test]
    fn test_phase_as_str() {
        assert_eq!(Phase::Observe.as_str(), "observe");
        assert_eq!(Phase::Regulate.as_str(), "regulate");
        assert_eq!(Phase::Outcome.as_str(), "outcome");
    }
}
