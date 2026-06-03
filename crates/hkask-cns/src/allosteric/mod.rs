//! Allosteric Regulation Logic (ARL) — MWC-regulated decision primitives
//!
//! ARL provides the Monod-Wyman-Changeux equation as a native regulation
//! primitive inside the CNS. Every ARL gate produces a `Distribution<Decision>`
//! that can be collapsed to a point estimate in the `act` phase.
//!
//! # Module structure
//!
//! - `distribution` — `Distribution<T>` type for uncertainty propagation
//! - `mwc` — MWC computation engine (state function, sensitivity)
//! - `gate` — `AllostericGate` with temporal dynamics
//! - `curation` — Curation confidence gate (IP-3)

pub mod curation;
pub mod distribution;
pub mod gate;
pub mod mwc;

pub use curation::{CurationConfidenceGate, CurationDecision, CurationPort};
pub use distribution::{DecisionLike, Distribution};
pub use gate::{AllostericGate, AllostericGateConfig};
pub use mwc::{AllostericError, mwc_sensitivity, mwc_state_function};

/// Decision outcome from an allosteric gate.
///
/// The two-state MWC model: T (tense/low-affinity) and R (relaxed/high-affinity).
/// In regulation terms: T = suppress/regulate, R = proceed/activate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Decision {
    /// T-state: suppress, regulate, withhold action
    Suppress,
    /// R-state: proceed, activate, allow action
    Proceed,
}

impl DecisionLike for Decision {
    fn is_r_state(&self) -> bool {
        matches!(self, Decision::Proceed)
    }
}
