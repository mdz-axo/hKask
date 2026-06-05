//! Allosteric Regulation Logic (ARL) — MWC-regulated decision primitives
//!
//! ARL provides the Monod-Wyman-Changeux equation as a native regulation
//! primitive. Every ARL gate produces a `BernoulliDistribution`
//! parameterized by R̄, which can be collapsed to a point estimate in the `act` phase.
//!
//! # Module structure
//!
//! - `distribution` — `BernoulliDistribution` type for MWC gate output
//! - `mwc` — MWC computation engine (state function, sensitivity)
//! - `gate` — `AllostericGate` with temporal dynamics
//!
//! Originally in `hkask-cns::allosteric` — relocated to `hkask-types` because
//! these are cross-loop primitives used by both L5 (CurationConfidenceGate)
//! and L6 (AlgedonicManager). Placing them at the substrate level corrects
//! the authority DAG inversion where L5 depended on L6 for its own regulation
//! primitive.

pub mod distribution;
pub mod gate;
pub mod mwc;

pub use distribution::BernoulliDistribution;
pub use gate::{AllostericGate, AllostericGateConfig};
pub use mwc::{AllostericError, mwc_state_function};
