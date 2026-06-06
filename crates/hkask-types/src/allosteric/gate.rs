//! `AllostericGate` — MWC-regulated decision point with temporal dynamics
//!
//! An allosteric gate is an MWC-regulated decision point in the 6-loop system.
//! It produces a clamped R̄ value (f64), preserving the regulation signal
//! through the pipeline.

use crate::allosteric::mwc::mwc_state_function;
use std::time::Duration;

/// Configuration for an `AllostericGate`.
///
/// All parameters are MEASURABLE OPERATIONAL QUANTITIES (not analyst encodings).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AllostericGateConfig {
    /// Gate name (for identification and coupling references).
    pub name: String,
    /// L: ratio of T/R decisions in neutral conditions (countable from logs).
    /// Large L = strong T preference (skepticism). Default: 1000.
    pub base_l: f64,
    /// c: sensitivity ratio under R vs T state (observable from response curve).
    /// Small c = large cooperativity. Range: (0, 1]. Default: 0.01.
    pub c: f64,
    /// n: number of evidence channels (determined by architecture). Default: 3.
    pub n: usize,
    /// Decision threshold: R̄ above this means Proceed. Default: 0.5.
    pub threshold: f64,
    /// τ: relaxation time — how fast the gate settles toward equilibrium.
    /// Governs the temporal dynamics. Default: 1 second.
    pub tau: Duration,
    /// Hysteresis: L adjustment from previous R̄. Positive value means
    /// the gate resists switching (adds inertia). Default: 0.0.
    pub hysteresis: f64,
}

impl Default for AllostericGateConfig {
    fn default() -> Self {
        Self {
            name: String::from("unnamed_gate"),
            base_l: 1000.0,
            c: 0.01,
            n: 3,
            threshold: 0.5,
            tau: Duration::from_secs(1),
            hysteresis: 0.0,
        }
    }
}

/// An allosteric gate — an MWC-regulated decision point in the 6-loop system.
///
/// Parameters are MEASURABLE OPERATIONAL QUANTITIES (not analyst encodings):
/// - L: ratio of T/R decisions in neutral conditions (countable from logs)
/// - c: sensitivity ratio under R vs T state (observable from response curve)
/// - n: number of evidence channels (determined by architecture)
/// - α: normalized deficit/deviation (read from Signal values)
/// - τ: relaxation time (how fast the gate settles)
/// - hysteresis: L adjustment from previous R̄
pub struct AllostericGate {
    /// Gate name (for identification and coupling references).
    pub name: String,
    /// Base allosteric constant L (before hysteresis adjustment).
    pub base_l: f64,
    /// Sensitivity ratio c = K_R/K_T.
    pub c: f64,
    /// Number of evidence channels (cooperativity dimensionality).
    pub n: usize,
    /// Normalized deficit/deviation (α ≥ 0, read from Signal values).
    pub alpha: f64,
    /// Decision threshold for R̄.
    pub threshold: f64,
    /// Relaxation time constant τ.
    pub tau: Duration,
    /// Hysteresis strength: how much the previous state biases L.
    pub hysteresis: f64,
    /// Previous R̄ value (for hysteresis computation).
    pub prev_r_bar: f64,
}

impl AllostericGate {
    /// Create a new allosteric gate from configuration.
    pub fn new(config: &AllostericGateConfig) -> Self {
        Self {
            name: config.name.clone(),
            base_l: config.base_l,
            c: config.c,
            n: config.n,
            alpha: 0.0,
            threshold: config.threshold,
            tau: config.tau,
            hysteresis: config.hysteresis,
            prev_r_bar: 0.0,
        }
    }

    /// Create a gate with specific parameters (convenience constructor).
    pub fn with_params(name: &str, base_l: f64, c: f64, n: usize, threshold: f64) -> Self {
        Self {
            name: name.to_string(),
            base_l,
            c,
            n,
            alpha: 0.0,
            threshold,
            tau: Duration::from_secs(1),
            hysteresis: 0.0,
            prev_r_bar: 0.0,
        }
    }

    /// Compute effective L including hysteresis from previous state.
    ///
    /// Hysteresis adds inertia to the gate: if the previous R̄ was high (R state),
    /// effective L is decreased (making R state more likely). If previous R̄
    /// was low (T state), effective L is increased (making T state more likely).
    ///
    /// effective_L = base_L * exp(hysteresis * (0.5 - prev_R̄))
    ///
    /// When hysteresis = 0, effective_L = base_L (no memory).
    /// When prev_R̄ = 0.5, effective_L = base_L (neutral).
    pub fn effective_l(&self) -> f64 {
        if self.hysteresis == 0.0 {
            return self.base_l;
        }
        self.base_l * (self.hysteresis * (0.5 - self.prev_r_bar)).exp()
    }

    /// Compute R̄ at equilibrium using the MWC state function.
    ///
    /// R̄ = (1+α)ⁿ / ((1+α)ⁿ + L_eff·(1+cα)ⁿ)
    pub fn r_bar_eq(&self) -> f64 {
        let l_eff = self.effective_l();
        mwc_state_function(l_eff, self.c, self.n as u32, self.alpha).unwrap_or(0.0)
    }

