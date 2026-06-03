//! `Distribution<T>` — Probability distribution type for uncertainty propagation
//!
//! Every ARL gate produces a `Distribution<Decision>` instead of a scalar f64.
//! The distribution is collapsed to a concrete decision in the `act` phase.

/// Trait for types that can be evaluated as R/T states by a distribution.
///
/// Used by `expected_r_bar` to collapse a distribution to a point estimate.
pub trait DecisionLike {
    /// Returns `true` if this value represents the R (proceed) state.
    fn is_r_state(&self) -> bool;
}

/// A probability distribution over values of type T.
///
/// Every ARL gate produces a `Distribution<Decision>` instead of a scalar f64.
/// This preserves uncertainty until the `act` phase collapses it.
#[derive(Debug, Clone)]
pub enum Distribution<T> {
    /// Deterministic: single outcome with probability 1.
    Deterministic(T),

    /// Bernoulli: two outcomes with `r_bar` probability of the R-state outcome.
    /// This is what the MWC equation produces.
    Bernoulli {
        r_outcome: T,
        t_outcome: T,
        r_bar: f64,
    },
}

impl<T: Clone> Distribution<T> {
    /// Wrap a certain value as a deterministic distribution.
    pub fn return_(value: T) -> Self {
        Distribution::Deterministic(value)
    }

    /// Collapse the distribution to a point estimate (R̄).
    ///
    /// Use this ONLY in the `act` phase, when a concrete decision is needed.
    /// Returns the expected probability of the R-state across the distribution.
    pub fn expected_r_bar(&self) -> f64
    where
        T: DecisionLike,
    {
        match self {
            Distribution::Deterministic(v) => {
                if v.is_r_state() {
                    1.0
                } else {
                    0.0
                }
            }
            Distribution::Bernoulli { r_bar, .. } => *r_bar,
        }
    }
}

impl<T: DecisionLike + Clone> Distribution<T> {
    /// Create a Bernoulli distribution from an MWC R̄ value.
    ///
    /// Constructs the two-outcome distribution where R̄ determines the
    /// probability of the R-state (proceed) outcome.
    pub fn from_r_bar(r_outcome: T, t_outcome: T, r_bar: f64) -> Self {
        Distribution::Bernoulli {
            r_outcome,
            t_outcome,
            r_bar: r_bar.clamp(0.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::allosteric::Decision;

    #[test]
    fn deterministic_return_wraps_value() {
        let d: Distribution<Decision> = Distribution::return_(Decision::Proceed);
        assert!((d.expected_r_bar() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn deterministic_t_state_has_zero_r_bar() {
        let d: Distribution<Decision> = Distribution::return_(Decision::Suppress);
        assert!((d.expected_r_bar() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bernoulli_r_bar_matches_input() {
        let d: Distribution<Decision> = Distribution::Bernoulli {
            r_outcome: Decision::Proceed,
            t_outcome: Decision::Suppress,
            r_bar: 0.7,
        };
        assert!((d.expected_r_bar() - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn from_r_bar_clamps_to_valid_range() {
        let d: Distribution<Decision> =
            Distribution::from_r_bar(Decision::Proceed, Decision::Suppress, 1.5);
        assert!((d.expected_r_bar() - 1.0).abs() < f64::EPSILON);

        let d: Distribution<Decision> =
            Distribution::from_r_bar(Decision::Proceed, Decision::Suppress, -0.5);
        assert!((d.expected_r_bar() - 0.0).abs() < f64::EPSILON);
    }
}
