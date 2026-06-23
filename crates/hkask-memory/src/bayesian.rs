//! Bayesian confidence operations
//!
//! Constants for episodic and semantic memory subloops.
//! Confidence decay is handled by `Confidence::decay()` (hkask-types).

/// Default half-life for episodic confidence decay (6 months in seconds).
///
/// After this duration, a triple's recall-time confidence has decayed to
/// half its stored value. Overridable via ServiceConfig.decay_half_life_months.
pub const DEFAULT_DECAY_HALF_LIFE_SECS: f64 = 6.0 * 30.0 * 24.0 * 3600.0; // 6 months

/// Default decay rate derived from half-life: λ = ln(2) / half_life.
///
/// With the default 3-month half-life this is ≈ 8.913 × 10⁻⁸,
/// giving confidence half-life of 90 days.
pub const DEFAULT_DECAY_RATE: f64 = std::f64::consts::LN_2 / DEFAULT_DECAY_HALF_LIFE_SECS;
