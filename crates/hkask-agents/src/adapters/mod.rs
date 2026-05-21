//! Adapter implementations for hexagonal ports
//!
//! This module provides concrete implementations of the port traits
//! defined in the parent module, enabling real-world integration with:
//! - ACP runtime (acp-runtime crate)
//! - MCP runtime (rmcp crate)
//! - CNS emitter (hkask-cns crate)
//! - Git CAS (gix crate)
//! - Keystore (hkask-keystore crate)
//! - Memory storage (hkask-storage crate)

pub mod acp_runtime;
pub mod cns_emitter;
pub mod git_cas;
pub mod keystore_port;
pub mod mcp_runtime;
pub mod memory_storage;

pub use acp_runtime::AcpRuntimeAdapter;
pub use cns_emitter::CnsEmitterAdapter;
pub use git_cas::{GitCasAdapter, MockGitCas};
pub use keystore_port::{KeystorePort, Secret};
pub use mcp_runtime::McpRuntimeAdapter;
pub use memory_storage::MemoryStorageAdapter;
