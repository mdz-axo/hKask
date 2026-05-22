//! Sovereignty unit tests migrated from inline tests
//!
//! Tests for: hkask-agents/src/sovereignty.rs and hkask-cns/src/observers/sovereignty.rs

use hkask_agents::sovereignty::SovereigntyChecker;
use hkask_agents::ports::sovereignty::{SovereigntyOperation, SovereigntyPort};
use hkask_cns::observers::sovereignty::{
    SovereigntyObserver, SovereigntyEvent, SovereigntyEventType,
};
use hkask_cns::algedonic::{AlgedonicManager, DEFAULT_THRESHOLD};
use hkask_types::{DataCategory, SovereigntyId, WebID};
use serde_json::json;

// ============================================================================
// Sovereignty Checker Tests (from hkask-agents/src/sovereignty.rs)
// ============================================================================

#[test]
fn test_sovereignty_checker_new() {
    let checker = SovereigntyChecker::new(WebID::new());
    assert!(!checker.is_compromised());
    assert!(!checker.kill_zone_active());
}

#[test]
fn test_can_access_sovereign_data() {
    let owner = WebID::new();
    let mut checker = SovereigntyChecker::new(owner);
    // Sovereign data requires consent
    assert!(!checker.can_access(&DataCategory::EpisodicMemory, &owner));

    // Grant consent
    checker.grant_consent();
    // Now accessible to owner
    assert!(checker.can_access(&DataCategory::EpisodicMemory, &owner));
    // But not to others
    assert!(!checker.can_access(&DataCategory::EpisodicMemory, &WebID::new()));
}

#[test]
fn test_can_access_public_data() {
    let checker = SovereigntyChecker::new(WebID::new());
    // Public data is always accessible
    assert!(checker.can_access(&DataCategory::HLexiconTerms, &WebID::new()));
}

#[test]
fn test_acquisition_resistance() {
    let checker = SovereigntyChecker::new(WebID::new());
    // Default resistance is High, which prevents passive acquisition
    assert!(!checker.check_operation("acquisition", &DataCategory::SemanticMemory));
}

#[test]
fn test_kill_zone_detection() {
    let mut checker = SovereigntyChecker::new(WebID::new());
    checker.mark_acquisition_attempt(&json!({}));
    checker.update_vc_investment(0.3);
    assert!(checker.is_compromised());
    assert!(checker.kill_zone_active());
}

#[test]
fn test_consent_tracking() {
    let mut checker = SovereigntyChecker::new(WebID::new());
    assert!(!checker.get_state().explicit_consent);
    checker.grant_consent();
    assert!(checker.get_state().explicit_consent);
    checker.revoke_consent();
    assert!(!checker.get_state().explicit_consent);
}

#[test]
fn test_sovereignty_port_check() {
    let owner = WebID::new();
    let mut checker = SovereigntyChecker::new(owner);

    // Sovereign data without consent should be denied
    let result = checker.check(
        DataCategory::EpisodicMemory,
        SovereigntyOperation::Read,
        &checker.owner_webid(),
    );
    assert!(!result.allowed);
    assert!(result.denial_reason.is_some());

    // Grant consent and retry
    let mut checker = SovereigntyChecker::new(owner);
    checker.grant_consent();
    let result = checker.check(
        DataCategory::EpisodicMemory,
        SovereigntyOperation::Read,
        &checker.owner_webid(),
    );
    assert!(result.allowed);
    assert!(result.denial_reason.is_none());
}

#[test]
fn test_sovereignty_port_acquisition_denied() {
    let checker = SovereigntyChecker::new(WebID::new());
    let result = checker.check(
        DataCategory::SemanticMemory,
        SovereigntyOperation::Acquisition,
        &WebID::new(),
    );
    assert!(!result.allowed);
    assert!(result.denial_reason.is_some());
}

// ============================================================================
// Sovereignty Observer Tests (from hkask-cns/src/observers/sovereignty.rs)
// ============================================================================

