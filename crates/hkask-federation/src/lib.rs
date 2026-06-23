//! hKask Federation — CRDT-synced curator federations
//!
//! Two modules:
//! - `crdt`: General-purpose CRDT data structures (OR-Set, LWW-Map, G-Set)
//! - `sync`: FederationSync (sync loop) + FederationLinkManager (lifecycle)

pub use hkask_ports::ReplicaId;

pub mod crdt;
pub mod sync;
