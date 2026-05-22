//! Sovereignty Unit Tests
//!
//! Tests for hkask-types sovereignty types

use hkask_types::sovereignty::{
    AcquisitionResistance, DataCategory, DataSovereigntyBoundary, KillZoneDetector, SovereigntyId,
    UserSovereigntyState,
};

#[test]
fn test_sovereignty_id_new() {
    let id1 = SovereigntyId::new();
    let id2 = SovereigntyId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_data_category_classification() {
    assert!(DataCategory::EpisodicMemory.is_typically_sovereign());
    assert!(DataCategory::SemanticMemory.is_typically_shared());
    assert!(DataCategory::HLexiconTerms.is_typically_public());
}

#[test]
fn test_data_category_as_str() {
    assert_eq!(DataCategory::EpisodicMemory.as_str(), "episodic_memory");
    assert_eq!(DataCategory::SemanticMemory.as_str(), "semantic_memory");
}

#[test]
fn test_acquisition_resistance_prevents_passive() {
    assert!(AcquisitionResistance::Medium.prevents_passive_acquisition());
    assert!(AcquisitionResistance::High.prevents_passive_acquisition());
    assert!(AcquisitionResistance::Maximum.prevents_passive_acquisition());
    assert!(!AcquisitionResistance::Low.prevents_passive_acquisition());
    assert!(!AcquisitionResistance::None.prevents_passive_acquisition());
}

#[test]
fn test_data_sovereignty_boundary_default() {
    let boundary = DataSovereigntyBoundary::hkask_default();
    assert!(boundary.is_sovereign(&DataCategory::EpisodicMemory));
    assert!(boundary.is_shared(&DataCategory::SemanticMemory));
    assert!(boundary.is_public(&DataCategory::HLexiconTerms));
    assert_eq!(boundary.resistance, AcquisitionResistance::High);
}

#[test]
fn test_data_sovereignty_is_sovereign() {
    let mut boundary = DataSovereigntyBoundary::new();
    boundary.add_sovereign(DataCategory::PersonalContext);
    assert!(boundary.is_sovereign(&DataCategory::PersonalContext));
    assert!(!boundary.is_sovereign(&DataCategory::EpisodicMemory));
}

#[test]
fn test_kill_zone_detector_no_alert() {
    let mut detector = KillZoneDetector::new();
    detector.update(0.8);
    assert!(!detector.needs_alert());
}

#[test]
fn test_kill_zone_detector_alert() {
    let mut detector = KillZoneDetector::new();
    detector.mark_acquisition_attempt();
    detector.update(0.3);
    assert!(detector.needs_alert());
}

#[test]
fn test_kill_zone_detector_threshold() {
    let mut detector = KillZoneDetector::new();
    detector.mark_acquisition_attempt();
    detector.update(0.5);
    assert!(!detector.needs_alert());
    detector.update(0.49);
    assert!(detector.needs_alert());
}

#[test]
fn test_user_sovereignty_state_compromised() {
    let mut state = UserSovereigntyState::new();
    assert!(!state.is_compromised());
    state.mark_acquisition_attempt();
    state.update_vc_investment(0.3);
    assert!(state.is_compromised());
}

#[test]
fn test_user_sovereignty_consent() {
    let mut state = UserSovereigntyState::new();
    assert!(!state.explicit_consent);
    state.grant_consent();
    assert!(state.explicit_consent);
    state.revoke_consent();
    assert!(!state.explicit_consent);
}
