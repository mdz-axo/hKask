//! Bayesian confidence operations — decay and evidence pooling.
//!
//! Exponential confidence decay is handled by `Confidence::decay()`
//! (hkask-types). This module adds log-odds Bayesian pooling for
//! combining independent confidence estimates — the mathematically
//! correct method for merging two sources of evidence about the same
//! proposition.

use hkask_types::Confidence;

// ── Decay constants ───────────────────────────────────────────────────────

/// Default half-life for episodic confidence decay (6 months in seconds).
///
/// After this duration, a triple's recall-time confidence has decayed to
/// half its stored value. Overridable via ServiceConfig.decay_half_life_months.
pub const DEFAULT_DECAY_HALF_LIFE_SECS: f64 = 6.0 * 30.0 * 24.0 * 3600.0; // 6 months

/// Default decay rate derived from half-life: λ = ln(2) / half_life.
///
/// With the default 6-month half-life this is ≈ 4.456 × 10⁻⁸,
/// giving confidence half-life of ~180 days.
pub const DEFAULT_DECAY_RATE: f64 = std::f64::consts::LN_2 / DEFAULT_DECAY_HALF_LIFE_SECS;

// ── Bayesian evidence pooling (log-odds) ───────────────────────────────────

/// Minimum confidence value used in log-odds computation to avoid ln(0).
const LOG_ODDS_EPSILON: f64 = 1e-6;

/// Combine two independent confidence estimates using log-odds (Bayesian) pooling.
///
/// Given two independent probability estimates `c₁` and `c₂` for the same
/// proposition, computes the combined posterior via log-odds addition:
///
/// ```text
/// L₁  = ln(c₁ / (1 − c₁))       // log-odds of source 1
/// L₂  = ln(c₂ / (1 − c₂))       // log-odds of source 2
/// L   = L₁ + L₂                 // additive: independent evidence
/// c   = 1 / (1 + exp(−L))       // convert back to probability
/// ```
///
/// This is the standard Bayesian method for pooling independent probability
/// judgments (log-odds pooling). Two independent confirmations produce
/// stronger belief than either alone — this is the mathematical expression
/// of "two witnesses are more credible than one."
///
/// # Properties
///
/// - **Consensus-strengthening:** c₁ = 0.8, c₂ = 0.8 → combined ≈ 0.941
/// - **Conflict-dampening:** c₁ = 0.8, c₂ = 0.2 → combined = 0.5
/// - **Low-evidence discounting:** c₁ = 0.51, c₂ = 0.51 → combined ≈ 0.520
/// - **High-certitude convergence:** c₁ = 0.99, c₂ = 0.99 → combined ≈ 0.9999
///
/// # Edge cases
///
/// Values of 0 and 1 are clamped to [ε, 1−ε] where ε = 10⁻⁶ to avoid
/// ln(0) and division-by-zero. This is a computational safeguard, not a
/// mathematical limitation — the limit as c → 0⁺ or c → 1⁻ is well-defined.
///
/// expect: "The system combines independent confidence estimates using Bayesian evidence pooling"
/// \[P3\] Motivating: Generative Space — fuses episodic and semantic evidence
/// \[P8\] Constraining: Semantic Grounding — log-odds pooling is the
///         mathematically correct method for combining independent probabilities
/// pre:  c₁, c₂ are in [0, 1]
/// post: returns Confidence in [0, 1]
/// post: combined ≥ max(c₁, c₂) when both > 0.5 (consensus strengthens)
/// post: combined = 0.5 when c₁ = 0.5 and c₂ = 0.5 (neutral evidence)
/// post: combined → 0.5 as c₁ → 0.5 (any value combined with 0.5 stays near 0.5)
pub fn combine_confidences(c1: Confidence, c2: Confidence) -> Confidence {
    let v1 = c1.value().clamp(LOG_ODDS_EPSILON, 1.0 - LOG_ODDS_EPSILON);
    let v2 = c2.value().clamp(LOG_ODDS_EPSILON, 1.0 - LOG_ODDS_EPSILON);

    // Log-odds: logit(p) = ln(p / (1 − p))
    let log_odds_1 = (v1 / (1.0 - v1)).ln();
    let log_odds_2 = (v2 / (1.0 - v2)).ln();

    // Additive for independent evidence
    let combined_log_odds = log_odds_1 + log_odds_2;

    // Inverse logit: σ(x) = 1 / (1 + e⁻ˣ)
    let combined = 1.0 / (1.0 + (-combined_log_odds).exp());

    Confidence::new(combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consensus_strengthening() {
        let c = combine_confidences(Confidence::new(0.8), Confidence::new(0.8));
        // 0.8 + 0.8 in log-odds: ln(4) + ln(4) = 2·ln(4) = ln(16)
        // σ(ln(16)) = 16/17 ≈ 0.941
        assert!(c.value() > 0.93 && c.value() < 0.95);
    }

    #[test]
    fn conflict_dampening() {
        let c = combine_confidences(Confidence::new(0.8), Confidence::new(0.2));
        // ln(4) + ln(0.25) = ln(4) − ln(4) = 0 → σ(0) = 0.5
        assert!((c.value() - 0.5).abs() < 0.001);
    }

    #[test]
    fn neutral_evidence_does_not_shift() {
        // 0.5 has zero log-odds → combining with 0.5 leaves the other unchanged
        let c = combine_confidences(Confidence::new(0.5), Confidence::new(0.9));
        assert!((c.value() - 0.9).abs() < 0.001);
    }

    #[test]
    fn low_evidence_stays_low() {
        let c = combine_confidences(Confidence::new(0.51), Confidence::new(0.51));
        assert!(c.value() > 0.5 && c.value() < 0.53);
    }

    #[test]
    fn edge_case_zero_clamped() {
        let c = combine_confidences(Confidence::new(0.0), Confidence::new(0.8));
        // 0.0 → ε ≈ 1e-6: log_odds ≈ ln(1e-6) ≈ −13.82
        // 0.8: log_odds = ln(4) ≈ 1.386
        // combined ≈ −12.43 → σ ≈ 4e-6 → still essentially 0
        assert!(c.value() < 1e-5);
    }

    #[test]
    fn edge_case_one_clamped() {
        let c = combine_confidences(Confidence::new(1.0), Confidence::new(0.8));
        // 1.0 → 1−ε: log_odds ≈ ln(1e6) ≈ 13.82
        // combined with ln(4) → σ ≈ very close to 1.0
        assert!(c.value() > 0.9999);
    }

    #[test]
    fn high_both_saturates() {
        let c = combine_confidences(Confidence::new(0.99), Confidence::new(0.99));
        assert!(c.value() > 0.9998);
    }

    #[test]
    fn monotonic_both_increase() {
        // Combining should produce a value ≥ both inputs when both > 0.5
        let c = combine_confidences(Confidence::new(0.6), Confidence::new(0.7));
        assert!(c.value() >= 0.6);
        assert!(c.value() >= 0.7);
    }
}
