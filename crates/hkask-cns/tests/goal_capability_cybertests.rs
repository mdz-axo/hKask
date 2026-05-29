//! Cybernetic unit tests for the goal-capability denial control loop.
//!
//! Policy objective: a forged or out-of-scope goal capability must be rejected,
//! and the denial must surface as governance telemetry (a sovereignty boundary
//! violation) rather than passing silently.
//!
//! Disturbance: an attacker tampers with a signed `GoalCapabilityToken` to grant
//! itself an operation it was never issued.
//!
//! Expected telemetry: `Sovereignty("alert.boundary_violation")` emitted through
//! the CNS span scope when the denial is reported outside the agent's allowed
//! span category.
//!
//! Adaptation/escalation: none at the unit scale — a single denial is absorbed,
//! not escalated (escalation is threshold-driven by the sovereignty observer).

use hkask_cns::{CnsEmit, SpanCategory, SpanEmitter, SpanScope};
use hkask_cybertest::{
    CaptureSink, CyberTestSpec, Disturbance, DisturbanceKind, DisturbanceMode,
    EscalationExpectation, TelemetryCapture,
};
use hkask_types::goal_capability::{GoalCapabilityToken, GoalOp};
use hkask_types::{GoalID, WebID};
use serde_json::json;
use std::collections::HashSet;

const SECRET: &[u8] = b"goal-capability-cyber-secret-32b";

#[test]
fn cyber_forged_goal_token_is_denied_and_emits_sovereignty_violation() {
    let spec = CyberTestSpec::builder(
        "a forged goal capability must be denied and surface a sovereignty boundary violation",
        "holder may only emit within the Tool span category",
        Disturbance::new(DisturbanceKind::CapabilityDenied, DisturbanceMode::Always),
    )
    .must_emit("Sovereignty(\"alert.boundary_violation\")")
    .with_escalation(EscalationExpectation::None)
    .build();

    // --- Disturbance: forge authority onto a validly signed token. ---
    let holder = WebID::new();
    let mut token = GoalCapabilityToken::new(GoalID::new(), holder, vec![GoalOp::Read], SECRET);
    // The attacker appends an Update operation without re-signing.
    token.operations.push(GoalOp::Update);

    // The hardened token binds operations into the HMAC, so the forgery is
    // detected: the token is invalid and grants nothing.
    assert!(
        !token.is_valid(SECRET),
        "policy: {} | forged token must be rejected",
        spec.policy
    );
    assert!(
        !token.can_perform(GoalOp::Update, SECRET),
        "policy: {} | forged operation must not be authorized",
        spec.policy
    );

    // --- Telemetry: the denial is reported through the governance path. ---
    // The agent's span scope only permits Tool telemetry; reporting the denial
    // on a connector span (an out-of-scope channel) is itself a boundary
    // violation, which is the observable governance signal.
    let capture = TelemetryCapture::default();
    let emitter =
        SpanEmitter::new(WebID::new()).with_sink(Box::new(CaptureSink::new(capture.clone())));
    let scope = SpanScope::new(emitter, HashSet::from([SpanCategory::Tool]), holder);

    scope.emit_event(
        "cns.connector.goal.capability.denied",
        "observe",
        &json!({
            "holder": holder.to_string(),
            "attempted_op": GoalOp::Update.as_str(),
            "reason": "signature_invalid",
        }),
        1.0,
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

#[test]
fn cyber_untampered_goal_token_within_scope_emits_no_violation() {
    // Control case: an honest tool-scoped denial report (e.g. a legitimately
    // expired token logged on the Tool channel) must NOT raise a sovereignty
    // violation — the governance signal must be specific to boundary breaches,
    // not to every denial.
    let spec = CyberTestSpec::builder(
        "an in-scope denial report must not raise a false sovereignty violation",
        "holder emits within the Tool span category",
        Disturbance::new(DisturbanceKind::CapabilityDenied, DisturbanceMode::Always),
    )
    .must_not_emit("Sovereignty(\"alert.boundary_violation\")")
    .with_escalation(EscalationExpectation::None)
    .build();

    let holder = WebID::new();
    let capture = TelemetryCapture::default();
    let emitter =
        SpanEmitter::new(WebID::new()).with_sink(Box::new(CaptureSink::new(capture.clone())));
    let scope = SpanScope::new(emitter, HashSet::from([SpanCategory::Tool]), holder);

    scope.emit_event(
        "cns.tool.goal.capability.denied",
        "observe",
        &json!({"reason": "expired"}),
        1.0,
    );

    let spans = capture.spans();
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
