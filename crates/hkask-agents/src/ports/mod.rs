//! Hexagonal Ports (Traits)
//!
//! Port definitions for hexagonal architecture.
//! All port traits live here so that domain code depends only on
//! these abstractions, never on concrete adapters.

pub mod a2a;
pub mod mcp_runtime;
pub mod memory_storage;
pub mod registry_source;

pub use crate::types::audit::{AuditEntry, AuditOutcome};
pub use a2a::A2APort;
pub use hkask_mcp::TemplateCrateLoader;
pub use mcp_runtime::MCPRuntimePort;
pub use memory_storage::{
    EpisodicStoragePort, RecallRequest, RecalledEpisode, RecalledSemantic, SemanticStoragePort,
    StorageRequest,
};
pub use registry_source::RegistrySourcePort;
