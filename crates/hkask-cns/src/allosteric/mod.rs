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
//
//!   NOTE: `curation` module relocated to `hkask_agents::curator::curation_gate`
//!   (Loop 5 types live in the Curation crate, not Cybernetics).

pub mod distribution;
pub mod gate;
pub mod mwc;

pub use distribution::BernoulliDistribution;
pub use gate::{AllostericGate, AllostericGateConfig};
pub use mwc::{AllostericError, mwc_state_function};
