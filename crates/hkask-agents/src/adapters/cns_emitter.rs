//! CNS Emitter Adapter
//!
//! Concrete implementation of CnsEmit using hkask-cns crate.

use hkask_cns::CnsEmit;
use hkask_cns::spans::SpanEmitter;
use hkask_types::{Span, WebID};

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
        _phase: &str,
        observation: &serde_json::Value,
        _confidence: f64,
    ) {
        let span = parse_span(span);
        self.emitter.emit(span, observation.clone());
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
    } else {
        Span::agent_pod(s)
    }
}
