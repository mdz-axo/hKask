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
