//! MWC computation engine — Monod-Wyman-Changeux state function
//!
//! Provides the verified MWC math: state function and sensitivity.
//! All parameters map to measurable operational quantities in the
//! regulation use case.
//!
//! # MWC Equation
//!
//! R̄ = (1+α)ⁿ / ((1+α)ⁿ + L·(1+cα)ⁿ)
//!
//! - L: ratio of T/R decisions in neutral conditions (allosteric constant)
//! - c: sensitivity ratio under R vs T state (affinity ratio, K_R/K_T)
//! - n: number of evidence channels (cooperativity dimensionality)
//! - α: normalized deficit/deviation (ligand concentration analog)

/// Errors from MWC computation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AllostericError {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Numerical overflow in MWC computation: {0}")]
    Overflow(String),
}

/// MWC state function: R̄ = (1+α)ⁿ / ((1+α)ⁿ + L·(1+cα)ⁿ)
///
/// Returns the probability of the R (relaxed/proceed) state given MWC parameters.
/// R̄ ∈ [0, 1]: 0 = fully T (suppress), 1 = fully R (proceed).
///
/// # Arguments
///
/// * `l` — Allosteric constant: ratio of T/R at equilibrium (L > 0).
///   Large L = strong T preference (skepticism). Countable from decision logs.
/// * `c` — Affinity ratio: K_R/K_T (0 < c ≤ 1). Observable from response curves.
///   Small c = large cooperativity.
/// * `n` — Number of binding sites (evidence channels). Determined by architecture.
/// * `alpha` — Normalized deficit/deviation (α ≥ 0). Read from Signal values.
///
/// # Errors
///
/// Returns `AllostericError::InvalidParameter` if L ≤ 0 or c ≤ 0.
/// Returns `AllostericError::Overflow` if the computation overflows f64.
pub fn mwc_state_function(l: f64, c: f64, n: u32, alpha: f64) -> Result<f64, AllostericError> {
    if l <= 0.0 {
        return Err(AllostericError::InvalidParameter(format!(
            "L must be positive, got {l}"
        )));
    }
    if c <= 0.0 {
        return Err(AllostericError::InvalidParameter(format!(
            "c must be positive, got {c}"
        )));
    }
    if alpha < 0.0 {
        return Err(AllostericError::InvalidParameter(format!(
            "α must be non-negative, got {alpha}"
        )));
    }

    let alpha_plus_one = 1.0 + alpha;
    let c_alpha_plus_one = 1.0 + c * alpha;

    let numerator = alpha_plus_one.powi(n as i32);
    let denominator_term_t = l * c_alpha_plus_one.powi(n as i32);
    let denominator = numerator + denominator_term_t;

    if !denominator.is_finite() || denominator == 0.0 {
        return Err(AllostericError::Overflow(format!(
            "denominator overflow: num={numerator}, L_term={denominator_term_t}"
        )));
    }

    let r_bar = numerator / denominator;

    if !r_bar.is_finite() {
        return Err(AllostericError::Overflow(format!(
            "R̄ is not finite: {r_bar}"
        )));
    }

    Ok(r_bar.clamp(0.0, 1.0))
}

