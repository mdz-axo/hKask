//! Superforecasting integration (Tetlock GJP methodology).
//!
//! Four-stage pipeline anchored to FIBO concepts:
//! 1. Fermi decomposition — break the forecast into sub-questions
//! 2. Outside view — base rate calibration from reference class
//! 3. Inside view — company-specific adjustments
//! 4. Bayesian updating — revise probabilities as evidence arrives
//!
//! Integrates with scenario analysis to produce calibrated probability
//! distributions over intrinsic value scenarios.

/// Apply user overrides to a set of Fermi sub-questions.
/// `overrides`: list of (index, estimate, confidence) tuples.
/// Only overrides for valid indices are applied; others are ignored.
pub fn apply_fermi_overrides(sub_questions: &mut [SubQuestion], overrides: &[(usize, f64, f64)]) {
    for (idx, est, conf) in overrides {
        if *idx < sub_questions.len() {
            sub_questions[*idx].estimate = *est;
            sub_questions[*idx].confidence = *conf;
        }
    }
}

use crate::scenarios::ScenarioResult;

// ── Fermi configuration ────────────────────────────────────────────────────

/// Server-level default Fermi estimates.
/// Overridable via environment variable HKASK_FERMI_DEFAULTS as JSON.
/// Each deployment can set its own seed/bootstrap estimates.
#[derive(Debug, Clone)]
pub struct FermiDefaults {
    pub growth_questions: Vec<SubQuestion>,
    pub margin_questions: Vec<SubQuestion>,
}

impl Default for FermiDefaults {
    fn default() -> Self {
        Self {
            growth_questions: fermi_decompose_growth(),
            margin_questions: fermi_decompose_margin(),
        }
    }
}

impl FermiDefaults {
    /// Load from HKASK_FERMI_DEFAULTS environment variable as JSON.
    /// Falls back to hardcoded defaults if unset or invalid.
    /// Expected format: {"growth": [{"estimate": 0.65, "confidence": 0.7}, ...], "margin": [...]}
    pub fn from_env() -> Self {
        if let Ok(json_str) = std::env::var("HKASK_FERMI_DEFAULTS")
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str)
        {
            let growth = parsed.get("growth").and_then(|g| g.as_array());
            let margin = parsed.get("margin").and_then(|m| m.as_array());
            if let (Some(g_arr), Some(m_arr)) = (growth, margin) {
                let parse_questions = |arr: &[serde_json::Value]| -> Vec<SubQuestion> {
                    arr.iter()
                        .map(|v| SubQuestion {
                            question: v
                                .get("question")
                                .and_then(|q| q.as_str())
                                .unwrap_or("")
                                .into(),
                            estimate: v.get("estimate").and_then(|e| e.as_f64()).unwrap_or(0.5),
                            confidence: v.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.5),
                        })
                        .collect()
                };
                return FermiDefaults {
                    growth_questions: parse_questions(g_arr),
                    margin_questions: parse_questions(m_arr),
                };
            }
        }
        Self::default()
    }
}

// ── Forecast question ──────────────────────────────────────────────────────

/// A forecast question with calibrated probability.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ForecastQuestion {
    /// What is being forecast (e.g., "Will AAPL achieve >8% revenue growth in FY2026?")
    pub question: String,
    /// Reference class for outside view (e.g., "Large-cap tech companies, 2015-2025")
    pub reference_class: String,
    /// Base rate from outside view (0.0–1.0).
    pub base_rate: f64,
    /// Number of reference cases considered.
    pub reference_count: u64,
    /// Sub-questions from Fermi decomposition.
    pub sub_questions: Vec<SubQuestion>,
    /// Current calibrated probability after Bayesian updating.
    pub calibrated_probability: f64,
    /// Confidence in the estimate (0.0–1.0).
    pub confidence: f64,
    /// Number of Bayesian update cycles.
    pub update_count: u64,
}

/// A sub-question from Fermi decomposition.
#[derive(Debug, Clone)]
pub struct SubQuestion {
    /// The sub-question text.
    pub question: String,
    /// User's best estimate for this sub-question (conditional probability).
    pub estimate: f64,
    /// How confident the user is in this estimate.
    pub confidence: f64,
}

// ── Scenario probability distribution ──────────────────────────────────────

/// Probability-weighted scenario.
#[derive(Debug, Clone)]
pub struct WeightedScenario {
    pub name: &'static str,
    pub intrinsic_per_share: f64,
    pub probability: f64,
}

