//! Kill zone detection — Cybernetics subloop 6.5
//!
//! Monitors for acquisition patterns. When VC investment drops below
//! threshold after an acquisition attempt, triggers an algedonic alert.
//! This is CNS regulation logic, not type data.

use hkask_types::sovereignty::{KillZoneConfig, KillZoneThresholds};

/// Kill zone detector — CNS regulation function.
///
/// Implements the sense phase of the kill-zone subloop:
/// sense (vc_investment) → compare (against threshold) → compute (alert?) → act (escalate)
///
/// Configuration (`KillZoneThresholds`) is set by Curation and is immutable.
/// State (`KillZoneConfig`) is the mutable operational state that Cybernetics
/// senses and compares.
pub(crate) struct KillZoneDetector {
    thresholds: KillZoneThresholds,
    state: KillZoneConfig,
}

impl KillZoneDetector {
    pub fn new(thresholds: KillZoneThresholds) -> Self {
        Self {
            thresholds,
            state: KillZoneConfig::default(),
        }
    }

    /// Update VC investment level and check for kill zone.
    pub fn update_vc_investment(&mut self, vc_investment: f32) {
        self.state.vc_investment = vc_investment.clamp(0.0, 1.0);
        self.state.kill_zone_active =
            self.state.acquisition_attempt && self.state.vc_investment < self.thresholds.threshold;
    }

    /// Mark that an acquisition attempt has been detected.
    pub fn mark_acquisition_attempt(&mut self) {
        self.state.acquisition_attempt = true;
        self.state.kill_zone_active =
            self.state.acquisition_attempt && self.state.vc_investment < self.thresholds.threshold;
    }

    /// Whether a kill zone alert should be triggered.
    pub fn needs_alert(&self) -> bool {
        self.state.kill_zone_active
    }

    /// Get a reference to the current state.
    pub fn state(&self) -> &KillZoneConfig {
        &self.state
    }

    /// Get a reference to the thresholds.
    pub fn thresholds(&self) -> &KillZoneThresholds {
        &self.thresholds
    }
}
