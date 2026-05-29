use hkask_cns::{CnsEmit, SpanCategory, SpanEmitter, SpanScope};
use hkask_cybertest::{
    CaptureSink, CyberExpectation, CyberTestSpec, Disturbance, EscalationExpectation,
    TelemetryCapture,
};
use hkask_types::WebID;
use serde_json::json;
use std::collections::HashSet;

#[test]
fn cyber_mcp_connector_transient_failure_stays_non_escalatory_when_tool_scope_allows() {
    let spec = CyberTestSpec::new(
        "tool-scoped MCP paths should emit tool telemetry without sovereignty escalation",
        "span scope allows tool category",
        Disturbance::transient_failures(2),
        CyberExpectation::default()
            .with_spans(vec!["Tool(cns.tool.mcp.dispatch.retry)"])
            .without_spans(vec!["Sovereignty(alert.boundary_violation)"])
            .with_escalation(EscalationExpectation::None),
    );

    let capture = TelemetryCapture::default();
    let emitter =
        SpanEmitter::new(WebID::new()).with_sink(Box::new(CaptureSink::new(capture.clone())));
    let scope = SpanScope::new(emitter, HashSet::from([SpanCategory::Tool]), WebID::new());

    // Simulate retries on transient disturbance through tool namespace.
    scope.emit_event(
        "cns.tool.mcp.dispatch.retry",
        "observe",
        &json!({"attempt": 1}),
        0.7,
    );
    scope.emit_event(
        "cns.tool.mcp.dispatch.retry",
        "observe",
        &json!({"attempt": 2}),
        0.7,
    );

    let spans = capture.spans();
    for expected in &spec.expectation.must_emit_spans {
        assert!(
            spans.iter().any(|s| s.contains(expected)),
            "policy: {} | expected telemetry span containing '{}', got {:?}",
            spec.policy,
            expected,
            spans
        );
    }

    for forbidden in &spec.expectation.must_not_emit_spans {
        assert!(
            !spans.iter().any(|s| s.contains(forbidden)),
            "policy: {} | unexpected telemetry span containing '{}', got {:?}",
            spec.policy,
            forbidden,
            spans
        );
    }
}