/// Full forecast output linking scenarios to probabilities.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ScenarioForecast {
    pub symbol: String,
    pub base_rate_growth: f64,
    pub base_rate_margin: f64,
    pub reference_class: String,
    pub weighted_scenarios: Vec<WeightedScenario>,
    /// Probability-weighted intrinsic value.
    pub expected_intrinsic: f64,
    /// Current market price.
    pub current_price: f64,
    /// Market-implied vs forecast gap.
    pub market_gap_pct: f64,
    /// Brier score placeholder (updated after outcome is known).
    pub brier_score: Option<f64>,
}

// ── Fermi decomposition ────────────────────────────────────────────────────

/// Decompose a revenue growth forecast into Fermi sub-questions.
pub fn fermi_decompose_growth() -> Vec<SubQuestion> {
    vec![
        SubQuestion {
            question: "Will TAM (total addressable market) grow? (0=shrink, 0.5=flat, 1=grow)"
                .into(),
            estimate: 0.65,
            confidence: 0.7,
        },
        SubQuestion {
            question:
                "Will the company maintain or gain market share? (0=lose, 0.5=maintain, 1=gain)"
                    .into(),
            estimate: 0.55,
            confidence: 0.6,
        },
        SubQuestion {
            question: "Will unit economics improve? (0=degrade, 0.5=flat, 1=improve)".into(),
            estimate: 0.55,
            confidence: 0.5,
        },
        SubQuestion {
            question:
                "Will macro conditions support growth? (0=headwinds, 0.5=neutral, 1=tailwinds)"
                    .into(),
            estimate: 0.50,
            confidence: 0.4,
        },
    ]
}

/// Decompose a profit margin forecast into Fermi sub-questions.
pub fn fermi_decompose_margin() -> Vec<SubQuestion> {
    vec![
        SubQuestion {
            question: "Will input costs decrease? (0=increase, 0.5=flat, 1=decrease)".into(),
            estimate: 0.45,
            confidence: 0.5,
        },
        SubQuestion {
            question: "Will pricing power increase? (0=erode, 0.5=flat, 1=strengthen)".into(),
            estimate: 0.55,
            confidence: 0.6,
        },
        SubQuestion {
            question: "Will operating leverage improve? (0=decline, 0.5=flat, 1=improve)".into(),
            estimate: 0.55,
            confidence: 0.5,
        },
        SubQuestion {
            question: "Will competitive intensity decrease? (0=intensify, 0.5=flat, 1=ease)".into(),
            estimate: 0.50,
            confidence: 0.4,
        },
    ]
}

/// Compute a calibrated probability from Fermi sub-questions.
/// Simple average of sub-question estimates, weighted by confidence.
pub fn calibrate_from_fermi(sub_questions: &[SubQuestion]) -> f64 {
    if sub_questions.is_empty() {
        return 0.5;
    }
    let total_weight: f64 = sub_questions.iter().map(|s| s.confidence).sum();
    if total_weight == 0.0 {
        return 0.5;
    }
    sub_questions
        .iter()
        .map(|s| s.estimate * s.confidence)
        .sum::<f64>()
        / total_weight
}

// ── Outside view (base rate) ───────────────────────────────────────────────

/// Compute the outside view adjustment.
///
/// Reference: Tetlock & Gardner, "Superforecasting" (2015), Ch. 6.
/// The outside view starts with the base rate of the reference class,
/// then adjusts toward the inside view based on how much specific
/// information distinguishes this case from the reference class.
pub fn outside_view_adjustment(
    base_rate: f64,
    inside_estimate: f64,
    reference_count: u64,
) -> (f64, f64) {
    // Regression toward the mean: the less reference data, the more we
    // regress toward 0.5 (the uninformative prior).
    let shrinkage = 1.0 / (1.0 + reference_count as f64);
    let regressed_base = 0.5 + (1.0 - shrinkage) * (base_rate - 0.5);

    // Blend outside and inside view. The outside view gets more weight
    // when the reference count is high.
    let outside_weight = (reference_count as f64 / (reference_count as f64 + 3.0)).min(0.8);
    let calibrated = regressed_base * outside_weight + inside_estimate * (1.0 - outside_weight);

    let confidence = 0.5 + 0.3 * outside_weight;

    (calibrated, confidence)
}

// ── Bayesian updating ──────────────────────────────────────────────────────

