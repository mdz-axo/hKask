//! Hexagonal Ports (Traits)
//!
//! Port definitions for hexagonal architecture.
//! All port traits live here so that domain code depends only on
//! these abstractions, never on concrete adapters.

pub mod acp;
pub mod audit_log;
pub mod git_cas;
pub mod mcp_runtime;
pub mod memory_storage;
pub mod registry_source;
pub mod standing_session;

pub use acp::AcpPort;
pub use audit_log::{AuditEntry, AuditOutcome};
pub use git_cas::GitCasAdapter;
pub use mcp_runtime::MCPRuntimePort;
pub use memory_storage::{EpisodicStoragePort, SemanticStoragePort};

pub use registry_source::RegistrySourcePort;
pub use standing_session::{MessageRecord, SessionRecord, StandingSessionPortError};
