//! hKask Federation — CRDT-synced curator federations
//!
//! Two modules:
//! - `crdt`: General-purpose CRDT data structures (OR-Set, LWW-Map, G-Set)
//! - `sync`: FederationSync (sync loop) + FederationLinkManager (lifecycle)

// Used via derive macros (serde/thiserror/async_trait) — invisible to unused_crate_dependencies lint
#![allow(unused_crate_dependencies)]

pub use hkask_ports::ReplicaId;

pub mod reg_span;
pub mod crdt;
pub mod service;
pub mod sync;
