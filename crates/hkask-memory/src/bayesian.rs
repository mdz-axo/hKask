//! Bayesian confidence operations
//!
//! Free functions for the episodic and semantic memory subloops:
//! - `decay` — Loop 2a.3: Confidence decay (RECONCILE)
//! - `retract` — Loop 2a.4: Confidence retraction (RECONCILE)
//!
//! **Cybernetics regulation note:** `decay` and `retract` are involuntary dampening
//! functions owned by the Cybernetics loop. They are invoked from
//! `EpisodicLoop::act()` for budget enforcement (pruning) and from `EpisodicMemory`
//! at recall time for time-based confidence decay.
//!
//! The loop membrane is the authority; domain code calls these functions only
//! for recall-time presentation (decay) and loop-directed retraction.

/// Subtract confidence (retraction)
///
/// Reduces `conf1` proportionally by `conf2`.
/// Used in Loop 2a.4 (Confidence Retraction) to reduce episodic triple
/// confidence without deleting the triple.
pub fn retract(conf1: f64, conf2: f64) -> f64 {
    // Simplified retraction: reduce confidence proportionally
    (conf1 * (1.0 - conf2)).clamp(0.0, 1.0)
}

/// Decay confidence over time
///
/// Exponential decay: `confidence × e^(-rate × time_elapsed)`.
/// Used in Loop 2a.3 (Confidence Decay) to reduce episodic triple
/// confidence at recall time based on time since storage.
pub fn decay(confidence: f64, decay_rate: f64, time_elapsed: f64) -> f64 {
    // Exponential decay: conf * e^(-rate * time)
    confidence * (-decay_rate * time_elapsed).exp()
}
