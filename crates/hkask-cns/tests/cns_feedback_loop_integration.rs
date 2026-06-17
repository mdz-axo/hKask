//! CNS feedback loop integration test — Wave 3 Task 3.4
//!
//! Verifies closed-loop CNS behavior: event injection → algedonic response →
//! homeostatic restoration. Uses MockCnsRuntime from hkask-test-harness.
//!
//! # Principle grounding
//! - P9 (Homeostatic Self-Regulation): the CNS must detect perturbations and restore homeostasis
//! - P8 (Semantic Grounding): every test asserts a stated behavioral property

use hkask_test_harness::{MockCnsRuntime, MockCnsState, MockToolState, test_event};
use hkask_types::event::{Phase, Span, SpanNamespace};

// REQ: INT-004 — CNS feedback loop closure (P9)
// The CNS detects perturbations and restores homeostasis.

#[test]
fn cns_detects_perturbation() {
    let cns = MockCnsRuntime::new();
    assert!(cns.is_homeostatic(), "CNS should start homeostatic");

    let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
    let event = test_event(span, Phase::Sense, None);
    cns.inject(event);

    assert!(!cns.is_homeostatic(), "CNS should detect perturbation");
    let signals = cns.recent_signals();
    assert!(
        signals.iter().any(|s| s.is_negative_valence()),
        "CNS should emit negative valence signal on perturbation"
    );
}

// REQ: INT-004 — cns restores homeostasis after time
#[test]
fn cns_restores_homeostasis_after_time() {
    let cns = MockCnsRuntime::new();

    // Perturb the system
    let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
    cns.inject(test_event(span, Phase::Sense, None));
    assert!(!cns.is_homeostatic());

    // Advance time to allow feedback processing
    cns.advance_time(std::time::Duration::from_secs(10));

    // System should return to homeostasis
    assert!(
        cns.is_homeostatic(),
        "CNS should restore homeostasis after sufficient time"
    );
    let signals = cns.recent_signals();
    assert!(
        signals.iter().any(|s| s.is_positive_valence()),
        "CNS should emit positive valence signal on homeostasis restoration"
    );
}

// REQ: INT-004 — cns throttles tool on budget exceeded
#[test]
fn cns_throttles_tool_on_budget_exceeded() {
    let cns = MockCnsRuntime::with_state(MockCnsState::perturbed("tool-x"));

    assert!(!cns.is_homeostatic());
    assert_eq!(cns.tool_state("tool-x"), MockToolState::Throttled);
    assert_eq!(cns.tool_state("tool-y"), MockToolState::Active);
}

// REQ: INT-004 — cns tracks variety by domain
#[test]
fn cns_tracks_variety_by_domain() {
    let cns = MockCnsRuntime::new();

    cns.record_variety("cns.tool");
    cns.record_variety("cns.tool");
    cns.record_variety("cns.inference");

    assert_eq!(cns.variety_for_domain("cns.tool"), 2);
    assert_eq!(cns.variety_for_domain("cns.inference"), 1);
    assert_eq!(cns.variety_for_domain("cns.unknown"), 0);
}

// REQ: INT-004 — cns multiple perturbations accumulate signals
#[test]
fn cns_multiple_perturbations_accumulate_signals() {
    let cns = MockCnsRuntime::new();

    let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
    cns.inject(test_event(span, Phase::Sense, None));
    cns.inject(test_event(
        Span::new(SpanNamespace::new("cns.inference"), "error"),
        Phase::Compute,
        None,
    ));

    let signals = cns.recent_signals();
    assert!(
        signals.len() >= 2,
        "multiple perturbations should produce multiple signals"
    );
    let negative_count = signals.iter().filter(|s| s.is_negative_valence()).count();
    assert!(
        negative_count >= 2,
        "each perturbation should produce a negative signal"
    );
}
