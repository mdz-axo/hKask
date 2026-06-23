//! hKask Federation — CRDT-synced curator federations
//!
//! Two modules:
//! - `crdt`: General-purpose CRDT data structures (OR-Set, LWW-Map, G-Set)
//! - `sync`: FederationSync (sync loop) + FederationLinkManager (lifecycle)

/// Replica identifier — unique per hKask server in the federation.
pub type ReplicaId = String;

pub mod crdt;
pub mod sync;
