//! CNS Emitter Adapter
//!
//! Concrete implementation of CnsEmit using hkask-cns crate.

use hkask_cns::CnsEmit;
use hkask_cns::spans::SpanEmitter;
use hkask_types::{Phase, Span, WebID};

/// CNS Emitter Adapter — Concrete implementation for span emission
pub struct CnsEmitterAdapter {
    emitter: SpanEmitter,
}

impl CnsEmitterAdapter {
    pub fn new(observer_webid: WebID) -> Self {
        Self {
            emitter: SpanEmitter::new(observer_webid),
        }
    }

    pub fn from_emitter(emitter: SpanEmitter) -> Self {
        Self { emitter }
    }
}

impl CnsEmit for CnsEmitterAdapter {
    fn emit_event(
        &self,
        span: &str,
        phase: &str,
        observation: &serde_json::Value,
        _confidence: f64,
    ) {
        let span = parse_span(span);
        let parsed_phase = Phase::from_str(phase);
        self.emitter
            .emit_with_phase(span, parsed_phase, observation.clone());
    }
}

fn parse_span(s: &str) -> Span {
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
    } else if s.starts_with("cns.template") {
        Span::template(s.strip_prefix("cns.template.").unwrap_or(s))
    } else if s.starts_with("cns.curation") {
        Span::curation(s.strip_prefix("cns.curation.").unwrap_or(s))
    } else if s.starts_with("cns.variety") {
        Span::variety(s.strip_prefix("cns.variety.").unwrap_or(s))
    } else if s.starts_with("cns.killzone") {
        Span::kill_zone(s.strip_prefix("cns.killzone.").unwrap_or(s))
    } else if s.starts_with("cns.sovereignty") {
        Span::sovereignty(s.strip_prefix("cns.sovereignty.").unwrap_or(s))
    } else if s.starts_with("cns.goal") {
        Span::goal(s.strip_prefix("cns.goal.").unwrap_or(s))
    } else if s.starts_with("cns.spec") {
        Span::spec(s.strip_prefix("cns.spec.").unwrap_or(s))
    } else {
        Span::agent_pod(s)
    }
}
