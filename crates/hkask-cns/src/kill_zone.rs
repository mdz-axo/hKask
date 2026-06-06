//! Kill zone detection — Cybernetics subloop 6.5
//!
//! Monitors for acquisition patterns. When VC investment drops below
//! threshold after an acquisition attempt, triggers an algedonic alert.
//! This is CNS regulation logic, not type data.

use hkask_types::sovereignty::KillZoneState;

/// Kill zone detector — CNS regulation function.
///
/// Implements the sense phase of the kill-zone subloop:
/// sense (vc_investment) → compare (against threshold) → compute (alert?) → act (escalate)
///
/// The threshold is set by Curation and is immutable at runtime.
/// State (`KillZoneState`) is the mutable operational state that Cybernetics
/// senses and compares.
pub(crate) struct KillZoneDetector {
    threshold: f32,
    state: KillZoneState,
}

impl KillZoneDetector {
    pub(crate) fn new(threshold: f32) -> Self {
        Self {
            threshold,
            state: KillZoneState::default(),
        }
    }

    /// Update VC investment level and check for kill zone.
    pub(crate) fn update_vc_investment(&mut self, vc_investment: f32) {
        self.state.vc_investment = vc_investment.clamp(0.0, 1.0);
        self.state.kill_zone_active =
            self.state.acquisition_attempt && self.state.vc_investment < self.threshold;
    }

    /// Mark that an acquisition attempt has been detected.
    pub(crate) fn mark_acquisition_attempt(&mut self) {
        self.state.acquisition_attempt = true;
        self.state.kill_zone_active =
            self.state.acquisition_attempt && self.state.vc_investment < self.threshold;
    }

    /// Whether a kill zone alert should be triggered.
    pub(crate) fn needs_alert(&self) -> bool {
        self.state.kill_zone_active
    }

    /// Get a reference to the current state.
    pub(crate) fn state(&self) -> &KillZoneState {
        &self.state
    }
}
