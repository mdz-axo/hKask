//! Curation confidence gate — metacognitive evaluation of curation decisions.
//!
//! Module moved to `hkask-cns::allosteric`. These types remain as compile-time
//! stubs pending migration of the CurationLoop's confidence gate to the CNS
//! allosteric gate infrastructure (OPEN_QUESTIONS.md §2.2).

/// Outcome of a curation confidence evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfidenceDecision {
    /// More evidence needed — escalate to Curator.
    SeekMoreEvidence,
    /// Evidence sufficient — proceed automatically.
    Proceed,
    /// Confidence too low — suppress action.
    Suppress,
}

/// Curation confidence gate — evaluates R̄-bar thresholds for curation decisions.
///
/// This is a stub pending migration to `hkask-cns::allosteric::AllostericGate`.
/// The gate currently applies a simple R̄-bar threshold: if the R̄-bar signal
/// for a given channel is in the transition zone (0.3 < R̄ < 0.8), it emits
/// `SeekMoreEvidence`; otherwise, it proceeds.
#[derive(Debug, Clone)]
pub struct CurationConfidenceGate {
    /// Lower threshold — below this, suppress.
    lower: f64,
    /// Upper threshold — above this, proceed confidently.
    upper: f64,
    /// Last computed R̄-bar value.
    last_r_bar: f64,
}

impl Default for CurationConfidenceGate {
    fn default() -> Self {
        Self {
            lower: 0.3,
            upper: 0.8,
            last_r_bar: 0.5,
        }
    }
}

impl CurationConfidenceGate {
    /// Evaluate the gate using the internally-tracked R̄-bar value.
    /// Returns the decision; the R̄-bar value is available via `confidence()`.
    pub fn decide(&mut self) -> ConfidenceDecision {
        if self.last_r_bar < self.lower {
            ConfidenceDecision::Suppress
        } else if self.last_r_bar < self.upper {
            ConfidenceDecision::SeekMoreEvidence
        } else {
            ConfidenceDecision::Proceed
        }
    }

    /// Return the last computed R̄-bar confidence value (0.0–1.0).
    pub fn confidence(&self) -> f64 {
        self.last_r_bar
    }

    /// Sensitivity analysis of each channel — returns (channel_name, sensitivity).
    /// The allosteric gate model (MWC sigmoid) is not yet implemented; this stub
    /// returns an empty analysis.
    pub fn sensitivity_analysis(&self) -> Vec<(String, f64)> {
        vec![]
    }
}
