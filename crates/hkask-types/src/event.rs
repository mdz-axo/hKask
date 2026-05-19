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

    pub fn with_outcome(mut self, outcome: Value) -> Self {
        self.outcome = Some(outcome);
        self
    }

    pub fn with_regulation(mut self, regulation: Value) -> Self {
        self.regulation = Some(regulation);
        self
    }

    pub fn with_parent(mut self, parent: EventID) -> Self {
        self.parent_event = Some(parent);
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
}

/// Phase of cybernetic cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Observe,
    Regulate,
    Outcome,
}
