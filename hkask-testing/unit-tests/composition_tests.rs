//! CNS Composition Observer Unit Tests
//!
//! Tests for hkask-cns composition observer

use hkask_cns::algedonic::AlertSeverity;
use hkask_cns::observers::composition::{
    CompositionMetrics, CompositionObserver, CompositionObserverState, VarietyCounter,
};
use hkask_types::WebID;

#[test]
fn test_composition_metrics_new() {
    let metrics = CompositionMetrics::new();
    assert_eq!(metrics.total_attempts, 0);
    assert_eq!(metrics.success_rate(), 0.0);
}

#[test]
fn test_composition_metrics_success_rate() {
    let mut metrics = CompositionMetrics::new();
    metrics.record_success(100);
    metrics.record_success(200);
    metrics.record_failure(150);

    assert_eq!(metrics.total_attempts, 3);
    assert_eq!(metrics.successful_translations, 2);
    assert_eq!(metrics.failed_translations, 1);
    assert!((metrics.success_rate() - 0.666).abs() < 0.01);
}

#[test]
fn test_variety_counter() {
    let mut counter = VarietyCounter::new("template", 100);
    assert_eq!(counter.count, 0);
    assert_eq!(counter.deficit(), 100);

    for _ in 0..50 {
        counter.increment();
    }
    assert_eq!(counter.count, 50);
    assert_eq!(counter.deficit(), 50);

    for _ in 0..60 {
        counter.increment();
    }
    assert_eq!(counter.count, 110);
    assert_eq!(counter.deficit(), 0);
    assert!(!counter.should_alert());
}

#[test]
fn test_composition_observer_state() {
    let state = CompositionObserverState::new(100);
    assert_eq!(state.variety_counters.len(), 3);
    assert_eq!(state.algedonic_threshold, 100);
}

#[test]
fn test_composition_observer() {
    let observer = CompositionObserver::new(WebID::new(), 100);

    observer.record_success(100);
    observer.record_success(200);
    observer.record_failure(150, "test error");

    assert!((observer.success_rate() - 0.666).abs() < 0.01);
}

// Note: test_algedonic_trigger removed - requires internal state manipulation
// that is not possible through public API

#[test]
fn test_calibration_prompt() {
    let observer = CompositionObserver::new(WebID::new(), 100);
    let prompt = observer.generate_calibration_prompt();
    assert!(prompt.contains("Composition calibration recommendations"));
}

#[test]
fn test_energy_variance() {
    let observer = CompositionObserver::new(WebID::new(), 100);
    observer.update_energy_variance(1000, 1200);
    assert!((observer.energy_variance() - 0.181).abs() < 0.01);
}
