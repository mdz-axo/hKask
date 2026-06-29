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
    pub phase: CyclePhase,
    pub observation: Value,
    pub regulation: Option<Value>,
    pub outcome: Option<Value>,
    pub recursion_depth: u8,
    pub parent_event: Option<EventID>,
    pub visibility: String,
}

impl NuEvent {
    /// Create a new NuEvent.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  observer is valid, span is valid, phase is valid
    /// post: returns NuEvent
    pub fn new(
        observer_webid: WebID,
        span: Span,
        phase: CyclePhase,
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  outcome is a valid serde_json::Value
    /// post: returns self with outcome set to Some(outcome)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_outcome(mut self, outcome: Value) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  regulation is a valid serde_json::Value
    /// post: returns self with regulation set to Some(regulation)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_regulation(mut self, regulation: Value) -> Self {
        self.regulation = Some(regulation);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  parent is a valid EventID
    /// post: returns self with parent_event set to Some(parent)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_parent(mut self, parent: EventID) -> Self {
        self.parent_event = Some(parent);
        self
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  visibility is a non-empty string (e.g. "private", "public")
    /// post: returns self with visibility set to visibility.to_string()
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

/// Canonical CNS span namespaces — mirrors `CnsSpan::as_str()` output plus namespaces
/// used by `SpanKind` (e.g. `cns.variety`).
const CANONICAL_NAMESPACES: &[&str] = &[
    "cns.acp.ide.connection_state",
    "cns.acp.replicant.memory_size",
    "cns.agent_pod",
    "cns.algedonic",
    "cns.architecture.seam.coverage",
    "cns.architecture.seam.drift",
    "cns.chat",
    "cns.consolidation",
    "cns.contract.accepted",
    "cns.contract.coverage",
    "cns.contract.proposed",
    "cns.contract.rejected",
    "cns.contract.violated",
    "cns.curation",
    "cns.cybernetics",
    "cns.cybernetics.backpressure",
    "cns.gas",
    "cns.inference",
    "cns.media",
    "cns.memory",
    "cns.memory.encode",
    "cns.outcome",
    "cns.sovereignty",
    "cns.spec",
    "cns.tool",
    "cns.tool.communication",
    "cns.tool.condenser",
    "cns.tool.kanban",
    "cns.tool.media",
    "cns.tool.registry",
    "cns.tool.replica",
    "cns.tool.research",
    "cns.tool.training",
    "cns.tool.wallet",
    "cns.tool.web_search",
    "cns.variety",
    "cns.communication.message",
    "cns.communication.thread",
    "cns.communication.agent",
    "cns.communication.listener",
    "cns.consent",
    "cns.wallet.balance",
    "cns.wallet.chain_error",
    "cns.wallet.conversion",
    "cns.wallet.deposit",
    "cns.wallet.deposit_shielded",
    "cns.wallet.key_exhausted",
    "cns.wallet.key_expired",
    "cns.wallet.key_issued",
    "cns.wallet.key_revoked",
    "cns.wallet.withdrawal",
];

impl SpanNamespace {
    /// Create a validated span namespace. Panics if the namespace is not canonical.
    /// Use `from_str` for fallible construction.
    /// Create a new SpanNamespace.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  namespace is non-empty
    /// post: returns SpanNamespace
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
    /// Parse a SpanNamespace from string.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns Some(SpanNamespace) if valid, None otherwise
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

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid SpanNamespace (canonical)
    /// post: returns the full namespace string (e.g. "cns.tool")
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid SpanNamespace (canonical, starts with "cns.")
    /// post: returns the short name after the "cns." prefix (e.g. "tool")
    pub fn short_name(&self) -> &str {
        &self.0[4..] // Skip "cns."
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid SpanNamespace (canonical)
    /// post: returns the SpanCategory for this namespace; unknown prefixes return SpanCategory::Unknown
    ///
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
            "wallet" => SpanCategory::Wallet,
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
    /// `cns.wallet*` — wallet operations (balance, keys, deposits, withdrawals).
    Wallet,
    /// Any other namespace. Callers decide the fallback policy.
    Unknown,
}

impl SpanCategory {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  s is a short_name() string (e.g. "variety", "variety.sensor")
    /// post: returns the matching SpanCategory; unrecognised prefixes return SpanCategory::Unknown
    pub fn from_short_name(s: &str) -> Self {
        let prefix = s.split('.').next().unwrap_or(s);
        match prefix {
            "variety" | "gas" => Self::Cybernetics,
            "curation" | "spec" => Self::Curation,
            "inference" => Self::Inference,
            "agent_pod" | "connector" => Self::Episodic,
            "wallet" => Self::Wallet,
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
            SpanCategory::Wallet => "wallet",
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

// ── CnsSpan ↔ SpanNamespace bridge (R5 migration) ──────────────────────

impl From<crate::cns::CnsSpan> for SpanNamespace {
    /// \[NORMATIVE\] Convert a typed `CnsSpan` to a `SpanNamespace`.
    /// The Display output of `CnsSpan` is the canonical namespace string,
    /// which is always valid for `SpanNamespace` construction (P8 — Semantic Grounding).
    fn from(span: crate::cns::CnsSpan) -> Self {
        SpanNamespace(span.to_string())
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
    /// Create a new Span.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  namespace is valid, path is non-empty
    /// post: returns Span
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

    /// Create a span from a typed `SpanKind` variant.
    ///
    /// Eliminates string typos at construction sites for the most common
    /// span paths. Each variant maps to a canonical (namespace, path) pair.
    /// Create a Span from a SpanKind.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  kind is valid
    /// post: returns Span with canonical namespace and path
    pub fn from_kind(kind: SpanKind) -> Self {
        let (ns, local_path) = kind.namespace_and_path();
        Span::new(SpanNamespace::new(ns), local_path)
    }
}

/// Typed span kind — canonical (namespace, path) pairs for common spans.
///
/// Use `Span::from_kind()` to construct spans without string literals,
/// reducing the risk of typos in span paths at construction sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpanKind {
    // ── Tool spans (cns.tool.*) ──
    /// Tool invocation started: `cns.tool.invoked`
    ToolInvoked,
    /// Tool invocation completed: `cns.tool.completed`
    ToolCompleted,
    /// Tool invocation errored: `cns.tool.error`
    ToolError,

    // ── Gas/energy spans (cns.gas.*) ──
    /// Gas reserved for an operation: `cns.gas.reserved`
    GasReserved,
    /// Gas settled after an operation: `cns.gas.settled`
    GasSettled,
    /// Gas budget depleted: `cns.gas.depleted`
    GasDepleted,

    // ── Curation spans (cns.curation.*) ──
    /// Curation directive acknowledged: `cns.curation.directive_acknowledged`
    CurationDirectiveAcknowledged,
    /// Curation escalation received: `cns.curation.escalation`
    CurationEscalation,

    // ── Agent pod spans (cns.agent_pod.*) ──
    /// Agent pod registered: `cns.agent_pod.registered`
    AgentPodRegistered,
    /// Agent pod activated: `cns.agent_pod.activated`
    AgentPodActivated,
    /// Agent pod deactivated: `cns.agent_pod.deactivated`
    AgentPodDeactivated,

    // ── Variety spans (cns.variety.*) ──
    /// Algedonic alert emitted: `cns.variety.algedonic_alert`
    VarietyAlgedonicAlert,

    // ── Wallet spans (cns.wallet.*) ──
    /// Deposit credited to wallet: `cns.wallet.deposit_credited`
    DepositCredited,
}

impl SpanKind {
    /// Return the (namespace, local_path) pair for this span kind.
    fn namespace_and_path(&self) -> (&'static str, &'static str) {
        match self {
            SpanKind::ToolInvoked => ("cns.tool", "invoked"),
            SpanKind::ToolCompleted => ("cns.tool", "completed"),
            SpanKind::ToolError => ("cns.tool", "error"),
            SpanKind::GasReserved => ("cns.gas", "reserved"),
            SpanKind::GasSettled => ("cns.gas", "settled"),
            SpanKind::GasDepleted => ("cns.gas", "depleted"),
            SpanKind::CurationDirectiveAcknowledged => ("cns.curation", "directive_acknowledged"),
            SpanKind::CurationEscalation => ("cns.curation", "escalation"),
            SpanKind::AgentPodRegistered => ("cns.agent_pod", "registered"),
            SpanKind::AgentPodActivated => ("cns.agent_pod", "activated"),
            SpanKind::AgentPodDeactivated => ("cns.agent_pod", "deactivated"),
            SpanKind::VarietyAlgedonicAlert => ("cns.variety", "algedonic_alert"),
            SpanKind::DepositCredited => ("cns.wallet", "deposit_credited"),
        }
    }
}

/// Phase of the cybernetic cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CyclePhase {
    Sense,
    Compute,
    Compare,
    Act,
}

impl CyclePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            CyclePhase::Sense => "sense",
            CyclePhase::Compute => "compute",
            CyclePhase::Compare => "compare",
            CyclePhase::Act => "act",
        }
    }

    /// Parse a phase string into a CyclePhase variant.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "sense" | "Sense" => CyclePhase::Sense,
            "compute" | "Compute" => CyclePhase::Compute,
            "compare" | "Compare" => CyclePhase::Compare,
            "act" | "Act" => CyclePhase::Act,
            _ => CyclePhase::Sense,
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

    #[test]
    fn nuevent_new_sets_correct_defaults() {
        let webid = WebID::from_persona(b"test-agent");
        let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
        let obs = serde_json::json!({"key": "value"});

        let event = NuEvent::new(webid, span, CyclePhase::Sense, obs.clone(), 0);

        assert_eq!(event.observer_webid, webid);
        assert_eq!(event.phase, CyclePhase::Sense);
        assert_eq!(event.observation, obs);
        assert_eq!(event.recursion_depth, 0);
        assert_eq!(event.visibility, "private");
        assert!(event.regulation.is_none());
        assert!(event.outcome.is_none());
        assert!(event.parent_event.is_none());
    }

    #[test]
    fn nuevent_builder_chain_sets_fields() {
        let webid = WebID::from_persona(b"test-agent");
        let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
        let parent_id = crate::id::EventID::new();

        let event = NuEvent::new(webid, span, CyclePhase::Act, serde_json::json!({}), 1)
            .with_outcome(serde_json::json!({"result": "ok"}))
            .with_regulation(serde_json::json!({"adj": 0.5}))
            .with_parent(parent_id)
            .with_visibility("public");

        assert_eq!(event.outcome, Some(serde_json::json!({"result": "ok"})));
        assert_eq!(event.regulation, Some(serde_json::json!({"adj": 0.5})));
        assert_eq!(event.parent_event, Some(parent_id));
        assert_eq!(event.visibility, "public");
    }

    #[test]
    fn spannamespace_parse_accepts_short_and_full_forms() {
        let full = SpanNamespace::parse("cns.tool");
        assert!(full.is_some());
        assert_eq!(full.unwrap().as_str(), "cns.tool");

        let short = SpanNamespace::parse("tool");
        assert!(short.is_some());
        assert_eq!(short.unwrap().as_str(), "cns.tool");
    }

    #[test]
    fn spannamespace_parse_rejects_invalid() {
        assert!(SpanNamespace::parse("cns.nonexistent").is_none());
        assert!(SpanNamespace::parse("invalid").is_none());
        assert!(SpanNamespace::parse("").is_none());
    }

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

    #[test]
    fn phase_from_str() {
        assert_eq!(CyclePhase::from_str("sense"), CyclePhase::Sense);
        assert_eq!(CyclePhase::from_str("compute"), CyclePhase::Compute);
        assert_eq!(CyclePhase::from_str("compare"), CyclePhase::Compare);
        assert_eq!(CyclePhase::from_str("act"), CyclePhase::Act);
        // Unknown falls back to Sense
        assert_eq!(CyclePhase::from_str("unknown"), CyclePhase::Sense);
    }

    #[test]
    fn span_new_constructs_full_path() {
        let ns = SpanNamespace::new("cns.tool");
        let span = Span::new(ns, "invoked");
        assert_eq!(span.as_str(), "cns.tool.invoked");
    }

    #[test]
    fn span_from_kind_produces_correct_paths() {
        assert_eq!(
            Span::from_kind(SpanKind::ToolInvoked).as_str(),
            "cns.tool.invoked"
        );
        assert_eq!(
            Span::from_kind(SpanKind::ToolCompleted).as_str(),
            "cns.tool.completed"
        );
        assert_eq!(
            Span::from_kind(SpanKind::GasReserved).as_str(),
            "cns.gas.reserved"
        );
        assert_eq!(
            Span::from_kind(SpanKind::GasSettled).as_str(),
            "cns.gas.settled"
        );
        assert_eq!(
            Span::from_kind(SpanKind::CurationDirectiveAcknowledged).as_str(),
            "cns.curation.directive_acknowledged"
        );
        assert_eq!(
            Span::from_kind(SpanKind::AgentPodRegistered).as_str(),
            "cns.agent_pod.registered"
        );
        assert_eq!(
            Span::from_kind(SpanKind::VarietyAlgedonicAlert).as_str(),
            "cns.variety.algedonic_alert"
        );
    }

    // ── Property tests (proptest) ───────────────────────────────────────────

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        fn canonical_namespace_str() -> impl Strategy<Value = String> {
            (0..CANONICAL_NAMESPACES.len()).prop_map(|i| CANONICAL_NAMESPACES[i].to_string())
        }

        proptest! {
            #[test]
            fn all_canonical_namespaces_parse(
                ns in canonical_namespace_str()
            ) {
                let parsed = SpanNamespace::parse(&ns);
                prop_assert!(parsed.is_some(), "canonical namespace should parse: {ns}");
                let span_ns = parsed.unwrap();
                prop_assert_eq!(span_ns.as_str(), ns.as_str());
            }
        }

        // e.g., "tool" → parse() → as_str() == "cns.tool"
        proptest! {
            #[test]
            fn short_form_round_trip(
                ns in canonical_namespace_str()
            ) {
                let short = &ns[4..]; // strip "cns." prefix
                let parsed = SpanNamespace::parse(short);
                prop_assert!(parsed.is_some(), "short form should parse: {short}");
                let span_ns = parsed.unwrap();
                prop_assert_eq!(span_ns.as_str(), ns.as_str());
            }
        }

        proptest! {
            #[test]
            fn non_canonical_returns_none(
                input in "\\PC*"
            ) {
                prop_assume!(!CANONICAL_NAMESPACES.contains(&input.as_str()));
                let full = format!("cns.{input}");
                prop_assume!(!CANONICAL_NAMESPACES.contains(&full.as_str()));

                let result = SpanNamespace::parse(&input);
                prop_assert!(result.is_none(), "non-canonical should return None: {input}");
            }
        }

        proptest! {
            #[test]
            fn from_short_name_known_prefixes(
                prefix in prop_oneof![
                    Just("variety"), Just("gas"),
                    Just("curation"), Just("spec"),
                    Just("inference"),
                    Just("agent_pod"), Just("connector"),
                ]
            ) {
                let category = SpanCategory::from_short_name(prefix);
                prop_assert!(category != SpanCategory::Unknown,
                    "known prefix should not be Unknown: {prefix}");
            }
        }

        proptest! {
            #[test]
            fn from_short_name_unknown_prefix(
                prefix in "[a-z][a-z0-9_]*"
            ) {
                prop_assume!(prefix != "variety" && prefix != "gas"
                    && prefix != "curation" && prefix != "spec"
                    && prefix != "inference"
                    && prefix != "agent_pod" && prefix != "connector");
                let category = SpanCategory::from_short_name(&prefix);
                prop_assert!(category == SpanCategory::Unknown,
                    "unknown prefix should be Unknown: {prefix}");
            }
        }

        proptest! {
            #[test]
            fn namespace_category_invariant(
                ns in canonical_namespace_str()
            ) {
                let parsed = SpanNamespace::parse(&ns).unwrap();
                let category = parsed.category();
                let short = parsed.short_name();
                let prefix = short.split('.').next().unwrap_or(short);

                let expected = match prefix {
                    "variety" | "gas" => SpanCategory::Cybernetics,
                    "curation" | "spec" => SpanCategory::Curation,
                    "inference" => SpanCategory::Inference,
                    "agent_pod" | "connector" => SpanCategory::Episodic,
                    "wallet" => SpanCategory::Wallet,
                    _ => SpanCategory::Unknown,
                };
                prop_assert!(category == expected,
                    "{ns}: expected {expected:?}, got {category:?}");
            }
        }
    }
}
