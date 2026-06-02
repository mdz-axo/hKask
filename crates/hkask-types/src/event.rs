//! ν-event types — Cross-cutting infrastructure
//!
//! ν-events are the cybernetic audit trail emitted by all loops.
//! They are not owned by any single loop — they are the shared
//! observability substrate that the CNS (Loop 6) senses and the
//! Curator (Loop 5) audits.

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

/// CNS span category — unified from CnsSpan + SpanCategory
///
/// Each variant maps to a `cns.*.` namespace prefix used for observability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanCategory {
    /// Tool governance, invocation (cns.tool.*)
    Tool,
    /// Prompt render, validate, outcome (cns.prompt.*)
    Prompt,
    /// Agent pod lifecycle, delegation (cns.agent_pod.*)
    AgentPod,
    /// External I/O: LLM, embeddings (cns.connector.*)
    Connector,
    /// Multi-stage processing flows (cns.pipeline.*)
    Pipeline,
    /// Energy cost tracking (cns.energy.*)
    Energy,
    /// Review queue events (cns.review.*)
    Review,
    /// Template invocation, registry (cns.template.*)
    Template,
    /// Curation decisions, OCAP boundaries (cns.curation.*)
    Curation,
    /// Variety monitoring, algedonic alerts (cns.variety.*)
    Variety,
    /// Kill zone detection (cns.killzone.*)
    KillZone,
    /// User sovereignty, acquisition resistance (cns.sovereignty.*)
    Sovereignty,
    /// Goal primitive (cns.goal.*)
    Goal,
    /// Specification operations (cns.spec.*)
    Spec,
}

impl SpanCategory {
    /// Full namespace prefix for this category (e.g., "cns.tool")
    pub fn as_str(&self) -> &'static str {
        match self {
            SpanCategory::Tool => "cns.tool",
            SpanCategory::Prompt => "cns.prompt",
            SpanCategory::AgentPod => "cns.agent_pod",
            SpanCategory::Connector => "cns.connector",
            SpanCategory::Pipeline => "cns.pipeline",
            SpanCategory::Energy => "cns.energy",
            SpanCategory::Review => "cns.review",
            SpanCategory::Template => "cns.template",
            SpanCategory::Curation => "cns.curation",
            SpanCategory::Variety => "cns.variety",
            SpanCategory::KillZone => "cns.killzone",
            SpanCategory::Sovereignty => "cns.sovereignty",
            SpanCategory::Goal => "cns.goal",
            SpanCategory::Spec => "cns.spec",
        }
    }

    /// Parse a category from a namespace string (e.g., "cns.tool" or "tool")
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "tool" | "cns.tool" => Some(SpanCategory::Tool),
            "prompt" | "cns.prompt" => Some(SpanCategory::Prompt),
            "agent_pod" | "cns.agent_pod" => Some(SpanCategory::AgentPod),
            "connector" | "cns.connector" => Some(SpanCategory::Connector),
            "pipeline" | "cns.pipeline" => Some(SpanCategory::Pipeline),
            "energy" | "cns.energy" => Some(SpanCategory::Energy),
            "review" | "cns.review" => Some(SpanCategory::Review),
            "template" | "cns.template" => Some(SpanCategory::Template),
            "curation" | "cns.curation" => Some(SpanCategory::Curation),
            "variety" | "cns.variety" => Some(SpanCategory::Variety),
            "killzone" | "cns.killzone" => Some(SpanCategory::KillZone),
            "sovereignty" | "cns.sovereignty" => Some(SpanCategory::Sovereignty),
            "goal" | "cns.goal" => Some(SpanCategory::Goal),
            "spec" | "cns.spec" => Some(SpanCategory::Spec),
            _ => None,
        }
    }
}

impl std::fmt::Display for SpanCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Unified CNS span — category + fully-qualified path
///
/// Collapsed from the former `CnsSpan` enum (category-only) and `Span` tagged
/// enum (variant + path). The struct form is more explicit and avoids the
/// redundant variant/tag duplication.
///
/// The `path` field stores the fully-qualified span path (e.g.,
/// "cns.tool.invoked"). Convenience constructors add the `cns.*.` prefix
/// automatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Span category (maps to cns.* namespace)
    pub category: SpanCategory,
    /// Fully-qualified span path (e.g., "cns.tool.invoked")
    pub path: String,
}

impl Span {
    // -- Convenience constructors (add cns.* prefix automatically) --

    pub fn prompt(path: &str) -> Self {
        Span {
            category: SpanCategory::Prompt,
            path: format!("cns.prompt.{}", path),
        }
    }

    pub fn tool(path: &str) -> Self {
        Span {
            category: SpanCategory::Tool,
            path: format!("cns.tool.{}", path),
        }
    }

    pub fn agent_pod(path: &str) -> Self {
        Span {
            category: SpanCategory::AgentPod,
            path: format!("cns.agent_pod.{}", path),
        }
    }

    pub fn connector(path: &str) -> Self {
        Span {
            category: SpanCategory::Connector,
            path: format!("cns.connector.{}", path),
        }
    }

    pub fn pipeline(path: &str) -> Self {
        Span {
            category: SpanCategory::Pipeline,
            path: format!("cns.pipeline.{}", path),
        }
    }

    pub fn energy(path: &str) -> Self {
        Span {
            category: SpanCategory::Energy,
            path: format!("cns.energy.{}", path),
        }
    }

    pub fn review(path: &str) -> Self {
        Span {
            category: SpanCategory::Review,
            path: format!("cns.review.{}", path),
        }
    }

    pub fn template(path: &str) -> Self {
        Span {
            category: SpanCategory::Template,
            path: format!("cns.template.{}", path),
        }
    }

    pub fn curation(path: &str) -> Self {
        Span {
            category: SpanCategory::Curation,
            path: format!("cns.curation.{}", path),
        }
    }

    pub fn variety(path: &str) -> Self {
        Span {
            category: SpanCategory::Variety,
            path: format!("cns.variety.{}", path),
        }
    }

    pub fn kill_zone(path: &str) -> Self {
        Span {
            category: SpanCategory::KillZone,
            path: format!("cns.killzone.{}", path),
        }
    }

    pub fn sovereignty(path: &str) -> Self {
        Span {
            category: SpanCategory::Sovereignty,
            path: format!("cns.sovereignty.{}", path),
        }
    }

    pub fn goal(path: &str) -> Self {
        Span {
            category: SpanCategory::Goal,
            path: format!("cns.goal.{}", path),
        }
    }

    pub fn spec(path: &str) -> Self {
        Span {
            category: SpanCategory::Spec,
            path: format!("cns.spec.{}", path),
        }
    }

    /// Returns the fully-qualified span path
    pub fn as_str(&self) -> &str {
        &self.path
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

    /// Parse a phase string into a Phase variant, defaulting to Observe
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "regulate" => Phase::Regulate,
            "outcome" => Phase::Outcome,
            _ => Phase::Observe,
        }
    }
}

/// NuEventSink — Trait for persisting CNS events
///
/// Implemented by storage backends (e.g., NuEventStore in hkask-storage).
pub trait NuEventSink: Send + Sync {
    fn persist(&self, event: &NuEvent) -> Result<(), crate::InfrastructureError>;
}
