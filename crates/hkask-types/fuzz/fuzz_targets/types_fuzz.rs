#![no_main]
use bolero::check;

/// QueueDepth invariant: value must never be negative.
/// Tests arbitrary f64 inputs — NaN, infinity, negative, zero, positive.
#[test]
fn fuzz_queue_depth_never_negative() {
    check!()
        .with_type::<f64>()
        .for_each(|v| {
            let qd = hkask_types::cns::QueueDepth::new(*v);
            assert!(qd.as_raw() >= 0.0, "QueueDepth must never be negative, got {}", qd.as_raw());
        });
}

/// CnsHealth coherence: zero deficit + zero critical → healthy.
#[test]
fn fuzz_cns_health_coherence() {
    check!()
        .with_type::<(u64, usize, usize, bool)>()
        .for_each(|(deficit, critical, warning, healthy)| {
            let health = hkask_types::cns::CnsHealth {
                overall_deficit: *deficit,
                critical_count: *critical,
                warning_count: *warning,
                healthy: *healthy,
            };
            if health.overall_deficit == 0 && health.critical_count == 0 {
                assert!(
                    health.healthy,
                    "zero deficit ({}) + zero critical ({}) → must be healthy",
                    health.overall_deficit, health.critical_count
                );
            }
        });
}

/// CnsSpan::from_str must never panic on arbitrary input.
#[test]
fn fuzz_cns_span_parse_never_panics() {
    check!()
        .with_type::<String>()
        .for_each(|s| {
            let _ = s.parse::<hkask_types::cns::CnsSpan>();
        });
}

/// CnsSpan Display + FromStr round-trip for all canonical strings.
/// Only valid variants should round-trip; invalid strings return Err.
#[test]
fn fuzz_cns_span_display_roundtrip() {
    check!()
        .with_type::<String>()
        .for_each(|s| {
            if let Ok(span) = s.parse::<hkask_types::cns::CnsSpan>() {
                let displayed = span.to_string();
                let reparsed: Result<hkask_types::cns::CnsSpan, _> = displayed.parse();
                assert!(
                    reparsed.is_ok(),
                    "round-trip failed: {s:?} → {displayed:?} → err"
                );
            }
        });
}
