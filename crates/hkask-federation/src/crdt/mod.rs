//! CRDT data structures for federation convergence.
//!
//! General-purpose, no hKask-specific dependencies beyond `ReplicaId`.

pub mod dot;
pub mod g_set;
pub mod lww_map;
pub mod or_set;
pub mod triple_key;
pub mod version_vector;