/// Sensitivity: ∂R̄/∂α at current α.
///
/// Measures how responsive the gate is to changes in the input signal.
/// High sensitivity means small changes in α produce large changes in R̄.
///
/// Computed analytically from the MWC equation:
/// ∂R̄/∂α = R̄ · (1-R̄) · n · (1/(1+α) - c/(1+cα))
///
/// Dead code: no production consumer yet. Retained because it has test coverage
/// and will be needed when the Curation Loop's sensitivity analysis path is wired
/// (Task 8: 4-stage cycle for all loops).
#[allow(dead_code)]
pub fn mwc_sensitivity(l: f64, c: f64, n: u32, alpha: f64) -> f64 {
    let r_bar = match mwc_state_function(l, c, n, alpha) {
        Ok(r) if r > 0.0 && r < 1.0 => r,
        _ => return 0.0, // Edge cases: sensitivity is zero at boundaries
    };

    let alpha_plus_one = 1.0 + alpha;
    let c_alpha_plus_one = 1.0 + c * alpha;

    let term = (1.0 / alpha_plus_one) - (c / c_alpha_plus_one);
    r_bar * (1.0 - r_bar) * n as f64 * term
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mwc_zero_alpha_gives_t_state_preference() {
        // α=0: R̄ = 1/(1+L). With L=1000, R̄ ≈ 0.001
        let r_bar = mwc_state_function(1000.0, 0.01, 3, 0.0).unwrap();
        assert!((r_bar - (1.0 / 1001.0)).abs() < 1e-10);
    }

    #[test]
    fn mwc_high_alpha_gives_r_state_preference() {
        // Large α: R̄ → 1
        let r_bar = mwc_state_function(1000.0, 0.01, 3, 1000.0).unwrap();
        assert!(r_bar > 0.99);
    }

    #[test]
    fn mwc_l_equals_1_gives_half_activation_at_zero() {
        // L=1: R̄ at α=0 is 0.5 (equal T and R)
        let r_bar = mwc_state_function(1.0, 0.5, 1, 0.0).unwrap();
        assert!((r_bar - 0.5).abs() < 1e-10);
    }

    #[test]
    fn mwc_rejects_invalid_l() {
        assert!(mwc_state_function(0.0, 0.5, 1, 1.0).is_err());
        assert!(mwc_state_function(-1.0, 0.5, 1, 1.0).is_err());
    }

    #[test]
    fn mwc_rejects_invalid_c() {
        assert!(mwc_state_function(100.0, 0.0, 1, 1.0).is_err());
        assert!(mwc_state_function(100.0, -0.1, 1, 1.0).is_err());
    }

    #[test]
    fn mwc_rejects_negative_alpha() {
        assert!(mwc_state_function(100.0, 0.1, 3, -1.0).is_err());
    }

    #[test]
    fn sensitivity_positive_for_cooperative_system() {
        // c < 1 means positive cooperativity: ∂R̄/∂α should be positive in transition zone
        let s = mwc_sensitivity(1000.0, 0.01, 6, 1.0);
        assert!(s > 0.0, "Expected positive sensitivity, got {s}");
    }

    #[test]
    fn sensitivity_zero_at_boundaries() {
        // At extreme α values, sensitivity approaches zero
        let s_low = mwc_sensitivity(1000.0, 0.01, 6, 0.0);
        // At α=0 with large L, R̄ ≈ 0 so sensitivity is small
        assert!(
            s_low.abs() < 0.1,
            "Expected small sensitivity at α=0, got {s_low}"
        );
    }

    #[test]
    fn mwc_backward_compatible_binary_threshold() {
        // In the limit of high cooperativity (small c, large n), the MWC sigmoid
        // approaches a step function. This is the backward-compatible case:
        // existing binary behavior is the limit of the smooth sigmoid.
        // With L=1000, c=0.1, n=6: the sigmoid transitions between α=2 and α=5
        let r_below = mwc_state_function(1000.0, 0.1, 6, 2.0).unwrap();
        let r_above = mwc_state_function(1000.0, 0.1, 6, 5.0).unwrap();
        assert!(
            r_below < 0.3,
            "Below transition zone should be mostly T, got R̄={r_below}"
        );
        assert!(
            r_above > 0.7,
            "Above transition zone should be mostly R, got R̄={r_above}"
        );
        // The transition is steeper for larger n at the half-saturation point.
        // For n=1 (no cooperativity), the sigmoid is gradual:
        // R̄ goes from 0.001 to 0.999 over a 1000x range in α.
        // For n=6, it goes from 0.1 to 0.9 over a ~2.5x range.
        let r_n1_below = mwc_state_function(1000.0, 0.1, 1, 2.0).unwrap();
        let r_n1_above = mwc_state_function(1000.0, 0.1, 1, 5.0).unwrap();
        let n1_range = r_n1_above - r_n1_below;
        let n6_range = r_above - r_below;
        assert!(
            n6_range > n1_range,
            "n=6 should have sharper transition than n=1: n6_range={n6_range}, n1_range={n1_range}"
        );
    }
}
