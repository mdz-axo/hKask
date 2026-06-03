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

/// Validated CNS span namespace.
///
/// Constructed via `SpanNamespace::new()` which validates against
/// the canonical set. The module path IS the loop assignment.
/// Cannot be forged — construction requires a valid namespace string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanNamespace(String);

/// Canonical CNS span namespaces — the only valid values.
const CANONICAL_NAMESPACES: &[&str] = &[
    "cns.tool",
    "cns.prompt",
    "cns.agent_pod",
    "cns.connector",
    "cns.pipeline",
    "cns.energy",
    "cns.review",
    "cns.template",
    "cns.curation",
    "cns.variety",
    "cns.killzone",
    "cns.sovereignty",
    "cns.goal",
    "cns.spec",
];

impl SpanNamespace {
    /// Create a validated span namespace. Panics if the namespace is not canonical.
    /// Use `from_str` for fallible construction.
    pub fn new(namespace: &str) -> Self {
        assert!(
            CANONICAL_NAMESPACES.contains(&namespace),
            "Invalid CNS namespace: {namespace}"
        );
        Self(namespace.to_string())
    }

    /// Fallible construction — returns None for invalid namespaces.
    /// Accepts both short ("tool") and full ("cns.tool") forms.
    pub fn from_str(s: &str) -> Option<Self> {
        let full = if s.starts_with("cns.") {
            s.to_string()
        } else {
            format!("cns.{s}")
        };
        if CANONICAL_NAMESPACES.contains(&full.as_str()) {
            Some(Self(full))
        } else {
            None
        }
    }

    /// The namespace prefix (e.g., "cns.tool")
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The short name after "cns." (e.g., "tool")
    pub fn short_name(&self) -> &str {
        &self.0[4..] // Skip "cns."
    }
}

impl std::fmt::Display for SpanNamespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unified CNS span — namespace + fully-qualified path
///
/// Constructed via `Span::new()` with a validated namespace.
/// The namespace is validated at construction time by `SpanNamespace`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// The validated namespace (e.g., SpanNamespace::new("cns.tool"))
    pub namespace: SpanNamespace,
    /// Fully-qualified span path (e.g., "cns.tool.invoked")
    pub path: String,
}

impl Span {
    /// Create a new span with validated namespace.
    ///
    /// Example: `Span::new(SpanNamespace::new("cns.tool"), "invoked")`
    pub fn new(namespace: SpanNamespace, path: &str) -> Self {
        let full_path = format!("{}.{}", namespace.as_str(), path);
        Self {
            namespace,
            path: full_path,
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
    Sense,
    Compare,
    Compute,
    Act,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Sense => "sense",
            Phase::Compare => "compare",
            Phase::Compute => "compute",
            Phase::Act => "act",
        }
    }

    /// Parse a phase string into a Phase variant, with backward-compatible
    /// mappings from the old names (observe→Sense, regulate→Compute, outcome→Act).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "compare" | "Compare" => Phase::Compare,
            "compute" | "Compute" | "regulate" | "Regulate" => Phase::Compute,
            "act" | "Act" | "outcome" | "Outcome" => Phase::Act,
            _ => Phase::Sense,
        }
    }
}

/// NuEventSink — Trait for persisting CNS events
///
/// Implemented by storage backends (e.g., NuEventStore in hkask-storage).
pub trait NuEventSink: Send + Sync {
    fn persist(&self, event: &NuEvent) -> Result<(), crate::InfrastructureError>;
}
