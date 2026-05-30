//! CNS span emission with full categorization

use hkask_types::{NuEvent, NuEventSink, Phase, Span, SpanCategory, WebID};
use serde_json::Value;
use std::collections::HashSet;
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

/// CNS span emitter
pub struct SpanEmitter {
    observer_webid: WebID,
    sink: Option<Box<dyn NuEventSink>>,
}

impl std::fmt::Debug for SpanEmitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpanEmitter")
            .field("observer_webid", &self.observer_webid)
            .field("sink", &self.sink.as_ref().map(|_| "NuEventSink"))
            .finish()
    }
}

impl Clone for SpanEmitter {
    fn clone(&self) -> Self {
        // Sinks are not cloneable (trait object), so the clone drops the sink.
        // This is intentional — sinks are optional and the clone preserves
        // the observer identity for span emission.
        Self {
            observer_webid: self.observer_webid,
            sink: None,
        }
    }
}

impl Default for SpanEmitter {
    fn default() -> Self {
        Self {
            observer_webid: WebID::new(),
            sink: None,
        }
    }
}

impl SpanEmitter {
    pub fn new(observer_webid: WebID) -> Self {
        Self {
            observer_webid,
            sink: None,
        }
    }

    pub fn with_sink(mut self, sink: Box<dyn NuEventSink>) -> Self {
        self.sink = Some(sink);
        self
    }

    /// Emit a CNS span event
    pub fn emit(&self, span: Span, observation: Value) {
        self.emit_with_phase(span, Phase::Observe, observation);
    }

    /// Emit a CNS span event with an explicit phase
    pub fn emit_with_phase(&self, span: Span, phase: Phase, observation: Value) {
        let event = NuEvent::new(self.observer_webid, span, phase, observation, 0);

        if let Some(sink) = &self.sink
            && let Err(e) = sink.persist(&event)
        {
            tracing::warn!(target: "cns", error = %e, "Failed to persist CNS event");
        }

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
        self.emit(Span::connector(action), observation);
    }

    /// Emit pipeline span (multi-stage processing)
    pub fn emit_pipeline(&self, stage: &str, observation: Value) {
        self.emit(Span::pipeline(stage), observation);
    }

    /// Emit tool span (tool invocation)
    pub fn emit_tool(&self, tool_name: &str, observation: Value) {
        self.emit(Span::tool(tool_name), observation);
    }

    /// Emit prompt span (template rendering/execution)
    pub fn emit_prompt(&self, phase: &str, observation: Value) {
        self.emit(Span::prompt(phase), observation);
    }

    /// Emit agent pod span (lifecycle event)
    pub fn emit_agent_pod(&self, lifecycle_event: &str, observation: Value) {
        self.emit(Span::agent_pod(lifecycle_event), observation);
    }

    /// Emit energy span (cost tracking)
    pub fn emit_energy(&self, energy_event: &str, observation: Value) {
        self.emit(Span::energy(energy_event), observation);
    }

    /// Emit sovereignty span (boundary, acquisition, kill-zone)
    pub fn emit_sovereignty(&self, sovereignty_event: &str, observation: Value) {
        self.emit(Span::sovereignty(sovereignty_event), observation);
    }

    /// Emit sovereignty alert (kill-zone detected)
    pub fn emit_sovereignty_alert(&self, alert_type: &str, observation: Value) {
        self.emit(
            Span::sovereignty(&format!("alert.{}", alert_type)),
            observation,
        );
    }

    /// Emit goal span (lifecycle event)
    pub fn emit_goal(&self, goal_event: &str, observation: Value) {
        self.emit(Span::goal(goal_event), observation);
    }

    /// Emit goal alert (variety deficit, algedonic)
    pub fn emit_goal_alert(&self, alert_type: &str, observation: Value) {
        self.emit(Span::goal(&format!("alert.{}", alert_type)), observation);
    }
}

