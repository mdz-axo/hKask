//! Hexagonal Ports (Traits)
//!
//! Port definitions for hexagonal architecture.
//! All port traits live here so that domain code depends only on
//! these abstractions, never on concrete adapters.
//!
//! # Port Location Rule (ADR-042)
//!
//! Port traits live in the domain crate that first consumes them.
//! When a second consumer needs the trait, it is promoted to the
//! domain crate nearest to both consumers.
//!
//! - `EpisodicStoragePort` / `SemanticStoragePort` → promoted to `hkask-memory`
//!   (two consumers: `hkask-agents` + `hkask-services-context`)

pub mod memory_storage;

pub use crate::types::audit::{AuditEntry, AuditOutcome};
pub use hkask_ports::consent_port::ConsentPort;
pub use hkask_ports::escalation::{
    EscalationBatch, EscalationEntry, EscalationPort, EscalationStatus,
};
pub use hkask_ports::registry_port::RegistryPort;

pub use memory_storage::{
    EpisodicStoragePort, MemoryPortError, RecallRequest, RecalledEpisode, RecalledSemantic,
    SemanticStoragePort, StorageRequest,
};
