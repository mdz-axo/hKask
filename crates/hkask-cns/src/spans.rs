//! CNS span emission with full categorization

use hkask_types::{NuEvent, NuEventSink, Span, WebID};
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

/// CNS span categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    /// Review queue (submitted, reviewed, approved, rejected)
    Review,
    /// Spec primitive (spec validation, compliance, verification)
    Spec,
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
            SpanCategory::Review => "cns.review",
            SpanCategory::Spec => "cns.spec",
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
            "review" | "cns.review" => Some(SpanCategory::Review),
            "spec" | "cns.spec" => Some(SpanCategory::Spec),
            _ => None,
        }
    }
}

/// CNS span emitter
pub struct SpanEmitter {
    observer_webid: WebID,
    sink: Option<Box<dyn NuEventSink>>,
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
        let event = NuEvent::new(
            self.observer_webid,
            span,
            hkask_types::Phase::Observe,
            observation,
            0,
        );

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
        let category = span_to_category(&span);
        if self.allowed_categories.contains(&category) {
            self.emitter.emit(span, observation);
            Ok(())
        } else {
            // Emit sovereignty boundary violation via the emitter itself
            // (sovereignty violations are always emitted, regardless of scope)
            self.emitter.emit_sovereignty_alert(
                "boundary_violation",
                serde_json::json!({
                    "observer": self.observer_webid.to_string(),
                    "attempted_category": category.as_str(),
                    "allowed_categories": self.allowed_categories.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
                    "violation_type": "span_scope_violation"
                }),
            );
            Err(SpanViolation {
                observer_webid: self.observer_webid,
                attempted_category: category,
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

/// Map a Span to its SpanCategory
fn span_to_category(span: &Span) -> SpanCategory {
    match span {
        Span::Connector(_) => SpanCategory::Connector,
        Span::Pipeline(_) => SpanCategory::Pipeline,
        Span::Tool(_) => SpanCategory::Tool,
        Span::Prompt(_) => SpanCategory::Prompt,
        Span::AgentPod(_) => SpanCategory::AgentPod,
        Span::Energy(_) => SpanCategory::Energy,
        Span::Review(_) => SpanCategory::Review,
        Span::Sovereignty(_) => SpanCategory::Sovereignty,
        Span::Goal(_) => SpanCategory::Goal,
        Span::Spec(_) => SpanCategory::Spec,
    }
}

impl CnsEmit for SpanScope {
    fn emit_event(&self, span: &str, _phase: &str, observation: &Value, confidence: f64) {
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
        } else {
            SpanCategory::AgentPod // default fallback
        };

        if self.allowed_categories.contains(&category) {
            // Determine the Span variant from the category and span string
            let span_variant = match category {
                SpanCategory::Connector => Span::Connector(span.to_string()),
                SpanCategory::Pipeline => Span::Pipeline(span.to_string()),
                SpanCategory::Tool => Span::Tool(span.to_string()),
                SpanCategory::Prompt => Span::Prompt(span.to_string()),
                SpanCategory::AgentPod => Span::AgentPod(span.to_string()),
                SpanCategory::Energy => Span::Energy(span.to_string()),
                SpanCategory::Review => Span::Review(span.to_string()),
                SpanCategory::Sovereignty => Span::Sovereignty(span.to_string()),
                SpanCategory::Goal => Span::Goal(span.to_string()),
                SpanCategory::Spec => Span::Spec(span.to_string()),
            };
            self.emitter.emit(span_variant, observation.clone());
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
                }),
            );
        }
    }
}

/// Get the allowed span categories for a given R7 bot
pub fn span_scope_for_bot(bot_name: &str) -> HashSet<SpanCategory> {
    match bot_name {
        "cns-curator-bot" => HashSet::from([SpanCategory::AgentPod, SpanCategory::Energy]),
        "memory-curator-bot" => HashSet::from([SpanCategory::Pipeline]),
        "inference-curator-bot" => HashSet::from([SpanCategory::Connector, SpanCategory::Energy]),
        "mcp-dispatch-bot" => HashSet::from([SpanCategory::Tool]),
        "ensemble-curator-bot" => HashSet::from([SpanCategory::AgentPod]),
        "git-curator-bot" => HashSet::from([SpanCategory::Tool]),
        "registry-dispatch-bot" => HashSet::from([SpanCategory::Prompt]),
        "kata-bot" => HashSet::from([SpanCategory::Prompt]),
        "Curator" => HashSet::from([
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
        ]),
        _ => HashSet::from([SpanCategory::AgentPod]), // minimal default
    }
}
