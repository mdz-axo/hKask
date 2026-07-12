//! CRDT data structures for federation convergence.
//!
//! General-purpose, no hKask-specific dependencies beyond `ReplicaId`.

pub mod dot;
pub mod g_set;
pub mod hmem_key;
pub mod lww_map;
pub mod or_set;
pub mod version_vector;

pub use dot::Dot;
pub use g_set::GSet;
pub use hmem_key::FederationHMemKey;
pub use lww_map::LWWMap;
pub use or_set::ORSet;
pub use version_vector::VersionVector;
