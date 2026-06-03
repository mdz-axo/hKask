//! Allosteric Regulation Logic (ARL) — MWC-regulated decision primitives
//!
//! ARL provides the Monod-Wyman-Changeux equation as a native regulation
//! primitive inside the CNS. Every ARL gate produces a `BernoulliDistribution`
//! parameterized by R̄, which can be collapsed to a point estimate in the `act` phase.
//!
//! # Module structure
//!
//! - `distribution` — `BernoulliDistribution` type for MWC gate output
//! - `mwc` — MWC computation engine (state function, sensitivity)
//! - `gate` — `AllostericGate` with temporal dynamics
//! - `curation` — Curation confidence gate (IP-3)

pub mod curation;
pub mod distribution;
pub mod gate;
pub mod mwc;

pub(crate) use curation::CurationPort;
pub use curation::{CurationConfidenceGate, CurationDecision};
pub(crate) use distribution::BernoulliDistribution;
pub(crate) use gate::{AllostericGate, AllostericGateConfig};
pub(crate) use mwc::{AllostericError, mwc_sensitivity, mwc_state_function};