#[test]
fn test_sovereignty_observer_new() {
    let manager = AlgedonicManager::new(DEFAULT_THRESHOLD);
    let observer = SovereigntyObserver::new(manager);
    assert_eq!(observer.acquisition_threshold, 5);
    assert_eq!(observer.violation_threshold, 3);
}

#[test]
fn test_sovereignty_observer_with_thresholds() {
    let manager = AlgedonicManager::new(DEFAULT_THRESHOLD);
    let observer = SovereigntyObserver::with_thresholds(manager, 10, 5);
    assert_eq!(observer.acquisition_threshold, 10);
    assert_eq!(observer.violation_threshold, 5);
}

#[test]
fn test_process_acquisition_attempts() {
    let manager = AlgedonicManager::new(DEFAULT_THRESHOLD);
    let observer = SovereigntyObserver::with_thresholds(manager, 3, 3);
    let webid = WebID::new();

    // Process acquisition attempts below threshold
    for _ in 0..2 {
        observer.process_event(SovereigntyEvent {
            event_type: SovereigntyEventType::AcquisitionAttempt,
            timestamp: std::time::Instant::now(),
            webid,
            sovereignty_id: SovereigntyId::new(),
            data_category: Some(DataCategory::EpisodicMemory),
            details: json!({}),
        });
    }

    assert_eq!(observer.get_acquisition_count(&webid), 2);

    // Third attempt should trigger alert
    observer.process_event(SovereigntyEvent {
        event_type: SovereigntyEventType::AcquisitionAttempt,
        timestamp: std::time::Instant::now(),
        webid,
        sovereignty_id: SovereigntyId::new(),
        data_category: Some(DataCategory::EpisodicMemory),
        details: json!({}),
    });

    assert_eq!(observer.get_acquisition_count(&webid), 3);
}

#[test]
fn test_process_kill_zone_alert() {
    let manager = AlgedonicManager::new(DEFAULT_THRESHOLD);
    let observer = SovereigntyObserver::new(manager);
    let webid = WebID::new();

    observer.process_event(SovereigntyEvent {
        event_type: SovereigntyEventType::KillZoneAlert,
        timestamp: std::time::Instant::now(),
        webid,
        sovereignty_id: SovereigntyId::new(),
        data_category: None,
        details: json!({"vc_investment": 0.3}),
    });

    assert_eq!(observer.get_state().kill_zone_alerts.len(), 1);
}

#[test]
fn test_process_boundary_violation() {
    let manager = AlgedonicManager::new(DEFAULT_THRESHOLD);
    let observer = SovereigntyObserver::with_thresholds(manager, 5, 2);
    let webid = WebID::new();

    // First violation
    observer.process_event(SovereigntyEvent {
        event_type: SovereigntyEventType::BoundaryViolation,
        timestamp: std::time::Instant::now(),
        webid,
        sovereignty_id: SovereigntyId::new(),
        data_category: Some(DataCategory::EpisodicMemory),
        details: json!({"denial_reason": "sovereign data"}),
    });

    assert_eq!(observer.get_violation_count(&webid), 1);

    // Second violation should trigger alert
    observer.process_event(SovereigntyEvent {
        event_type: SovereigntyEventType::BoundaryViolation,
        timestamp: std::time::Instant::now(),
        webid,
        sovereignty_id: SovereigntyId::new(),
        data_category: Some(DataCategory::EpisodicMemory),
        details: json!({"denial_reason": "sovereign data"}),
    });

    assert_eq!(observer.get_violation_count(&webid), 2);
}

#[test]
fn test_observer_reset() {
    let manager = AlgedonicManager::new(DEFAULT_THRESHOLD);
    let observer = SovereigntyObserver::new(manager);
    let webid = WebID::new();

    observer.process_event(SovereigntyEvent {
        event_type: SovereigntyEventType::AcquisitionAttempt,
        timestamp: std::time::Instant::now(),
        webid,
        sovereignty_id: SovereigntyId::new(),
        data_category: Some(DataCategory::EpisodicMemory),
        details: json!({}),
    });

    assert_eq!(observer.get_acquisition_count(&webid), 1);
    observer.reset();
    assert_eq!(observer.get_acquisition_count(&webid), 0);
}
