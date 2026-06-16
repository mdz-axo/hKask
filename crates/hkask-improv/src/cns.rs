//! CNS integration for improv — span registration and monitoring.
//!
//! Registers improv-specific CNS spans following the canonical hierarchy
//! defined in `docs/architecture/PRINCIPLES.md` §1.4.

use hkask_types::event::{Span, SpanNamespace};

/// Trait for CNS integration — allows hkask-cns to receive improv span registrations.
pub trait ImprovCns {
    /// Register all improv-related CNS spans.
    fn register_improv_spans(&mut self);
}

/// Default CNS integration — logs span registrations via `tracing`.
///
/// Used when the full CNS runtime isn't wired yet. Spans are emitted
/// at `info` level under the `cns.improv` target.
pub struct TracingImprovCns;

impl ImprovCns for TracingImprovCns {
    fn register_improv_spans(&mut self) {
        for ns in IMPROV_SPAN_NAMESPACES {
            tracing::info!(
                target: "cns.improv",
                namespace = ns,
                "Improv CNS span registered"
            );
        }
    }
}

/// Canonical improv CNS span namespaces.
///
/// These must be added to `CANONICAL_NAMESPACES` in `hkask-types::event`
/// before they can be used with `SpanNamespace::new()`.
pub const IMPROV_SPAN_NAMESPACES: &[&str] = &[
    "cns.improv.mode.active",
    "cns.improv.plussing.ratio",
    "cns.improv.freestyle.coherence",
    "cns.kata.improv.effectiveness",
    "cns.improv.cascade.depth",
];

/// Build a CNS span for improv mode tracking.
pub fn improv_span(namespace: &str, path: &str) -> Option<Span> {
    let ns = SpanNamespace::parse(namespace)?;
    Some(Span::new(ns, path))
}

/// Build the mode.active span for tracking which improv mode is active.
pub fn mode_active_span(mode_label: &str) -> Option<Span> {
    improv_span("cns.improv.mode.active", mode_label)
}

/// Build the plussing ratio span for tracking constructive ratio.
pub fn plussing_ratio_span() -> Option<Span> {
    improv_span("cns.improv.plussing.ratio", "constructive_ratio")
}

/// Build the freestyle coherence span.
pub fn freestyle_coherence_span() -> Option<Span> {
    improv_span("cns.improv.freestyle.coherence", "coherence")
}

/// Build the kata improv effectiveness span.
pub fn kata_improv_effectiveness_span() -> Option<Span> {
    improv_span("cns.kata.improv.effectiveness", "automaticity_delta")
}

/// Build the cascade depth span for tracking recursion depth.
pub fn cascade_depth_span(depth: u8) -> Option<Span> {
    improv_span("cns.improv.cascade.depth", &depth.to_string())
}

// ── CNS Alert Thresholds (reasonable starting estimates; tune through use) ──

/// Plussing constructive ratio: alert if below 0.4 (less than 40% agreeable).
/// Warning threshold: below 0.5.
pub const PLUSSING_RATIO_ALERT_THRESHOLD: f64 = 0.4;
pub const PLUSSING_RATIO_WARN_THRESHOLD: f64 = 0.5;

/// Cascade depth: warn at 5, critical at matryoshka limit (7).
pub const CASCADE_DEPTH_WARN: u8 = 5;
pub const CASCADE_DEPTH_CRITICAL: u8 = 7;

/// Freestyle coherence: alert if session produces 0 turns.
/// Warning if fewer than 3 turns in a session.
pub const FREESTYLE_MIN_TURNS_WARN: usize = 3;
pub const FREESTYLE_MIN_TURNS_ALERT: usize = 0;

/// Kata improv effectiveness: alert if automaticity delta is negative
/// (improv made kata performance worse).
pub const KATA_IMPROV_EFFECTIVENESS_ALERT: f64 = 0.0;

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: IMPROV_SPAN_NAMESPACES contains all five required spans
    #[test]
    fn improv_span_namespaces_are_defined() {
        assert_eq!(IMPROV_SPAN_NAMESPACES.len(), 5);
        assert!(IMPROV_SPAN_NAMESPACES.contains(&"cns.improv.mode.active"));
        assert!(IMPROV_SPAN_NAMESPACES.contains(&"cns.improv.plussing.ratio"));
        assert!(IMPROV_SPAN_NAMESPACES.contains(&"cns.improv.freestyle.coherence"));
        assert!(IMPROV_SPAN_NAMESPACES.contains(&"cns.kata.improv.effectiveness"));
        assert!(IMPROV_SPAN_NAMESPACES.contains(&"cns.improv.cascade.depth"));
    }

    // REQ: IMPROV-CNS-001 — Span builders return Some when namespace is registered
    #[test]
    fn span_builders_return_some_after_registration() {
        let span = mode_active_span("plussing");
        assert!(
            span.is_some(),
            "Span should be Some after CNS registration — namespaces are in CANONICAL_NAMESPACES"
        );
        if let Some(s) = span {
            assert!(s.as_str().contains("cns.improv.mode.active"));
            assert!(s.as_str().contains("plussing"));
        }
    }
}