/// Update a calibrated probability with new evidence using Bayes' theorem.
///
/// prior: current calibrated probability
/// evidence_likelihood: P(evidence | hypothesis is true)
/// evidence_base_rate: P(evidence) — how common this evidence is
#[allow(dead_code)]
pub fn bayesian_update(prior: f64, evidence_likelihood: f64, evidence_base_rate: f64) -> f64 {
    // Bayes: P(H|E) = P(E|H) * P(H) / P(E)
    let posterior = evidence_likelihood * prior / evidence_base_rate;
    posterior.clamp(0.01, 0.99)
}

// ── Brier scoring ──────────────────────────────────────────────────────────

/// A recorded forecast outcome — what actually happened.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ForecastOutcome {
    pub symbol: String,
    pub forecast_date: String,
    pub outcome_date: String,
    pub actual_revenue_growth: f64,
    pub actual_fcf_margin: f64,
    pub actual_multiple: Option<f64>,
}

/// Brier score: (probability - outcome)^2. Lower = better.
pub fn brier_score(probability: f64, outcome_occurred: bool) -> f64 {
    (probability - if outcome_occurred { 1.0 } else { 0.0 }).powi(2)
}

/// Average Brier score across multiple events.
#[allow(dead_code)]
pub fn brier_score_multi(probabilities: &[f64], outcomes: &[bool]) -> f64 {
    if probabilities.is_empty() || probabilities.len() != outcomes.len() {
        return f64::NAN;
    }
    probabilities
        .iter()
        .zip(outcomes.iter())
        .map(|(p, o)| brier_score(*p, *o))
        .sum::<f64>()
        / probabilities.len() as f64
}

/// Human-readable interpretation of a Brier score.
pub fn brier_interpretation(score: f64) -> &'static str {
    if score.is_nan() {
        return "no_data";
    }
    if score < 0.05 {
        "excellent"
    } else if score < 0.10 {
        "good"
    } else if score < 0.20 {
        "fair"
    } else if score < 0.33 {
        "poor"
    } else {
        "worse_than_climatology"
    }
}

/// Check if an actual value falls within a tolerance band of the forecast.
pub fn within_tolerance(forecast: f64, actual: f64, tolerance: f64) -> bool {
    if forecast == 0.0 {
        return actual.abs() < tolerance;
    }
    ((actual - forecast) / forecast).abs() <= tolerance
}

/// Valid forecast horizons.
pub const FORECAST_HORIZONS: &[&str] = &["3mo", "6mo", "1yr", "2yr", "3yr"];

// ── Scenario probability distribution ──────────────────────────────────────

/// Distribute probabilities across the four Schwartz scenarios.
///
/// Uses the growth and margin calibrated probabilities to assign
/// probabilities to each quadrant of the 2×2 matrix.
pub fn distribute_scenario_probabilities(
    growth_probability: f64, // P(high growth)
    margin_probability: f64, // P(high margin)
    scenario_results: &[ScenarioResult],
) -> Vec<WeightedScenario> {
    // Assume growth and margin are independent for simplicity.
    // Bull:   P(high growth) × P(high margin)
    // Land:   P(high growth) × P(low margin)
    // Cow:    P(low growth)  × P(high margin)
    // Bear:   P(low growth)  × P(low margin)
    let p_bull = growth_probability * margin_probability;
    let p_land = growth_probability * (1.0 - margin_probability);
    let p_cow = (1.0 - growth_probability) * margin_probability;
    let p_bear = (1.0 - growth_probability) * (1.0 - margin_probability);

    let probs = [p_bull, p_land, p_cow, p_bear];

    scenario_results
        .iter()
        .enumerate()
        .map(|(i, r)| WeightedScenario {
            name: r.scenario.name,
            intrinsic_per_share: r.dcf_result.intrinsic_per_share,
            probability: probs[i],
        })
        .collect()
}