/// Span scope — OCAP-enforced allowed span categories per bot
///
/// When a bot's pod is created, its SpanEmitter is constructed with a scoped
/// set of allowed SpanCategory values derived from its manifest's capabilities
/// and responsibilities. If a bot attempts to emit a span outside its allowed
/// categories, the emission is logged as a sovereignty boundary violation and
/// the SovereigntyObserver is notified.
pub struct SpanScope {
    emitter: SpanEmitter,
    allowed_categories: HashSet<SpanCategory>,
    observer_webid: WebID,
}

impl SpanScope {
    /// Create a new scoped span emitter
    pub fn new(
        emitter: SpanEmitter,
        allowed_categories: HashSet<SpanCategory>,
        observer_webid: WebID,
    ) -> Self {
        Self {
            emitter,
            allowed_categories,
            observer_webid,
        }
    }

    /// Get the allowed categories
    pub fn allowed_categories(&self) -> &HashSet<SpanCategory> {
        &self.allowed_categories
    }

    /// Check if a category is allowed
    pub fn is_allowed(&self, category: &SpanCategory) -> bool {
        self.allowed_categories.contains(category)
    }

    /// Emit a span, checking scope first
    /// Returns Ok(()) if allowed, Err with the violation details if not
    pub fn emit_scoped(&self, span: Span, observation: Value) -> Result<(), SpanViolation> {
        if self.allowed_categories.contains(&span.category) {
            self.emitter.emit(span, observation);
            Ok(())
        } else {
            // Emit sovereignty boundary violation via the emitter itself
            // (sovereignty violations are always emitted, regardless of scope)
            self.emitter.emit_sovereignty_alert(
                "boundary_violation",
                serde_json::json!({
                    "observer": self.observer_webid.to_string(),
                    "attempted_category": span.category.as_str(),
                    "allowed_categories": self.allowed_categories.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
                    "violation_type": "span_scope_violation"
                }),
            );
            Err(SpanViolation {
                observer_webid: self.observer_webid,
                attempted_category: span.category,
                allowed_categories: self.allowed_categories.clone(),
            })
        }
    }
}

/// Span violation — emitted when a bot attempts to emit a span outside its scope
#[derive(Debug, Clone)]
pub struct SpanViolation {
    pub observer_webid: WebID,
    pub attempted_category: SpanCategory,
    pub allowed_categories: HashSet<SpanCategory>,
}

impl std::fmt::Display for SpanViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Span scope violation: observer {:?} attempted to emit {:?} span, allowed: {:?}",
            self.observer_webid, self.attempted_category, self.allowed_categories
        )
    }
}

/// Get the allowed span categories for a given domain
///
/// Each domain maps to the CNS span categories its templates/manifests
/// are expected to emit. The mapping is derived from the domain's
/// responsibilities declared in its YAML manifest.
pub fn span_scope_for_domain(domain: &str) -> HashSet<SpanCategory> {
    match domain {
        "storage" => HashSet::from([SpanCategory::Pipeline, SpanCategory::Energy]),
        "memory" => HashSet::from([SpanCategory::Pipeline, SpanCategory::Prompt]),
        "cns" => HashSet::from([
            SpanCategory::AgentPod,
            SpanCategory::Energy,
            SpanCategory::Variety,
        ]),
        "templates" => HashSet::from([SpanCategory::Prompt, SpanCategory::Template]),
        "registry" => HashSet::from([SpanCategory::Prompt, SpanCategory::Template]),
        "agents" => HashSet::from([SpanCategory::AgentPod]),
        "ensemble" => HashSet::from([SpanCategory::AgentPod, SpanCategory::Prompt]),
        "kata" => HashSet::from([SpanCategory::Prompt, SpanCategory::Goal]),
        "mcp" => HashSet::from([SpanCategory::Tool]),
        "inference" => HashSet::from([SpanCategory::Connector, SpanCategory::Energy]),
        "git" => HashSet::from([SpanCategory::Tool]),
        "web" => HashSet::from([SpanCategory::Connector, SpanCategory::Tool]),
        "condenser" => HashSet::from([SpanCategory::Connector, SpanCategory::Pipeline]),
        "github" => HashSet::from([SpanCategory::Tool]),
        "gml" => HashSet::from([SpanCategory::Prompt, SpanCategory::Pipeline]),
        "spec" => HashSet::from([SpanCategory::Spec, SpanCategory::Goal]),
        "fmp" => HashSet::from([SpanCategory::Tool]),
        "telnyx" => HashSet::from([SpanCategory::Connector]),
        "fal" => HashSet::from([SpanCategory::Connector]),
        "rss-reader" => HashSet::from([SpanCategory::Connector]),
        "cli" => HashSet::from([SpanCategory::Prompt, SpanCategory::Tool]),
        "api" => HashSet::from([SpanCategory::Prompt, SpanCategory::Tool]),
        unknown => {
            tracing::warn!(
                target: "cns.spans",
                domain = %unknown,
                "Unknown domain - using minimal AgentPod span scope"
            );
            HashSet::from([SpanCategory::AgentPod])
        }
    }
}

