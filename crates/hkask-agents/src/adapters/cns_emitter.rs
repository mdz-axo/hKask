//! CNS Emitter Adapter
//!
//! Concrete implementation of CNSSpanPort using hkask-cns crate.

use crate::pod::CNSSpanPort;
use hkask_cns::spans::SpanEmitter;
use hkask_types::{NuEvent, Phase, Span, WebID};

/// CNS Emitter Adapter — Concrete implementation for span emission
pub struct CnsEmitterAdapter {
    emitter: SpanEmitter,
    observer_webid: WebID,
}

impl CnsEmitterAdapter {
    /// Create new CNS emitter adapter
    pub fn new(observer_webid: WebID) -> Self {
        Self {
            emitter: SpanEmitter::new(),
            observer_webid,
        }
    }

    /// Create from existing emitter
    pub fn from_emitter(emitter: SpanEmitter, observer_webid: WebID) -> Self {
        Self { emitter, observer_webid }
    }
}

impl CNSSpanPort for CnsEmitterAdapter {
    fn emit_event(&self, span: &str, phase: &str, observation: &serde_json::Value, confidence: f64) {
        // Create CNS span from string
        let span = parse_span(span);
        
        // Create phase from string
        let phase = parse_phase(phase);
        
        // Create and emit NuEvent
        let event = NuEvent::new(
            self.observer_webid.clone(),
            span,
            phase,
            observation.clone(),
            confidence,
        );
        
        self.emitter.emit(event);
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cns_emitter_adapter_new() {
        let webid = WebID::new();
        let adapter = CnsEmitterAdapter::new(webid);
        // Adapter created successfully
        assert!(true);
    }
    
    #[test]
    fn test_cns_emitter_emit_event() {
        let webid = WebID::new();
        let adapter = CnsEmitterAdapter::new(webid);
        
        let observation = serde_json::json!({"test": "event"});
        adapter.emit_event("cns.agent_pod.test", "observe", &observation, 1.0);
        
        // Event emitted (no return value to check)
        assert!(true);
    }
    
    #[test]
    fn test_parse_span_agent_pod() {
        let span = parse_span("cns.agent_pod.registered");
        assert!(matches!(span, Span::AgentPod(_)));
    }
    
    #[test]
    fn test_parse_phase() {
        assert_eq!(parse_phase("observe"), Phase::Observe);
        assert_eq!(parse_phase("regulate"), Phase::Regulate);
        assert_eq!(parse_phase("outcome"), Phase::Outcome);
        assert_eq!(parse_phase("unknown"), Phase::Observe);
    }
}