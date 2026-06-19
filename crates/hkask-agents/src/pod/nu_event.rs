//! Pod lifecycle NuEvent emission
//!
//! Bot operations emit NuEvents through CNS so that bot health and failure
//! are observable by Cybernetics (Loop 6) and reviewable by Curation (Loop 5).
//!
//! Per the loop architecture remediation: bots signal through CNS, not through
//! direct function calls to Curation. Cybernetics senses these through its
//! normal `sense()` cycle. Curation reviews them through the algedonic query
//! from the NuEvent store.

use hkask_types::NuEventSink;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
use hkask_types::id::WebID;

/// Emit a pod lifecycle NuEvent for the `cns.agent_pod` namespace.
///
/// This function creates and persists a NuEvent for pod lifecycle transitions.
/// It gracefully handles persistence failures by logging a warning.
#[allow(dead_code)]
pub fn emit_pod_event(
    sink: &dyn NuEventSink,
    agent: WebID,
    lifecycle_verb: &str,
    observation: serde_json::Value,
) {
    let span = Span::new(SpanNamespace::from(CnsSpan::AgentPod), lifecycle_verb);
    let event = NuEvent::new(agent, span, Phase::Act, observation, 0);

    if let Err(e) = sink.persist(&event) {
        tracing::warn!(
            target: "cns.agent_pod",
            error = %e,
            verb = lifecycle_verb,
            "Failed to persist pod lifecycle NuEvent"
        );
    }
}

/// Emit a pod registration event.
#[allow(dead_code)]
pub fn emit_pod_registered(sink: &dyn NuEventSink, agent: WebID, pod_id: &str, agent_type: &str) {
    emit_pod_event(
        sink,
        agent,
        "registered",
        serde_json::json!({
            "pod_id": pod_id,
            "agent_type": agent_type,
        }),
    );
}

/// Emit a pod activation event.
#[allow(dead_code)]
pub fn emit_pod_activated(sink: &dyn NuEventSink, agent: WebID, pod_id: &str) {
    emit_pod_event(
        sink,
        agent,
        "activated",
        serde_json::json!({
            "pod_id": pod_id,
        }),
    );
}

/// Emit a pod deactivation event.
#[allow(dead_code)]
pub fn emit_pod_deactivated(sink: &dyn NuEventSink, agent: WebID, pod_id: &str) {
    emit_pod_event(
        sink,
        agent,
        "deactivated",
        serde_json::json!({
            "pod_id": pod_id,
        }),
    );
}
