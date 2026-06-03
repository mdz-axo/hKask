//! Kill zone detection — Cybernetics subloop 6.5
//!
//! Monitors for acquisition patterns. When VC investment drops below
//! threshold after an acquisition attempt, triggers an algedonic alert.
//! This is CNS regulation logic, not type data.

use hkask_types::sovereignty::KillZoneConfig;

/// Kill zone detector — CNS regulation function.
///
/// Implements the sense phase of the kill-zone subloop:
/// sense (vc_investment) → compare (against threshold) → compute (alert?) → act (escalate)
pub struct KillZoneDetector {
    config: KillZoneConfig,
}

impl KillZoneDetector {
    pub fn new(config: KillZoneConfig) -> Self {
        Self { config }
    }

    /// Update VC investment level and check for kill zone.
    pub fn update_vc_investment(&mut self, vc_investment: f32) {
        self.config.vc_investment = vc_investment.clamp(0.0, 1.0);
        self.config.kill_zone_active =
            self.config.acquisition_attempt && self.config.vc_investment < self.config.threshold;
    }

    /// Mark that an acquisition attempt has been detected.
    pub fn mark_acquisition_attempt(&mut self) {
        self.config.acquisition_attempt = true;
        self.config.kill_zone_active =
            self.config.acquisition_attempt && self.config.vc_investment < self.config.threshold;
    }

    /// Whether a kill zone alert should be triggered.
    pub fn needs_alert(&self) -> bool {
        self.config.kill_zone_active
    }

    /// Get a reference to the current config.
    pub fn config(&self) -> &KillZoneConfig {
        &self.config
    }
}
