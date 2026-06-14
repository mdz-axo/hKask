//! Hexagonal Ports (Traits)
//!
//! Port definitions for hexagonal architecture.
//! All port traits live here so that domain code depends only on
//! these abstractions, never on concrete adapters.

pub mod acp;
pub mod mcp_runtime;
pub mod memory_storage;
pub mod registry_source;

pub use acp::AcpPort;
pub use hkask_mcp::GitCasAdapter;
pub use hkask_types::audit::{AuditEntry, AuditOutcome};
pub use mcp_runtime::MCPRuntimePort;
pub use memory_storage::{
    EpisodicStoragePort, RecallRequest, RecalledEpisode, RecalledSemantic, SemanticStoragePort,
    StorageRequest,
};
pub use registry_source::RegistrySourcePort;
