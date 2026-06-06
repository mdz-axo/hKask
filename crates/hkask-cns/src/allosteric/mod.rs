//! Allosteric Regulation Logic (ARL) — MWC-regulated decision primitives
//!
//! **Relocated to `hkask_types::allosteric`.** This module re-exports the types
//! from the substrate crate.
//!
//! ARL provides the Monod-Wyman-Changeux equation as a native regulation
//! primitive inside the CNS. Every ARL gate produces a clamped R̄ value (f64),
//! which can be compared against a threshold in the `act` phase.
//!
//! # Module structure
//!
//! - `mwc` — MWC computation engine (state function, sensitivity)
//! - `gate` — `AllostericGate` with temporal dynamics
//
//!   NOTE: `curation` module relocated to `hkask_agents::curator::curation_gate`
//!   (Loop 5 types live in the Curation crate, not Cybernetics).

// Re-export from hkask-types (canonical location since v0.22.0 authority DAG fix)
pub use hkask_types::allosteric::gate;
pub use hkask_types::allosteric::mwc;

pub use gate::{AllostericGate, AllostericGateConfig};
pub use mwc::{AllostericError, mwc_state_function};
