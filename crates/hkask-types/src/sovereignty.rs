//! User sovereignty and anti-catch-kill types
//!
//! These types enforce the Magna Carta of hKask:
//! - Clear boundaries that honor user sovereignty
//! - Acquisition resistance mechanisms
//! - Kill-zone detection for VC investment patterns

pub mod category;

pub use category::{DataCategory, DataSovereignty};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// SovereigntyId — Unique identifier for sovereignty boundaries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SovereigntyId(pub Uuid);

impl SovereigntyId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SovereigntyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SovereigntyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Acquisition resistance level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionResistance {
    /// No resistance — open to acquisition
    None,
    /// Low resistance — some user controls
    Low,
    /// Medium resistance — significant user sovereignty
    Medium,
    /// High resistance — strong anti-acquisition measures
    High,
    /// Maximum resistance — acquisition impossible without user consent
    #[default]
    Maximum,
}

impl AcquisitionResistance {
    /// Default resistance level for hKask pods
    pub fn default_for_pods() -> Self {
        Self::High
    }

    /// Check if resistance is sufficient to prevent passive acquisition
    pub fn prevents_passive_acquisition(&self) -> bool {
        matches!(self, Self::Medium | Self::High | Self::Maximum)
    }
}

impl std::fmt::Display for AcquisitionResistance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcquisitionResistance::None => write!(f, "none (open to acquisition)"),
            AcquisitionResistance::Low => write!(f, "low (some user controls)"),
            AcquisitionResistance::Medium => write!(f, "medium (significant sovereignty)"),
            AcquisitionResistance::High => write!(f, "high (strong anti-acquisition)"),
            AcquisitionResistance::Maximum => write!(f, "maximum (requires user consent)"),
        }
    }
}

/// Data sovereignty boundary — defines what data the user controls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSovereigntyBoundary {
    pub id: SovereigntyId,
    /// What data is under user sovereignty
    pub sovereign_data: Vec<String>,
    /// What data may be shared (with explicit consent)
    pub shared_data: Vec<String>,
    /// What data is public (no sovereignty claim)
    pub public_data: Vec<String>,
    /// Resistance level for this boundary
    pub resistance: AcquisitionResistance,
}

impl DataSovereigntyBoundary {
    pub fn new() -> Self {
        Self {
            id: SovereigntyId::default(),
            sovereign_data: Vec::new(),
            shared_data: Vec::new(),
            public_data: Vec::new(),
            resistance: AcquisitionResistance::default(),
        }
    }

    /// Create boundary with typical hKask defaults
    pub fn hkask_default() -> Self {
        Self {
            id: SovereigntyId::default(),
            sovereign_data: vec![
                "episodic_memory".to_string(),
                "personal_context".to_string(),
                "capability_tokens".to_string(),
                "ocap_boundaries".to_string(),
            ],
            shared_data: vec![
                "semantic_memory".to_string(),
                "template_invocations".to_string(),
            ],
            public_data: vec![
                "hlexicon_terms".to_string(),
                "template_registry".to_string(),
            ],
            resistance: AcquisitionResistance::default_for_pods(),
        }
    }

    /// Add sovereign data category
    pub fn add_sovereign(&mut self, category: &str) {
        if !self.sovereign_data.contains(&category.to_string()) {
            self.sovereign_data.push(category.to_string());
        }
    }

    /// Check if data category is under user sovereignty
    pub fn is_sovereign(&self, category: &str) -> bool {
        self.sovereign_data.contains(&category.to_string())
    }
}

impl Default for DataSovereigntyBoundary {
    fn default() -> Self {
        Self::new()
    }
}

/// Kill zone detection — monitors for acquisition patterns
///
/// Kill zone: VC investment < 0.5 after acquisition attempt
/// This triggers CNS algedonic alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillZoneDetector {
    /// Current VC investment level (0.0 to 1.0)
    pub vc_investment: f32,
    /// Threshold for kill zone alert
    pub threshold: f32,
    /// Whether kill zone is detected
    pub kill_zone_active: bool,
    /// Acquisition attempt detected
    pub acquisition_attempt: bool,
}

impl KillZoneDetector {
    pub fn new() -> Self {
        Self {
            vc_investment: 1.0,
            threshold: 0.5,
            kill_zone_active: false,
            acquisition_attempt: false,
        }
    }

    /// Update VC investment level and check for kill zone
    pub fn update(&mut self, vc_investment: f32) {
        self.vc_investment = vc_investment.clamp(0.0, 1.0);
        self.check_kill_zone();
    }

    /// Check if kill zone is active
    pub fn check_kill_zone(&mut self) {
        self.kill_zone_active = self.acquisition_attempt && self.vc_investment < self.threshold;
    }

    /// Mark acquisition attempt detected
    pub fn mark_acquisition_attempt(&mut self) {
        self.acquisition_attempt = true;
        self.check_kill_zone();
    }

    /// Check if kill zone alert should be triggered
    pub fn needs_alert(&self) -> bool {
        self.kill_zone_active
    }

    /// Reset detector state
    pub fn reset(&mut self) {
        self.vc_investment = 1.0;
        self.kill_zone_active = false;
        self.acquisition_attempt = false;
    }
}

impl Default for KillZoneDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// User sovereignty state — aggregate view of user's sovereignty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSovereigntyState {
    pub boundary: DataSovereigntyBoundary,
    pub detector: KillZoneDetector,
    /// Whether user has explicitly consented to data sharing
    pub explicit_consent: bool,
    /// Timestamp of last sovereignty check
    pub last_check: chrono::DateTime<chrono::Utc>,
}

impl UserSovereigntyState {
    pub fn new() -> Self {
        Self {
            boundary: DataSovereigntyBoundary::hkask_default(),
            detector: KillZoneDetector::new(),
            explicit_consent: false,
            last_check: chrono::Utc::now(),
        }
    }

    /// Update sovereignty state with current VC investment
    pub fn update_vc_investment(&mut self, vc_investment: f32) {
        self.detector.update(vc_investment);
        self.last_check = chrono::Utc::now();
    }

    /// Mark acquisition attempt
    pub fn mark_acquisition_attempt(&mut self) {
        self.detector.mark_acquisition_attempt();
        self.last_check = chrono::Utc::now();
    }

    /// Check if sovereignty is compromised
    pub fn is_compromised(&self) -> bool {
        self.detector.needs_alert()
    }

    /// Grant explicit consent for data sharing
    pub fn grant_consent(&mut self) {
        self.explicit_consent = true;
    }

    /// Revoke explicit consent
    pub fn revoke_consent(&mut self) {
        self.explicit_consent = false;
    }
}

impl Default for UserSovereigntyState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sovereignty_id_new() {
        let id1 = SovereigntyId::new();
        let id2 = SovereigntyId::new();
        assert_ne!(id1, id2);
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
        assert!(boundary
            .sovereign_data
            .contains(&"episodic_memory".to_string()));
        assert!(boundary
            .shared_data
            .contains(&"semantic_memory".to_string()));
        assert!(boundary.public_data.contains(&"hlexicon_terms".to_string()));
        assert_eq!(boundary.resistance, AcquisitionResistance::High);
    }

    #[test]
    fn test_data_sovereignty_is_sovereign() {
        let mut boundary = DataSovereigntyBoundary::new();
        boundary.add_sovereign("test_data");
        assert!(boundary.is_sovereign("test_data"));
        assert!(!boundary.is_sovereign("other_data"));
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
}