    /// Compute R̄ after a time step dt, with relaxation toward equilibrium.
    ///
    /// R̄(t+dt) = R̄(t) + (R̄_eq - R̄(t)) * (1 - exp(-dt/τ))
    ///
    /// This gives the gate temporal dynamics: it doesn't jump to equilibrium
    /// instantly but relaxes with time constant τ. Large τ = slow response,
    /// small τ = fast response.
    pub fn r_bar_at(&mut self, dt: Duration) -> f64 {
        let r_bar_eq = self.r_bar_eq();
        let tau_secs = self.tau.as_secs_f64().max(f64::EPSILON);
        let dt_secs = dt.as_secs_f64();

        let relaxation = 1.0 - (-dt_secs / tau_secs).exp();
        let r_bar_new = self.prev_r_bar + (r_bar_eq - self.prev_r_bar) * relaxation;

        // Update hysteresis state
        self.prev_r_bar = r_bar_new;
        r_bar_new
    }

    /// Produce the gate's R̄ value from the current state.
    ///
    /// Returns the clamped R̄ value that can be compared against the threshold
    /// to produce a concrete decision.
    pub fn decide(&self) -> f64 {
        self.r_bar_eq().clamp(0.0, 1.0)
    }

    /// Produce R̄ with temporal relaxation over dt.
    ///
    /// Combines `r_bar_at(dt)` with clamping. Updates the
    /// gate's hysteresis state as a side effect.
    pub fn decide_at(&mut self, dt: Duration) -> f64 {
        self.r_bar_at(dt).clamp(0.0, 1.0)
    }

    /// Set the input signal (α) from a normalized deficit/deviation value.
    ///
    /// α is typically computed as: deficit / threshold or
    /// |signal.value - signal.set_point| / signal.set_point.
    pub fn set_alpha(&mut self, alpha: f64) {
        self.alpha = alpha.max(0.0);
    }

    /// Reset the gate to its initial state (clearing hysteresis).
    pub fn reset(&mut self) {
        self.alpha = 0.0;
        self.prev_r_bar = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_default_config_creates_valid_gate() {
        let config = AllostericGateConfig::default();
        let gate = AllostericGate::new(&config);
        assert_eq!(gate.name, "unnamed_gate");
        assert!((gate.effective_l() - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn gate_low_alpha_produces_suppress() {
        let mut gate = AllostericGate::with_params("test", 1000.0, 0.01, 3, 0.5);
        gate.set_alpha(0.0);
        let r_bar = gate.r_bar_eq();
        assert!(r_bar < 0.01, "Low α should give T-state, got R̄={r_bar}");
    }

    #[test]
    fn gate_high_alpha_produces_proceed() {
        let mut gate = AllostericGate::with_params("test", 1000.0, 0.01, 3, 0.5);
        // For L=1000, c=0.01, n=3: half-saturation is around α≈10.
        // α=100 gives R̄ close to 1.
        gate.set_alpha(100.0);
        let r_bar = gate.r_bar_eq();
        assert!(r_bar > 0.9, "High α should give R-state, got R̄={r_bar}");
    }

    #[test]
    fn gate_hysteresis_biases_effective_l() {
        let mut gate = AllostericGate::with_params("test", 1000.0, 0.01, 3, 0.5);
        gate.hysteresis = 2.0;
        gate.prev_r_bar = 0.8; // Previously in R state
        let l_eff = gate.effective_l();
        // When prev_R̄ > 0.5, effective L < base_L (R state bias)
        assert!(
            l_eff < 1000.0,
            "Hysteresis should decrease L when prev in R-state, got L_eff={l_eff}"
        );
    }

    #[test]
    fn gate_no_hysteresis_effective_l_equals_base() {
        let gate = AllostericGate::with_params("test", 1000.0, 0.01, 3, 0.5);
        assert!((gate.effective_l() - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn gate_relaxation_converges_to_equilibrium() {
        let mut gate = AllostericGate::with_params("test", 1000.0, 0.01, 3, 0.5);
        gate.set_alpha(1.0);
        gate.prev_r_bar = 0.0; // Start far from equilibrium

        let r_bar_eq = gate.r_bar_eq();

        // Many small time steps should converge to equilibrium
        for _ in 0..100 {
            gate.r_bar_at(Duration::from_millis(100));
        }

        let final_r_bar = gate.prev_r_bar;
        assert!(
            (final_r_bar - r_bar_eq).abs() < 0.01,
            "Should converge to equilibrium, got {final_r_bar} vs {r_bar_eq}"
        );
    }

    #[test]
    fn gate_decide_produces_clamped_r_bar() {
        let mut gate = AllostericGate::with_params("test", 100.0, 0.1, 3, 0.5);
        gate.set_alpha(1.0);
        let r_bar = gate.decide();
        assert!(
            r_bar > 0.0 && r_bar < 1.0,
            "R̄ should be in (0,1), got {r_bar}"
        );
    }

    #[test]
    fn gate_set_alpha_clamps_negative() {
        let mut gate = AllostericGate::with_params("test", 1000.0, 0.01, 3, 0.5);
        gate.set_alpha(-5.0);
        assert!(
            (gate.alpha - 0.0).abs() < f64::EPSILON,
            "α should be clamped to 0"
        );
    }

    #[test]
    fn gate_reset_clears_state() {
        let mut gate = AllostericGate::with_params("test", 1000.0, 0.01, 3, 0.5);
        gate.set_alpha(5.0);
        gate.prev_r_bar = 0.8;
        gate.reset();
        assert!((gate.alpha - 0.0).abs() < f64::EPSILON);
        assert!((gate.prev_r_bar - 0.0).abs() < f64::EPSILON);
    }
}
