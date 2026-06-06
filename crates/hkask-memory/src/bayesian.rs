//! Bayesian confidence operations
//!
//! Constants for episodic and semantic memory subloops.
//! Confidence decay is handled by `Confidence::decay()` (hkask-types).

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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::Confidence;

    #[test]
    fn decay_half_life_default() {
        let original = Confidence::full();
        let decayed = original.decay(DEFAULT_DECAY_RATE, DEFAULT_DECAY_HALF_LIFE_SECS);
        assert!(
            (decayed.value() - 0.5).abs() < 0.001,
            "Expected ~0.5, got {}",
            decayed.value()
        );
    }

    #[test]
    fn decay_no_time_elapsed() {
        let original = Confidence::new(0.8);
        let decayed = original.decay(DEFAULT_DECAY_RATE, 0.0);
        assert!((decayed.value() - original.value()).abs() < f64::EPSILON);
    }

    #[test]
    fn decay_two_half_lives() {
        let original = Confidence::full();
        let decayed = original.decay(DEFAULT_DECAY_RATE, DEFAULT_DECAY_HALF_LIFE_SECS * 2.0);
        assert!(
            (decayed.value() - 0.25).abs() < 0.001,
            "Expected ~0.25, got {}",
            decayed.value()
        );
    }
}
