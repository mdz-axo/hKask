//! CNS Emitter Adapter
//!
//! Concrete implementation of CnsEmit using hkask-cns crate.

use hkask_cns::CnsEmit;
use hkask_cns::spans::SpanEmitter;
use hkask_types::event::{Span, SpanCategory};
use hkask_types::{Phase, WebID};

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
    let category = s
        .split_once('.')
        .and_then(|(prefix, _rest)| match prefix {
            "cns.tool" => Some(SpanCategory::Tool),
            "cns.prompt" => Some(SpanCategory::Prompt),
            "cns.agent_pod" => Some(SpanCategory::AgentPod),
            "cns.connector" => Some(SpanCategory::Connector),
            "cns.pipeline" => Some(SpanCategory::Pipeline),
            "cns.energy" => Some(SpanCategory::Energy),
            "cns.review" => Some(SpanCategory::Review),
            "cns.template" => Some(SpanCategory::Template),
            "cns.curation" => Some(SpanCategory::Curation),
            "cns.variety" => Some(SpanCategory::Variety),
            "cns.killzone" => Some(SpanCategory::KillZone),
            "cns.sovereignty" => Some(SpanCategory::Sovereignty),
            "cns.goal" => Some(SpanCategory::Goal),
            "cns.spec" => Some(SpanCategory::Spec),
            _ => None,
        })
        .unwrap_or(SpanCategory::AgentPod);

    Span {
        category,
        path: s.to_string(),
    }
}
