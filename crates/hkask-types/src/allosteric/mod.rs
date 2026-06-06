//! Allosteric Regulation Logic (ARL) — MWC-regulated decision primitives
//!
//! ARL provides the Monod-Wyman-Changeux equation as a native regulation
//! primitive. Every ARL gate produces a clamped R̄ value (f64),
//! which can be compared against a threshold in the `act` phase.
//!
//! # Module structure
//!
//! - `mwc` — MWC computation engine (state function, sensitivity)
//! - `gate` — `AllostericGate` with temporal dynamics
//!
//! Originally in `hkask-cns::allosteric` — relocated to `hkask-types` because
//! these are cross-loop primitives used by both L5 (CurationConfidenceGate)
//! and L6 (AlgedonicManager). Placing them at the substrate level corrects
//! the authority DAG inversion where L5 depended on L6 for its own regulation
//! primitive.

pub mod gate;
pub mod mwc;

pub use gate::{AllostericGate, AllostericGateConfig};
pub use mwc::{AllostericError, mwc_state_function};
