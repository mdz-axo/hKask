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
    "cns.killzone",
    "cns.sovereignty",
    "cns.goal",
    "cns.spec",
    "cns.test",
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
            "variety" | "gas" | "killzone" => SpanCategory::Cybernetics,
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
    /// `cns.variety*`, `cns.gas*`, `cns.killzone*` — the cybernetics loop.
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
            "variety" | "gas" | "killzone" => Self::Cybernetics,
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
    use std::panic::{AssertUnwindSafe, catch_unwind};

    // ── SpanNamespace ──────────────────────────────────────────────

    #[test]
    // P8 invariant: every canonical namespace constructs successfully
    fn span_namespace_new_valid_namespaces() {
        for &ns in CANONICAL_NAMESPACES {
            let span = SpanNamespace::new(ns);
            assert_eq!(span.as_str(), ns);
        }
    }

    #[test]
    // P8 invariant: invalid namespace panics at construction
    fn span_namespace_new_invalid_namespace_panics() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            SpanNamespace::new("invalid.namespace");
        }));
        assert!(result.is_err());
    }

    #[test]
    // P8 invariant: short form 'tool' parses to cns.tool
    fn span_namespace_parse_short_form() {
        let ns = SpanNamespace::parse("tool");
        assert!(ns.is_some());
        assert_eq!(ns.unwrap().short_name(), "tool");
    }

    #[test]
    // P8 invariant: full form 'cns.tool' parses to cns.tool
    fn span_namespace_parse_full_form() {
        let ns = SpanNamespace::parse("cns.tool");
        assert!(ns.is_some());
        assert_eq!(ns.unwrap().as_str(), "cns.tool");
    }

    #[test]
    // P8 invariant: invalid namespace returns None
    fn span_namespace_parse_invalid_returns_none() {
        assert!(SpanNamespace::parse("invalid").is_none());
    }

    #[test]
    // P8 invariant: from_str roundtrips for all canonical namespaces
    fn span_namespace_from_str_roundtrip() {
        for &ns in CANONICAL_NAMESPACES {
            let short = &ns[4..]; // strip "cns."
            let parsed: SpanNamespace = short.parse().expect("short form should parse");
            assert_eq!(parsed.as_str(), ns);

            let parsed_full: SpanNamespace = ns.parse().expect("full form should parse");
            assert_eq!(parsed_full.as_str(), ns);
        }
    }

    #[test]
    // P8 invariant: Display format equals the full namespace string
    fn span_namespace_display_matches_as_str() {
        for &ns in CANONICAL_NAMESPACES {
            let span = SpanNamespace::new(ns);
            assert_eq!(format!("{span}"), span.as_str());
        }
    }

    #[test]
    // P8 invariant: short_name() returns the part after 'cns.'
    fn span_namespace_short_name_skips_cns_prefix() {
        for &ns in CANONICAL_NAMESPACES {
            let span = SpanNamespace::new(ns);
            assert_eq!(span.short_name(), &ns[4..]);
        }
    }

    #[test]
    // P8 invariant: cns.test is a valid canonical namespace
    fn span_namespace_cns_test_is_valid() {
        let ns = SpanNamespace::new("cns.test");
        assert_eq!(ns.as_str(), "cns.test");
        let parsed = SpanNamespace::parse("test");
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap().as_str(), "cns.test");
    }

    // ── Phase ──────────────────────────────────────────────────────

    #[test]
    // P8 invariant: every Phase variant roundtrips through as_str() and from_str()
    fn phase_as_str_roundtrip() {
        for variant in [Phase::Sense, Phase::Compute, Phase::Compare, Phase::Act] {
            assert_eq!(Phase::from_str(variant.as_str()), variant);
        }
    }

    #[test]
    // P8 invariant: backward-compatible names (observe→Sense, regulate→Compute, outcome→Act) parse correctly
    fn phase_from_str_backward_compat() {
        assert_eq!(Phase::from_str("observe"), Phase::Sense);
        assert_eq!(Phase::from_str("regulate"), Phase::Compute);
        assert_eq!(Phase::from_str("outcome"), Phase::Act);
    }

    #[test]
    // P8 invariant: from_str handles mixed case
    fn phase_from_str_case_insensitive() {
        assert_eq!(Phase::from_str("Sense"), Phase::Sense);
        assert_eq!(Phase::from_str("Act"), Phase::Act);
        assert_eq!(Phase::from_str("Compute"), Phase::Compute);
        assert_eq!(Phase::from_str("Compare"), Phase::Compare);
    }

    #[test]
    // P8 invariant: unknown phase string defaults to Sense
    fn phase_from_str_unknown_defaults_to_sense() {
        assert_eq!(Phase::from_str("unknown"), Phase::Sense);
    }

    // ── Span ──────────────────────────────────────────────────────

    #[test]
    // P8 invariant: Span::new concatenates namespace and path
    fn span_new_constructs_full_path() {
        let ns = SpanNamespace::new("cns.tool");
        let span = Span::new(ns, "invoked");
        assert_eq!(span.path, "cns.tool.invoked");
    }

    // ── SpanCategory (F-SYN-009) ─────────────────────────────────

    #[test]
    // P8 invariant: every canonical namespace maps to a known category
    fn span_category_classifies_canonical_namespaces() {
        use std::collections::HashMap;
        // Map each canonical namespace to its expected category.
        // The dispatch is on the *prefix* of the short_name, so
        // namespaces with the same prefix share a category.
        let expectations: HashMap<&str, SpanCategory> = [
            ("cns.variety", SpanCategory::Cybernetics),
            ("cns.gas", SpanCategory::Cybernetics),
            ("cns.killzone", SpanCategory::Cybernetics),
            ("cns.curation", SpanCategory::Curation),
            ("cns.spec", SpanCategory::Curation),
            ("cns.inference", SpanCategory::Inference),
            ("cns.agent_pod", SpanCategory::Episodic),
            ("cns.connector", SpanCategory::Episodic),
        ]
        .iter()
        .copied()
        .collect();
        for (ns_str, expected) in &expectations {
            let ns = SpanNamespace::new(ns_str);
            assert_eq!(ns.category(), *expected, "namespace {ns_str}");
        }
    }

    #[test]
    // P8 invariant: canonical namespaces that are not in the dispatch
    // set return Unknown (the explicit fallback at the type level).
    fn span_category_unknown_for_unrelated_namespaces() {
        // Canonical but not in the dispatch set: tool, pipeline,
        // review, template, goal, test, sovereignty, prompt.
        for ns_str in [
            "cns.tool",
            "cns.prompt",
            "cns.pipeline",
            "cns.review",
            "cns.template",
            "cns.goal",
            "cns.test",
            "cns.sovereignty",
        ] {
            let ns = SpanNamespace::new(ns_str);
            assert_eq!(ns.category(), SpanCategory::Unknown, "namespace {ns_str}");
        }
    }

    #[test]
    // P8 invariant: SpanCategory::from_short_name handles the hierarchical
    // form (e.g. `variety.sensor` → Cybernetics) and the unknown
    // form (`unknown.foo` → Unknown).
    fn span_category_from_short_name_handles_hierarchy() {
        assert_eq!(
            SpanCategory::from_short_name("variety"),
            SpanCategory::Cybernetics
        );
        assert_eq!(
            SpanCategory::from_short_name("variety.sensor"),
            SpanCategory::Cybernetics
        );
        assert_eq!(
            SpanCategory::from_short_name("gas.depleted"),
            SpanCategory::Cybernetics
        );
        assert_eq!(
            SpanCategory::from_short_name("killzone"),
            SpanCategory::Cybernetics
        );
        assert_eq!(
            SpanCategory::from_short_name("curation"),
            SpanCategory::Curation
        );
        assert_eq!(
            SpanCategory::from_short_name("spec.drift"),
            SpanCategory::Curation
        );
        assert_eq!(
            SpanCategory::from_short_name("inference"),
            SpanCategory::Inference
        );
        assert_eq!(
            SpanCategory::from_short_name("agent_pod.launched"),
            SpanCategory::Episodic
        );
        assert_eq!(
            SpanCategory::from_short_name("connector"),
            SpanCategory::Episodic
        );
        assert_eq!(SpanCategory::from_short_name("tool"), SpanCategory::Unknown);
        assert_eq!(SpanCategory::from_short_name(""), SpanCategory::Unknown);
    }

    #[test]
    // P8 invariant: SpanCategory Display is the canonical name
    fn span_category_display_matches_variant() {
        assert_eq!(SpanCategory::Cybernetics.to_string(), "cybernetics");
        assert_eq!(SpanCategory::Curation.to_string(), "curation");
        assert_eq!(SpanCategory::Inference.to_string(), "inference");
        assert_eq!(SpanCategory::Episodic.to_string(), "episodic");
        assert_eq!(SpanCategory::Unknown.to_string(), "unknown");
    }
}
