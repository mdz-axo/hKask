//! CNS Emitter Adapter
//!
//! Concrete implementation of CNSSpanPort using hkask-cns crate.

use crate::pod::CNSSpanPort;
use hkask_cns::spans::SpanEmitter;
use hkask_types::{Phase, Span, WebID};

/// CNS Emitter Adapter — Concrete implementation for span emission
pub struct CnsEmitterAdapter {
    emitter: SpanEmitter,
    /// Observer WebID (reserved for future contextual emission)
    #[allow(dead_code)]
    observer_webid: WebID,
}

impl CnsEmitterAdapter {
    pub fn new(observer_webid: WebID) -> Self {
        Self {
            emitter: SpanEmitter::new(observer_webid),
            observer_webid,
        }
    }

    pub fn from_emitter(emitter: SpanEmitter, observer_webid: WebID) -> Self {
        Self {
            emitter,
            observer_webid,
        }
    }
}

impl CNSSpanPort for CnsEmitterAdapter {
    fn emit_event(
        &self,
        span: &str,
        phase: &str,
        observation: &serde_json::Value,
        _confidence: f64,
    ) {
        let span = parse_span(span);
        let _phase = parse_phase(phase);

        self.emitter.emit(span, observation.clone());
    }
}

fn parse_span(s: &str) -> Span {
    // Parse span string to Span enum
    // Default to AgentPod for agent-related spans
    if s.starts_with("cns.tool") {
        Span::tool(s.strip_prefix("cns.tool.").unwrap_or(s))
    } else if s.starts_with("cns.prompt") {
        Span::prompt(s.strip_prefix("cns.prompt.").unwrap_or(s))
    } else if s.starts_with("cns.agent_pod") {
        Span::agent_pod(s.strip_prefix("cns.agent_pod.").unwrap_or(s))
    } else if s.starts_with("cns.connector") {
        Span::connector(s.strip_prefix("cns.connector.").unwrap_or(s))
    } else if s.starts_with("cns.pipeline") {
        Span::pipeline(s.strip_prefix("cns.pipeline.").unwrap_or(s))
    } else if s.starts_with("cns.energy") {
        Span::energy(s.strip_prefix("cns.energy.").unwrap_or(s))
    } else if s.starts_with("cns.review") {
        Span::review(s.strip_prefix("cns.review.").unwrap_or(s))
    } else {
        Span::agent_pod(s)
    }
}

fn parse_phase(s: &str) -> Phase {
    // Parse phase string to Phase enum
    match s.to_lowercase().as_str() {
        "observe" => Phase::Observe,
        "regulate" => Phase::Regulate,
        "outcome" => Phase::Outcome,
        _ => Phase::Observe,
    }
}
