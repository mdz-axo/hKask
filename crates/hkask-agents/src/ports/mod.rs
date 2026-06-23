//! Hexagonal Ports (Traits)
//!
//! Port definitions for hexagonal architecture.
//! All port traits live here so that domain code depends only on
//! these abstractions, never on concrete adapters.

pub mod mcp_runtime;
pub mod memory_storage;

pub use crate::types::audit::{AuditEntry, AuditOutcome};
pub use mcp_runtime::MCPRuntimePort;
pub use memory_storage::{
    EpisodicStoragePort, RecallRequest, RecalledEpisode, RecalledSemantic, SemanticStoragePort,
    StorageRequest,
};
