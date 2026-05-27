//! Integration Tests for CNS Span Emission
//!
//! Tests for CNS span emission during operations.

#[allow(unused_imports)] // TODO: used only in #[test] functions
use hkask_cns::SpanEmitter;
#[allow(unused_imports)] // TODO: used only in #[test] functions
use hkask_types::WebID;
#[allow(unused_imports)] // TODO: used only in #[test] functions
use serde_json::json;

#[test]
fn test_cns_tool_span_emission() {
    let observer = WebID::new();
    let emitter = SpanEmitter::new(observer);

    emitter.emit_tool(
        "cns.tool.inference",
        json!({
            "tool_name": "inference",
            "success": true,
            "model": "qwen3:8b",
        }),
    );
}

#[test]
fn test_cns_connector_span_emission() {
    let observer = WebID::new();
    let emitter = SpanEmitter::new(observer);

    emitter.emit_connector(
        "cns.connector.llm.generate",
        json!({
            "model": "qwen3:8b",
            "tokens": 100,
        }),
    );
}

#[test]
fn test_cns_prompt_span_emission() {
    let observer = WebID::new();
    let emitter = SpanEmitter::new(observer);

    emitter.emit_prompt(
        "cns.prompt.render",
        json!({
            "template_id": "test/template",
            "success": true,
        }),
    );
}

#[test]
fn test_cns_agent_pod_span_emission() {
    let observer = WebID::new();
    let emitter = SpanEmitter::new(observer);

    emitter.emit_agent_pod(
        "cns.agent_pod.created",
        json!({
            "pod_id": "test-pod-123",
            "template": "test/template",
        }),
    );
}

#[test]
fn test_cns_goal_span_emission() {
    let observer = WebID::new();
    let emitter = SpanEmitter::new(observer);

    emitter.emit_goal(
        "cns.goal.created",
        json!({
            "goal_id": "test-goal-123",
            "text": "Test goal",
        }),
    );
}

#[test]
fn test_cns_energy_span_emission() {
    let observer = WebID::new();
    let emitter = SpanEmitter::new(observer);

    emitter.emit_energy(
        "cns.energy.consumed",
        json!({
            "tokens": 1000,
            "estimated_cost": 0.002,
        }),
    );
}

#[test]
fn test_cns_sovereignty_span_emission() {
    let observer = WebID::new();
    let emitter = SpanEmitter::new(observer);

    emitter.emit_sovereignty(
        "cns.sovereignty.access_check",
        json!({
            "category": "episodic_memory",
            "granted": true,
        }),
    );
}