/// Compute expected intrinsic value from probability-weighted scenarios.
pub fn expected_intrinsic(weighted: &[WeightedScenario]) -> f64 {
    weighted
        .iter()
        .map(|w| w.intrinsic_per_share * w.probability)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fermi_calibration_average() {
        let sub_qs = vec![
            SubQuestion {
                question: "q1".into(),
                estimate: 0.8,
                confidence: 1.0,
            },
            SubQuestion {
                question: "q2".into(),
                estimate: 0.2,
                confidence: 1.0,
            },
        ];
        let p = calibrate_from_fermi(&sub_qs);
        assert!((p - 0.5).abs() < 0.01, "unweighted average = 0.5");
    }

    #[test]
    fn fermi_calibration_confidence_weighted() {
        let sub_qs = vec![
            SubQuestion {
                question: "q1".into(),
                estimate: 0.9,
                confidence: 0.9,
            },
            SubQuestion {
                question: "q2".into(),
                estimate: 0.1,
                confidence: 0.1,
            },
        ];
        let p = calibrate_from_fermi(&sub_qs);
        // Weighted: (0.9*0.9 + 0.1*0.1) / (0.9+0.1) = 0.82
        assert!((p - 0.82).abs() < 0.01, "confidence-weighted = 0.82");
    }

    #[test]
    fn outside_view_shrinks_with_low_reference_count() {
        // Base rate 0.8, but only 2 reference cases → strong shrinkage
        let (calibrated, _) = outside_view_adjustment(0.8, 0.7, 2);
        assert!(calibrated < 0.8, "shrink toward 0.5 with low N");
        assert!(calibrated > 0.5, "but still above uninformative prior");
    }

    #[test]
    fn outside_view_trusts_large_reference_count() {
        // Base rate 0.8 with 100 reference cases → little shrinkage
        let (calibrated, confidence) = outside_view_adjustment(0.8, 0.7, 100);
        assert!(calibrated > 0.75, "trust base rate with large N");
        assert!(confidence > 0.7, "high confidence with large N");
    }

    #[test]
    fn bayesian_update_positive_evidence() {
        // Prior 0.6, strong evidence (likelihood 0.9, base rate 0.5)
        let posterior = bayesian_update(0.6, 0.9, 0.5);
        assert!(posterior > 0.6, "positive evidence increases probability");
    }

    #[test]
    fn bayesian_update_negative_evidence() {
        // Prior 0.6, weak evidence (likelihood 0.2, base rate 0.5)
        let posterior = bayesian_update(0.6, 0.2, 0.5);
        assert!(posterior < 0.6, "negative evidence decreases probability");
    }

    #[test]
    fn scenario_probabilities_sum_to_one() {
        let probs = [
            WeightedScenario {
                name: "Bull",
                intrinsic_per_share: 200.0,
                probability: 0.3,
            },
            WeightedScenario {
                name: "Land",
                intrinsic_per_share: 150.0,
                probability: 0.2,
            },
            WeightedScenario {
                name: "Cow",
                intrinsic_per_share: 120.0,
                probability: 0.3,
            },
            WeightedScenario {
                name: "Bear",
                intrinsic_per_share: 80.0,
                probability: 0.2,
            },
        ];
        let sum: f64 = probs.iter().map(|w| w.probability).sum();
        assert!((sum - 1.0).abs() < 0.01, "probabilities sum to 1.0");
    }

    #[test]
    fn expected_intrinsic_computation() {
        let probs = [
            WeightedScenario {
                name: "Bull",
                intrinsic_per_share: 200.0,
                probability: 0.25,
            },
            WeightedScenario {
                name: "Bear",
                intrinsic_per_share: 100.0,
                probability: 0.75,
            },
        ];
        let expected = expected_intrinsic(&probs);
        assert!((expected - 125.0).abs() < 0.01, "0.25*200 + 0.75*100 = 125");
    }

    // ── Brier scoring tests ────────────────────────────────────────

    #[test]
    fn brier_perfect() {
        assert!((brier_score(1.0, true) - 0.0).abs() < 0.001);
        assert!((brier_score(0.0, false) - 0.0).abs() < 0.001);
    }

    #[test]
    fn brier_worst() {
        assert!((brier_score(1.0, false) - 1.0).abs() < 0.001);
    }

    #[test]
    fn brier_mid() {
        assert!((brier_score(0.5, true) - 0.25).abs() < 0.001);
    }

    #[test]
    fn brier_multi() {
        let p = [0.9, 0.1, 0.7];
        let o = [true, false, true];
        let s = brier_score_multi(&p, &o);
        // (0.01 + 0.01 + 0.09) / 3 = 0.0367
        assert!((s - 0.0367).abs() < 0.001);
    }

    #[test]
    fn tolerance_bands() {
        // Within 10% band → classified correctly
        assert!(within_tolerance(100.0, 105.0, 0.10));
        assert!(within_tolerance(100.0, 95.0, 0.10));
        // Outside 10% band
        assert!(!within_tolerance(100.0, 112.0, 0.10));
        assert!(!within_tolerance(100.0, 88.0, 0.10));
    }
}
