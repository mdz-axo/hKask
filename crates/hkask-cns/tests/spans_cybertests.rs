use hkask_cns::{CnsEmit, SpanCategory, SpanEmitter, SpanScope};
use hkask_cybertest::{
    CaptureSink, CyberTestSpec, Disturbance, DisturbanceKind, DisturbanceMode,
    EscalationExpectation, TelemetryCapture,
};
use hkask_types::WebID;
use serde_json::json;
use std::collections::HashSet;

#[test]
fn cyber_span_scope_capability_denied_emits_sovereignty_violation() {
    let spec = CyberTestSpec::builder(
        "must deny out-of-scope emissions and surface sovereignty telemetry",
        "span scope allows tool only",
        Disturbance::new(DisturbanceKind::CapabilityDenied, DisturbanceMode::Always),
    )
    .must_emit("category: Sovereignty")
    .with_escalation(EscalationExpectation::None)
    .build();

    let capture = TelemetryCapture::default();
    let emitter =
        SpanEmitter::new(WebID::new()).with_sink(Box::new(CaptureSink::new(capture.clone())));
    let scope = SpanScope::new(emitter, HashSet::from([SpanCategory::Tool]), WebID::new());

    scope.emit_event(
        "cns.connector.llm.tokens",
        "observe",
        &json!({"tokens": 10}),
        0.8,
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
}
