//! `BernoulliDistribution` — Bernoulli distribution for MWC gate output
//!
//! Every ARL gate produces a `BernoulliDistribution` parameterized by R̄.
//! The distribution is collapsed to a concrete decision in the `act` phase.

/// A Bernoulli distribution over R/T states, parameterized by R̄.
///
/// This is what the MWC equation produces: a single probability value
/// representing the equilibrium fraction of gates in the R (proceed) state.
/// The `act` phase compares R̄ against a threshold to produce a concrete `Decision`.
#[derive(Debug, Clone, Copy)]
pub struct BernoulliDistribution {
    /// Probability of the R-state (proceed) outcome, clamped to [0, 1].
    r_bar: f64,
}

impl BernoulliDistribution {
    /// Create a Bernoulli distribution from an MWC R̄ value.
    ///
    /// R̄ is clamped to [0, 1].
    pub fn from_r_bar(r_bar: f64) -> Self {
        Self {
            r_bar: r_bar.clamp(0.0, 1.0),
        }
    }

    /// Collapse the distribution to a point estimate (R̄).
    ///
    /// Use this ONLY in the `act` phase, when a concrete decision is needed.
    /// Returns the probability of the R-state outcome.
    pub fn expected_r_bar(&self) -> f64 {
        self.r_bar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_r_bar_clamps_to_valid_range() {
        let d = BernoulliDistribution::from_r_bar(1.5);
        assert!((d.expected_r_bar() - 1.0).abs() < f64::EPSILON);

        let d = BernoulliDistribution::from_r_bar(-0.5);
        assert!((d.expected_r_bar() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bernoulli_r_bar_matches_input() {
        let d = BernoulliDistribution::from_r_bar(0.7);
        assert!((d.expected_r_bar() - 0.7).abs() < f64::EPSILON);
    }
}