/// Get the allowed span categories for an R7 bot by unioning all owned domains
pub fn span_scope_for_r7_bot(bot: &hkask_types::R7BotIdentity) -> HashSet<SpanCategory> {
    let mut scope = HashSet::new();
    for domain in &bot.domains {
        scope.extend(span_scope_for_domain(domain));
    }
    if scope.is_empty() {
        scope.insert(SpanCategory::AgentPod);
    }
    scope
}

/// Curator has full span scope — all categories
pub fn curator_span_scope() -> HashSet<SpanCategory> {
    HashSet::from([
        SpanCategory::AgentPod,
        SpanCategory::Energy,
        SpanCategory::Connector,
        SpanCategory::Pipeline,
        SpanCategory::Tool,
        SpanCategory::Prompt,
        SpanCategory::Goal,
        SpanCategory::Sovereignty,
        SpanCategory::Spec,
        SpanCategory::Review,
        SpanCategory::Template,
        SpanCategory::Curation,
        SpanCategory::Variety,
        SpanCategory::KillZone,
    ])
}

impl CnsEmit for SpanScope {
    fn emit_event(&self, span: &str, phase: &str, observation: &Value, confidence: f64) {
        // Parse the span string to determine category
        let category = if span.starts_with("cns.connector") {
            SpanCategory::Connector
        } else if span.starts_with("cns.pipeline") {
            SpanCategory::Pipeline
        } else if span.starts_with("cns.tool") {
            SpanCategory::Tool
        } else if span.starts_with("cns.prompt") {
            SpanCategory::Prompt
        } else if span.starts_with("cns.agent_pod") {
            SpanCategory::AgentPod
        } else if span.starts_with("cns.energy") {
            SpanCategory::Energy
        } else if span.starts_with("cns.review") {
            SpanCategory::Review
        } else if span.starts_with("cns.sovereignty") {
            SpanCategory::Sovereignty
        } else if span.starts_with("cns.goal") {
            SpanCategory::Goal
        } else if span.starts_with("cns.spec") {
            SpanCategory::Spec
        } else if span.starts_with("cns.template") {
            SpanCategory::Template
        } else if span.starts_with("cns.curation") {
            SpanCategory::Curation
        } else if span.starts_with("cns.variety") {
            SpanCategory::Variety
        } else if span.starts_with("cns.killzone") {
            SpanCategory::KillZone
        } else {
            SpanCategory::AgentPod // default fallback
        };

        // Parse the phase
        let parsed_phase = Phase::from_str(phase);

        if self.allowed_categories.contains(&category) {
            // Construct the Span from category + path
            let span_event = Span {
                category,
                path: span.to_string(),
            };
            self.emitter
                .emit_with_phase(span_event, parsed_phase, observation.clone());
        } else {
            // Emit sovereignty boundary violation
            self.emitter.emit_sovereignty_alert(
                "boundary_violation",
                serde_json::json!({
                    "observer": self.observer_webid.to_string(),
                    "attempted_span": span,
                    "attempted_category": category.as_str(),
                    "allowed_categories": self.allowed_categories.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
                    "violation_type": "span_scope_violation",
                    "original_confidence": confidence,
                    "original_phase": phase,
                }),
            );
        }
    }
}
