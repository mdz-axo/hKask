//! hKask Federation — CRDT-synced curator federations
//!
//! Three modules:
//! - `crdt`: General-purpose CRDT data structures (OR-Set, LWW-Map, G-Set)
//! - `sync`: FederationSync (sync loop) + FederationLinkManager (lifecycle)
//! - `registry`: FederationRegistry (merged user/agent resolution)

/// Replica identifier — unique per hKask server in the federation.
pub type ReplicaId = String;

pub mod crdt;
pub mod registry;
pub mod sync;
