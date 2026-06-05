//! Bayesian confidence operations
//!
//! Free functions for the episodic and semantic memory subloops:
//! - `decay` — Loop 2a.3: Confidence decay (RECONCILE)
//!
//! **Cybernetics regulation note:** `decay` is an involuntary dampening
//! function owned by the Cybernetics loop. It is invoked from
//! `EpisodicMemory` at recall time for time-based confidence decay.

/// Default half-life for episodic confidence decay (3 months in seconds).
///
/// After this duration, a triple's recall-time confidence has decayed to
/// half its stored value. Set to 3 months so that recently-learned knowledge
/// remains strong while stale knowledge gradually fades over months.
pub const DEFAULT_DECAY_HALF_LIFE_SECS: f64 = 90.0 * 24.0 * 3600.0; // 3 months

/// Default decay rate derived from half-life: λ = ln(2) / half_life.
///
/// With the default 3-month half-life this is ≈ 8.913 × 10⁻⁸,
/// giving confidence half-life of 90 days.
pub const DEFAULT_DECAY_RATE: f64 = std::f64::consts::LN_2 / DEFAULT_DECAY_HALF_LIFE_SECS;

/// Decay confidence over time
///
/// Exponential decay: `confidence × e^(-rate × time_elapsed)`.
/// Used in Loop 2a.3 (Confidence Decay) to reduce episodic triple
/// confidence at recall time based on time since storage.
pub fn decay(confidence: f64, decay_rate: f64, time_elapsed: f64) -> f64 {
    // Exponential decay: conf * e^(-rate * time)
    confidence * (-decay_rate * time_elapsed).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_half_life_default() {
        // After one half-life, confidence should be approximately half
        let original = 1.0;
        let decayed = decay(original, DEFAULT_DECAY_RATE, DEFAULT_DECAY_HALF_LIFE_SECS);
        assert!(
            (decayed - 0.5).abs() < 0.001,
            "Expected ~0.5, got {}",
            decayed
        );
    }

    #[test]
    fn decay_no_time_elapsed() {
        let confidence = 0.8;
        let decayed = decay(confidence, DEFAULT_DECAY_RATE, 0.0);
        assert!((decayed - confidence).abs() < f64::EPSILON);
    }

    #[test]
    fn decay_two_half_lives() {
        let original = 1.0;
        let decayed = decay(
            original,
            DEFAULT_DECAY_RATE,
            DEFAULT_DECAY_HALF_LIFE_SECS * 2.0,
        );
        assert!(
            (decayed - 0.25).abs() < 0.001,
            "Expected ~0.25, got {}",
            decayed
        );
    }
}
