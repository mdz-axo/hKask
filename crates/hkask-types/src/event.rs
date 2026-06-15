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
use std::str::FromStr;

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
    "cns.inference",
    "cns.agent_pod",
    "cns.connector",
    "cns.pipeline",
    "cns.gas",
    "cns.review",
    "cns.template",
    "cns.curation",
    "cns.variety",
    "cns.sovereignty",
    "cns.goal",
    "cns.spec",
    "cns.test",
    "cns.chat",
    // Hierarchical spans — registered from PRINCIPLES.md §1.4 (P2-06-D1)
    "cns.cybernetics.backpressure",
    "cns.cybernetics.cadence",
    "cns.set_point",
    "cns.memory.encode",
    "cns.memory.budget",
    // Wallet spans — rJoule payments, multi-chain deposits, API key lifecycle
    "cns.wallet.balance",
    "cns.wallet.deposit",
    "cns.wallet.deposit_shielded",
    "cns.wallet.withdrawal",
    "cns.wallet.conversion",
    "cns.wallet.key_issued",
    "cns.wallet.key_revoked",
    "cns.wallet.key_expired",
    "cns.wallet.key_exhausted",
    "cns.wallet.treasury",
    "cns.wallet.chain_error",
    "cns.wallet.privacy.shield",
    "cns.wallet.privacy.unshield",
    "cns.wallet.privacy_error",
    // Lazy Universe spans — least-action monitoring (P5 grounding, TASK 4.2)
    "cns.condenser.compression_ratio",
    "cns.evolution.energy_delta",
    "cns.architecture.module_depth",
    // Improv spans — composable interaction grammar (hkask-improv crate)
    "cns.improv.mode.active",
    "cns.improv.plussing.ratio",
    "cns.improv.freestyle.coherence",
    "cns.kata.improv.effectiveness",
    "cns.improv.cascade.depth",
    // Outcome quality spans — success/failure rate tracking per domain
    "cns.outcome.tool",
    "cns.outcome.inference",
    "cns.outcome.memory",
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

    /// Fallible construction — returns Err for invalid namespaces.
    /// Accepts both short ("tool") and full ("cns.tool") forms.
    ///
    /// Implements `FromStr` so that `"variety".parse::<SpanNamespace>()` works.
    pub fn parse(s: &str) -> Option<Self> {
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

    /// F-SYN-009: classify this namespace into a `SpanCategory` for
    /// typed dispatch (e.g. by `DecayConfig::lambda_for`).
    ///
    /// Hierarchical matches by `short_name()` prefix are preserved
    /// (e.g. `cns.variety.sensor` → `Variety`). Unknown namespaces
    /// return `SpanCategory::Unknown` so the caller can decide the
    /// fallback policy explicitly (the historical behaviour was
    /// `cybernetics_lambda`).
    pub fn category(&self) -> SpanCategory {
        let s = self.short_name();
        let prefix = s.split('.').next().unwrap_or(s);
        match prefix {
            "variety" | "gas" => SpanCategory::Cybernetics,
            "curation" | "spec" => SpanCategory::Curation,
            "inference" => SpanCategory::Inference,
            "agent_pod" | "connector" => SpanCategory::Episodic,
            _ => SpanCategory::Unknown,
        }
    }
}

/// F-SYN-009: typed dispatch key for span-category-dependent logic
/// (e.g. `DecayConfig::lambda_for`).
///
/// Replaces the previous `&str` dispatch with a closed enum, while
/// preserving the hierarchical `.starts_with` matches that the old
/// string-based dispatch used. An `Unknown` variant makes the
/// fallback policy explicit at the type level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanCategory {
    /// `cns.variety*`, `cns.gas*` — the cybernetics loop.
    Cybernetics,
    /// `cns.curation*`, `cns.spec*` — the curation loop.
    Curation,
    /// `cns.inference*` — the inference loop.
    Inference,
    /// `cns.agent_pod*`, `cns.connector*` — episodic memory.
    Episodic,
    /// Any other namespace. Callers decide the fallback policy.
    Unknown,
}

impl SpanCategory {
    /// Parse a `SpanCategory` from a `short_name()` string (e.g. `variety`,
    /// `variety.sensor`, `agent_pod.registered`). Returns `Unknown`
    /// for unrecognised prefixes.
    pub fn from_short_name(s: &str) -> Self {
        let prefix = s.split('.').next().unwrap_or(s);
        match prefix {
            "variety" | "gas" => Self::Cybernetics,
            "curation" | "spec" => Self::Curation,
            "inference" => Self::Inference,
            "agent_pod" | "connector" => Self::Episodic,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for SpanCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SpanCategory::Cybernetics => "cybernetics",
            SpanCategory::Curation => "curation",
            SpanCategory::Inference => "inference",
            SpanCategory::Episodic => "episodic",
            SpanCategory::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

impl FromStr for SpanNamespace {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(())
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
    Compute,
    Compare,
    Act,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Sense => "sense",
            Phase::Compute => "compute",
            Phase::Compare => "compare",
            Phase::Act => "act",
        }
    }

    /// Parse a phase string into a Phase variant, with backward-compatible
    /// mappings from the old names (observe→Sense, regulate→Compute, outcome→Act).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "sense" | "Sense" | "observe" | "Observe" => Phase::Sense,
            "compute" | "Compute" | "regulate" | "Regulate" => Phase::Compute,
            "compare" | "Compare" => Phase::Compare,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::WebID;

    // REQ: types-event-001 — NuEvent::new() sets correct defaults
    #[test]
    fn nuevent_new_sets_correct_defaults() {
        let webid = WebID::from_persona(b"test-agent");
        let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
        let obs = serde_json::json!({"key": "value"});

        let event = NuEvent::new(webid.clone(), span, Phase::Sense, obs.clone(), 0);

        assert_eq!(event.observer_webid, webid);
        assert_eq!(event.phase, Phase::Sense);
        assert_eq!(event.observation, obs);
        assert_eq!(event.recursion_depth, 0);
        assert_eq!(event.visibility, "private");
        assert!(event.regulation.is_none());
        assert!(event.outcome.is_none());
        assert!(event.parent_event.is_none());
    }

    // REQ: types-event-002 — NuEvent builder chain produces correct fields
    #[test]
    fn nuevent_builder_chain_sets_fields() {
        let webid = WebID::from_persona(b"test-agent");
        let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
        let parent_id = crate::id::EventID::new();

        let event = NuEvent::new(webid, span, Phase::Act, serde_json::json!({}), 1)
            .with_outcome(serde_json::json!({"result": "ok"}))
            .with_regulation(serde_json::json!({"adj": 0.5}))
            .with_parent(parent_id.clone())
            .with_visibility("public");

        assert_eq!(event.outcome, Some(serde_json::json!({"result": "ok"})));
        assert_eq!(event.regulation, Some(serde_json::json!({"adj": 0.5})));
        assert_eq!(event.parent_event, Some(parent_id));
        assert_eq!(event.visibility, "public");
    }

    // REQ: types-event-003 — SpanNamespace::parse() accepts short and full forms
    #[test]
    fn spannamespace_parse_accepts_short_and_full_forms() {
        let full = SpanNamespace::parse("cns.tool");
        assert!(full.is_some());
        assert_eq!(full.unwrap().as_str(), "cns.tool");

        let short = SpanNamespace::parse("tool");
        assert!(short.is_some());
        assert_eq!(short.unwrap().as_str(), "cns.tool");
    }

    // REQ: types-event-004 — SpanNamespace::parse() rejects invalid namespaces
    #[test]
    fn spannamespace_parse_rejects_invalid() {
        assert!(SpanNamespace::parse("cns.nonexistent").is_none());
        assert!(SpanNamespace::parse("invalid").is_none());
        assert!(SpanNamespace::parse("").is_none());
    }

    // REQ: types-event-005 — SpanNamespace::category() classifies correctly
    #[test]
    fn spannamespace_category_classifies_correctly() {
        assert_eq!(
            SpanNamespace::new("cns.variety").category(),
            SpanCategory::Cybernetics
        );
        assert_eq!(
            SpanNamespace::new("cns.gas").category(),
            SpanCategory::Cybernetics
        );
        assert_eq!(
            SpanNamespace::new("cns.curation").category(),
            SpanCategory::Curation
        );
        assert_eq!(
            SpanNamespace::new("cns.inference").category(),
            SpanCategory::Inference
        );
        assert_eq!(
            SpanNamespace::new("cns.agent_pod").category(),
            SpanCategory::Episodic
        );
        assert_eq!(
            SpanNamespace::new("cns.tool").category(),
            SpanCategory::Unknown
        );
    }

    // REQ: types-event-006 — SpanCategory::from_short_name() parses correctly
    #[test]
    fn spancategory_from_short_name_parses_correctly() {
        assert_eq!(
            SpanCategory::from_short_name("variety"),
            SpanCategory::Cybernetics
        );
        assert_eq!(
            SpanCategory::from_short_name("curation"),
            SpanCategory::Curation
        );
        assert_eq!(
            SpanCategory::from_short_name("inference"),
            SpanCategory::Inference
        );
        assert_eq!(
            SpanCategory::from_short_name("agent_pod"),
            SpanCategory::Episodic
        );
        assert_eq!(
            SpanCategory::from_short_name("unknown_ns"),
            SpanCategory::Unknown
        );
    }

    // REQ: types-event-007 — Phase::from_str() backward-compatible parsing
    #[test]
    fn phase_from_str_backward_compatible() {
        // New names
        assert_eq!(Phase::from_str("sense"), Phase::Sense);
        assert_eq!(Phase::from_str("compute"), Phase::Compute);
        assert_eq!(Phase::from_str("compare"), Phase::Compare);
        assert_eq!(Phase::from_str("act"), Phase::Act);
        // Backward-compatible old names
        assert_eq!(Phase::from_str("observe"), Phase::Sense);
        assert_eq!(Phase::from_str("regulate"), Phase::Compute);
        assert_eq!(Phase::from_str("outcome"), Phase::Act);
        // Unknown falls back to Sense
        assert_eq!(Phase::from_str("unknown"), Phase::Sense);
    }

    // REQ: types-event-008 — Span::new() constructs correct full path
    #[test]
    fn span_new_constructs_full_path() {
        let ns = SpanNamespace::new("cns.tool");
        let span = Span::new(ns, "invoked");
        assert_eq!(span.as_str(), "cns.tool.invoked");
    }
}
